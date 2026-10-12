//! Provider kind, config resolution, request planning.
//!
//! Ports vendor `auth.rs` (`ProviderKind::from_name`, env-var names, key
//! priority CLI > env > config) in slim form: no `rig`, no HTTP. Key lookup
//! is injectable via [`ProviderConfig::resolve_with`] so tests stay
//! deterministic without touching the process environment.

use std::collections::HashMap;

/// Supported LLM providers. Vendor aliases kept: `"custom"` -> OpenAI
/// (OpenAI-compatible gateway), `"google"` -> Gemini.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    OpenRouter,
}

impl ProviderKind {
    /// Parse a provider name. Case-insensitive.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "openai" | "custom" => Some(Self::OpenAI),
            "anthropic" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            "ollama" => Some(Self::Ollama),
            "openrouter" => Some(Self::OpenRouter),
            _ => None,
        }
    }

    /// Canonical lowercase name.
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
            Self::OpenRouter => "openrouter",
        }
    }

    /// Default environment variable holding the API key.
    pub fn default_key_env(self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
            Self::Ollama => "OLLAMA_API_KEY",
            Self::OpenRouter => "OPENROUTER_API_KEY",
        }
    }

    /// Default chat endpoint base. `None` means localhost-style default is
    /// handled by the vendor module (Ollama).
    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenAI => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com",
            Self::Gemini => "https://generativelanguage.googleapis.com",
            Self::Ollama => "http://localhost:11434",
            Self::OpenRouter => "https://openrouter.ai/api/v1",
        }
    }

    /// Built-in fallback model (vendor `default_model_for_provider`).
    pub fn default_model(self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-5.1",
            Self::Anthropic => "claude-sonnet-4-6",
            Self::Gemini => "gemini-2.5-pro",
            Self::Ollama => "llama3.1",
            Self::OpenRouter => "openrouter/auto",
        }
    }
}

/// Resolved provider configuration: everything needed to build (later: send)
/// a chat request. `api_key` is `None` when no key source yielded one
/// (Ollama works keyless against localhost).
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    /// Env var actually consulted (explicit override or provider default).
    pub api_key_env: String,
    /// Resolved key, if any.
    pub api_key: Option<String>,
    /// Explicit base URL or the provider default.
    pub base_url: String,
    /// Model id to request.
    pub model: String,
}

impl ProviderConfig {
    /// Resolve from the process environment.
    pub fn resolve_from_env(
        name: &str,
        model: Option<&str>,
        api_key_env: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<Self, String> {
        Self::resolve_with(name, model, api_key_env, base_url, &HashMap::new(), |var| {
            std::env::var(var).ok()
        })
    }

    /// Resolve with injectable env + config-file keys. Priority (vendor
    /// `AuthResolver`): explicit `api_key_env` lookup > provider default env
    /// var > `config_keys` map. Pure: no I/O, no network.
    pub fn resolve_with(
        name: &str,
        model: Option<&str>,
        api_key_env: Option<&str>,
        base_url: Option<&str>,
        config_keys: &HashMap<String, String>,
        get_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let kind = ProviderKind::from_name(name).ok_or_else(|| {
            format!("unknown provider: '{name}' (openai, anthropic, gemini, ollama, openrouter)")
        })?;
        let env_var = api_key_env
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| kind.default_key_env());
        let mut api_key = get_env(env_var).filter(|k| !k.is_empty());
        if api_key.is_none() {
            api_key = config_keys.get(kind.name()).cloned();
        }
        Ok(Self {
            kind,
            api_key_env: env_var.to_string(),
            api_key,
            base_url: base_url
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| kind.default_base_url().to_string()),
            model: model
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| kind.default_model().to_string()),
        })
    }
}

/// One chat message in provider-neutral shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// System + user messages in order.
pub fn build_messages(system: &str, user: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: "system".into(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".into(),
            content: user.to_string(),
        },
    ]
}

/// A planned (not yet sent) HTTP request. The engine logs/replays this; no
/// transport exists in v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubPlanned {
    pub method: String,
    pub url: String,
    pub model: String,
}

/// Plan the chat request for a resolved config: POST to the vendor chat URL.
pub fn plan(config: &ProviderConfig) -> StubPlanned {
    let url = match config.kind {
        ProviderKind::OpenAI => super::openai::chat_url(&config.base_url),
        ProviderKind::Anthropic => super::anthropic::chat_url(&config.base_url),
        ProviderKind::Gemini => super::gemini::chat_url(&config.base_url, &config.model),
        ProviderKind::Ollama => super::ollama::chat_url(&config.base_url),
        ProviderKind::OpenRouter => super::openrouter::chat_url(&config.base_url),
    };
    StubPlanned {
        method: "POST".into(),
        url,
        model: config.model.clone(),
    }
}

/// Build the vendor-specific request payload (no network).
pub fn build_payload(config: &ProviderConfig, system: &str, user: &str) -> serde_json::Value {
    match config.kind {
        ProviderKind::OpenAI => super::openai::build_payload(&config.model, system, user),
        ProviderKind::Anthropic => super::anthropic::build_payload(&config.model, system, user),
        ProviderKind::Gemini => super::gemini::build_payload(&config.model, system, user),
        ProviderKind::Ollama => super::ollama::build_payload(&config.model, system, user),
        ProviderKind::OpenRouter => super::openrouter::build_payload(&config.model, system, user),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(name: &str) -> ProviderConfig {
        ProviderConfig::resolve_with(name, None, None, None, &HashMap::new(), |_| None).unwrap()
    }

    #[test]
    fn from_name_aliases() {
        assert_eq!(
            ProviderKind::from_name("custom"),
            Some(ProviderKind::OpenAI)
        );
        assert_eq!(
            ProviderKind::from_name("GOOGLE"),
            Some(ProviderKind::Gemini)
        );
        assert_eq!(ProviderKind::from_name("nope"), None);
    }

    #[test]
    fn unknown_provider_errors() {
        assert!(
            ProviderConfig::resolve_with("nope", None, None, None, &HashMap::new(), |_| None)
                .is_err()
        );
    }

    #[test]
    fn env_priority_over_config_map() {
        let mut map = HashMap::new();
        map.insert("openai".to_string(), "cfg-key".to_string());
        let c = ProviderConfig::resolve_with("openai", None, None, None, &map, |_| {
            Some("env-key".to_string())
        })
        .unwrap();
        assert_eq!(c.api_key.as_deref(), Some("env-key"));
        let c2 = ProviderConfig::resolve_with("openai", None, None, None, &map, |_| None).unwrap();
        assert_eq!(c2.api_key.as_deref(), Some("cfg-key"));
    }

    #[test]
    fn ollama_keyless_ok() {
        let c = cfg("ollama");
        assert_eq!(c.api_key, None);
        assert_eq!(c.model, "llama3.1");
    }

    #[test]
    fn plan_is_post_with_model() {
        for name in ["openai", "anthropic", "gemini", "ollama", "openrouter"] {
            let p = plan(&cfg(name));
            assert_eq!(p.method, "POST");
            assert!(p.url.starts_with("http"), "{name}: {}", p.url);
            assert!(!p.model.is_empty());
        }
    }

    #[test]
    fn build_messages_order() {
        let m = build_messages("s", "u");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].role, "system");
        assert_eq!(m[1].role, "user");
    }
}
