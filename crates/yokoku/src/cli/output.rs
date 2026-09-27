use std::fmt::{self, Display};

use owo_colors::{OwoColorize, Stream, Style};

/// Writes a line to stdout like `println!`, but returns the `io::Result` instead of panicking.
macro_rules! say {
    ($($arg:tt)*) => {{
        use ::std::io::Write as _;
        writeln!(::std::io::stdout(), $($arg)*)
    }};
}

/// `say!` in green.
macro_rules! success {
    ($($arg:tt)+) => { say!("{}", $crate::cli::output::Paint::green(format_args!($($arg)+))) };
}

/// `say!` in red.
macro_rules! failure {
    ($($arg:tt)+) => { say!("{}", $crate::cli::output::Paint::red(format_args!($($arg)+))) };
}

/// `say!` in yellow.
macro_rules! caution {
    ($($arg:tt)+) => { say!("{}", $crate::cli::output::Paint::yellow(format_args!($($arg)+))) };
}

/// `say!` dimmed.
macro_rules! hint {
    ($($arg:tt)+) => { say!("{}", $crate::cli::output::Paint::dimmed(format_args!($($arg)+))) };
}

/// Styles a value shown on stdout; it stays plain when stdout takes no colors.
pub trait Paint: Display + Sized {
    fn paint(self, style: Style) -> Painted<Self> {
        Painted(self, style)
    }

    fn bold(self) -> Painted<Self> {
        self.paint(Style::new().bold())
    }

    fn italic(self) -> Painted<Self> {
        self.paint(Style::new().italic())
    }

    fn dimmed(self) -> Painted<Self> {
        self.paint(Style::new().dimmed())
    }

    fn green(self) -> Painted<Self> {
        self.paint(Style::new().green())
    }

    fn red(self) -> Painted<Self> {
        self.paint(Style::new().red())
    }

    fn yellow(self) -> Painted<Self> {
        self.paint(Style::new().yellow())
    }

    /// Colored by what the shown word means, e.g. `missing` in red.
    fn tone(self) -> Painted<Self> {
        let style = match self.to_string().as_str() {
            "downloaded" | "released" | "continuing" | "seeding" | "finished" | "done" | "certain" | "monitored" => {
                Style::new().green()
            },
            "missing" | "error" | "failed" | "unknown" => Style::new().red(),
            "upcoming" | "on break" | "announced" | "in cinemas" | "needs review" | "guess" => Style::new().yellow(),
            "ended" | "queued" | "paused" | "removed" | "unmonitored" => Style::new().dimmed(),
            _ => Style::new(),
        };
        self.paint(style)
    }
}

impl<T: Display> Paint for T {}

pub struct Painted<T>(T, Style);

impl<T: Display> Display for Painted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0.if_supports_color(Stream::Stdout, |value| value.style(self.1)), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_the_value_inside_its_style() {
        owo_colors::set_override(true);

        assert_eq!(format!("{:<10}|", "missing".tone()), "\x1b[31mmissing   \x1b[0m|");
    }
}
