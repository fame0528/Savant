use savant_core::error::SavantError;

/// MCP server instance exposing local tools.
pub struct McpServer {
    // state mapping
}

impl McpServer {
    /// Starts the server instance.
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn start(&self) -> Result<(), SavantError> {
        Err(SavantError::Unknown("MCP Server implementation is currently disabled in this epoch.".into()))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
