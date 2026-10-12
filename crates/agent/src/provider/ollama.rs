//! Ollama local chat payload (keyless by default).

/// POST `{base}/api/chat`.
pub fn chat_url(base: &str) -> String {
    format!("{}/api/chat", base.trim_end_matches('/'))
}

/// `{"model":..,"messages":[...],"stream":false}` (v1 polls, no streaming).
pub fn build_payload(model: &str, system: &str, user: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "stream": false,
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
    fn local_default_url() {
        assert_eq!(
            chat_url("http://localhost:11434"),
            "http://localhost:11434/api/chat"
        );
    }

    #[test]
    fn payload_is_non_streaming() {
        let p = build_payload("llama3.1", "sys", "hi");
        assert_eq!(p["stream"], false);
        assert_eq!(p["model"], "llama3.1");
    }
}
