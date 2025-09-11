use esp_idf_svc::sys::esp_sr;

#[macro_export]
macro_rules! call_c_method {
        ($c_ptr: expr, $method: ident, $($args: expr),*) => {
            unsafe {
                if $c_ptr.is_null() {
                    Err(anyhow::anyhow!("Null pointer provided to {}", stringify!($method)))
                } else if let Some(inner_func) = (*$c_ptr).$method {
                    Ok(inner_func($($args),*))
                } else {
                    Err(anyhow::anyhow!("Failed to call method {}", stringify!($method)))
                }
            }
        };
    }

pub fn print_afe_config(afe_config: *const esp_sr::afe_config_t) {
    unsafe {
        log::info!("--- AFE Configuration ---");

        // AEC configuration
        log::info!(
            "AEC: init={}, mode={}, filter_length={}",
            (*afe_config).aec_init,
            (*afe_config).aec_mode,
            (*afe_config).aec_filter_length
        );

        // SE configuration
        log::info!("SE: init={}", (*afe_config).se_init);

        // NS configuration
        log::info!(
            "NS: init={}, mode={}",
            (*afe_config).ns_init,
            (*afe_config).afe_ns_mode
        );

        // VAD configuration
        log::info!("VAD: init={}, mode={}, min_speech_ms={}, min_noise_ms={}, delay_ms={}, mute_playback={}, enable_channel_trigger={}",
            (*afe_config).vad_init,
            (*afe_config).vad_mode,
            (*afe_config).vad_min_speech_ms,
            (*afe_config).vad_min_noise_ms,
            (*afe_config).vad_delay_ms,
            (*afe_config).vad_mute_playback,
            (*afe_config).vad_enable_channel_trigger);

        // WakeNet configuration
        log::info!(
            "WakeNet: init={}, mode={}",
            (*afe_config).wakenet_init,
            (*afe_config).wakenet_mode
        );

        // AGC configuration
        log::info!(
            "AGC: init={}, mode={}, compression_gain_db={}, target_level_dbfs={}",
            (*afe_config).agc_init,
            (*afe_config).agc_mode,
            (*afe_config).agc_compression_gain_db,
            (*afe_config).agc_target_level_dbfs
        );

        // PCM configuration
        log::info!(
            "PCM: total_ch_num={}, mic_num={}, ref_num={}, sample_rate={}",
            (*afe_config).pcm_config.total_ch_num,
            (*afe_config).pcm_config.mic_num,
            (*afe_config).pcm_config.ref_num,
            (*afe_config).pcm_config.sample_rate
        );

        // General AFE configuration
        log::info!("General AFE: mode={}, type={}, preferred_core={}, preferred_priority={}, ringbuf_size={}, linear_gain={}",
            (*afe_config).afe_mode,
            (*afe_config).afe_type,
            (*afe_config).afe_perferred_core,
            (*afe_config).afe_perferred_priority,
            (*afe_config).afe_ringbuf_size,
            (*afe_config).afe_linear_gain);

        log::info!(
            "Memory allocation mode={}, debug_init={}, fixed_first_channel={}",
            (*afe_config).memory_alloc_mode,
            (*afe_config).debug_init,
            (*afe_config).fixed_first_channel
        );

        log::info!("--- End of AFE Configuration ---");
    }
}
