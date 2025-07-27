use chrono::TimeZone;
use lazy_static::lazy_static;
use orwell::shared::helper::get_now_timestamp;
use ratatui::style::{Color, Style};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Debug,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeFormat {
    Short, // MM/DD HH:mm
    Full,  // YYYY/MM/DD HH:mm:ss
}

impl MessageLevel {
    #[allow(dead_code)]
    pub fn to_string(&self) -> &'static str {
        match self {
            MessageLevel::Info => "INFO",
            MessageLevel::Warning => "WARN",
            MessageLevel::Error => "ERROR",
            MessageLevel::Debug => "DEBUG",
            MessageLevel::Success => "SUCCESS",
        }
    }

    #[allow(dead_code)]
    pub fn style(&self) -> Style {
        match self {
            MessageLevel::Info => Style::default().fg(Color::Rgb(137, 220, 235)), // Sky blue
            MessageLevel::Warning => Style::default().fg(Color::Rgb(249, 226, 175)), // Yellow
            MessageLevel::Error => Style::default().fg(Color::Rgb(243, 139, 168)), // Red
            MessageLevel::Debug => Style::default().fg(Color::Rgb(166, 173, 200)), // Subtext0
            MessageLevel::Success => Style::default().fg(Color::Rgb(166, 227, 161)), // Green
        }
    }
}

/// A text span with specific styling
#[derive(Debug, Clone)]
pub struct TextSpan {
    content: String,
    style: Style,
}

impl TextSpan {
    pub fn new(content: impl Into<String>, style: Style) -> Self {
        Self {
            content: content.into(),
            style,
        }
    }

    pub fn plain(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn with_color(content: impl Into<String>, color: Color) -> Self {
        Self {
            content: content.into(),
            style: Style::default().fg(color),
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn style(&self) -> Style {
        self.style
    }
}

/// A line composed of multiple text spans with different styles
#[derive(Debug, Clone)]
pub struct Line {
    timestamp: u64,
    sender: TextSpan,
    spans: Vec<TextSpan>,
}

impl Line {
    pub fn new(timestamp: u64, sender: TextSpan) -> Self {
        Self {
            timestamp,
            sender,
            spans: vec![],
        }
    }

    pub fn push_span(&mut self, span: TextSpan) {
        self.spans.push(span);
    }

    pub fn push_plain(&mut self, content: impl Into<String>) {
        self.spans.push(TextSpan::plain(content));
    }

    pub fn push_styled(&mut self, content: impl Into<String>, style: Style) {
        self.spans.push(TextSpan::new(content, style));
    }

    pub fn push_colored(&mut self, content: impl Into<String>, color: Color) {
        self.spans.push(TextSpan::with_color(content, color));
    }

    pub fn spans(&self) -> &[TextSpan] {
        &self.spans
    }

    /// Format timestamp based on the given format
    pub fn format_timestamp(&self, format: TimeFormat) -> String {
        let datetime = chrono::DateTime::from_timestamp_millis(self.timestamp as i64)
            .unwrap_or_else(chrono::Utc::now);
        let utc8_time = datetime.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());

        match format {
            TimeFormat::Short => utc8_time.format("%m/%d %H:%M").to_string(),
            TimeFormat::Full => utc8_time.format("%Y/%m/%d %H:%M:%S.%3f").to_string(),
        }
    }

    /// Calculate the prefix width for this line with given time format
    pub fn calculate_prefix_width(&self, time_format: TimeFormat) -> usize {
        let time_str = self.format_timestamp(time_format);
        let time_width = time_str.len();
        let mut sender_width = self.sender.content().len();
        let separator_width = 3; // " | ".len()

        // correction
        sender_width = match self.sender.content() {
            "→" => 1,
            "←" => 1,
            "\u{f04b2}" => 1,
            "\u{f04b3}" => 1,
            _ => sender_width,
        };

        time_width + 1 + sender_width + separator_width // +1 for space between time and sender
    }

    /// Get formatted spans with fixed-width prefix (time sender | content)
    pub fn formatted_spans(&self, prefix_width: usize, time_format: TimeFormat) -> Vec<TextSpan> {
        let mut result = Vec::new();

        let time_str = self.format_timestamp(time_format);

        // Calculate required padding between time and sender
        let used_width = self.calculate_prefix_width(time_format);
        let padding = if used_width < prefix_width {
            prefix_width - used_width + 1
        } else {
            1 // At least one space
        };

        // Add formatted prefix parts
        result.push(TextSpan::with_color(
            time_str,
            *TIMESTAMP_STYLE.fg.as_ref().unwrap_or(&Color::Gray),
        ));
        result.push(TextSpan::plain(" ".repeat(padding)));
        result.push(self.sender.clone());
        result.push(TextSpan::plain(" | "));

        // Add content spans
        result.extend(self.spans.clone());

        result
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn sender(&self) -> &TextSpan {
        &self.sender
    }

    /// Convert to plain text (no styling)
    pub fn to_plain_text(&self) -> String {
        self.spans.iter().map(|span| span.content()).collect()
    }

    /// Check if the line is empty
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.spans.iter().all(|span| span.content().is_empty())
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new(
            get_now_timestamp(),
            TextSpan::with_color("*", Color::LightCyan),
        )
    }
}

impl From<String> for Line {
    fn from(content: String) -> Self {
        let mut line = Line::default();
        line.push_plain(content);
        line
    }
}

impl From<&str> for Line {
    fn from(content: &str) -> Self {
        let mut line = Line::default();
        line.push_plain(content);
        line
    }
}

#[derive(Debug, Clone)]
pub struct DebugMessage {
    level: MessageLevel,
    content: String,
}

impl DebugMessage {
    fn new(level: MessageLevel, content: String) -> Self {
        Self { level, content }
    }

    pub fn format(&self) -> String {
        format!("[{}] {}", self.level.to_string(), self.content)
    }

    pub fn level(&self) -> MessageLevel {
        self.level
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

lazy_static! {
    static ref MESSAGE_MANAGER: Mutex<MessageManager> = Mutex::new(MessageManager::new());
    static ref TIMESTAMP_STYLE: Style = Style::default().fg(Color::Rgb(100, 100, 100)); // Dark gray color
    static ref TIME_FORMAT: Mutex<TimeFormat> = Mutex::new(TimeFormat::Short);
}

pub struct MessageManager {
    chat_messages: Vec<Line>,
    debug_messages: Vec<DebugMessage>,
}

impl MessageManager {
    fn new() -> Self {
        Self {
            chat_messages: vec![],
            debug_messages: vec![],
        }
    }

    pub fn insert_chat_message(&mut self, index: usize, message: Line) {
        self.chat_messages.insert(index, message);
    }

    pub fn add_chat_message(&mut self, message: Line) {
        self.chat_messages.push(message);
    }

    pub fn add_debug_message(&mut self, level: MessageLevel, message: String) {
        let debug_message = DebugMessage::new(level, message);
        self.debug_messages.push(debug_message);
        if self.debug_messages.len() > 500 {
            self.debug_messages.remove(0);
        }
    }

    pub fn get_chat_messages(&self) -> &[Line] {
        &self.chat_messages
    }

    pub fn get_debug_messages(&self) -> &[DebugMessage] {
        &self.debug_messages
    }

    pub fn clear_chat_messages(&mut self) {
        self.chat_messages.clear();
    }

    pub fn clear_debug_messages(&mut self) {
        self.debug_messages.clear();
    }
}

// Public interface for other modules to use

/// Add a chat message with rich text support
pub fn add_chat_message_rich(line: Line, index: Option<usize>) {
    if let Ok(mut manager) = MESSAGE_MANAGER.lock() {
        // Notifier::flash_window();
        if let Some(index) = index {
            manager.insert_chat_message(index, line);
        } else {
            manager.add_chat_message(line);
        }
    }
}

/// Add a plain chat message (backward compatibility)
pub fn add_chat_message(message: impl Into<String>) {
    let line = Line::from(message.into());
    add_chat_message_rich(line, None);
}

pub fn add_debug_message(level: MessageLevel, message: impl Into<String>) {
    if let Ok(mut manager) = MESSAGE_MANAGER.lock() {
        manager.add_debug_message(level, message.into());
    }
}

/// Get chat messages as rich text lines
pub fn get_chat_messages() -> Vec<Line> {
    MESSAGE_MANAGER
        .lock()
        .map(|manager| manager.get_chat_messages().to_vec())
        .unwrap_or_default()
}

/// Get chat messages as plain text (backward compatibility)
pub fn get_chat_messages_plain() -> Vec<String> {
    get_chat_messages()
        .into_iter()
        .map(|line| line.to_plain_text())
        .collect()
}

pub fn get_debug_messages() -> Vec<DebugMessage> {
    MESSAGE_MANAGER
        .lock()
        .map(|manager| manager.get_debug_messages().to_vec())
        .unwrap_or_default()
}

pub fn clear_chat_messages() {
    if let Ok(mut manager) = MESSAGE_MANAGER.lock() {
        manager.clear_chat_messages();
    }
}

pub fn clear_debug_messages() {
    if let Ok(mut manager) = MESSAGE_MANAGER.lock() {
        manager.clear_debug_messages();
    }
}

// Convenience builder for creating rich text lines
pub struct LineBuilder {
    line: Line,
}

impl LineBuilder {
    pub fn new() -> Self {
        Self {
            line: Line::default(),
        }
    }

    pub fn time(mut self, timestamp: u64) -> Self {
        self.line.timestamp = timestamp;
        self
    }

    pub fn sender(mut self, sender: TextSpan) -> Self {
        self.line.sender = sender;
        self
    }

    pub fn plain(mut self, content: impl Into<String>) -> Self {
        self.line.push_plain(content);
        self
    }

    pub fn styled(mut self, content: impl Into<String>, style: Style) -> Self {
        self.line.push_styled(content, style);
        self
    }

    pub fn colored(mut self, content: impl Into<String>, color: Color) -> Self {
        self.line.push_colored(content, color);
        self
    }

    pub fn info(mut self, content: impl Into<String>) -> Self {
        self.line.push_styled(content, MessageLevel::Info.style());
        self
    }

    pub fn warning(mut self, content: impl Into<String>) -> Self {
        self.line
            .push_styled(content, MessageLevel::Warning.style());
        self
    }

    pub fn error(mut self, content: impl Into<String>) -> Self {
        self.line.push_styled(content, MessageLevel::Error.style());
        self
    }

    pub fn debug(mut self, content: impl Into<String>) -> Self {
        self.line.push_styled(content, MessageLevel::Debug.style());
        self
    }

    pub fn success(mut self, content: impl Into<String>) -> Self {
        self.line
            .push_styled(content, MessageLevel::Success.style());
        self
    }

    pub fn build(self) -> Line {
        self.line
    }
}

impl Default for LineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate optimal prefix width for all messages
pub fn calculate_optimal_prefix_width(messages: &[Line]) -> usize {
    let time_format = *TIME_FORMAT.lock().unwrap();
    let min_width = 20;

    let max_width = messages
        .iter()
        .map(|msg| msg.calculate_prefix_width(time_format))
        .max()
        .unwrap_or(min_width);

    max_width.max(min_width)
}

/// Get current time format
pub fn get_time_format() -> TimeFormat {
    *TIME_FORMAT.lock().unwrap()
}

/// Toggle time format between Short and Full
pub fn toggle_time_format() {
    let mut format = TIME_FORMAT.lock().unwrap();
    *format = match *format {
        TimeFormat::Short => TimeFormat::Full,
        TimeFormat::Full => TimeFormat::Short,
    };
}
