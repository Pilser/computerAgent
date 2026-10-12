//! OpenRouter payload (OpenAI-compatible + Claude routing).
//!
//! Slim port of vendor `openrouter_anthropic_routing`: Claude models must be
//! pinned to the Anthropic direct route (`provider.order = ["Anthropic"]`,
//! fallbacks allowed) so `cache_control` breakpoints are honored.

/// POST `{base}/chat/completions`.
pub fn chat_url(base: &str) -> String {
    format!("{}/chat/completions", base.trim_end_matches('/'))
}

/// True for Claude slugs (`anthropic/...`, optional leading `~` float marker).
pub fn is_claude_model(model_id: &str) -> bool {
    let slug = model_id.strip_prefix('~').unwrap_or(model_id);
    slug.starts_with("anthropic/")
}

/// OpenAI-shaped body, plus the Anthropic-route pin for Claude models.
pub fn build_payload(model: &str, system: &str, user: &str) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
    });
    if is_claude_model(model) {
        body["provider"] = serde_json::json!({"order": ["Anthropic"], "allow_fallbacks": true});
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_gets_routing_pin() {
        let p = build_payload("anthropic/claude-3-haiku", "s", "u");
        assert_eq!(p["provider"]["order"][0], "Anthropic");
        assert_eq!(p["provider"]["allow_fallbacks"], true);
    }

    #[test]
    fn tilde_slug_counts_as_claude() {
        assert!(is_claude_model("~anthropic/claude-sonnet-latest"));
        assert!(!is_claude_model("openai/gpt-4"));
    }

    #[test]
    fn non_claude_has_no_provider_key() {
        let p = build_payload("openrouter/auto", "s", "u");
        assert!(p.get("provider").is_none());
    }
}
