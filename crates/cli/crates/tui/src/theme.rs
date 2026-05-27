use ratatui::style::{Color, Modifier, Style};

/// Cyberpunk neon color palette for the Savant CLI TUI
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub primary: Color,
    pub alert: Color,
    pub warning: Color,
    pub success: Color,
    pub accent: Color,
}

impl Theme {
    /// Void Indigo — default cyberpunk theme
    pub const VOID_INDIGO: Self = Self {
        background: Color::Rgb(0, 14, 26),
        primary: Color::Rgb(0, 136, 255),
        alert: Color::Rgb(255, 0, 0),
        warning: Color::Rgb(238, 255, 0),
        success: Color::Rgb(26, 255, 0),
        accent: Color::Rgb(255, 0, 230),
    };

    /// Body text style (green on void)
    pub fn body_text(&self) -> Style {
        Style::default().fg(self.success).bg(self.background)
    }

    /// Header style (blue bold on void)
    pub fn header(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    /// Code style (blue on void)
    pub fn code(&self) -> Style {
        Style::default().fg(self.primary).bg(self.background)
    }

    /// Border style (blue at reduced intensity)
    pub fn border(&self) -> Style {
        Style::default().fg(self.primary).bg(self.background)
    }

    /// Error style (red on void)
    pub fn error(&self) -> Style {
        Style::default()
            .fg(self.alert)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    /// Warning style (yellow on void)
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning).bg(self.background)
    }

    /// Success style (green on void)
    pub fn success(&self) -> Style {
        Style::default()
            .fg(self.success)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    /// Accent style (magenta on void)
    pub fn accent(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    /// Dimmed style (primary at low intensity)
    pub fn dimmed(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(0, 68, 128))
            .bg(self.background)
    }

    /// Input area style (void with blue border when focused)
    pub fn input(&self) -> Style {
        Style::default().fg(self.success).bg(self.background)
    }

    /// Status bar style
    pub fn status_bar(&self) -> Style {
        Style::default().fg(self.primary).bg(self.background)
    }

    /// Panel title style
    pub fn panel_title(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    /// Selection highlight style
    pub fn selection(&self) -> Style {
        Style::default()
            .fg(self.success)
            .bg(Color::Rgb(0, 40, 0))
            .add_modifier(Modifier::BOLD)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::VOID_INDIGO
    }
}
