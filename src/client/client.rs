use std::{sync::RwLock, time::Duration};

use anyhow::Result;
use lazy_static::lazy_static;
use orwell::{pb::orwell::ClientStatus, shared::helper::get_hash_version};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Line as RatatuiLine,
    widgets::{Block, Borders},
    DefaultTerminal, Frame,
};

use crate::{
    command_adapter::{CommandAdapterRegistry, CommandContext},
    commands::create_command_registry,
    message::{
        add_chat_message, add_debug_message, get_chat_messages, get_debug_messages,
        toggle_time_format, MessageLevel,
    },
    renderer::{ChatRenderer, DebugRenderer, StateRenderer},
    service::Service,
};
use crate::{theme::THEME, widgets::MultiInput};

mod adapters;
mod command_adapter;
mod commands;
mod config;
mod key;
mod message;
mod message_adapter;
mod message_adapters;
mod network;
mod notify;
mod packet_adapter;
mod renderer;
mod service;
mod theme;
mod widgets;

#[derive(PartialEq)]
enum Page {
    Chat,
}
pub struct State {
    pub server_url: String,
    pub logged: bool,
    pub connected: bool,
    pub processing: bool,
    pub processed_bytes: u64,
    pub ratchet_roll_time: u64,
    pub start_time: u64,
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
        let state = STATE.read().unwrap();
        if state.processing {
            return;
        }
        drop(state);
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
                            let registry = COMMAND_REGISTRY.read().unwrap();
                            let context = CommandContext { app: self };
                            if let Err(e) = registry.process_command(&message, context) {
                                add_chat_message(format!("命令执行失败: {}", e));
                            }
                        } else if let Err(e) = Service::broadcast_message(message) {
                            add_chat_message(format!("发送失败: {}", e));
                        } else {
                            add_debug_message(MessageLevel::Info, "发送成功");
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
        server_url: "".to_string(),
        logged: false,
        connected: false,
        processing: false,
        processed_bytes: 0,
        ratchet_roll_time: 0,
        start_time: 0,
    });
    static ref COMMAND_REGISTRY: std::sync::RwLock<CommandAdapterRegistry> =
        std::sync::RwLock::new(create_command_registry());
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut app_guard = APP.write().unwrap();
    if app_guard.is_none() {
        app_guard.replace(App::new());
    }
    let app = app_guard.as_mut().unwrap();
    add_chat_message("W3LC0ME T0 0RW3LL");
    add_chat_message(format!("VERSION={}", get_hash_version()));
    Service::check_login(app);

    let mut sleep_time = 200;

    loop {
        terminal.draw(|frame| render(frame, app))?;
        if event::poll(Duration::from_millis(sleep_time))? {
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
        if notify::Notifier::is_focused() {
            // If the console is focused, reset sleep time to 500ms
            sleep_time = 200;
        } else {
            // If not focused, increase sleep time to reduce CPU usage
            sleep_time = 3000;
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
    let [chat_area, right_area] = horizontal.areas(main_area);

    let vertical = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
    let [state_area, debug_area] = vertical.areas(right_area);

    // Chat area layout
    let chat_layout = Layout::vertical([Constraint::Min(0), Constraint::Length(6)]);
    let [messages_area, input_area] = chat_layout.areas(chat_area);

    // Get messages from the message manager
    let chat_messages = get_chat_messages();
    let debug_messages = get_debug_messages();

    // Render chat messages using ChatRenderer
    let ratatui_lines =
        ChatRenderer::render(frame, messages_area, &chat_messages, app.scroll_offset);
    let (visible_lines, adjusted_scroll_offset) =
        ChatRenderer::handle_scrolling(ratatui_lines, messages_area, app.scroll_offset);
    app.scroll_offset = adjusted_scroll_offset;

    let messages_widget = ChatRenderer::create_widget(visible_lines);
    frame.render_widget(messages_widget, messages_area);

    // Render chat input
    frame.render_widget(&mut app.chat_input, input_area);

    // Render debug output using DebugRenderer
    let debug_lines = DebugRenderer::render(debug_area, &debug_messages);
    let debug_widget = DebugRenderer::create_widget(debug_lines, debug_area);
    frame.render_widget(debug_widget, debug_area);

    // Render state information using StateRenderer
    let state_lines = if state.connected {
        StateRenderer::render_connected(&state)
    } else {
        StateRenderer::render_disconnected()
    };
    let state_widget = StateRenderer::create_widget(state_lines);
    frame.render_widget(state_widget, state_area);
}
