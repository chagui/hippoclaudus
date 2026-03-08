#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as JSON and feed to extraction functions
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let _ = hippoclaudus::session::extract_user_text(&value);
        let _ = hippoclaudus::session::extract_assistant_text(&value);
    }
});
