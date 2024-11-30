use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_double;
use std::os::raw::c_int;
use strtod;

/// Parses a 64-bit floating point number.
///
/// If no floating point number can be built, it returns 0 and the len is -1.
/// If a floating point number is returned, the len is the number of character used to build that number
#[no_mangle]
pub extern "C" fn strtod_rs(c_str: *const c_char, c_len: *mut c_int) -> c_double {
    let str_opt = unsafe { CStr::from_ptr(c_str).to_str().ok() };

    if let Some(str) = str_opt {
        if let Some(result) = strtod(str) {
            unsafe {
                *c_len = result.1 as c_int;
            }
            result.0
        } else {
            unsafe {
                *c_len = -1;
            }
            0f64
        }
    } else {
        unsafe {
            *c_len = -1;
        }
        0f64
    }
}
