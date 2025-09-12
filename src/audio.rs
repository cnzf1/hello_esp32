use esp_idf_svc::sys::esp_sr;

use crate::call_c_method;

pub const SAMPLE_RATE: u32 = 16000;

static WAKENET_STATE: std::sync::atomic::AtomicI8 = std::sync::atomic::AtomicI8::new(0);

unsafe fn afe_init() -> Afe {
    let models = esp_sr::esp_srmodel_init("model\0".as_ptr() as *const _);
    if models.is_null() {
        log::info!("Failed to initialize models");
    } else {
        log::info!("models: {models:#?}");
    }

    let afe_config = esp_sr::afe_config_init(
        "MR\0".as_ptr() as _,
        models,
        esp_sr::afe_type_t_AFE_TYPE_SR,
        esp_sr::afe_mode_t_AFE_MODE_HIGH_PERF,
    );
    let afe_config = afe_config.as_mut().unwrap();
    afe_config.pcm_config.total_ch_num = 1;
    afe_config.pcm_config.mic_num = 1;
    afe_config.pcm_config.ref_num = 0;
    afe_config.pcm_config.sample_rate = 16000;
    afe_config.afe_ringbuf_size = 25;
    afe_config.vad_min_noise_ms = 500;
    // afe_config.vad_min_speech_ms = 32;
    afe_config.vad_mode = esp_sr::vad_mode_t_VAD_MODE_1;
    afe_config.agc_init = true;
    // afe_config.vad_delay_ms = 128;
    // use WakeNet 唤醒
    afe_config.wakenet_init = true;
    afe_config.wakenet_mode = esp_sr::det_mode_t_DET_MODE_90;

    if !afe_config.wakenet_model_name.is_null() {
        log::info!("wakenet model name:{:?}", afe_config.wakenet_model_name);
    }
    // 打印唤醒词模型信息
    if !afe_config.wakenet_model_name_2.is_null() {
        log::info!("wakenet model name:{:?}", afe_config.wakenet_model_name_2);
    }

    log::info!("{afe_config:#?}");

    let afe_ringbuf_size = afe_config.afe_ringbuf_size;
    log::info!("afe ringbuf size: {afe_ringbuf_size}");

    let afe_handle = esp_sr::esp_afe_handle_from_config(afe_config);
    let afe_handle = afe_handle.as_mut().unwrap();
    let afe_data = (afe_handle.create_from_config.unwrap())(afe_config);
    let audio_chunksize = (afe_handle.get_feed_chunksize.unwrap())(afe_data);
    // let feed_channel = afe_handle.get_feed_channel_num.unwrap()(afe_data);
    log::info!("audio chunksize: {audio_chunksize}");

    (afe_handle.enable_wakenet.unwrap())(afe_data);

    esp_sr::afe_config_free(afe_config);
    Afe {
        handle: afe_handle,
        data: afe_data,
        feed_chunksize: audio_chunksize as usize,
        // wakenet_handle,
        // wakenet_data
    }
}

#[derive(Clone, Copy)]
pub struct Afe {
    pub handle: *mut esp_sr::esp_afe_sr_iface_t,
    pub data: *mut esp_sr::esp_afe_sr_data_t,
    pub feed_chunksize: usize,
    // pub wakenet_handle: *const esp_sr::esp_wn_iface_t,
    // pub wakenet_data: *mut esp_sr::model_iface_data_t,
}

unsafe impl Send for Afe {}
unsafe impl Sync for Afe {}

pub struct AFEResult {
    pub data: Vec<u8>,
    pub speech: bool,
}

impl Afe {
    pub fn new() -> Self {
        unsafe {
            afe_init()
            // let (handle, data, feed_chunksize) = afe_init();
            //
            // AFE {
            //     handle,
            //     data,
            //     feed_chunksize: feed_chunksize as usize ,
            // }
        }
    }

    #[allow(dead_code)]
    fn reset(&self) {
        let afe_handle = self.handle;
        let afe_data = self.data;
        unsafe {
            (afe_handle.as_ref().unwrap().reset_vad.unwrap())(afe_data);
        }
    }

    pub fn feed(&self, data: &[u8]) -> i32 {
        let afe_handle = self.handle;
        let afe_data = self.data;
        unsafe {
            (afe_handle.as_ref().unwrap().feed.unwrap())(afe_data, data.as_ptr() as *const i16)
        }
    }

    pub fn fetch(&self) -> Result<AFEResult, i32> {
        let afe_handle = self.handle;
        let afe_data = self.data;
        unsafe {
            let result = (afe_handle.as_ref().unwrap().fetch.unwrap())(afe_data)
                .as_mut()
                .unwrap();

            if result.ret_value != 0 {
                return Err(result.ret_value);
            }

            if WAKENET_STATE.load(std::sync::atomic::Ordering::SeqCst) != result.wakeup_state as i8
            {
                WAKENET_STATE.store(
                    result.wakeup_state as i8,
                    std::sync::atomic::Ordering::SeqCst,
                );
                log::info!(
                    "afe wakenet state: {} wake_word_index: {} model_index: {}",
                    wakenet_state_to_str(result.wakeup_state),
                    result.wake_word_index,
                    result.wakenet_model_index
                );
            }
            if result.vad_state != esp_sr::vad_state_t_VAD_SPEECH {
                // log::info!("no speech detected");
                return Err(result.ret_value);
            }

            let data_size = result.data_size;
            let vad_state = result.vad_state;
            let mut data = Vec::with_capacity(data_size as usize + result.vad_cache_size as usize);
            if result.vad_cache_size > 0 {
                let data_ptr = result.vad_cache as *const u8;
                let data_ = std::slice::from_raw_parts(data_ptr, (result.vad_cache_size) as usize);
                data.extend_from_slice(data_);
            }
            if data_size > 0 {
                let data_ptr = result.data as *const u8;
                let data_ = std::slice::from_raw_parts(data_ptr, (data_size) as usize);
                data.extend_from_slice(data_);
            };

            let speech = vad_state == esp_sr::vad_state_t_VAD_SPEECH;
            Ok(AFEResult { data, speech })
        }
    }

    pub fn detect(&self, _data: &mut [i16]) {
        // let handle = self.wakenet_handle;
        // let wakenet_data = self.wakenet_data;
        // let data = data.as_mut_ptr();
        // let res = call_c_method!(handle, detect, wakenet_data, data).unwrap();
        // log::info!("wakenet detect result: {}", wakenet_state_to_str(res));
    }

    pub fn set_wakenet_threashold(&self) {
        let afe_handle = self.handle;
        let afe_data = self.data;
        unsafe {
            (afe_handle.as_ref().unwrap().set_wakenet_threshold.unwrap())(afe_data, 1, 0.05);
            (afe_handle.as_ref().unwrap().set_wakenet_threshold.unwrap())(afe_data, 2, 0.05);
            (afe_handle
                .as_ref()
                .unwrap()
                .reset_wakenet_threshold
                .unwrap())(afe_data, 1);
            (afe_handle
                .as_ref()
                .unwrap()
                .reset_wakenet_threshold
                .unwrap())(afe_data, 2);
        }
    }
}

pub fn wakenet_state_to_str(state: esp_sr::wakenet_state_t) -> &'static str {
    match state {
        esp_sr::wakenet_state_t_WAKENET_NO_DETECT => "NO_DETECT",
        esp_sr::wakenet_state_t_WAKENET_DETECTED => "DETECTED",
        esp_sr::wakenet_state_t_WAKENET_CHANNEL_VERIFIED => "CHANNEL_VERIFIED",
        _ => "UNKNOWN",
    }
}
