//! Tool registry: name -> prompt schema for the v1 coding profile.
//!
//! The registry carries schemas (name, description, JSON parameters) for
//! prompt assembly in phase 7. Actual dispatch happens in the engine (phase
//! 8); `register` / `list` / `schemas` are the full v1 surface.

use crate::tools::{BashTool, ReadTool, Tool, WriteTool};

/// One tool's prompt-facing schema.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Ordered set of tools offered to the model.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: Vec<ToolInfo>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool instance's schema. Re-registering a name replaces it.
    pub fn register<T: Tool>(&mut self, tool: &T) {
        let info = ToolInfo {
            name: T::NAME.to_string(),
            description: tool.description(),
            parameters: tool.parameters(),
        };
        if let Some(slot) = self.tools.iter_mut().find(|t| t.name == info.name) {
            *slot = info;
        } else {
            self.tools.push(info);
        }
    }

    /// Registered tool names, in registration order.
    pub fn list(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name.as_str()).collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    pub fn get(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.iter().find(|t| t.name == name)
    }

    pub fn schemas(&self) -> &[ToolInfo] {
        &self.tools
    }

    /// v1 coding profile: `read`, `write`, `bash` (no policy bound here;
    /// tools are constructed with their gate at dispatch time in phase 8).
    pub fn default_coding() -> Self {
        let mut r = Self::new();
        r.register(&ReadTool::new(None, None, None));
        r.register(&WriteTool::new(None, None));
        r.register(&BashTool::new(
            None,
            computer_agent_sys_tools::sandbox::Sandbox::default(),
            None,
        ));
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_lists_coding_tools() {
        let r = ToolRegistry::default_coding();
        assert_eq!(r.list(), vec!["read", "write", "bash"]);
    }

    #[test]
    fn schemas_carry_valid_parameters() {
        for info in ToolRegistry::default_coding().schemas() {
            assert_eq!(info.parameters["type"], "object");
            assert!(info.parameters.get("properties").is_some(), "{}", info.name);
        }
        let reg = ToolRegistry::default_coding();
        let read = reg.get("read").unwrap();
        assert!(read.description.contains("offset/limit"));
    }

    #[test]
    fn re_register_replaces_schema() {
        let mut r = ToolRegistry::default_coding();
        let n = r.schemas().len();
        r.register(&ReadTool::new(None, None, Some(10)));
        assert_eq!(r.schemas().len(), n);
        assert!(r.get("read").unwrap().description.contains("10 lines"));
    }
}
