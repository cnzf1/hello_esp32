use std::ffi::CString;

use esp_idf_svc::sys::esp_sr::{
    self, afe_config_free, afe_config_init, esp_afe_handle_from_config, esp_srmodel_filter,
};

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

pub struct MultiNet {
    iface: *mut esp_sr::esp_afe_sr_iface_t,
    data: *mut esp_sr::esp_afe_sr_data_t,
    mn_iface: *mut esp_sr::esp_mn_iface_t,
    model_iface_data: *mut esp_sr::model_iface_data_t,
}

impl MultiNet {
    pub fn new() -> Result<MultiNet, anyhow::Error> {
        let part_name = CString::new("/vfat").unwrap();
        let models = unsafe { esp_sr::esp_srmodel_init(part_name.as_ptr() as *const _) };
        if models.is_null() {
            log::error!("Failed to initialize speech recognition models");
            return Err(anyhow::anyhow!(
                "Failed to initialize speech recognition models"
            ));
        }

        let input_format = CString::new("M").unwrap();
        let afe_config = unsafe {
            afe_config_init(
                input_format.as_ptr(),
                models,
                esp_sr::afe_type_t_AFE_TYPE_SR,
                esp_sr::afe_mode_t_AFE_MODE_LOW_COST,
            )
        };

        if afe_config.is_null() {
            log::error!("Failed to initialize AFE configuration");
            return Err(anyhow::anyhow!("Failed to initialize AFE configuration"));
        }

        // Print the AFE configuration
        print_afe_config(afe_config);

        // Initialize AFE
        let afe_handle = unsafe { esp_afe_handle_from_config(afe_config) };
        if afe_handle.is_null() {
            log::error!("Failed to create AFE handle from config");
            unsafe { afe_config_free(afe_config) };
            return Err(anyhow::anyhow!("Failed to create AFE handle"));
        }

        let afe_data = match call_c_method!(afe_handle, create_from_config, afe_config) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to create AFE data: {}", e);
                unsafe { afe_config_free(afe_config) };
                return Err(e);
            }
        };

        // Free config after use
        unsafe { afe_config_free(afe_config) };

        let prefix_str = Vec::from(esp_sr::ESP_MN_PREFIX);
        let chinese_str = Vec::from(esp_sr::ESP_MN_CHINESE);
        let mn_name = unsafe {
            esp_srmodel_filter(
                models,
                prefix_str.as_ptr() as *const i8,
                chinese_str.as_ptr() as *const i8,
            )
        };

        if mn_name.is_null() {
            log::error!("Failed to filter speech recognition model");
            return Err(anyhow::anyhow!("Failed to filter speech recognition model"));
        }

        let multinet = unsafe { esp_mn_handle_from_name(mn_name) };
        if multinet.is_null() {
            log::error!("Failed to get multinet handle");
            return Err(anyhow::anyhow!("Failed to get multinet handle"));
        }

        let model_data = match call_c_method!(multinet, create, mn_name, 6000) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to create model data: {}", e);
                return Err(anyhow::anyhow!("Failed to create model data: {}", e));
            }
        };

        // Setup speech commands
        unsafe {
            esp_mn_commands_clear();
            esp_mn_commands_add(1, Vec::from(b"wo you ge wen ti\0").as_ptr() as *const i8);
            esp_mn_commands_update();
        }

        Ok(MultiNet {
            iface: afe_handle,
            data: afe_data,
            mn_iface: multinet,
            model_iface_data: model_data,
        })
    }
}
