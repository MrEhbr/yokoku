use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token {
    Title,
    Year,
    Season,
    Episodes,
    EpisodeTitle,
}

impl Token {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "title" => Self::Title,
            "year" => Self::Year,
            "season" => Self::Season,
            "episodes" => Self::Episodes,
            "episode_title" => Self::EpisodeTitle,
            _ => return None,
        })
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Title => "title",
            Self::Year => "year",
            Self::Season => "season",
            Self::Episodes => "episodes",
            Self::EpisodeTitle => "episode_title",
        };
        write!(f, "{{{name}}}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Literal(String),
    Token(Token),
    /// Rendered only when every token inside has a value.
    Optional(Vec<Part>),
}

/// A pattern for one path component: literals, `{token}`s and `[optional groups]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Template {
    parts: Vec<Part>,
}

impl Template {
    pub(crate) fn parse(source: &str, allowed: &[Token], required: &[Token]) -> Result<Self, String> {
        let mut parts = Vec::new();
        let mut group: Option<Vec<Part>> = None;
        let mut literal = String::new();
        let mut chars = source.chars();

        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    let mut name = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(c) => name.push(c),
                            None => return Err(format!("unclosed {{{name}")),
                        }
                    }
                    let token = Token::parse(&name).ok_or_else(|| format!("unknown token {{{name}}}"))?;
                    if !allowed.contains(&token) {
                        return Err(format!("{token} cannot be used in this pattern"));
                    }
                    flush(&mut literal, &mut parts, &mut group);
                    target(&mut parts, &mut group).push(Part::Token(token));
                },
                '}' => return Err("unmatched }".into()),
                '[' if group.is_some() => return Err("optional groups cannot be nested".into()),
                '[' => {
                    flush(&mut literal, &mut parts, &mut group);
                    group = Some(Vec::new());
                },
                ']' => {
                    flush(&mut literal, &mut parts, &mut group);
                    parts.push(Part::Optional(group.take().ok_or("unmatched ]")?));
                },
                '/' | '\\' => return Err("a pattern names one folder or file and cannot contain / or \\".into()),
                c => literal.push(c),
            }
        }
        if group.is_some() {
            return Err("unclosed [".into());
        }
        flush(&mut literal, &mut parts, &mut group);

        let template = Self { parts };
        if let Some(missing) = required.iter().find(|&&token| !template.contains(token)) {
            return Err(format!("must contain {missing}"));
        }
        if template.parts.is_empty() {
            return Err("must not be empty".into());
        }
        Ok(template)
    }

    pub(crate) fn render(&self, value: &dyn Fn(Token) -> Option<String>) -> String {
        let mut rendered = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(text) => rendered.push_str(text),
                Part::Token(token) => rendered.push_str(&value(*token).unwrap_or_default()),
                Part::Optional(inner) => rendered.push_str(&render_group(inner, value).unwrap_or_default()),
            }
        }
        rendered
    }

    fn contains(&self, token: Token) -> bool {
        self.parts.iter().any(|part| match part {
            Part::Token(candidate) => *candidate == token,
            Part::Optional(inner) => inner.contains(&Part::Token(token)),
            Part::Literal(_) => false,
        })
    }
}

fn render_group(parts: &[Part], value: &dyn Fn(Token) -> Option<String>) -> Option<String> {
    let mut rendered = String::new();
    for part in parts {
        match part {
            Part::Literal(text) => rendered.push_str(text),
            Part::Token(token) => rendered.push_str(&value(*token)?),
            Part::Optional(_) => unreachable!("optional groups are never nested"),
        }
    }
    Some(rendered)
}

fn target<'a>(parts: &'a mut Vec<Part>, group: &'a mut Option<Vec<Part>>) -> &'a mut Vec<Part> {
    group.as_mut().unwrap_or(parts)
}

fn flush(literal: &mut String, parts: &mut Vec<Part>, group: &mut Option<Vec<Part>>) {
    if !literal.is_empty() {
        target(parts, group).push(Part::Literal(std::mem::take(literal)));
    }
}
