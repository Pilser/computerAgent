//! OpenAI chat-completions payload (also covers OpenAI-compatible gateways).

/// POST `{base}/chat/completions`.
pub fn chat_url(base: &str) -> String {
    format!("{}/chat/completions", base.trim_end_matches('/'))
}

/// `{"model":..,"messages":[{system},{user}]}`.
pub fn build_payload(model: &str, system: &str, user: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_trims_slash() {
        assert_eq!(chat_url("https://x/v1/"), "https://x/v1/chat/completions");
    }

    #[test]
    fn payload_shape() {
        let p = build_payload("gpt-5.1", "sys", "hi");
        assert_eq!(p["model"], "gpt-5.1");
        assert_eq!(p["messages"][0]["role"], "system");
        assert_eq!(p["messages"][1]["content"], "hi");
    }
}
