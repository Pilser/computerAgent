//! Gemini `generateContent` payload.

/// POST `{base}/v1beta/models/{model}:generateContent`.
pub fn chat_url(base: &str, model: &str) -> String {
    format!(
        "{}/v1beta/models/{}:generateContent",
        base.trim_end_matches('/'),
        model
    )
}

/// `{"system_instruction":{...},"contents":[{role:user,...}]}`.
pub fn build_payload(model: &str, system: &str, user: &str) -> serde_json::Value {
    let _ = model; // model travels in the URL for Gemini
    serde_json::json!({
        "system_instruction": {"parts": [{"text": system}]},
        "contents": [{"role": "user", "parts": [{"text": user}]}],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_embeds_model() {
        let u = chat_url("https://x", "gemini-2.5-pro");
        assert!(u.contains("gemini-2.5-pro:generateContent"));
    }

    #[test]
    fn payload_shape() {
        let p = build_payload("gemini-2.5-pro", "sys", "hi");
        assert_eq!(p["system_instruction"]["parts"][0]["text"], "sys");
        assert_eq!(p["contents"][0]["parts"][0]["text"], "hi");
    }
}
