//! Signature dictionaries, /ByteRange and /Contents CMS envelopes of untrusted bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = sunipdf_core::certificate::verify_signatures(data, None);
});
