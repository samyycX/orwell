use lazy_static::lazy_static;
use ratatui::style::{Color, Style};

pub struct Theme {
    // Base colors
    base: Color,     // Base background
    mantle: Color,   // Slightly lighter background
    crust: Color,    // Darker background
    surface0: Color, // Surface color for input
    surface1: Color, // Surface color for messages

    // Text colors
    text: Color,     // Primary text
    subtext0: Color, // Secondary text
    subtext1: Color, // Tertiary text

    // Accent colors
    rosewater: Color, // Accent 1
    flamingo: Color,  // Accent 2
    pink: Color,      // Accent 3
    mauve: Color,     // Accent 4
    red: Color,       // Error
    maroon: Color,    // Warning
    peach: Color,     // Highlight
    yellow: Color,    // Attention
    green: Color,     // Success
    teal: Color,      // Info
    sky: Color,       // Link
    sapphire: Color,  // Border
    blue: Color,      // Primary
    lavender: Color,  // Secondary
}

impl Theme {
    fn catppuccin() -> Self {
        Self {
            // Base colors
            base: Color::Rgb(24, 24, 37),     // #1e1e2e
            mantle: Color::Rgb(36, 36, 35),   // #1e1e2e
            crust: Color::Rgb(17, 17, 27),    // #11111b
            surface0: Color::Rgb(49, 50, 68), // #313244
            surface1: Color::Rgb(69, 71, 90), // #45475a

            // Text colors
            text: Color::Rgb(255, 255, 255),     // #cdd6f4
            subtext0: Color::Rgb(166, 173, 200), // #a6adc8
            subtext1: Color::Rgb(186, 194, 222), // #bac2de

            // Accent colors
            rosewater: Color::Rgb(245, 224, 220), // #f5e0dc
            flamingo: Color::Rgb(242, 205, 205),  // #f2cdcd
            pink: Color::Rgb(245, 194, 231),      // #f5c2e7
            mauve: Color::Rgb(203, 166, 247),     // #cba6f7
            red: Color::Rgb(243, 139, 168),       // #f38ba8
            maroon: Color::Rgb(235, 160, 172),    // #eba0ac
            peach: Color::Rgb(250, 179, 135),     // #fab387
            yellow: Color::Rgb(249, 226, 175),    // #f9e2af
            green: Color::Rgb(166, 227, 161),     // #a6e3a1
            teal: Color::Rgb(148, 226, 213),      // #94e2d5
            sky: Color::Rgb(137, 220, 235),       // #89dceb
            sapphire: Color::Rgb(116, 199, 236),  // #74c7ec
            blue: Color::Rgb(89, 188, 241),       // #59b6f4
            lavender: Color::Rgb(180, 190, 254),  // #b4befe
        }
    }

    // Title style
    pub fn title_style(&self) -> Style {
        Style::default().fg(self.mauve).bg(self.crust)
    }

    // Input box style
    pub fn input_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.mantle)
    }

    // Focused input style
    pub fn input_focused_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.mantle)
    }

    // Message style
    pub fn message_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.mantle)
    }

    // Debug message style
    pub fn debug_style(&self) -> Style {
        Style::default().fg(self.subtext0).bg(self.mantle)
    }

    // Border style
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.lavender).bg(self.mantle)
    }

    // Cursor style
    pub fn cursor_style(&self) -> Style {
        Style::default().fg(self.blue).bg(self.surface0)
    }

    // Counter style
    pub fn counter_style(&self) -> Style {
        Style::default().fg(self.peach).bg(self.surface0)
    }

    // Timestamp style
    pub fn timestamp_style(&self) -> Style {
        Style::default().fg(Color::Rgb(100, 100, 100)) // Dark gray color
    }

    // Error style
    pub fn error_style(&self) -> Style {
        Style::default().fg(self.red).bg(self.mantle)
    }

    // Highlight style
    pub fn highlight_style(&self) -> Style {
        Style::default().fg(self.peach).bg(self.mantle)
    }
}

lazy_static! {
    pub static ref THEME: Theme = Theme::catppuccin();
}
