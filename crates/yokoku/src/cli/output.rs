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

    fn dimmed(self) -> Painted<Self> {
        self.paint(Style::new().dimmed())
    }

    fn green(self) -> Painted<Self> {
        self.paint(Style::new().green())
    }

    fn red(self) -> Painted<Self> {
        self.paint(Style::new().red())
    }
}

impl<T: Display> Paint for T {}

pub struct Painted<T>(T, Style);

impl<T: Display> Display for Painted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0.if_supports_color(Stream::Stdout, |value| value.style(self.1)), f)
    }
}
