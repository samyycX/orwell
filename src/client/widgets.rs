use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::THEME;

pub struct MultiInput {
    inputs: HashMap<String, String>,
    focused_id: Option<String>,
    cursor_position: usize,
    cursor_visible: bool,
    style: Style,
    block: Option<Block<'static>>,
    width: u16,
    max_height: u16,
    current_height: u16,
}

impl Default for MultiInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiInput {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            focused_id: None,
            cursor_position: 0,
            cursor_visible: true,
            style: Style::default(),
            block: None,
            width: 0,
            max_height: 0,
            current_height: 0,
        }
    }

    pub fn add_input(&mut self, id: String, initial_text: String) {
        self.inputs.insert(id.clone(), initial_text.clone());
        if self.focused_id.is_none() {
            self.focused_id = Some(id.clone());
            self.cursor_position = initial_text.len();
        }
    }

    pub fn focus(&mut self, id: &str) {
        if self.inputs.contains_key(id) {
            self.focused_id = Some(id.to_string());
            if let Some(text) = self.inputs.get(id) {
                self.cursor_position = text.len();
            }
        }
    }

    pub fn handle_input(&mut self, input: &str) {
        if let Some(id) = &self.focused_id {
            if let Some(text) = self.inputs.get_mut(id) {
                let graphemes: Vec<&str> = text.graphemes(true).collect();
                let mut new_text = String::new();

                // Add text before cursor
                for i in 0..self.cursor_position {
                    if i < graphemes.len() {
                        new_text.push_str(graphemes[i]);
                    }
                }

                // Add new input
                new_text.push_str(input);

                // Add text after cursor
                for i in self.cursor_position..graphemes.len() {
                    new_text.push_str(graphemes[i]);
                }

                *text = new_text;
                self.cursor_position += input.graphemes(true).count();
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        if let Some(id) = &self.focused_id {
            if let Some(text) = self.inputs.get_mut(id) {
                if self.cursor_position > 0 {
                    let graphemes: Vec<&str> = text.graphemes(true).collect();
                    let mut new_text = String::new();

                    // Add all graphemes except the one before cursor
                    for i in 0..graphemes.len() {
                        if i != self.cursor_position - 1 {
                            new_text.push_str(graphemes[i]);
                        }
                    }

                    *text = new_text;
                    self.cursor_position -= 1;
                }
            }
        }
    }

    pub fn handle_delete(&mut self) {
        if let Some(id) = &self.focused_id {
            if let Some(text) = self.inputs.get_mut(id) {
                let graphemes: Vec<&str> = text.graphemes(true).collect();
                if self.cursor_position < graphemes.len() {
                    let mut new_text = String::new();

                    // Add all graphemes except the one at cursor
                    for i in 0..graphemes.len() {
                        if i != self.cursor_position {
                            new_text.push_str(graphemes[i]);
                        }
                    }

                    *text = new_text;
                }
            }
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(id) = &self.focused_id {
            if let Some(text) = self.inputs.get(id) {
                let grapheme_count = text.graphemes(true).count();
                if self.cursor_position < grapheme_count {
                    self.cursor_position += 1;
                }
            }
        }
    }

    pub fn get_text(&self, id: &str) -> Option<String> {
        self.inputs.get(id).map(|text| {
            // Remove any padding spaces and cursor character
            text.replace('|', "").trim_end().to_string()
        })
    }

    pub fn get_raw_text(&self, id: &str) -> Option<String> {
        self.inputs.get(id).map(|text| {
            if self.focused_id == Some(id.to_string()) {
                let mut text = text.clone();
                let cursor_pos = self.cursor_position.min(text.len());
                text.insert(cursor_pos, '|');
                text
            } else {
                text.clone()
            }
        })
    }

    pub fn get_text_ref(&self, id: &str) -> Option<&str> {
        self.inputs.get(id).map(|s| s.as_str())
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn get_focused_id(&self) -> Option<&str> {
        self.focused_id.as_deref()
    }

    pub fn set_focused_id(&mut self, id: Option<String>) {
        self.focused_id = id;
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn get_character_count(&self, id: &str) -> usize {
        self.inputs
            .get(id)
            .map_or(0, |text| text.graphemes(true).count())
    }

    pub fn get_current_height(&self) -> u16 {
        self.current_height
    }

    fn calculate_cursor_position(&self, text: &str, target_grapheme_pos: usize) -> (u16, u16) {
        let graphemes: Vec<&str> = text.graphemes(true).collect();
        let mut x = 0;
        let mut y = 0;
        let mut current_width = 0;

        for (i, grapheme) in graphemes.iter().enumerate() {
            if i == target_grapheme_pos {
                break;
            }

            let width = UnicodeWidthStr::width(*grapheme);
            if current_width + width > self.width as usize {
                y += 1;
                x = width as u16;
                current_width = width;
            } else {
                x += width as u16;
                current_width += width;
            }
        }

        (x, y)
    }

    fn wrap_text(&self, text: &str) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for grapheme in text.graphemes(true) {
            let width = UnicodeWidthStr::width(grapheme);
            if current_width + width > self.width as usize {
                lines.push(current_line);
                current_line = grapheme.to_string();
                current_width = width;
            } else {
                current_line.push_str(grapheme);
                current_width += width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }
}

impl Widget for &mut MultiInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block.clone().unwrap_or_else(|| {
            Block::default()
                .borders(Borders::ALL)
                .border_style(THEME.border_style())
        });
        let inner_area = block.inner(area);
        block.render(area, buf);

        self.width = inner_area.width;
        self.max_height = inner_area.height;
        self.current_height = 0;

        let mut y = inner_area.y;
        for (id, text) in &self.inputs {
            let is_focused = self.focused_id.as_ref() == Some(id);
            let style = if is_focused {
                THEME.input_focused_style()
            } else {
                THEME.input_style()
            };

            // Wrap text into lines
            let wrapped_lines = self.wrap_text(text);
            let prefix = ">> ".to_string();
            let prefix_width = UnicodeWidthStr::width(prefix.as_str());

            // Calculate cursor position
            let (cursor_x, cursor_y) = if is_focused {
                self.calculate_cursor_position(text, self.cursor_position)
            } else {
                (0, 0)
            };

            // Calculate character count
            let char_count = self.get_character_count(id);
            let count_text = format!("({})", char_count);
            let count_width = UnicodeWidthStr::width(count_text.as_str());

            // Check if we need to adjust the starting position
            let total_lines = wrapped_lines.len() as u16;
            let available_height = self.max_height - 1; // Leave space for character count
            let start_y = if total_lines > available_height {
                // When content overflows, start from the top and let it scroll down
                inner_area.y
            } else {
                y
            };

            // Calculate visible lines based on cursor position
            let mut visible_start = 0;
            if is_focused && total_lines > available_height {
                // If cursor is in the overflow area, adjust visible_start to show cursor
                if cursor_y >= available_height {
                    visible_start = (cursor_y + 1).saturating_sub(available_height) as usize;
                }
            }

            // First, calculate how many lines we need to render
            let total_visible_lines = if wrapped_lines.is_empty() {
                1 // At least one line for empty input
            } else {
                wrapped_lines.len()
            };

            // Render all visible lines
            for line_idx in visible_start..total_visible_lines {
                let current_y = start_y + (line_idx - visible_start) as u16;
                if current_y >= inner_area.y + inner_area.height {
                    break;
                }

                // Always fill the entire line with background color first
                for x in 0..inner_area.width {
                    buf.get_mut(inner_area.x + x, current_y).set_style(style);
                }

                // Then render the text content
                if wrapped_lines.is_empty() {
                    // Empty input case - just show prefix
                    if line_idx == 0 {
                        let line = Line::from(vec![Span::styled(prefix.clone(), style)]);
                        buf.set_line(inner_area.x, current_y, &line, inner_area.width);

                        // Show cursor after prefix if focused and visible
                        if is_focused && self.cursor_visible {
                            let cursor_pos = prefix_width;
                            if cursor_pos < inner_area.width as usize {
                                buf.get_mut(inner_area.x + cursor_pos as u16, current_y)
                                    .set_style(THEME.cursor_style());
                            }
                        }
                    }
                } else {
                    // Normal case with text
                    let line = &wrapped_lines[line_idx];
                    let display_text = if line_idx == 0 {
                        format!("{}{}", prefix, line)
                    } else {
                        format!("{}{}", " ".repeat(prefix_width), line)
                    };

                    let line = Line::from(vec![Span::styled(display_text, style)]);
                    buf.set_line(inner_area.x, current_y, &line, inner_area.width);

                    // Show cursor if this is the cursor line and cursor is visible
                    if is_focused && self.cursor_visible && line_idx == cursor_y as usize {
                        let cursor_pos = if line_idx == 0 {
                            prefix_width + cursor_x as usize
                        } else {
                            prefix_width + cursor_x as usize
                        };
                        if cursor_pos < inner_area.width as usize {
                            buf.get_mut(inner_area.x + cursor_pos as u16, current_y)
                                .set_style(THEME.cursor_style());
                        }
                    }
                }
            }

            // Fill any remaining lines with background color
            let remaining_lines = inner_area
                .height
                .saturating_sub((total_visible_lines - visible_start) as u16);
            for i in 0..remaining_lines {
                let current_y = start_y + (total_visible_lines - visible_start) as u16 + i;
                if current_y >= inner_area.y + inner_area.height {
                    break;
                }
                for x in 0..inner_area.width {
                    buf.get_mut(inner_area.x + x, current_y).set_style(style);
                }
            }

            // Render character count in the border
            let count_y = area.y + area.height - 1;
            let count_x = area.x + area.width - count_width as u16 - 1; // -1 to leave space for border
            let count_line = Line::from(vec![Span::styled(count_text, THEME.counter_style())]);
            buf.set_line(count_x, count_y, &count_line, count_width as u16);

            self.current_height = total_lines;
            y += total_lines;
        }
    }
}
