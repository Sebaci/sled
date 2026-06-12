use crate::diagnostic::{Diagnostic, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Ident(String),
    Int(i64),
    Text(String),
    LParen,
    RParen,
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    let mut chars = source.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(ch) = chars.peek().cloned() {
        match ch {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '"' => {
                chars.next();
                let mut text = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match chars.next() {
                            Some('"') => text.push('"'),
                            Some('\\') => text.push('\\'),
                            Some('n') => text.push('\n'),
                            Some(other) => {
                                return Err(Diagnostic::new(format!(
                                    "unsupported escape sequence: \\{}",
                                    other
                                )));
                            }
                            None => return Err(Diagnostic::new("unterminated escape sequence")),
                        },
                        other => text.push(other),
                    }
                }
                if !closed {
                    return Err(Diagnostic::new("unterminated text literal"));
                }
                tokens.push(Token::Text(text));
            }
            c if c.is_ascii_digit() || c == '-' => {
                let mut number = String::new();
                number.push(ch);
                chars.next();
                while let Some(c) = chars.peek() {
                    if c.is_ascii_digit() {
                        number.push(*c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let value = number
                    .parse()
                    .map_err(|_| Diagnostic::new(format!("invalid integer literal: {}", number)))?;
                tokens.push(Token::Int(value));
            }
            _ => {
                let mut ident = String::new();
                while let Some(c) = chars.peek() {
                    if c.is_whitespace() || *c == '(' || *c == ')' {
                        break;
                    }
                    ident.push(*c);
                    chars.next();
                }
                tokens.push(Token::Ident(ident));
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_flat_pipeline() {
        assert_eq!(
            lex("lines map len sum").unwrap(),
            vec![
                Token::Ident("lines".into()),
                Token::Ident("map".into()),
                Token::Ident("len".into()),
                Token::Ident("sum".into())
            ]
        );
    }

    #[test]
    fn lexes_text_literal() {
        assert_eq!(
            lex(r#"split "\n""#).unwrap(),
            vec![Token::Ident("split".into()), Token::Text("\n".into())]
        );
    }

    #[test]
    fn rejects_unterminated_text_literal() {
        assert_eq!(
            lex(r#""abc"#).unwrap_err().message,
            "unterminated text literal"
        );
    }
}
