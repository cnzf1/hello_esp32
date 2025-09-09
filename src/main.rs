mod audio;
mod multinet;
mod ui;
use crate::ui::display;
use audio::{AFE, SAMPLE_RATE};
use esp_idf_svc::{
    hal::{
        gpio::AnyIOPin,
        i2s::{config, I2sDriver, I2S0, I2S1},
    },
    io::Read,
};
use std::time::Duration;

static WAV_DATA: &[u8] = include_bytes!("../assets/hello_16.wav");
static PLAYING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn record(i2s: I2S0, ws: AnyIOPin, sck: AnyIOPin, din: AnyIOPin, mclk: Option<AnyIOPin>, afe: AFE) {
    let i2s_config = config::StdConfig::new(
        config::Config::default().auto_clear(true),
        config::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        config::StdSlotConfig::philips_slot_default(
            config::DataBitWidth::Bits16,
            config::SlotMode::Mono,
        ),
        config::StdGpioConfig::default(),
    );

    let mut rx_driver = I2sDriver::new_std_rx(i2s, &i2s_config, sck, din, mclk, ws).unwrap();

    debug("start recording...");
    rx_driver.rx_enable().unwrap();

    let bytes_per_feed = afe.feed_chunksize * 2;
    loop {
        if PLAYING.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        // feed the AFE with data
        let mut tmp_buffer = vec![0u8; bytes_per_feed];
        rx_driver.read_exact(&mut tmp_buffer).unwrap();
        if tmp_buffer.is_empty() {
            log::warn!("read zero");
            break;
        }
        afe.feed(&tmp_buffer);
    }

    // rx_driver.rx_disable().unwrap();
}

fn play(
    i2s1: I2S1,
    bclk: AnyIOPin,
    dout: AnyIOPin,
    lrclk: AnyIOPin,
    mclk2: Option<AnyIOPin>,
    afe: AFE,
) {
    let i2s_config = config::StdConfig::new(
        config::Config::default().auto_clear(true),
        config::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        config::StdSlotConfig::philips_slot_default(
            config::DataBitWidth::Bits16,
            config::SlotMode::Mono,
        ),
        config::StdGpioConfig::default(),
    );

    let mut tx_driver = I2sDriver::new_std_tx(i2s1, &i2s_config, bclk, dout, mclk2, lrclk).unwrap();
    tx_driver.tx_enable().unwrap();

    // play the WAV file
    tx_driver.write_all(WAV_DATA, 1000).unwrap();

    log_heap();

    // play hello end
    PLAYING.store(false, std::sync::atomic::Ordering::SeqCst);

    loop {
        debug("Waiting for speech...");

        let max_bytes = 30 * SAMPLE_RATE as usize;
        let mut buffer = Vec::<u8>::with_capacity(max_bytes);
        let mut length = 0;

        loop {
            match afe.fetch() {
                Ok(v) => {
                    let data = v.data;
                    length += data.len();
                    buffer.extend_from_slice(&data);
                    log::info!("Recording... total {length} bytes");
                    if length >= max_bytes {
                        log::info!("Recording time exceeded {max_bytes} bytes.");
                        break;
                    }
                }
                Err(_) => {
                    if length == 0 {
                        continue;
                    }
                    log::info!("Recorded... total {length} bytes");
                    break;
                }
            }
        }

        PLAYING.store(true, std::sync::atomic::Ordering::SeqCst);

        buffer.truncate(length);
        log::info!("Recording complete, length: {} bytes", buffer.len());

        if length == 0 {
            PLAYING.store(false, std::sync::atomic::Ordering::SeqCst);
            continue;
        }

        debug("play record...");
        tx_driver.write_all(&buffer, 1000 * 60).unwrap();
        debug("play record end");

        PLAYING.store(false, std::sync::atomic::Ordering::SeqCst);

        drop(buffer);
        log_heap();
    }
}

fn debug(s: &str) {
    log::info!("{s}");
    display(s).unwrap();
}

pub fn log_heap() {
    unsafe {
        use esp_idf_svc::sys::{heap_caps_get_free_size, MALLOC_CAP_INTERNAL, MALLOC_CAP_SPIRAM};

        log::info!(
            "Free SPIRAM heap size: {}",
            heap_caps_get_free_size(MALLOC_CAP_SPIRAM)
        );
        log::info!(
            "Free INTERNAL heap size: {}",
            heap_caps_get_free_size(MALLOC_CAP_INTERNAL)
        );
    }
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let _fs = esp_idf_svc::io::vfs::MountedEventfs::mount(20).unwrap();
    let peripherals = esp_idf_svc::hal::prelude::Peripherals::take().unwrap();

    log::info!("Hello, world!");
    log_heap();

    let sck = peripherals.pins.gpio5;
    let din = peripherals.pins.gpio6;
    let ws = peripherals.pins.gpio4;
    let dout = peripherals.pins.gpio7;
    let bclk = peripherals.pins.gpio15;
    let lrclk = peripherals.pins.gpio16;

    let _spi_driver = ui::init_ui_rs(
        peripherals.spi3,
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio47.into(),
        None,
    )
    .unwrap();
    log::info!("UI initialized");
    ui::greeting().unwrap();

    let afe = AFE::new();
    let afe_ = afe;

    std::thread::spawn(move || {
        play(
            peripherals.i2s1,
            bclk.into(),
            dout.into(),
            lrclk.into(),
            None,
            afe_,
        )
    });

    record(
        peripherals.i2s0,
        ws.into(),
        sck.into(),
        din.into(),
        None,
        afe,
    );
}
