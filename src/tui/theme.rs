use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface2: Color,
    pub border: Color,
    pub accent: Color,
    pub accent2: Color,
    pub text: Color,
    pub muted: Color,
    pub green: Color,
    pub red: Color,
    pub purple: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Reset,
            surface: Color::Reset,
            surface2: Color::Rgb(28, 28, 42),
            border: Color::Rgb(48, 48, 72),
            accent: Color::Rgb(255, 177, 0), // amber
            accent2: Color::Rgb(80, 200, 255), // sky blue
            text: Color::Rgb(220, 220, 230),
            muted: Color::Rgb(120, 120, 150),
            green: Color::Rgb(100, 220, 100),
            red: Color::Rgb(255, 90, 90),
            purple: Color::Rgb(180, 120, 255),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(240, 240, 245),
            surface: Color::Rgb(255, 255, 255),
            surface2: Color::Rgb(230, 230, 235),
            border: Color::Rgb(180, 180, 200),
            accent: Color::Rgb(200, 100, 0),
            accent2: Color::Rgb(0, 100, 200),
            text: Color::Rgb(40, 40, 50),
            muted: Color::Rgb(120, 120, 140),
            green: Color::Rgb(0, 150, 0),
            red: Color::Rgb(200, 0, 0),
            purple: Color::Rgb(100, 0, 200),
        }
    }

    pub fn terminal() -> Self {
        Self {
            bg: Color::Reset,
            surface: Color::Reset,
            surface2: Color::Indexed(8), // usually bright black/gray
            border: Color::Indexed(8),
            accent: Color::Yellow,
            accent2: Color::Cyan,
            text: Color::Reset,
            muted: Color::Indexed(8),
            green: Color::Green,
            red: Color::Red,
            purple: Color::Magenta,
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "light" => Self::light(),
            "terminal" | "system" => Self::terminal(),
            _ => Self::dark(),
        }
    }
}
