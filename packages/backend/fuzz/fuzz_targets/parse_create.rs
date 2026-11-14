#![no_main]

use cryptgeon::note::parse_create;
use libfuzzer_sys::fuzz_target;

// Feed arbitrary bytes through the msgpack request parser and validator.
// Must never panic.
fuzz_target!(|data: &[u8]| {
    let _ = parse_create(data);
});
