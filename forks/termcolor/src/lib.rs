//! Termcolor crate for cross-platform colored terminal output

pub mod ansi;
mod traits;
mod types;
mod writers;

// Re-export core traits and types
pub use ansi::{AnsiColor, ansi_color, ansi_color_only, ansi_spec};
pub use traits::WriteColor;
pub use types::{
    Color, ColorChoice, ColorChoiceParseError, ColorSpec, ColorSpecParseError,
    HyperlinkSpec, ParseColorError,
};
pub use writers::{
    Ansi, Buffer, BufferWriter, BufferedStandardStream, NoColor,
    StandardStream, StandardStreamLock, StringWriter, TermString,
};
