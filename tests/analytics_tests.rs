use hippoclaudus::analytics::compute_distribution;
use hippoclaudus::pricing::{cost_usd, model_family};
use hippoclaudus::session::extract_session_metadata;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn analytics_pipeline_on_fixture() {
    // End-to-end smoke: pull metadata from a real fixture, run it through the
    // cost + distribution pipeline, and assert the key invariants hold.
    let path = fixture("session_with_tools.jsonl");
    let info = extract_session_metadata(&path).expect("metadata parses");

    // Model family classification matches what the fixture uses.
    assert_eq!(model_family(&info.model), "sonnet");

    // Cost is non-zero because the fixture has real token usage.
    let cost = cost_usd(
        &info.model,
        info.total_input_tokens,
        info.total_output_tokens,
        info.total_cache_read_tokens,
        info.total_cache_creation_tokens,
    );
    assert!(cost > 0.0, "expected non-zero cost, got {}", cost);

    // Single-sample distributions: all percentiles equal the sample value.
    let d = compute_distribution(&[cost]);
    assert_eq!(d.count, 1);
    assert_eq!(d.p50, cost);
    assert_eq!(d.p99, cost);
}
