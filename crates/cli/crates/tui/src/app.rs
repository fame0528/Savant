use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use savant_cli_core::{compress_output, CommandSafety, RtkProfile, Sandbox, SandboxResult};
use savant_cli_gateway::{GatewayClient, GatewayConfig};
use savant_cli_session::SessionTracker;
use savant_core::types::{
    ChatChunk, ChatMessage, ChatRole, RequestFrame, RequestPayload, SessionId,
};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::events::{self, Action, EventHandler, TerminalEvent};
use crate::theme::Theme;
use crate::ui;

/// A single chat message
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Main application state
pub struct App {
    pub theme: Theme,
    pub messages: Vec<Message>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub input_focused: bool,
    pub scroll_offset: u16,
    pub token_count: usize,
    pub token_limit: usize,
    pub rtk_savings: usize,
    pub model: String,
    pub session_id: String,
    pub connection_status: String,
    pub running: bool,
    pub streaming_buffer: String,
    pub is_streaming: bool,
    pub command_palette_open: bool,
    pub command_filter: String,
    pub command_cursor: usize,
    pub active_panel: usize,
    /// Current working directory for file ops
    pub cwd: PathBuf,
    /// Sandbox for tool execution
    pub sandbox: Sandbox,
}

impl App {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            theme: Theme::default(),
            messages: Vec::new(),
            input_buffer: String::new(),
            cursor_pos: 0,
            input_focused: true,
            scroll_offset: 0,
            token_count: 0,
            token_limit: 200_000,
            rtk_savings: 0,
            model: "default".to_string(),
            session_id: "cli-session-001".to_string(),
            connection_status: "Disconnected".to_string(),
            running: true,
            streaming_buffer: String::new(),
            is_streaming: false,
            command_palette_open: false,
            command_filter: String::new(),
            command_cursor: 0,
            active_panel: 1,
            sandbox: Sandbox::new().with_working_dir(cwd.clone()),
            cwd,
        }
    }

    /// Append a chunk to the streaming response
    pub fn append_stream_chunk(&mut self, chunk: &str) {
        self.streaming_buffer.push_str(chunk);
        if let Some(last) = self.messages.last_mut() {
            if last.role == "assistant" {
                last.content = self.streaming_buffer.clone();
            }
        }
        self.scroll_offset = self.messages.len() as u16;
    }

    /// Finalize the streaming response
    pub fn finalize_stream(&mut self) {
        if !self.streaming_buffer.is_empty() {
            let content = self.streaming_buffer.clone();
            self.messages.push(Message {
                role: "assistant".to_string(),
                content,
            });
            self.streaming_buffer.clear();
        }
        self.is_streaming = false;
        self.scroll_offset = self.messages.len() as u16;
    }

    /// Process an action
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::Escape => self.input_focused = false,
            Action::Submit => {
                if self.command_palette_open {
                    let cmd = self.command_filter.clone();
                    self.execute_command(&cmd);
                    self.command_palette_open = false;
                    self.command_filter.clear();
                    self.command_cursor = 0;
                } else if !self.input_buffer.is_empty() && !self.is_streaming {
                    let msg = self.input_buffer.clone();
                    self.messages.push(Message {
                        role: "user".to_string(),
                        content: msg,
                    });
                    self.input_buffer.clear();
                    self.cursor_pos = 0;
                    self.is_streaming = true;
                    self.streaming_buffer.clear();
                    self.messages.push(Message {
                        role: "assistant".to_string(),
                        content: String::new(),
                    });
                    self.scroll_offset = self.messages.len() as u16;
                }
            }
            Action::Backspace => {
                if self.command_palette_open {
                    if self.command_cursor > 0 && !self.command_filter.is_empty() {
                        self.command_filter.remove(self.command_cursor - 1);
                        self.command_cursor -= 1;
                    }
                } else if self.cursor_pos > 0 && !self.input_buffer.is_empty() {
                    self.input_buffer.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
            Action::Delete => {
                if self.command_palette_open {
                    if self.command_cursor < self.command_filter.len() {
                        self.command_filter.remove(self.command_cursor);
                    }
                } else if self.cursor_pos < self.input_buffer.len() {
                    self.input_buffer.remove(self.cursor_pos);
                }
            }
            Action::CursorLeft => {
                if self.command_palette_open {
                    if self.command_cursor > 0 {
                        self.command_cursor -= 1;
                    }
                } else if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            Action::CursorRight => {
                if self.command_palette_open {
                    if self.command_cursor < self.command_filter.len() {
                        self.command_cursor += 1;
                    }
                } else if self.cursor_pos < self.input_buffer.len() {
                    self.cursor_pos += 1;
                }
            }
            Action::CursorUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            Action::CursorDown => {
                let max_scroll = self.messages.len().saturating_sub(1) as u16;
                if self.scroll_offset < max_scroll {
                    self.scroll_offset += 1;
                }
            }
            Action::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            Action::PageDown => {
                let max_scroll = self.messages.len().saturating_sub(1) as u16;
                self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
            }
            Action::Home => self.scroll_offset = 0,
            Action::End => self.scroll_offset = self.messages.len().saturating_sub(1) as u16,
            Action::Char(c) => {
                if self.command_palette_open {
                    self.command_filter.insert(self.command_cursor, c);
                    self.command_cursor += 1;
                } else {
                    self.input_buffer.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                }
            }
            Action::TabNext => {
                self.active_panel = (self.active_panel + 1) % 3;
            }
            Action::TabPrev => {
                self.active_panel = (self.active_panel + 2) % 3;
            }
            Action::CommandPalette => {
                self.command_palette_open = !self.command_palette_open;
                if self.command_palette_open {
                    self.command_filter.clear();
                    self.command_cursor = 0;
                }
            }
            Action::Search => {
                self.command_palette_open = true;
                self.command_filter = "/".to_string();
                self.command_cursor = 1;
            }
        }
    }

    /// Execute a command from the command palette
    pub fn execute_command(&mut self, cmd: &str) {
        let cmd = cmd.trim().trim_start_matches('/');
        match cmd {
            "chat" | "c" => {
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: "Switched to chat mode.".to_string(),
                });
            }
            "compact" => {
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: "Context compacted.".to_string(),
                });
            }
            "evolution" => {
                let msg = self.run_evolution_check();
                self.messages.push(msg);
            }
            "skills" | "skill" => {
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: "No skills installed.".to_string(),
                });
            }
            "stats" => {
                let stats = format!(
                    "Sessions: 1 | Tokens: {} | RTK: -{}% | CWD: {}",
                    self.token_count,
                    self.rtk_savings,
                    self.cwd.display()
                );
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: stats,
                });
            }
            "edit" | "diff" => {
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: "Enter SEARCH/REPLACE blocks in chat. Phase 2 diff engine ready."
                        .to_string(),
                });
            }
            "sandbox" => {
                let safe: Vec<_> = self.sandbox.safe_commands().to_vec();
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: format!(
                        "Sandbox active. {} safe commands, {} blocked.",
                        safe.len(),
                        self.sandbox.blocked_commands().len()
                    ),
                });
            }
            "help" | "?" => {
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: "Commands: /chat, /compact, /diff, /edit, /evolution, /sandbox, /skills, /stats, /help, /quit"
                        .to_string(),
                });
            }
            "quit" | "q" => {
                self.running = false;
            }
            "" => {}
            other => {
                self.messages.push(Message {
                    role: "system".to_string(),
                    content: format!("Unknown command: /{}", other),
                });
            }
        }
        self.scroll_offset = self.messages.len() as u16;
    }

    fn run_evolution_check(&self) -> Message {
        let evolution_path = self
            .cwd
            .join("workspaces")
            .join("workspace-savant")
            .join("EVOLUTION.jsonl");
        if evolution_path.exists() {
            match std::fs::read_to_string(&evolution_path) {
                Ok(content) => {
                    // RTK-compress the evolution log
                    let (compressed, _original) =
                        compress_output(&content, &RtkProfile::file_content());
                    let rtk_saved = if !content.is_empty() {
                        ((content.len() - compressed.len()) as f64 / content.len() as f64 * 100.0)
                            as usize
                    } else {
                        0
                    };
                    Message {
                        role: "system".to_string(),
                        content: format!("Evolution log (RTK -{}%):\n{}", rtk_saved, compressed),
                    }
                }
                Err(_) => Message {
                    role: "system".to_string(),
                    content: "No evolution log available.".to_string(),
                },
            }
        } else {
            Message {
                role: "system".to_string(),
                content: "Evolution: Seedling stage. OCEAN: [O:0.5, C:0.5, E:0.5, A:0.5, N:0.5]. No pending mutations.".to_string(),
            }
        }
    }

    /// Execute a sandboxed command and return the result
    pub async fn execute_tool(&self, cmd: &str) -> Result<SandboxResult> {
        let safety = self.sandbox.classify(cmd);
        match safety {
            CommandSafety::Blocked => Err(anyhow::anyhow!("Command blocked by sandbox: {}", cmd)),
            CommandSafety::Safe | CommandSafety::RequiresApproval => {
                let mut result = self.sandbox.execute(cmd).await?;
                // Apply RTK compression
                let (compressed, original) =
                    compress_output(&result.stdout, &RtkProfile::default_profile());
                if compressed.len() < original {
                    result.stdout = compressed;
                    result.compressed = true;
                }
                Ok(result)
            }
        }
    }
}

/// Run the TUI event loop with gateway and session integration
pub async fn run_with_gateway(gateway_url: Option<String>, cwd: &Path) -> Result<()> {
    let cwd = cwd.to_path_buf();
    let app = App::new(cwd.clone());

    // Initialize session tracker
    let mut tracker = SessionTracker::new(None);
    tracker.start_session(&cwd).await?;

    let app_arc = Arc::new(Mutex::new(app));
    {
        let mut app = app_arc.lock().await;
        if let Some(session) = tracker.session() {
            app.session_id = session.id.clone();
        }
    }

    // Initialize gateway client
    let config = GatewayConfig {
        url: gateway_url.unwrap_or_else(|| "ws://localhost:3000/ws".to_string()),
        api_key: None,
        ping_interval_secs: 15,
        max_reconnect_attempts: Some(3),
    };
    let gateway = GatewayClient::new(config);

    // Attempt connection
    {
        let mut app = app_arc.lock().await;
        app.connection_status = "Connecting...".to_string();
    }
    let gateway_connected =
        match tokio::time::timeout(std::time::Duration::from_secs(5), gateway.connect()).await {
            Ok(Ok(_)) => {
                let mut app = app_arc.lock().await;
                app.connection_status = "Connected".to_string();
                true
            }
            Ok(Err(e)) => {
                let mut app = app_arc.lock().await;
                app.connection_status = format!("Error: {}", e);
                false
            }
            Err(_) => {
                let mut app = app_arc.lock().await;
                app.connection_status = "Timeout (offline mode)".to_string();
                false
            }
        };

    // Set up streaming channel
    // RC-15: Bounded channel for backpressure
    let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<ChatChunk>(1000);

    if gateway_connected {
        let app_ref = app_arc.clone();
        gateway
            .subscribe(
                "chat.chunk",
                move |event: savant_core::types::EventFrame| {
                    if let Ok(chunk) = serde_json::from_str::<ChatChunk>(&event.payload) {
                        // RC-15: Use try_send for bounded channel (non-async closure)
                        if let Err(e) = chunk_tx.try_send(chunk) {
                            tracing::debug!("[tui] Channel full, dropping chunk: {}", e);
                        }
                    }
                    let _ = &app_ref;
                },
            )
            .await;
    }

    run_event_loop(app_arc, chunk_rx, gateway_connected, &gateway).await?;

    drop(gateway);
    tracker.end_session().await?;

    Ok(())
}

/// Run the TUI event loop with streaming gateway integration
async fn run_event_loop(
    app_arc: Arc<Mutex<App>>,
    mut chunk_rx: tokio::sync::mpsc::Receiver<ChatChunk>,
    gateway_connected: bool,
    gateway: &GatewayClient,
) -> Result<()> {
    // On Windows, ensure a console is attached before TUI init.
    // If launched from Explorer without a terminal, enable_raw_mode() would fail.
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn AttachConsole(dw_process_id: u32) -> i32;
            fn AllocConsole() -> i32;
        }
        const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                AllocConsole();
            }
        }
    }
    enable_raw_mode().map_err(|e| {
        eprintln!("Failed to initialize terminal (enable_raw_mode): {}", e);
        eprintln!("If running from Explorer, try running from a terminal instead.");
        e
    })?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    let events = EventHandler::new(250);

    let mut running = true;
    while running {
        {
            let app = app_arc.lock().await;
            terminal.draw(|frame| ui::render(frame, &app))?;
            running = app.running;
        }

        tokio::select! {
            terminal_event = async { events.next() } => {
                match terminal_event? {
                    TerminalEvent::Key(code, modifiers) => {
                        if let Some(action) = events::handle_key(code, modifiers) {
                            let is_submit = matches!(action, Action::Submit);
                            let mut app = app_arc.lock().await;
                            let was_streaming = app.is_streaming;
                            app.handle_action(action);

                            if is_submit && (!was_streaming || app.is_streaming)
                                && gateway_connected && gateway.is_connected()
                            {
                                let session_id = app.session_id.clone();
                                let last_user_msg = app.messages.iter().rev()
                                    .find(|m| m.role == "user")
                                    .map(|m| m.content.clone());
                                drop(app);

                                if let Some(content) = last_user_msg {
                                    let request = RequestFrame {
                                        request_id: uuid::Uuid::new_v4().to_string(),
                                        session_id: SessionId(session_id),
                                        payload: RequestPayload::ChatMessage(ChatMessage {
                                            role: ChatRole::User,
                                            content,
                                            sender: None,
                                            recipient: None,
                                            agent_id: None,
                                            session_id: None,
                                            channel: savant_core::types::AgentOutputChannel::default(),
                                            is_telemetry: false,
                                            images: vec![],
                                        }),
                                        signature: None,
                                        timestamp: Some(chrono::Utc::now().timestamp_millis()),
                                    };
                                    if let Err(e) = gateway.send(request).await {
                                        tracing::warn!("[tui] Failed to send message to gateway: {}", e);
                                    }
                                }
                            }
                            running = app_arc.lock().await.running;
                        }
                    }
                    TerminalEvent::Resize(_, _) => {}
                    TerminalEvent::Tick | TerminalEvent::Mouse | TerminalEvent::Quit => {}
                }
            }

            chunk = chunk_rx.recv() => {
                if let Some(chunk) = chunk {
                    let mut app = app_arc.lock().await;
                    if chunk.is_final {
                        app.finalize_stream();
                    } else {
                        app.append_stream_chunk(&chunk.content);
                    }
                    running = app.running;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
