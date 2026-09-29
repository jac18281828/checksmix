#![no_main]

use checksmix::MmoDecoder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let decoder = MmoDecoder::new(data.to_vec());
    let _ = decoder.decode(|_, _| {});
});
