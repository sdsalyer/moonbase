use crossterm::{
    QueueableCommand,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::Write;

#[derive(Debug, Clone)]
pub struct BoxStyle {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
    pub cross: char,
    pub tee_down: char,
    pub tee_up: char,
    pub tee_left: char,
    pub tee_right: char,
}

impl BoxStyle {
    pub fn double() -> Self {
        Self {
            top_left: '╔',
            top_right: '╗',
            bottom_left: '╚',
            bottom_right: '╝',
            horizontal: '═',
            vertical: '║',
            cross: '╬',
            tee_down: '╦',
            tee_up: '╩',
            tee_left: '╣',
            tee_right: '╠',
        }
    }

    pub fn single() -> Self {
        Self {
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
            horizontal: '─',
            vertical: '│',
            cross: '┼',
            tee_down: '┬',
            tee_up: '┴',
            tee_left: '┤',
            tee_right: '├',
        }
    }

    pub fn rounded() -> Self {
        Self {
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
            horizontal: '─',
            vertical: '│',
            cross: '┼',
            tee_down: '┬',
            tee_up: '┴',
            tee_left: '┤',
            tee_right: '├',
        }
    }

    pub fn ascii() -> Self {
        Self {
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
            horizontal: '-',
            vertical: '|',
            cross: '+',
            tee_down: '+',
            tee_up: '+',
            tee_left: '+',
            tee_right: '+',
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoxRenderer {
    pub style: BoxStyle,
    pub default_color: Option<Color>,
}

impl BoxRenderer {
    pub fn new(style: BoxStyle) -> Self {
        Self {
            style,
            default_color: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.default_color = Some(color);
        self
    }

    /// Render a simple box with title
    pub fn render_title_box<W: Write>(
        &self,
        writer: &mut W,
        title: &str,
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        let box_color = color.or(self.default_color);

        if let Some(c) = box_color {
            writer.queue(SetForegroundColor(c))?;
        }

        // Top border with title
        let title_len = title.chars().count();
        let padding = if width > title_len + 4 {
            (width - title_len - 4) / 2
        } else {
            0
        };

        writer.queue(Print(self.style.top_left))?;

        // Left padding
        for _ in 0..padding {
            writer.queue(Print(self.style.horizontal))?;
        }

        // Title
        writer.queue(Print(format!(" {} ", title)))?;

        // Right padding
        let remaining = width.saturating_sub(2 + padding + title_len + 2);
        for _ in 0..remaining {
            writer.queue(Print(self.style.horizontal))?;
        }

        writer.queue(Print(self.style.top_right))?;
        writer.queue(Print('\n'))?;

        if let Some(_) = box_color {
            writer.queue(ResetColor)?;
        }

        Ok(())
    }

    /// Render a content line within a box
    pub fn render_content_line<W: Write>(
        &self,
        writer: &mut W,
        content: &str,
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        let box_color = color.or(self.default_color);

        if let Some(c) = box_color {
            writer.queue(SetForegroundColor(c))?;
        }

        writer.queue(Print(self.style.vertical))?;

        if let Some(_) = box_color {
            writer.queue(ResetColor)?;
        }

        // Content with padding
        let content_len = content.chars().count();
        let total_padding = width.saturating_sub(2);

        if content_len <= total_padding {
            writer.queue(Print(format!(
                " {:<width$} ",
                content,
                width = total_padding.saturating_sub(2)
            )))?;
        } else {
            let truncated: String = content
                .chars()
                .take(total_padding.saturating_sub(5))
                .collect();
            writer.queue(Print(format!(" {}... ", truncated)))?;
        }

        if let Some(c) = box_color {
            writer.queue(SetForegroundColor(c))?;
        }

        writer.queue(Print(self.style.vertical))?;
        writer.queue(Print('\n'))?;

        if let Some(_) = box_color {
            writer.queue(ResetColor)?;
        }

        Ok(())
    }

    /// Render a separator line (middle of box)
    pub fn render_separator<W: Write>(
        &self,
        writer: &mut W,
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        let box_color = color.or(self.default_color);

        if let Some(c) = box_color {
            writer.queue(SetForegroundColor(c))?;
        }

        writer.queue(Print(self.style.tee_right))?;

        for _ in 0..width.saturating_sub(2) {
            writer.queue(Print(self.style.horizontal))?;
        }

        writer.queue(Print(self.style.tee_left))?;
        writer.queue(Print('\n'))?;

        if let Some(_) = box_color {
            writer.queue(ResetColor)?;
        }

        Ok(())
    }

    /// Render bottom border
    pub fn render_bottom<W: Write>(
        &self,
        writer: &mut W,
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        let box_color = color.or(self.default_color);

        if let Some(c) = box_color {
            writer.queue(SetForegroundColor(c))?;
        }

        writer.queue(Print(self.style.bottom_left))?;

        for _ in 0..width.saturating_sub(2) {
            writer.queue(Print(self.style.horizontal))?;
        }

        writer.queue(Print(self.style.bottom_right))?;
        writer.queue(Print('\n'))?;

        if let Some(_) = box_color {
            writer.queue(ResetColor)?;
        }

        Ok(())
    }

    /// Render a complete box with multiple content lines
    pub fn render_box<W: Write>(
        &self,
        writer: &mut W,
        title: &str,
        content_lines: &[impl AsRef<str>],
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        self.render_title_box(writer, title, width, color)?;

        for line in content_lines {
            self.render_content_line(writer, line.as_ref(), width, color)?;
        }

        self.render_bottom(writer, width, color)?;

        Ok(())
    }

    /// Render a menu box with automatic numbering
    pub fn render_menu<W: Write>(
        &self,
        writer: &mut W,
        title: &str,
        menu_items: &[MenuItem],
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        self.render_title_box(writer, title, width, color)?;

        let mut has_separator = false;

        for item in menu_items {
            match item {
                MenuItem::Header(text) => {
                    if has_separator {
                        self.render_separator(writer, width, color)?;
                    }
                    self.render_content_line(writer, text, width, color)?;
                    has_separator = true;
                }
                MenuItem::Option {
                    key,
                    description,
                    enabled,
                } => {
                    let content = if *enabled {
                        format!("[{}] {}", key, description)
                    } else {
                        format!("[{}] {} (disabled)", key, description)
                    };
                    self.render_content_line(writer, &content, width, color)?;
                }
                MenuItem::Separator => {
                    self.render_separator(writer, width, color)?;
                    has_separator = false;
                }
                MenuItem::Info(text) => {
                    self.render_content_line(writer, text, width, color)?;
                }
            }
        }

        self.render_bottom(writer, width, color)?;

        Ok(())
    }

    /// Render a simple message box
    pub fn render_message_box<W: Write>(
        &self,
        writer: &mut W,
        title: &str,
        message: &str,
        width: usize,
        color: Option<Color>,
    ) -> std::io::Result<()> {
        self.render_title_box(writer, title, width, color)?;

        // Empty line for padding
        self.render_content_line(writer, "", width, color)?;

        // Split message into lines that fit
        let max_content_width = width.saturating_sub(4);
        let words: Vec<&str> = message.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + word.len() + 1 <= max_content_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                // Render current line and start new one
                self.render_content_line(writer, &current_line, width, color)?;
                current_line = word.to_string();
            }
        }

        // Render remaining line if any
        if !current_line.is_empty() {
            self.render_content_line(writer, &current_line, width, color)?;
        }

        // Empty line for padding
        self.render_content_line(writer, "", width, color)?;

        self.render_bottom(writer, width, color)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum MenuItem {
    Header(String),
    Option {
        key: String,
        description: String,
        enabled: bool,
    },
    Separator,
    Info(String),
}

impl MenuItem {
    pub fn header(text: &str) -> Self {
        MenuItem::Header(text.to_string())
    }

    pub fn option(key: &str, description: &str) -> Self {
        MenuItem::Option {
            key: key.to_string(),
            description: description.to_string(),
            enabled: true,
        }
    }

    pub fn disabled_option(key: &str, description: &str) -> Self {
        MenuItem::Option {
            key: key.to_string(),
            description: description.to_string(),
            enabled: false,
        }
    }

    pub fn separator() -> Self {
        MenuItem::Separator
    }

    pub fn info(text: &str) -> Self {
        MenuItem::Info(text.to_string())
    }
}

// Box drawing character sets that can be configured
#[derive(Debug, Clone)]
pub enum BoxStyleName {
    Double,
    Single,
    Rounded,
    Ascii,
}

impl BoxStyleName {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "double" => Ok(BoxStyleName::Double),
            "single" => Ok(BoxStyleName::Single),
            "rounded" => Ok(BoxStyleName::Rounded),
            "ascii" => Ok(BoxStyleName::Ascii),
            _ => Err(format!("Unknown box style: {}", s)),
        }
    }

    pub fn to_style(&self) -> BoxStyle {
        match self {
            BoxStyleName::Double => BoxStyle::double(),
            BoxStyleName::Single => BoxStyle::single(),
            BoxStyleName::Rounded => BoxStyle::rounded(),
            BoxStyleName::Ascii => BoxStyle::ascii(),
        }
    }
}
