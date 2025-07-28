use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line as RatatuiLine, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    message::{calculate_optimal_prefix_width, get_time_format, DebugMessage, Line},
    service::{ClientManager, Service},
    theme::{Theme, THEME},
};
use orwell::pb::orwell::ClientStatus;

/// 聊天消息渲染器
pub struct ChatRenderer;

impl ChatRenderer {
    /// 渲染聊天消息区域
    pub fn render(
        _frame: &mut Frame,
        area: Rect,
        messages: &[Line],
        _scroll_offset: u16,
    ) -> Vec<RatatuiLine<'static>> {
        let mut ratatui_lines: Vec<RatatuiLine> = Vec::new();
        let area_width = area.width.saturating_sub(2) as usize; // Account for borders
        let prefix_width = calculate_optimal_prefix_width(messages); // Auto-calculated width
        let time_format = get_time_format();

        for msg in messages {
            // Get formatted spans with fixed-width prefix
            let formatted_spans = msg.formatted_spans(prefix_width, time_format);

            // Separate prefix spans from content spans
            let mut prefix_spans = Vec::new();
            let mut content_spans = Vec::new();
            let mut prefix_complete = false;

            for span in formatted_spans {
                let content_str = span.content().to_string();
                let span_style = span.style();

                if !prefix_complete && content_str.contains(" | ") {
                    // This is the last prefix span, split it
                    if let Some(pipe_pos) = content_str.find(" | ") {
                        let prefix_part = &content_str[..pipe_pos + 3]; // Include " | "
                        let content_part = &content_str[pipe_pos + 3..];

                        prefix_spans.push(Span::styled(prefix_part.to_string(), span_style));
                        if !content_part.is_empty() {
                            content_spans.push(Span::styled(content_part.to_string(), span_style));
                        }
                        prefix_complete = true;
                    }
                } else if !prefix_complete {
                    prefix_spans.push(Span::styled(content_str, span_style));
                } else {
                    content_spans.push(Span::styled(content_str, span_style));
                }
            }

            // Calculate prefix width for continuation lines
            let prefix_text_width = prefix_spans
                .iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>();

            // Calculate time span width (first span should be the timestamp)
            let time_span_width = if !prefix_spans.is_empty() {
                UnicodeWidthStr::width(prefix_spans[0].content.as_ref())
            } else {
                0
            };

            // Calculate how much padding needed after timestamp for continuation lines
            let continuation_padding_width = prefix_text_width.saturating_sub(time_span_width + 3); // -3 for " | "
            let continuation_padding = " ".repeat(continuation_padding_width);

            // Process content spans with wrapping
            let mut current_line_spans = prefix_spans.clone();
            let mut current_width = prefix_text_width;

            for span in content_spans {
                let content_string = span.content.to_string();
                let style = span.style;

                // Process each grapheme in the span
                for grapheme in content_string.graphemes(true) {
                    let grapheme_width = UnicodeWidthStr::width(grapheme);

                    if current_width + grapheme_width > area_width {
                        // Need to wrap - push current line and start new one
                        if !current_line_spans.is_empty() {
                            ratatui_lines.push(RatatuiLine::from(current_line_spans.clone()));
                        }

                        // Start new line with continuation prefix (timestamp + padding + |)
                        current_line_spans = if !prefix_spans.is_empty() {
                            vec![
                                prefix_spans[0].clone(), // Keep the timestamp
                                Span::styled(continuation_padding.clone(), Style::default()),
                                Span::styled(" | ", Style::default()),
                            ]
                        } else {
                            vec![
                                Span::styled(" ".repeat(prefix_text_width), Style::default()),
                                Span::styled(" | ", Style::default()),
                            ]
                        };
                        current_width = prefix_text_width; // Reset to the base prefix width
                    }

                    // Add grapheme to current span
                    if let Some(last_span) = current_line_spans.last_mut() {
                        if last_span.style == style {
                            // Extend existing span with same style
                            let mut new_content = last_span.content.to_string();
                            new_content.push_str(grapheme);
                            *last_span = Span::styled(new_content, style);
                        } else {
                            // Different style, create new span
                            current_line_spans.push(Span::styled(grapheme.to_string(), style));
                        }
                    } else {
                        // First span on line
                        current_line_spans.push(Span::styled(grapheme.to_string(), style));
                    }

                    current_width += grapheme_width;
                }
            }

            // Add the last line if it has content
            if !current_line_spans.is_empty() {
                ratatui_lines.push(RatatuiLine::from(current_line_spans));
            }
        }

        ratatui_lines
    }

    /// 处理滚动逻辑并返回可见的行
    pub fn handle_scrolling(
        lines: Vec<RatatuiLine>,
        area: Rect,
        scroll_offset: u16,
    ) -> (Vec<RatatuiLine>, u16) {
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        let total_lines = lines.len();

        let adjusted_scroll_offset = if total_lines > visible_height {
            scroll_offset.min((total_lines - visible_height) as u16)
        } else {
            0
        };

        // Apply scrolling - take the visible portion
        let start_idx = if total_lines > visible_height {
            total_lines.saturating_sub(visible_height + adjusted_scroll_offset as usize)
        } else {
            0
        };
        let end_idx = if total_lines > visible_height {
            total_lines.saturating_sub(adjusted_scroll_offset as usize)
        } else {
            total_lines
        };

        let visible_lines: Vec<RatatuiLine> = if start_idx < end_idx && start_idx < lines.len() {
            lines[start_idx..end_idx.min(lines.len())].to_vec()
        } else {
            Vec::new()
        };

        (visible_lines, adjusted_scroll_offset)
    }

    /// 创建聊天消息的 Paragraph widget
    pub fn create_widget(lines: Vec<RatatuiLine>) -> Paragraph {
        let messages_block = Block::default()
            .title("Chat")
            .borders(Borders::ALL)
            .border_style(THEME.border_style())
            .style(THEME.message_style());

        Paragraph::new(lines)
            .block(messages_block)
            .wrap(Wrap { trim: true })
    }
}

/// 调试消息渲染器
pub struct DebugRenderer;

impl DebugRenderer {
    /// 渲染调试消息区域
    pub fn render(area: Rect, messages: &[DebugMessage]) -> Vec<RatatuiLine> {
        let mut debug_lines: Vec<RatatuiLine> = Vec::new();

        for msg in messages {
            let level = msg.level();
            let content = msg.content();

            // Calculate available width for content (accounting for level)
            let level_width = UnicodeWidthStr::width(format!("[{}] ", level.to_string()).as_str());
            let content_width = area.width.saturating_sub(2) as usize - level_width; // Account for borders

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
                    " ".repeat(level_width).to_string()
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

        debug_lines
    }

    /// 创建调试消息的 Paragraph widget
    pub fn create_widget(lines: Vec<RatatuiLine>, area: Rect) -> Paragraph {
        // Only scroll if we have more messages than visible lines
        let debug_height = area.height.saturating_sub(2) as usize; // Account for borders
        let scroll_offset = if lines.len() > debug_height {
            (lines.len() - debug_height) as u16
        } else {
            0
        };

        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Debug")
                    .borders(Borders::ALL)
                    .border_style(THEME.border_style())
                    .style(THEME.debug_style()),
            )
            .scroll((scroll_offset, 0))
    }
}

/// 状态信息渲染器
pub struct StateRenderer;

impl StateRenderer {
    /// 渲染状态信息区域（连接状态）
    pub fn render_connected(state: &crate::State) -> Vec<RatatuiLine> {
        let mut state_text = vec![
            RatatuiLine::from(vec![Span::styled(
                format!(" \u{eb50} {}", state.server_url),
                Style::default().fg(Theme::catppuccin().lavender),
            )]),
            RatatuiLine::from(vec![Span::styled(
                format!(" \u{eae8} {} B", state.processed_bytes),
                Style::default().fg(Theme::catppuccin().lavender),
            )]),
            RatatuiLine::from(vec![Span::styled(
                format!(" \u{f013} 棘轮转动 {} 次", state.ratchet_roll_time),
                Style::default().fg(Theme::catppuccin().lavender),
            )]),
            RatatuiLine::from(vec![Span::styled(
                format!(" \u{f199f} {}", Service::get_online_time(state.start_time)),
                Style::default().fg(Theme::catppuccin().lavender),
            )]),
            RatatuiLine::from(vec![]),
            RatatuiLine::from(vec![Span::styled(
                format!(
                    "\u{f007} 用户列表 ({}/{})",
                    ClientManager::get_all_clients()
                        .iter()
                        .filter(|c| c.status == ClientStatus::Online)
                        .count(),
                    ClientManager::get_all_clients().len(),
                ),
                Style::default().fg(Theme::catppuccin().lavender),
            )]),
        ];

        ClientManager::get_all_clients_sorted()
            .iter()
            .for_each(|client| {
                state_text.push(RatatuiLine::from(vec![
                    Span::styled(
                        "\u{f1eb} ".to_string(),
                        Style::default().fg(match client.status {
                            ClientStatus::Online => Theme::catppuccin().green,
                            ClientStatus::Offline => Theme::catppuccin().red,
                            ClientStatus::Afk => Theme::catppuccin().yellow,
                        }),
                    ),
                    Span::styled(
                        client.name.to_string(),
                        Style::default().fg(Theme::catppuccin().lavender),
                    ),
                ]));
            });

        state_text
    }

    /// 渲染状态信息区域（未连接状态）
    pub fn render_disconnected<'a>() -> Vec<RatatuiLine<'a>> {
        vec![RatatuiLine::from(vec![Span::styled(
            "未连接",
            Style::default().fg(Theme::catppuccin().lavender),
        )])]
    }

    /// 创建状态信息的 Paragraph widget
    pub fn create_widget(lines: Vec<RatatuiLine>) -> Paragraph {
        let state_block = Block::default()
            .title("State")
            .borders(Borders::ALL)
            .border_style(THEME.border_style())
            .style(THEME.message_style());

        Paragraph::new(lines)
            .block(state_block)
            .wrap(Wrap { trim: true })
    }
}
