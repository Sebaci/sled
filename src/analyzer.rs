use crate::diagnostic::{Diagnostic, Result};
use crate::parser::{Program, Term};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallableShape {
    pub needs_left: bool,
    pub right_arity: usize,
}

pub fn analyze(program: &Program) -> Result<()> {
    analyze_terms(&program.terms)
}

fn analyze_terms(terms: &[Term]) -> Result<()> {
    if terms.is_empty() {
        return Err(Diagnostic::new("program is empty"));
    }

    let mut index = 0;
    let mut has_value = false;
    while index < terms.len() {
        match &terms[index] {
            Term::Ident(name) => {
                let shape = builtin_shape(name)
                    .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
                if shape.needs_left && !has_value {
                    return Err(Diagnostic::new(format!(
                        "{} needs a value from the left",
                        name
                    )));
                }
                if index + 1 + shape.right_arity > terms.len() {
                    return Err(Diagnostic::new(format!(
                        "{} expects {} right-hand argument(s)",
                        name, shape.right_arity
                    )));
                }
                for arg in &terms[index + 1..index + 1 + shape.right_arity] {
                    if let Term::Group(grouped) = arg {
                        analyze_terms(grouped)?;
                    }
                }
                if name == "map" {
                    analyze_map_transform(&terms[index + 1])?;
                }
                has_value = true;
                index += 1 + shape.right_arity;
            }
            Term::Int(_) | Term::Text(_) | Term::Group(_) => {
                if has_value {
                    return Err(Diagnostic::new("literal values need a consuming operator"));
                }
                if let Term::Group(grouped) = &terms[index] {
                    analyze_terms(grouped)?;
                }
                has_value = true;
                index += 1;
            }
        }
    }

    Ok(())
}

fn analyze_map_transform(term: &Term) -> Result<()> {
    match term {
        Term::Ident(name) => {
            let shape = builtin_shape(name)
                .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
            if shape.needs_left && shape.right_arity == 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(format!(
                    "map expects a complete transform, but {} needs {} right-hand argument(s)",
                    name, shape.right_arity
                )))
            }
        }
        _ => Err(Diagnostic::new(
            "map currently expects a builtin transform name",
        )),
    }
}

pub fn builtin_shape(name: &str) -> Option<CallableShape> {
    let shape = match name {
        "input" => CallableShape {
            needs_left: false,
            right_arity: 0,
        },
        "lines" | "len" | "sum" => CallableShape {
            needs_left: true,
            right_arity: 0,
        },
        "split" | "map" => CallableShape {
            needs_left: true,
            right_arity: 1,
        },
        "range" => CallableShape {
            needs_left: false,
            right_arity: 2,
        },
        _ => return None,
    };
    Some(shape)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::lex, parser::parse};

    #[test]
    fn reports_missing_left_value() {
        let program = parse(&lex("len").unwrap()).unwrap();
        assert_eq!(
            analyze(&program).unwrap_err().message,
            "len needs a value from the left"
        );
    }

    #[test]
    fn accepts_input_pipeline() {
        let program = parse(&lex("input lines map len sum").unwrap()).unwrap();
        analyze(&program).unwrap();
    }

    #[test]
    fn reports_incomplete_map_transform() {
        let program = parse(&lex("input lines map split").unwrap()).unwrap();
        assert_eq!(
            analyze(&program).unwrap_err().message,
            "map expects a complete transform, but split needs 1 right-hand argument(s)"
        );
    }
}
