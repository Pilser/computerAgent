//! Baked model catalog: id -> context window + price per 1M tokens.
//!
//! Slim port of vendor `models_catalog.rs` (which embeds `data/models.json`)
//! + `pricing.rs` (`estimate_cost`). A snapshot only: the live `/models`
//! listing arrives with HTTP in a later milestone. Prices below are
//! $/1M tokens copied from the vendor snapshot.

/// One catalog row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelInfo {
    pub provider: &'static str,
    pub id: &'static str,
    pub context_window: u32,
    /// $ per 1M input tokens.
    pub input_per_mtok: f64,
    /// $ per 1M output tokens.
    pub output_per_mtok: f64,
}

/// Snapshot of well-known models (subset of vendor `data/models.json`).
pub const MODELS: &[ModelInfo] = &[
    // Anthropic
    ModelInfo {
        provider: "anthropic",
        id: "claude-sonnet-4-6",
        context_window: 1_000_000,
        input_per_mtok: 3.0,
        output_per_mtok: 15.0,
    },
    ModelInfo {
        provider: "anthropic",
        id: "claude-haiku-4-5",
        context_window: 200_000,
        input_per_mtok: 1.0,
        output_per_mtok: 5.0,
    },
    ModelInfo {
        provider: "anthropic",
        id: "claude-fable-5",
        context_window: 1_000_000,
        input_per_mtok: 10.0,
        output_per_mtok: 50.0,
    },
    // OpenAI
    ModelInfo {
        provider: "openai",
        id: "gpt-5.1",
        context_window: 400_000,
        input_per_mtok: 1.25,
        output_per_mtok: 10.0,
    },
    ModelInfo {
        provider: "openai",
        id: "gpt-4-turbo",
        context_window: 128_000,
        input_per_mtok: 10.0,
        output_per_mtok: 30.0,
    },
    ModelInfo {
        provider: "openai",
        id: "gpt-4",
        context_window: 8_192,
        input_per_mtok: 30.0,
        output_per_mtok: 60.0,
    },
    ModelInfo {
        provider: "openai",
        id: "gpt-3.5-turbo",
        context_window: 16_385,
        input_per_mtok: 0.5,
        output_per_mtok: 1.5,
    },
    // Gemini
    ModelInfo {
        provider: "gemini",
        id: "gemini-2.5-pro",
        context_window: 1_048_576,
        input_per_mtok: 1.25,
        output_per_mtok: 10.0,
    },
    ModelInfo {
        provider: "gemini",
        id: "gemini-2.5-flash",
        context_window: 1_048_576,
        input_per_mtok: 0.3,
        output_per_mtok: 2.5,
    },
    // Ollama (local: no price, practical window depends on host RAM)
    ModelInfo {
        provider: "ollama",
        id: "llama3.1",
        context_window: 128_000,
        input_per_mtok: 0.0,
        output_per_mtok: 0.0,
    },
    // OpenRouter (price varies by route; snapshot had none)
    ModelInfo {
        provider: "openrouter",
        id: "openrouter/auto",
        context_window: 200_000,
        input_per_mtok: 0.0,
        output_per_mtok: 0.0,
    },
    ModelInfo {
        provider: "openrouter",
        id: "anthropic/claude-3-haiku",
        context_window: 200_000,
        input_per_mtok: 0.0,
        output_per_mtok: 0.0,
    },
];

/// Find a model by provider + id.
pub fn lookup(provider: &str, id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.provider == provider && m.id == id)
}

/// Context window for a model, or `None` when not cataloged.
pub fn context_window(provider: &str, id: &str) -> Option<u32> {
    lookup(provider, id).map(|m| m.context_window)
}

/// Estimate cost in dollars (vendor `pricing::estimate_cost`). Returns 0.0
/// when the model is unpriced (local / routed).
pub fn estimate_cost(provider: &str, id: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let Some(m) = lookup(provider, id) else {
        return 0.0;
    };
    input_tokens as f64 * m.input_per_mtok / 1_000_000.0
        + output_tokens as f64 * m.output_per_mtok / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_context_windows() {
        assert_eq!(
            context_window("anthropic", "claude-sonnet-4-6"),
            Some(1_000_000)
        );
        assert_eq!(context_window("openai", "gpt-4"), Some(8_192));
        assert_eq!(context_window("gemini", "gemini-2.5-pro"), Some(1_048_576));
        assert_eq!(context_window("ollama", "llama3.1"), Some(128_000));
    }

    #[test]
    fn unknown_model_is_none() {
        assert_eq!(context_window("openai", "gpt-99"), None);
        assert_eq!(lookup("custom", "anything"), None);
    }

    #[test]
    fn cost_math() {
        // 1M in + 1M out on gpt-4-turbo (10/30) = $40.
        let c = estimate_cost("openai", "gpt-4-turbo", 1_000_000, 1_000_000);
        assert!((c - 40.0).abs() < 1e-9, "{c}");
        // Unpriced local model costs nothing.
        assert_eq!(estimate_cost("ollama", "llama3.1", 99, 99), 0.0);
        assert_eq!(estimate_cost("openai", "gpt-99", 99, 99), 0.0);
    }
}
