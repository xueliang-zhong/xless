use crossterm::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextStyle {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub style: TextStyle,
}

impl StyledSpan {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
        }
    }
}

impl TextStyle {
    pub fn to_ansi_prefix(self) -> String {
        let mut codes = Vec::new();
        if self.bold {
            codes.push("1".to_string());
        }
        if self.dim {
            codes.push("2".to_string());
        }
        if self.italic {
            codes.push("3".to_string());
        }
        if self.underline {
            codes.push("4".to_string());
        }
        if self.reverse {
            codes.push("7".to_string());
        }
        if let Some(fg) = self.fg {
            codes.push(format!("38;2;{};{};{}", fg.r, fg.g, fg.b));
        }
        if let Some(bg) = self.bg {
            codes.push(format!("48;2;{};{};{}", bg.r, bg.g, bg.b));
        }
        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }

    pub fn inverted(self) -> Self {
        Self {
            reverse: !self.reverse,
            ..self
        }
    }
}

impl From<syntect::highlighting::Color> for Rgb {
    fn from(value: syntect::highlighting::Color) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
        }
    }
}

impl From<Color> for Rgb {
    fn from(value: Color) -> Self {
        match value {
            Color::Rgb { r, g, b } => Self { r, g, b },
            Color::Black => Self { r: 0, g: 0, b: 0 },
            Color::DarkRed => Self { r: 128, g: 0, b: 0 },
            Color::DarkGreen => Self { r: 0, g: 128, b: 0 },
            Color::DarkYellow => Self { r: 128, g: 128, b: 0 },
            Color::DarkBlue => Self { r: 0, g: 0, b: 128 },
            Color::DarkMagenta => Self { r: 128, g: 0, b: 128 },
            Color::DarkCyan => Self { r: 0, g: 128, b: 128 },
            Color::Grey => Self { r: 192, g: 192, b: 192 },
            Color::Red => Self { r: 255, g: 0, b: 0 },
            Color::Green => Self { r: 0, g: 255, b: 0 },
            Color::Yellow => Self { r: 255, g: 255, b: 0 },
            Color::Blue => Self { r: 0, g: 0, b: 255 },
            Color::Magenta => Self { r: 255, g: 0, b: 255 },
            Color::Cyan => Self { r: 0, g: 255, b: 255 },
            Color::White => Self { r: 255, g: 255, b: 255 },
            Color::DarkGrey => Self { r: 128, g: 128, b: 128 },
            Color::Reset => Self { r: 255, g: 255, b: 255 },
            Color::AnsiValue(v) => Self { r: v, g: v, b: v },
        }
    }
}

