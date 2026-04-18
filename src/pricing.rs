//! Model pricing — single source of truth for converting token usage to USD.
//!
//! Rates are per-million tokens, as published by Anthropic. They're grouped
//! by model family (Opus / Sonnet / Haiku) rather than by specific model ID,
//! matching the pattern originally used in the Swift UI.

/// Pricing for a single model family, in USD per million tokens.
#[derive(Debug, Clone, Copy)]
struct ModelPricing {
    input_per_1m: f64,
    output_per_1m: f64,
    cache_read_per_1m: f64,
    cache_create_per_1m: f64,
}

const OPUS: ModelPricing = ModelPricing {
    input_per_1m: 15.0,
    output_per_1m: 75.0,
    cache_read_per_1m: 1.5,
    cache_create_per_1m: 18.75,
};

const SONNET: ModelPricing = ModelPricing {
    input_per_1m: 3.0,
    output_per_1m: 15.0,
    cache_read_per_1m: 0.3,
    cache_create_per_1m: 3.75,
};

const HAIKU: ModelPricing = ModelPricing {
    input_per_1m: 0.80,
    output_per_1m: 4.0,
    cache_read_per_1m: 0.08,
    cache_create_per_1m: 1.0,
};

fn pricing_for(model: &str) -> ModelPricing {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        OPUS
    } else if m.contains("haiku") {
        HAIKU
    } else {
        // Default to Sonnet pricing — matches the previous Swift behavior and
        // is the most common model family.
        SONNET
    }
}

/// Canonical model family for grouping (Opus / Sonnet / Haiku / other).
pub fn model_family(model: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        "opus"
    } else if m.contains("sonnet") {
        "sonnet"
    } else if m.contains("haiku") {
        "haiku"
    } else {
        "other"
    }
}

/// Compute the USD cost for a session given its model and token counts.
///
/// `input_tokens` is the total input tokens reported in the `usage` object;
/// cache-read and cache-create tokens are subtracted from it so they're not
/// double-billed.
pub fn cost_usd(
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
) -> f64 {
    let p = pricing_for(model);
    let non_cached_input = input_tokens
        .saturating_sub(cache_read_tokens)
        .saturating_sub(cache_create_tokens);
    let total = non_cached_input as f64 * p.input_per_1m
        + output_tokens as f64 * p.output_per_1m
        + cache_read_tokens as f64 * p.cache_read_per_1m
        + cache_create_tokens as f64 * p.cache_create_per_1m;
    total / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sonnet_basic_cost() {
        // 1M output tokens on Sonnet = $15
        let cost = cost_usd("claude-sonnet-4-20250514", 0, 1_000_000, 0, 0);
        assert!((cost - 15.0).abs() < 1e-9);
    }

    #[test]
    fn opus_basic_cost() {
        let cost = cost_usd("claude-opus-4-7", 0, 1_000_000, 0, 0);
        assert!((cost - 75.0).abs() < 1e-9);
    }

    #[test]
    fn haiku_basic_cost() {
        let cost = cost_usd("claude-haiku-4-5-20251001", 0, 1_000_000, 0, 0);
        assert!((cost - 4.0).abs() < 1e-9);
    }

    #[test]
    fn cache_tokens_are_not_double_billed() {
        // 1M input reported, of which 500k is cache-read, 300k cache-create.
        // Non-cached input = 200k. Sonnet rates: 200k*3 + 500k*0.3 + 300k*3.75 = $0.60 + $0.15 + $1.125 = $1.875
        let cost = cost_usd("sonnet", 1_000_000, 0, 500_000, 300_000);
        assert!((cost - 1.875).abs() < 1e-9, "got {}", cost);
    }

    #[test]
    fn zero_tokens_zero_cost() {
        assert_eq!(cost_usd("opus", 0, 0, 0, 0), 0.0);
    }

    #[test]
    fn unknown_model_defaults_to_sonnet() {
        let unknown = cost_usd("claude-future-5", 0, 1_000_000, 0, 0);
        let sonnet = cost_usd("sonnet", 0, 1_000_000, 0, 0);
        assert_eq!(unknown, sonnet);
    }

    #[test]
    fn model_family_classification() {
        assert_eq!(model_family("claude-opus-4-7"), "opus");
        assert_eq!(model_family("claude-sonnet-4-20250514"), "sonnet");
        assert_eq!(model_family("claude-haiku-4-5-20251001"), "haiku");
        assert_eq!(model_family(""), "other");
        assert_eq!(model_family("something-else"), "other");
    }

    #[test]
    fn cache_tokens_exceeding_input_do_not_underflow() {
        // Defensive: if cache tokens exceed input_tokens (shouldn't happen in practice),
        // saturating_sub prevents underflow.
        let cost = cost_usd("sonnet", 100, 0, 1000, 0);
        // non_cached_input saturates to 0, so cost = 1000 * 0.3 / 1e6 = 0.0003
        assert!((cost - 0.0003).abs() < 1e-9, "got {}", cost);
    }
}
