pub(crate) const MAX_COMPONENT_BYTES: usize = 255;
pub(crate) const MAX_STEM_BYTES: usize = 200;

const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// One path component that is valid on Linux, macOS, Windows and SMB shares, at most 255 bytes.
pub fn sanitize(name: &str) -> String {
    sanitize_to(name, MAX_COMPONENT_BYTES)
}

pub(crate) fn sanitize_to(name: &str, max_bytes: usize) -> String {
    let mut replaced = String::with_capacity(name.len());
    for c in name.replace(": ", " - ").chars() {
        match c {
            ':' | '/' | '\\' | '|' => replaced.push('-'),
            '"' => replaced.push('\''),
            '<' | '>' | '?' | '*' => {},
            c if c.is_whitespace() => replaced.push(' '),
            c if c.is_control() => {},
            c => replaced.push(c),
        }
    }

    let collapsed = replaced.split(' ').filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ");
    let mut name = truncate(&collapsed, max_bytes).trim_end_matches(['.', ' ']).to_owned();

    if is_reserved(&name) {
        name.push('_');
    }
    if name.is_empty() {
        name.push('_');
    }
    name
}

/// `stem.extension`, with the stem cleaned and capped at 200 bytes and the extension lowercased.
pub(crate) fn file_name(stem: &str, extension: &str) -> String {
    let stem = sanitize_to(stem, MAX_STEM_BYTES);
    let extension: String =
        extension.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
    if extension.is_empty() { stem } else { format!("{stem}.{extension}") }
}

pub(crate) fn truncate(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let end = (0..=max_bytes).rev().find(|&index| text.is_char_boundary(index)).unwrap_or(0);
    &text[..end]
}

fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    RESERVED_NAMES.iter().any(|reserved| stem.eq_ignore_ascii_case(reserved))
}
