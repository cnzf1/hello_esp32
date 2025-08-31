use std::sync::{Arc, Mutex};

use esp_idf_svc::{
    hal::{
        gpio::AnyIOPin,
        i2s::{config, I2sDriver, I2S0, I2S1},
    },
    io::Read,
};

use crate::ui::set_lcd;

mod network;
mod ui;

const SAMPLE_RATE: u32 = 16000;

static WAV_DATA: &[u8] = include_bytes!("../assets/hello.wav");

fn player_wav(
    i2s1: I2S1,
    bclk: AnyIOPin,
    dout: AnyIOPin,
    lrclk: AnyIOPin,
    mclk: Option<AnyIOPin>,
    data: Option<&[u8]>,
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

    let mut tx_driver = I2sDriver::new_std_tx(i2s1, &i2s_config, bclk, dout, mclk, lrclk).unwrap();

    tx_driver.tx_enable().unwrap();

    if let Some(data) = data {
        tx_driver.write_all(data, 1000).unwrap();
    } else {
        tx_driver.write_all(WAV_DATA, 1000).unwrap();
    }
}

fn record(
    i2s: I2S0,
    ws: AnyIOPin,
    sck: AnyIOPin,
    din: AnyIOPin,
    mclk: Option<AnyIOPin>,
) -> Vec<u8> {
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
    rx_driver.rx_enable().unwrap();

    let mut buffer = vec![0u8; 5 * SAMPLE_RATE as usize * 2]; // 5 seconds of audio at 16kHz, 16-bit mono
    rx_driver.read_exact(&mut buffer).unwrap();
    buffer
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = esp_idf_svc::hal::prelude::Peripherals::take().unwrap();
    let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take().unwrap();
    let _fs = esp_idf_svc::io::vfs::MountedEventfs::mount(20).unwrap();
    let partition = esp_idf_svc::nvs::EspDefaultNvsPartition::take().unwrap();

    log::info!("Hello, world!");

    // ui::init_ui().unwrap();
    let _spi_driver = ui::init_ui_rs(
        peripherals.spi3,
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio47.into(),
        None,
    )
    .unwrap();
    log::info!("UI initialized");
    ui::hello_lcd().unwrap();

    let sck = peripherals.pins.gpio5;
    let din = peripherals.pins.gpio6;
    let ws = peripherals.pins.gpio4;

    let dout = peripherals.pins.gpio7;
    let bclk = peripherals.pins.gpio15;
    let lrclk = peripherals.pins.gpio16;

    let mut button = esp_idf_svc::hal::gpio::PinDriver::input(peripherals.pins.gpio0).unwrap();
    button.set_pull(esp_idf_svc::hal::gpio::Pull::Up).unwrap();
    // button
    //     .set_interrupt_type(esp_idf_svc::hal::gpio::InterruptType::PosEdge)
    //     .unwrap();

    log::info!("capacity of SPIRAM: {} KB", get_cap_spiram() / 1024); // it will show 8M if open CONFIG_SPIRAM in sdkconfig.default, else 0
    log::info!("capacity of internal RAM: {} KB", get_cap_internal() / 1024); // 363KB
    log::info!("stack high: {}", get_stack_high());

    // try malloc a large buffer to test memory
    // if not open CONFIG_SPIRAM, it will panic and restart
    let _large_buffer = Vec::<u8>::with_capacity(1024 * 1024); // 1MB

    log::info!("Waiting for button press...");
    // load SSID and password from environment variables on build time
    const SSID: Option<&str> = option_env!("SSID");
    const PASSWORD: Option<&str> = option_env!("PASSWORD");
    const SERVER_URL: Option<&str> = option_env!("SERVER_URL");

    let _wifi = network::wifi(
        SSID.unwrap(),
        PASSWORD.unwrap_or_default(),
        peripherals.modem,
        sysloop,
    )
    .unwrap();

    let tokio_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // tokio_rt.block_on(async {
    //     log::info!("Starting HTTP GET request...");
    //     match network::http_get("http://httpbin.org/get").await {
    //         Ok(response) => log::info!("HTTP GET response: {}", response),
    //         Err(e) => log::error!("HTTP GET error: {}", e),
    //     }
    // });

    // if let Some(server_url) = SERVER_URL {
    //     tokio_rt.block_on(async {
    //         log::info!("Starting WebSocket task...");
    //         match network::ws_task(server_url).await {
    //             Ok(_) => log::info!("WebSocket task completed successfully"),
    //             Err(e) => log::error!("WebSocket task error: {}", e),
    //         }
    //     });
    // } else {
    //     log::warn!("No SERVER_URL provided, skipping WebSocket connection");
    // }

    let i2s_config = config::StdConfig::new(
        config::Config::default().auto_clear(true),
        config::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        config::StdSlotConfig::philips_slot_default(
            config::DataBitWidth::Bits16,
            config::SlotMode::Mono,
        ),
        config::StdGpioConfig::default(),
    );

    let mut rx_driver = I2sDriver::new_std_rx(
        peripherals.i2s0,
        &i2s_config,
        sck,
        din,
        Option::<AnyIOPin>::None,
        ws,
    )
    .unwrap();
    rx_driver.rx_enable().unwrap();

    let mut tx_driver = I2sDriver::new_std_tx(
        peripherals.i2s1,
        &i2s_config,
        bclk,
        dout,
        Option::<AnyIOPin>::None,
        lrclk,
    )
    .unwrap();
    tx_driver.tx_enable().unwrap();

    const BUFFER_SIZE: usize = 1024;
    const MAX_SAMPLES: usize = 160_000 * 5;
    let audio_buffer = Arc::new(Mutex::new(Vec::<u8>::with_capacity(MAX_SAMPLES)));

    loop {
        log::info!("Waiting for long press...");
        set_lcd("Waiting for long press...").unwrap();
        tokio_rt.block_on(button.wait_for_low()).unwrap();

        {
            let mut buf = audio_buffer.lock().unwrap();
            buf.clear();
        }

        loop {
            if button.is_high() {
                break;
            }

            // let samples = record(&i2s0, ws.into(), sck.into(), din.into(), None);
            let mut tmp = [0u8; BUFFER_SIZE];
            rx_driver.read_exact(&mut tmp).unwrap();
            let read_samples = tmp.len();
            if read_samples > 0 {
                let mut buf = audio_buffer.lock().unwrap();
                if buf.len() + read_samples <= MAX_SAMPLES {
                    buf.extend_from_slice(&tmp[..read_samples]);
                }
                log::info!("Recording complete, length: {} bytes", buf.len());
            }
        }

        log::info!("Playing back answer...");
        set_lcd("Playing back answer...").unwrap();

        {
            let buf = audio_buffer.lock().unwrap();
            tx_driver.write_all(&buf, 1000).unwrap();
        }
    }

    #[allow(unreachable_code)]
    unsafe {
        esp_idf_svc::sys::esp_restart()
    }
}

pub fn get_stack_high() -> u32 {
    unsafe { esp_idf_svc::sys::uxTaskGetStackHighWaterMark2(std::ptr::null_mut()) }
}

pub fn get_cap_spiram() -> usize {
    unsafe {
        use esp_idf_svc::sys::{heap_caps_get_free_size, MALLOC_CAP_SPIRAM};
        heap_caps_get_free_size(MALLOC_CAP_SPIRAM)
    }
}

pub fn get_cap_internal() -> usize {
    unsafe {
        use esp_idf_svc::sys::{heap_caps_get_free_size, MALLOC_CAP_INTERNAL};
        heap_caps_get_free_size(MALLOC_CAP_INTERNAL)
    }
}
