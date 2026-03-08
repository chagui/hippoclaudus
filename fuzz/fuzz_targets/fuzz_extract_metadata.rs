#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Write arbitrary bytes to a temp .jsonl file, call extract_session_metadata(), assert no panic
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(data).unwrap();
    tmp.flush().unwrap();
    let _ = hippoclaudus::session::extract_session_metadata(tmp.path());
});
