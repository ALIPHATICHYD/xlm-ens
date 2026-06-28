#![no_main]

use libfuzzer_sys::fuzz_target;
use xlm_ns_common::validation::parse_fqdn;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // The goal is to ensure this function never panics on any valid string input.
        let _ = parse_fqdn(s);
    }
});