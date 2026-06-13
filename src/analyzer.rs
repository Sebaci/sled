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
                let canonical_name = canonical_builtin_name(name)
                    .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
                let shape = builtin_shape(canonical_name)
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
                if canonical_name == "map" || canonical_name == "filter" {
                    analyze_transform_arg(canonical_name, &terms[index + 1])?;
                } else {
                    for arg in &terms[index + 1..index + 1 + shape.right_arity] {
                        if let Term::Group(grouped) = arg {
                            analyze_terms(grouped)?;
                        }
                    }
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

fn analyze_transform_arg(operator: &str, term: &Term) -> Result<()> {
    match term {
        Term::Ident(name) => {
            let canonical_name = canonical_builtin_name(name)
                .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
            let shape = builtin_shape(canonical_name)
                .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
            if shape.needs_left && shape.right_arity == 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(format!(
                    "{} expects a complete transform, but {} needs {} right-hand argument(s)",
                    operator, name, shape.right_arity
                )))
            }
        }
        Term::Group(terms) => analyze_grouped_transform_arg(operator, terms),
        _ => Err(Diagnostic::new(format!(
            "{} expects a builtin transform",
            operator
        ))),
    }
}

fn analyze_grouped_transform_arg(operator: &str, terms: &[Term]) -> Result<()> {
    let name = match terms.first() {
        Some(Term::Ident(name)) => name,
        _ => {
            return Err(Diagnostic::new(format!(
                "{} expects a grouped builtin transform",
                operator
            )));
        }
    };

    let canonical_name = canonical_builtin_name(name)
        .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
    let shape = builtin_shape(canonical_name)
        .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
    if !shape.needs_left {
        return Err(Diagnostic::new(format!(
            "{} expects a transform, but {} is a producer",
            operator, name
        )));
    }
    if terms.len() != shape.right_arity + 1 {
        return Err(Diagnostic::new(format!(
            "{} expects a complete transform, but {} needs {} right-hand argument(s)",
            operator, name, shape.right_arity
        )));
    }

    for arg in &terms[1..] {
        if let Term::Group(grouped) = arg {
            analyze_terms(grouped)?;
        }
    }

    Ok(())
}

pub fn canonical_builtin_name(name: &str) -> Option<&'static str> {
    let canonical = match name {
        "input" => "input",
        "lines" | "L" => "lines",
        "words" | "W" => "words",
        "chars" | "C" => "chars",
        "int" | "i" => "int",
        "len" => "len",
        "sum" => "sum",
        "split" | "sp" => "split",
        "map" | "m" => "map",
        "filter" | "f" => "filter",
        "window" | "win" => "window",
        "range" => "range",
        ">" => ">",
        "<" => "<",
        ">=" => ">=",
        "<=" => "<=",
        "=" => "=",
        "!=" => "!=",
        _ => return None,
    };
    Some(canonical)
}

pub fn builtin_shape(name: &str) -> Option<CallableShape> {
    let shape = match name {
        "input" => CallableShape {
            needs_left: false,
            right_arity: 0,
        },
        "lines" | "words" | "chars" | "int" | "len" | "sum" => CallableShape {
            needs_left: true,
            right_arity: 0,
        },
        "split" | "map" | "filter" | "window" | ">" | "<" | ">=" | "<=" | "=" | "!=" => {
            CallableShape {
                needs_left: true,
                right_arity: 1,
            }
        }
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
        let program = parse(&lex("input L m len sum").unwrap()).unwrap();
        analyze(&program).unwrap();
    }

    #[test]
    fn reports_incomplete_map_transform() {
        let program = parse(&lex("input L m sp").unwrap()).unwrap();
        assert_eq!(
            analyze(&program).unwrap_err().message,
            "map expects a complete transform, but sp needs 1 right-hand argument(s)"
        );
    }

    #[test]
    fn resolves_builtin_aliases() {
        assert_eq!(canonical_builtin_name("L"), Some("lines"));
        assert_eq!(canonical_builtin_name("m"), Some("map"));
        assert_eq!(canonical_builtin_name("i"), Some("int"));
        assert_eq!(canonical_builtin_name("f"), Some("filter"));
        assert_eq!(canonical_builtin_name("win"), Some("window"));
    }

    #[test]
    fn accepts_grouped_section_transform() {
        let program = parse(&lex("input L m i f (> 0) len").unwrap()).unwrap();
        analyze(&program).unwrap();
    }

    #[test]
    fn reports_incomplete_filter_transform() {
        let program = parse(&lex("input L f (> )").unwrap()).unwrap();
        assert_eq!(
            analyze(&program).unwrap_err().message,
            "filter expects a complete transform, but > needs 1 right-hand argument(s)"
        );
    }
}
