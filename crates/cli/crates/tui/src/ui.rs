use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::theme::Theme;

/// Render the full TUI layout
pub fn render(frame: &mut Frame, app: &crate::app::App) {
    let theme = &app.theme;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, layout[0], theme);
    render_main_area(frame, layout[1], app, theme);
    render_input_area(frame, layout[2], app, theme);
    render_status_bar(frame, layout[3], app, theme);

    if app.command_palette_open {
        render_command_palette(frame, frame.area(), app, theme);
    }
}

/// Render the top header bar
fn render_header(frame: &mut Frame, area: Rect, theme: &Theme) {
    let header_text = format!(" SAVANT CLI COMPANION — v{} ", env!("CARGO_PKG_VERSION"));
    let header = Paragraph::new(header_text)
        .style(theme.header())
        .alignment(Alignment::Center);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .style(Style::default().bg(theme.background)),
        area,
    );
    let inner = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    frame.render_widget(header, inner);
}

/// Render the main chat area
fn render_main_area(frame: &mut Frame, area: Rect, app: &crate::app::App, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(area);

    render_sidebar(frame, chunks[0], app, theme);
    render_chat(frame, chunks[1], app, theme);
    render_context_panel(frame, chunks[2], app, theme);
}

/// Render the left sidebar
fn render_sidebar(frame: &mut Frame, area: Rect, _app: &crate::app::App, theme: &Theme) {
    let items = ["Sessions", "Agents", "Models", "History"];

    let content: String = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let prefix = if i == 0 { "▸ " } else { "  " };
            format!("{}{}", prefix, item)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sidebar = Paragraph::new(content)
        .style(theme.body_text())
        .wrap(Wrap { trim: false });

    frame.render_widget(
        Block::default()
            .title(" NAV ")
            .title_style(theme.panel_title())
            .borders(Borders::ALL)
            .border_style(theme.border())
            .style(Style::default().bg(theme.background)),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(sidebar, inner);
}

/// Render the main chat panel
fn render_chat(frame: &mut Frame, area: Rect, app: &crate::app::App, theme: &Theme) {
    let mut lines: Vec<Line> = Vec::new();

    if app.messages.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Welcome to Savant CLI Companion",
            theme.header(),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Type a message and press Enter to chat.",
            theme.body_text(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Press : for commands, / for search.",
            theme.dimmed(),
        )]));
    } else {
        for msg in &app.messages {
            let role_style = match msg.role.as_str() {
                "user" => theme.accent(),
                "assistant" => theme.success(),
                "system" => theme.dimmed(),
                _ => theme.body_text(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", msg.role), role_style),
                Span::styled(&msg.content, theme.body_text()),
            ]));
            lines.push(Line::from(""));
        }
    }

    let chat = Paragraph::new(lines)
        .style(theme.body_text())
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    frame.render_widget(
        Block::default()
            .title(" CHAT ")
            .title_style(theme.panel_title())
            .borders(Borders::ALL)
            .border_style(theme.border())
            .style(Style::default().bg(theme.background)),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(chat, inner);
}

/// Render the right context panel
fn render_context_panel(frame: &mut Frame, area: Rect, app: &crate::app::App, theme: &Theme) {
    let token_info = format!(
        "Tokens: {}\nRTK: -{}%\nModel: {}",
        app.token_count, app.rtk_savings, app.model
    );

    let context = Paragraph::new(token_info)
        .style(theme.code())
        .wrap(Wrap { trim: false });

    frame.render_widget(
        Block::default()
            .title(" CONTEXT ")
            .title_style(theme.panel_title())
            .borders(Borders::ALL)
            .border_style(theme.border())
            .style(Style::default().bg(theme.background)),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(context, inner);
}

/// Render the input area
fn render_input_area(frame: &mut Frame, area: Rect, app: &crate::app::App, theme: &Theme) {
    let input_style = if app.input_focused {
        Style::default()
            .fg(theme.success)
            .bg(theme.background)
            .add_modifier(Modifier::BOLD)
    } else {
        theme.input()
    };

    let border_style = if app.input_focused {
        theme.success()
    } else {
        theme.border()
    };

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(input_style)
        .wrap(Wrap { trim: false });

    let title = if app.input_focused {
        " INPUT (focused) "
    } else {
        " INPUT "
    };

    frame.render_widget(
        Block::default()
            .title(title)
            .title_style(theme.panel_title())
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(theme.background)),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    frame.render_widget(input, inner);

    if app.input_focused {
        frame.set_cursor_position((inner.x + app.cursor_pos as u16, inner.y));
    }
}

/// Render the bottom status bar
fn render_status_bar(frame: &mut Frame, area: Rect, app: &crate::app::App, theme: &Theme) {
    let status = format!(
        " {} | Tokens: {}/{} | RTK: -{}% | Session: {} | j/k scroll · i chat · : cmd · Tab panels · / search ",
        app.connection_status,
        app.token_count,
        app.token_limit,
        app.rtk_savings,
        app.session_id,
    );

    let status_bar = Paragraph::new(status)
        .style(theme.status_bar())
        .alignment(Alignment::Left);

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .style(Style::default().bg(theme.background)),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    frame.render_widget(status_bar, inner);
}

/// Render the command palette overlay
fn render_command_palette(frame: &mut Frame, area: Rect, app: &crate::app::App, theme: &Theme) {
    let commands = [
        ("chat", "Switch to chat mode"),
        ("compact", "Compact conversation context"),
        ("evolution", "Show evolution status"),
        ("skills", "List installed skills"),
        ("stats", "Show session statistics"),
        ("help", "Show available commands"),
        ("quit", "Exit Savant CLI"),
    ];

    let filter = app.command_filter.to_lowercase();
    let filtered: Vec<_> = commands
        .iter()
        .filter(|(name, desc)| name.contains(&filter) || desc.to_lowercase().contains(&filter))
        .collect();

    let palette_height = (filtered.len() as u16 + 3).min(12);
    let palette_width = 50.min(area.width - 4);

    let palette_area = Rect {
        x: (area.width - palette_width) / 2,
        y: area.height / 4,
        width: palette_width,
        height: palette_height,
    };

    frame.render_widget(
        Block::default()
            .title(" COMMAND PALETTE ")
            .title_style(theme.accent())
            .borders(Borders::ALL)
            .border_style(theme.accent())
            .style(Style::default().bg(theme.background)),
        palette_area,
    );

    let inner = palette_area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("> ", theme.accent()),
        Span::styled(&app.command_filter, theme.input()),
    ]));
    lines.push(Line::from(""));

    for (i, (name, desc)) in filtered.iter().enumerate() {
        let style = if i == app.command_cursor {
            theme.selection()
        } else {
            theme.body_text()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", name), style),
            Span::styled(desc.to_string(), theme.dimmed()),
        ]));
    }

    let palette_content = Paragraph::new(lines)
        .style(theme.body_text())
        .wrap(Wrap { trim: false });

    frame.render_widget(palette_content, inner);
}
