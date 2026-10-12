//! Anthropic Messages API payload.

/// API version header value sent with every request.
pub const API_VERSION: &str = "2023-06-01";

/// POST `{base}/v1/messages`.
pub fn chat_url(base: &str) -> String {
    format!("{}/v1/messages", base.trim_end_matches('/'))
}

/// `{"model":..,"max_tokens":..,"system":..,"messages":[{role:user,content}]}`.
/// Unlike OpenAI, Anthropic takes `system` as a top-level field.
pub fn build_payload(model: &str, system: &str, user: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system,
        "messages": [{"role": "user", "content": user}],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_shape() {
        assert_eq!(
            chat_url("https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn system_is_top_level() {
        let p = build_payload("claude-sonnet-4-6", "sys", "hi");
        assert_eq!(p["system"], "sys");
        assert_eq!(p["messages"][0]["role"], "user");
        assert!(p["max_tokens"].as_u64().unwrap() > 0);
    }
}
