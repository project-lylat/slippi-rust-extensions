use std::ffi::{CString, c_char};

use slippi_config::SlippiConfig;

fn into_c_string(value: Option<String>) -> *mut c_char {
    CString::new(value.unwrap_or_default())
        .expect("Lylat config string had a nul byte")
        .into_raw()
}

/// Free it with `slprs_lylat_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_origin() -> *mut c_char {
    into_c_string(SlippiConfig::get().origin)
}

/// Free it with `slprs_lylat_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_mm_host() -> *mut c_char {
    into_c_string(SlippiConfig::get().mm_host)
}

/// 0 when the environment has none.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_mm_port() -> u16 {
    SlippiConfig::get().mm_port.unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_free_string(value: *mut c_char) {
    if !value.is_null() {
        // Safety: the pointer came from `CString::into_raw` in this module.
        drop(unsafe { CString::from_raw(value) });
    }
}
