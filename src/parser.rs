use crate::diagnostic::{Diagnostic, Result};
use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub terms: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Ident(String),
    Int(i64),
    Text(String),
    Group(Vec<Term>),
}

pub fn parse(tokens: &[Token]) -> Result<Program> {
    let mut cursor = Cursor { tokens, index: 0 };
    let terms = cursor.parse_terms(false)?;
    if cursor.index != tokens.len() {
        return Err(Diagnostic::new("unexpected trailing tokens"));
    }
    Ok(Program { terms })
}

struct Cursor<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Cursor<'a> {
    fn parse_terms(&mut self, stop_at_rparen: bool) -> Result<Vec<Term>> {
        let mut terms = Vec::new();

        while let Some(token) = self.tokens.get(self.index) {
            match token {
                Token::Ident(name) => {
                    self.index += 1;
                    terms.push(Term::Ident(name.clone()));
                }
                Token::Int(value) => {
                    self.index += 1;
                    terms.push(Term::Int(*value));
                }
                Token::Text(value) => {
                    self.index += 1;
                    terms.push(Term::Text(value.clone()));
                }
                Token::LParen => {
                    self.index += 1;
                    let grouped = self.parse_terms(true)?;
                    terms.push(Term::Group(grouped));
                }
                Token::RParen if stop_at_rparen => {
                    self.index += 1;
                    return Ok(terms);
                }
                Token::RParen => return Err(Diagnostic::new("unmatched ')'")),
            }
        }

        if stop_at_rparen {
            return Err(Diagnostic::new("unclosed '('"));
        }

        Ok(terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn parses_grouped_rhs_expression() {
        assert_eq!(
            parse(&lex("xs zip (range 1 3)").unwrap()).unwrap(),
            Program {
                terms: vec![
                    Term::Ident("xs".into()),
                    Term::Ident("zip".into()),
                    Term::Group(vec![
                        Term::Ident("range".into()),
                        Term::Int(1),
                        Term::Int(3)
                    ])
                ]
            }
        );
    }
}
