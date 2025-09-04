use esp_idf_svc::sys::esp_sr;

pub const SAMPLE_RATE: u32 = 16000;

unsafe fn afe_init() -> (
    *mut esp_sr::esp_afe_sr_iface_t,
    *mut esp_sr::esp_afe_sr_data_t,
    i32
) {
    let models = esp_sr::esp_srmodel_init(c"model".as_ptr() as *const _);
    let afe_config = esp_sr::afe_config_init(
        c"M".as_ptr() as _,
        models,
        esp_sr::afe_type_t_AFE_TYPE_VC,
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

    log::info!("{afe_config:?}");

    let afe_ringbuf_size = afe_config.afe_ringbuf_size;
    log::info!("afe ringbuf size: {afe_ringbuf_size}");

    let afe_handle = esp_sr::esp_afe_handle_from_config(afe_config);
    let afe_handle = afe_handle.as_mut().unwrap();
    let afe_data = (afe_handle.create_from_config.unwrap())(afe_config);
    let audio_chunksize = (afe_handle.get_feed_chunksize.unwrap())(afe_data);
    log::info!("audio chunksize: {audio_chunksize}");

    esp_sr::afe_config_free(afe_config);
    (afe_handle, afe_data, audio_chunksize)
}

#[derive(Clone, Copy)]
pub struct AFE {
    pub handle: *mut esp_sr::esp_afe_sr_iface_t,
    pub data: *mut esp_sr::esp_afe_sr_data_t,
    pub feed_chunksize: usize,
}

unsafe impl Send for AFE {}
unsafe impl Sync for AFE {}

pub struct AFEResult {
    pub data: Vec<u8>,
    pub speech: bool,
}

impl AFE {
    pub fn new() -> Self {
        unsafe {
            let (handle, data, feed_chunksize) = afe_init();

            AFE {
                handle,
                data,
                feed_chunksize: feed_chunksize as usize ,
            }
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

            log::info!(
                "afe fetch data size: {}, state: {}, cache size: {}",
                result.data_size,
                result.vad_state,
                result.vad_cache_size
            );
            if result.vad_state != esp_sr::vad_state_t_VAD_SPEECH {
                log::info!("no speech detected");
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
}
