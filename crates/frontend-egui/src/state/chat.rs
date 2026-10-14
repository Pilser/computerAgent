//! Chat messages, draft, and send flag (egui-free, unit-testable).

use computer_agent_session_store::session::{MessageRole, Session};

/// One chat bubble: role + plain text + token footnote.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tokens: u64,
}

impl Message {
    /// Fresh message with a heuristic token estimate.
    pub fn new(role: MessageRole, content: &str) -> Self {
        Self {
            role,
            content: content.to_string(),
            tokens: Session::estimate_tokens(content),
        }
    }
}

/// Ordered bubbles plus composer draft and in-flight flag.
#[derive(Debug, Default)]
pub struct ChatLog {
    messages: Vec<Message>,
    pub draft: String,
    pub sending: bool,
}

impl ChatLog {
    /// Empty log, nothing sending.
    pub fn new() -> Self {
        Self::default()
    }

    /// All bubbles, oldest first.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Replace the log from a persisted session (oldest first).
    pub fn load_session(&mut self, session: &Session) {
        self.messages = session
            .messages
            .iter()
            .map(|m| Message::new(m.role, &m.content))
            .collect();
        self.sending = false;
    }

    /// Append a user bubble and clear the draft.
    pub fn push_user(&mut self, text: &str) {
        self.messages.push(Message::new(MessageRole::User, text));
        self.draft.clear();
    }

    /// Open an empty assistant bubble for streaming chunks.
    pub fn begin_assistant(&mut self) {
        self.messages.push(Message::new(MessageRole::Assistant, ""));
        self.sending = true;
    }

    /// Append streamed text to the trailing assistant bubble.
    pub fn append_chunk(&mut self, chunk: &str) {
        match self.messages.last_mut() {
            Some(last) if last.role == MessageRole::Assistant => {
                last.content.push_str(chunk);
                last.tokens = Session::estimate_tokens(&last.content);
            }
            _ => self
                .messages
                .push(Message::new(MessageRole::Assistant, chunk)),
        }
        self.sending = true;
    }

    /// Settle the trailing assistant bubble with the final reply.
    pub fn finish_assistant(&mut self, response: &str) {
        match self.messages.last_mut() {
            Some(last) if last.role == MessageRole::Assistant => {
                last.content = response.to_string();
                last.tokens = Session::estimate_tokens(&last.content);
            }
            _ => self
                .messages
                .push(Message::new(MessageRole::Assistant, response)),
        }
        self.sending = false;
    }

    /// Drop every bubble (threads switch / clear).
    pub fn clear(&mut self) {
        self.messages.clear();
        self.sending = false;
    }
}
