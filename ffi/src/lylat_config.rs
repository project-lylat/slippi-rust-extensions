//! Lylat: the Lylat addresses Dolphin needs outside the EXI device (matchmaking, QR sign-in),
//! from the environment this library was built for. See the `slippi-config` crate.

use std::ffi::{CString, c_char};

use slippi_config::SlippiConfig;

fn into_c_string(value: Option<String>) -> *mut c_char {
    CString::new(value.unwrap_or_default())
        .expect("Lylat config string had a nul byte")
        .into_raw()
}

/// The Lylat site, i.e: `https://lylat.gg`. Free it with `slprs_lylat_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_origin() -> *mut c_char {
    into_c_string(SlippiConfig::get().origin)
}

/// The Lylat matchmaking host, i.e: `lylat.gg`. Free it with `slprs_lylat_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_mm_host() -> *mut c_char {
    into_c_string(SlippiConfig::get().mm_host)
}

/// The Lylat matchmaking port, 0 when the environment has none.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_mm_port() -> u16 {
    SlippiConfig::get().mm_port.unwrap_or(0)
}

/// Frees a string returned by the `slprs_lylat_*` functions.
#[unsafe(no_mangle)]
pub extern "C" fn slprs_lylat_free_string(value: *mut c_char) {
    if !value.is_null() {
        // Safety: the pointer came from `CString::into_raw` in this module.
        drop(unsafe { CString::from_raw(value) });
    }
}
