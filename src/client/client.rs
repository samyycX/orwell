use std::{sync::RwLock, thread, time::Duration};

use anyhow::Result;
use lazy_static::lazy_static;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line as RatatuiLine, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::THEME;
use crate::widgets::MultiInput;
use crate::{
    message::{
        add_chat_message, add_debug_message, calculate_optimal_prefix_width, get_chat_messages,
        get_debug_messages, get_time_format, toggle_time_format, MessageLevel,
    },
    service::Service,
};

mod config;
mod key;
mod message;
mod network;
mod service;
mod theme;
mod widgets;

#[derive(PartialEq)]
enum Page {
    Chat,
}

struct State {
    logged: bool,
    connected: bool,
    processing: bool,
}

struct App {
    chat_input: MultiInput,
    current_page: Page,
    scroll_offset: u16,
}

impl App {
    fn new() -> Self {
        let mut chat_input = MultiInput::new();
        chat_input.set_style(THEME.input_style());
        chat_input.add_input("chat".to_string(), "".to_string());

        Self {
            chat_input,
            current_page: Page::Chat,
            scroll_offset: 0,
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if STATE.read().unwrap().processing {
            return;
        }
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                std::process::exit(0);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                toggle_time_format();
                add_debug_message(MessageLevel::Info, "时间格式已切换");
            }
            KeyCode::Enter => {
                if let Some(message) = self.chat_input.get_text("chat") {
                    if !message.trim().is_empty() {
                        if message.starts_with("/") {
                            Service::check_command(&message, self);
                        } else {
                            if let Err(e) = Service::broadcast_message(message) {
                                add_chat_message(format!("发送失败: {}", e));
                            } else {
                                add_debug_message(MessageLevel::Info, "发送成功");
                            }
                        }
                        // Clear the input by creating a new MultiInput
                        let mut new_input = MultiInput::new();
                        new_input.set_style(THEME.input_style());
                        new_input.add_input("chat".to_string(), "".to_string());
                        self.chat_input = new_input;
                        self.chat_input.focus("chat");
                        // Reset scroll offset when new message is added
                        self.scroll_offset = 0;
                    }
                }
            }
            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let current = self.chat_input.get_focused_id().map(|s| s.to_string());
                if current.is_none() {
                    self.chat_input.focus("chat");
                }
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.chat_input.handle_input(&c.to_string());
                }
            }
            KeyCode::Backspace => self.chat_input.handle_backspace(),
            KeyCode::Delete => self.chat_input.handle_delete(),
            KeyCode::Left => self.chat_input.move_cursor_left(),
            KeyCode::Right => self.chat_input.move_cursor_right(),
            KeyCode::Up => {
                // Scroll up
                if self.scroll_offset < get_chat_messages().len() as u16 {
                    self.scroll_offset += 1;
                }
            }
            KeyCode::Down => {
                // Scroll down
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::PageUp => {
                // Scroll up by page
                self.scroll_offset = self.scroll_offset.saturating_add(10);
            }
            KeyCode::PageDown => {
                // Scroll down by page
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            KeyCode::Esc => std::process::exit(0),
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

lazy_static! {
    static ref APP: RwLock<Option<App>> = RwLock::new(None);
    static ref STATE: RwLock<State> = RwLock::new(State {
        logged: false,
        connected: false,
        processing: false,
    });
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut app_guard = APP.write().unwrap();
    if app_guard.is_none() {
        app_guard.replace(App::new());
    }
    let mut app = app_guard.as_mut().unwrap();
    add_chat_message("W3LC0ME T0 0RW3LL");
    Service::check_login(&app);

    loop {
        terminal.draw(|frame| render(frame, &mut app))?;
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => app.handle_key_event(key),
                Event::Paste(text) => {
                    // Handle pasted text (including Chinese characters)
                    if let Some(id) = app.chat_input.get_focused_id() {
                        app.chat_input.handle_input(&text);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

fn render(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]);
    let [title_area, main_area] = vertical.areas(frame.area());

    let mut widget = Block::bordered()
        .title("0RW3LL")
        .style(THEME.title_style())
        .borders(Borders::ALL)
        .border_style(THEME.border_style());

    let state = STATE.read().unwrap();

    if !state.logged {
        widget = widget.title(RatatuiLine::from("UNLOGGED").style(Style::default().fg(Color::Red)));
    } else {
        widget = widget.title(RatatuiLine::from("LOGGED").style(Style::default().fg(Color::Green)));
    }

    if !state.connected {
        widget = widget.title(RatatuiLine::from("OFFLINE").style(Style::default().fg(Color::Red)));
    } else {
        widget = widget.title(RatatuiLine::from("ONLINE").style(Style::default().fg(Color::Green)));
    }

    // Render title with theme
    frame.render_widget(widget, title_area);

    let horizontal = Layout::horizontal([Constraint::Percentage(80), Constraint::Percentage(20)]);
    let [chat_area, debug_area] = horizontal.areas(main_area);

    // Chat area layout
    let chat_layout = Layout::vertical([Constraint::Min(0), Constraint::Length(6)]);
    let [messages_area, input_area] = chat_layout.areas(chat_area);

    // Get messages from the message manager
    let chat_messages = get_chat_messages();
    let debug_messages = get_debug_messages();

    // Handle chat messages with proper Line support
    let mut ratatui_lines: Vec<RatatuiLine> = Vec::new();
    let area_width = messages_area.width.saturating_sub(2) as usize; // Account for borders
    let prefix_width = calculate_optimal_prefix_width(&chat_messages); // Auto-calculated width
    let time_format = get_time_format();

    for msg in &chat_messages {
        // For each Line, we need to wrap it if it's too long
        let mut current_width = 0;
        let mut current_spans: Vec<Span> = Vec::new();

        // Use formatted spans with fixed-width prefix
        for span in msg.formatted_spans(prefix_width, time_format) {
            let content_string = span.content().to_string();
            let style = span.style();

            // Process each grapheme in the span
            for grapheme in content_string.graphemes(true) {
                let grapheme_width = UnicodeWidthStr::width(grapheme);

                if current_width + grapheme_width > area_width {
                    // Need to wrap - push current line and start new one
                    if !current_spans.is_empty() {
                        ratatui_lines.push(RatatuiLine::from(current_spans));
                        current_spans = Vec::new();
                        current_width = 0;
                    }
                }

                // Add grapheme to current span
                if let Some(last_span) = current_spans.last_mut() {
                    if last_span.style == style {
                        // Extend existing span with same style
                        let mut new_content = last_span.content.to_string();
                        new_content.push_str(grapheme);
                        *last_span = Span::styled(new_content, style);
                    } else {
                        // Different style, create new span
                        current_spans.push(Span::styled(grapheme.to_string(), style));
                    }
                } else {
                    // First span on line
                    current_spans.push(Span::styled(grapheme.to_string(), style));
                }

                current_width += grapheme_width;
            }
        }

        // Add the last line if it has content
        if !current_spans.is_empty() {
            ratatui_lines.push(RatatuiLine::from(current_spans));
        }
    }

    // Handle scrolling
    let visible_height = messages_area.height.saturating_sub(2) as usize; // Account for borders
    let total_lines = ratatui_lines.len();

    if total_lines > visible_height {
        app.scroll_offset = app.scroll_offset.min((total_lines - visible_height) as u16);
    } else {
        app.scroll_offset = 0;
    }

    // Apply scrolling - take the visible portion
    let start_idx = if total_lines > visible_height {
        total_lines.saturating_sub(visible_height + app.scroll_offset as usize)
    } else {
        0
    };
    let end_idx = if total_lines > visible_height {
        total_lines.saturating_sub(app.scroll_offset as usize)
    } else {
        total_lines
    };

    let visible_lines: Vec<RatatuiLine> = if start_idx < end_idx && start_idx < ratatui_lines.len()
    {
        ratatui_lines[start_idx..end_idx.min(ratatui_lines.len())].to_vec()
    } else {
        Vec::new()
    };

    // Create a block for the messages area with background
    let messages_block = Block::default()
        .title("Chat")
        .borders(Borders::ALL)
        .border_style(THEME.border_style())
        .style(THEME.message_style());

    let messages_widget = Paragraph::new(visible_lines)
        .block(messages_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(messages_widget, messages_area);

    // Render chat input
    frame.render_widget(&mut app.chat_input, input_area);

    // Render debug output
    let mut debug_lines: Vec<RatatuiLine> = Vec::new();
    for msg in &debug_messages {
        let level = msg.level();
        let content = msg.content();

        // Calculate available width for content (accounting for level)
        let level_width = UnicodeWidthStr::width(format!("[{}] ", level.to_string()).as_str());
        let content_width = debug_area.width.saturating_sub(2) as usize - level_width; // Account for borders

        // Wrap the content
        let mut current_line = String::new();
        let mut current_width = 0;
        let mut lines = Vec::new();

        // First, collect all wrapped lines
        for grapheme in content.graphemes(true) {
            let grapheme_width = UnicodeWidthStr::width(grapheme);
            if current_width + grapheme_width > content_width {
                lines.push(current_line);
                current_line = grapheme.to_string();
                current_width = grapheme_width;
            } else {
                current_line.push_str(grapheme);
                current_width += grapheme_width;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Then create the styled lines
        for (i, line) in lines.iter().enumerate() {
            let prefix = if i == 0 {
                // First line includes level
                format!("[{}] ", level.to_string())
            } else {
                // Subsequent lines are indented
                format!("{}", " ".repeat(level_width))
            };

            let spans = if i == 0 {
                // First line has level
                vec![
                    Span::styled(format!("[{}] ", level.to_string()), level.style()),
                    Span::styled(line.clone(), THEME.debug_style()),
                ]
            } else {
                // Subsequent lines only have content
                vec![
                    Span::styled(prefix, THEME.debug_style()),
                    Span::styled(line.clone(), THEME.debug_style()),
                ]
            };

            debug_lines.push(RatatuiLine::from(spans));
        }
    }

    // Only scroll if we have more messages than visible lines
    let debug_height = debug_area.height.saturating_sub(2) as usize; // Account for borders
    let scroll_offset = if debug_lines.len() > debug_height {
        (debug_lines.len() - debug_height) as u16
    } else {
        0
    };

    let debug_widget = Paragraph::new(debug_lines)
        .block(
            Block::default()
                .title("Debug")
                .borders(Borders::ALL)
                .border_style(THEME.border_style())
                .style(THEME.debug_style()),
        )
        .scroll((scroll_offset, 0));
    frame.render_widget(debug_widget, debug_area);
}
