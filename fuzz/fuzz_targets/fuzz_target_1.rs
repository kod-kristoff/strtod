#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate strtod;

fuzz_target!(|data: &[u8]| {
    std::str::from_utf8(data).map(|s| strtod::strtod(s));
});
