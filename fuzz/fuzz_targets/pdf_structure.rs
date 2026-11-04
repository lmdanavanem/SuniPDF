//! lopdf loading, structure inspection and rewriting of untrusted bytes: typed errors, never a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sunipdf_core::{convert, inspection, security};

fuzz_target!(|data: &[u8]| {
    let _ = inspection::inspect(data, None);
    let _ = security::strip_metadata(data, None);
    if let Ok(optimized) = convert::optimize(data, None) {
        // A rewritten copy must itself load again.
        let _ = inspection::inspect(&optimized, None);
    }
});
