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
        let mut builder = Builder::default();
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
                    builder.push(Part::Token(token));
                },
                '}' => return Err("unmatched }".into()),
                '[' if builder.group.is_some() => return Err("optional groups cannot be nested".into()),
                '[' => {
                    builder.flush();
                    builder.group = Some(Vec::new());
                },
                ']' => {
                    builder.flush();
                    let group = builder.group.take().ok_or("unmatched ]")?;
                    builder.parts.push(Part::Optional(group));
                },
                '/' | '\\' => return Err("a pattern names one folder or file and cannot contain / or \\".into()),
                c => builder.literal.push(c),
            }
        }
        if builder.group.is_some() {
            return Err("unclosed [".into());
        }
        builder.flush();

        let template = Self { parts: builder.parts };
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

/// Parts collected while parsing; new parts go into the open optional group, if any.
#[derive(Default)]
struct Builder {
    parts: Vec<Part>,
    group: Option<Vec<Part>>,
    literal: String,
}

impl Builder {
    fn push(&mut self, part: Part) {
        self.flush();
        self.group.as_mut().unwrap_or(&mut self.parts).push(part);
    }

    fn flush(&mut self) {
        if !self.literal.is_empty() {
            let literal = Part::Literal(std::mem::take(&mut self.literal));
            self.group.as_mut().unwrap_or(&mut self.parts).push(literal);
        }
    }
}
