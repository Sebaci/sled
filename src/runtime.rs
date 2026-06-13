use crate::analyzer::{builtin_shape, canonical_builtin_name};
use crate::diagnostic::{Diagnostic, Result};
use crate::parser::{Program, Term};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Text(String),
    List(Vec<Value>),
    Transform(Transform),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transform {
    name: String,
    args: Vec<Value>,
}

impl Value {
    pub fn render(&self) -> String {
        match self {
            Value::Bool(value) => value.to_string(),
            Value::Int(value) => value.to_string(),
            Value::Text(value) => value.clone(),
            Value::List(values) => {
                let rendered = values
                    .iter()
                    .map(Value::render)
                    .collect::<Vec<_>>()
                    .join("\n");
                rendered
            }
            Value::Transform(transform) => format!("<transform {}>", transform.name),
        }
    }
}

pub fn eval(program: &Program, input: &str) -> Result<Value> {
    eval_terms(&program.terms, input)
}

fn eval_terms(terms: &[Term], input: &str) -> Result<Value> {
    let mut index = 0;
    let mut current = None;

    while index < terms.len() {
        match &terms[index] {
            Term::Ident(name) => {
                let canonical_name = canonical_builtin_name(name)
                    .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
                let shape = builtin_shape(canonical_name)
                    .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
                let mut args = Vec::new();
                for (offset, arg) in terms[index + 1..index + 1 + shape.right_arity]
                    .iter()
                    .enumerate()
                {
                    if (canonical_name == "map" || canonical_name == "filter") && offset == 0 {
                        args.push(Value::Transform(eval_transform_arg(arg, input)?));
                    } else {
                        args.push(eval_arg(arg, input)?);
                    }
                }
                current = Some(apply_builtin(name, current.take(), args, input)?);
                index += 1 + shape.right_arity;
            }
            term => {
                if current.is_some() {
                    return Err(Diagnostic::new("literal values need a consuming operator"));
                }
                current = Some(eval_arg(term, input)?);
                index += 1;
            }
        }
    }

    current.ok_or_else(|| Diagnostic::new("program is empty"))
}

fn eval_arg(term: &Term, input: &str) -> Result<Value> {
    match term {
        Term::Int(value) => Ok(Value::Int(*value)),
        Term::Text(value) => Ok(Value::Text(value.clone())),
        Term::Ident(name) => {
            if let Some(canonical_name) = canonical_builtin_name(name) {
                let shape = builtin_shape(canonical_name)
                    .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
                if shape.needs_left && shape.right_arity == 0 {
                    return Ok(Value::Transform(Transform {
                        name: name.clone(),
                        args: Vec::new(),
                    }));
                }
                Ok(Value::Text(name.clone()))
            } else {
                Err(Diagnostic::new(format!("unknown identifier: {}", name)))
            }
        }
        Term::Group(terms) => eval_terms(terms, input),
    }
}

fn eval_transform_arg(term: &Term, input: &str) -> Result<Transform> {
    match term {
        Term::Ident(name) => {
            let canonical_name = canonical_builtin_name(name)
                .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
            let shape = builtin_shape(canonical_name)
                .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
            if shape.needs_left && shape.right_arity == 0 {
                Ok(Transform {
                    name: name.clone(),
                    args: Vec::new(),
                })
            } else {
                Err(Diagnostic::new(format!(
                    "{} needs {} right-hand argument(s)",
                    name, shape.right_arity
                )))
            }
        }
        Term::Group(terms) => eval_grouped_transform_arg(terms, input),
        _ => Err(Diagnostic::new("expected builtin transform")),
    }
}

fn eval_grouped_transform_arg(terms: &[Term], input: &str) -> Result<Transform> {
    let name = match terms.first() {
        Some(Term::Ident(name)) => name,
        _ => return Err(Diagnostic::new("expected grouped builtin transform")),
    };

    let canonical_name = canonical_builtin_name(name)
        .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
    let shape = builtin_shape(canonical_name)
        .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;
    if !shape.needs_left {
        return Err(Diagnostic::new(format!("{} is not a transform", name)));
    }
    if terms.len() != shape.right_arity + 1 {
        return Err(Diagnostic::new(format!(
            "{} needs {} right-hand argument(s)",
            name, shape.right_arity
        )));
    }

    let mut args = Vec::new();
    for arg in &terms[1..] {
        args.push(eval_arg(arg, input)?);
    }

    Ok(Transform {
        name: name.clone(),
        args,
    })
}

fn apply_builtin(name: &str, left: Option<Value>, args: Vec<Value>, input: &str) -> Result<Value> {
    let canonical_name = canonical_builtin_name(name)
        .ok_or_else(|| Diagnostic::new(format!("unknown builtin: {}", name)))?;

    match canonical_name {
        "input" => Ok(Value::Text(input.to_string())),
        "lines" => match expect_left(name, left)? {
            Value::Text(text) => Ok(Value::List(
                text.lines()
                    .map(|line| Value::Text(line.to_string()))
                    .collect(),
            )),
            _ => Err(Diagnostic::new("lines expects text")),
        },
        "words" => match expect_left(name, left)? {
            Value::Text(text) => Ok(Value::List(
                text.split_whitespace()
                    .map(|word| Value::Text(word.to_string()))
                    .collect(),
            )),
            _ => Err(Diagnostic::new("words expects text")),
        },
        "chars" => match expect_left(name, left)? {
            Value::Text(text) => Ok(Value::List(
                text.chars().map(|ch| Value::Text(ch.to_string())).collect(),
            )),
            _ => Err(Diagnostic::new("chars expects text")),
        },
        "int" => match expect_left(name, left)? {
            Value::Text(text) => text
                .trim()
                .parse()
                .map(Value::Int)
                .map_err(|_| Diagnostic::new(format!("int could not parse: {}", text))),
            Value::Int(value) => Ok(Value::Int(value)),
            Value::Bool(_) | Value::List(_) | Value::Transform(_) => {
                Err(Diagnostic::new("int expects text"))
            }
        },
        "split" => {
            let sep = expect_text_arg(name, &args, 0)?;
            match expect_left(name, left)? {
                Value::Text(text) => Ok(Value::List(
                    text.split(&sep)
                        .map(|part| Value::Text(part.to_string()))
                        .collect(),
                )),
                _ => Err(Diagnostic::new("split expects text")),
            }
        }
        "len" => match expect_left(name, left)? {
            Value::Text(text) => Ok(Value::Int(text.chars().count() as i64)),
            Value::List(values) => Ok(Value::Int(values.len() as i64)),
            Value::Bool(_) | Value::Int(_) | Value::Transform(_) => {
                Err(Diagnostic::new("len expects text or list"))
            }
        },
        "sum" => match expect_left(name, left)? {
            Value::List(values) => {
                let mut total = 0;
                for value in values {
                    match value {
                        Value::Int(n) => total += n,
                        _ => return Err(Diagnostic::new("sum expects a list of integers")),
                    }
                }
                Ok(Value::Int(total))
            }
            _ => Err(Diagnostic::new("sum expects a list")),
        },
        "map" => {
            let transform = expect_transform_arg(name, &args, 0)?;
            match expect_left(name, left)? {
                Value::List(values) => values
                    .into_iter()
                    .map(|value| apply_transform(&transform, value, input))
                    .collect::<Result<Vec<_>>>()
                    .map(Value::List),
                _ => Err(Diagnostic::new("map expects a list")),
            }
        }
        "filter" => {
            let transform = expect_transform_arg(name, &args, 0)?;
            match expect_left(name, left)? {
                Value::List(values) => {
                    let mut filtered = Vec::new();
                    for value in values {
                        match apply_transform(&transform, value.clone(), input)? {
                            Value::Bool(true) => filtered.push(value),
                            Value::Bool(false) => {}
                            _ => return Err(Diagnostic::new("filter transform must return bool")),
                        }
                    }
                    Ok(Value::List(filtered))
                }
                _ => Err(Diagnostic::new("filter expects a list")),
            }
        }
        "window" => {
            let size = expect_int_arg(name, &args, 0)?;
            if size <= 0 {
                return Err(Diagnostic::new("window size must be positive"));
            }
            match expect_left(name, left)? {
                Value::List(values) => {
                    let size = size as usize;
                    if size > values.len() {
                        return Ok(Value::List(Vec::new()));
                    }
                    Ok(Value::List(
                        values
                            .windows(size)
                            .map(|window| Value::List(window.to_vec()))
                            .collect(),
                    ))
                }
                _ => Err(Diagnostic::new("window expects a list")),
            }
        }
        "range" => {
            let start = expect_int_arg(name, &args, 0)?;
            let end = expect_int_arg(name, &args, 1)?;
            Ok(Value::List((start..=end).map(Value::Int).collect()))
        }
        ">" | "<" | ">=" | "<=" => {
            let right = expect_int_arg(name, &args, 0)?;
            match expect_left(name, left)? {
                Value::Int(left) => {
                    let result = match canonical_name {
                        ">" => left > right,
                        "<" => left < right,
                        ">=" => left >= right,
                        "<=" => left <= right,
                        _ => unreachable!(),
                    };
                    Ok(Value::Bool(result))
                }
                _ => Err(Diagnostic::new(format!("{} expects integers", name))),
            }
        }
        "=" | "!=" => {
            let right = args
                .get(0)
                .ok_or_else(|| Diagnostic::new(format!("{} expects argument 1", name)))?;
            let left = expect_left(name, left)?;
            let equal = left == *right;
            Ok(Value::Bool(if canonical_name == "=" {
                equal
            } else {
                !equal
            }))
        }
        _ => Err(Diagnostic::new(format!("unknown builtin: {}", name))),
    }
}

fn apply_transform(transform: &Transform, left: Value, input: &str) -> Result<Value> {
    apply_builtin(&transform.name, Some(left), transform.args.clone(), input)
}

fn expect_left(name: &str, left: Option<Value>) -> Result<Value> {
    left.ok_or_else(|| Diagnostic::new(format!("{} needs a value from the left", name)))
}

fn expect_text_arg(name: &str, args: &[Value], index: usize) -> Result<String> {
    match args.get(index) {
        Some(Value::Text(value)) => Ok(value.clone()),
        _ => Err(Diagnostic::new(format!(
            "{} expects text argument {}",
            name,
            index + 1
        ))),
    }
}

fn expect_transform_arg(name: &str, args: &[Value], index: usize) -> Result<Transform> {
    match args.get(index) {
        Some(Value::Transform(value)) => Ok(value.clone()),
        _ => Err(Diagnostic::new(format!(
            "{} expects transform argument {}",
            name,
            index + 1
        ))),
    }
}

fn expect_int_arg(name: &str, args: &[Value], index: usize) -> Result<i64> {
    match args.get(index) {
        Some(Value::Int(value)) => Ok(*value),
        _ => Err(Diagnostic::new(format!(
            "{} expects integer argument {}",
            name,
            index + 1
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::lex, parser::parse};

    fn run(source: &str, input: &str) -> Result<Value> {
        eval(&parse(&lex(source)?)?, input)
    }

    #[test]
    fn evaluates_line_lengths() {
        assert_eq!(
            run("input L m len sum", "abc\nde\n").unwrap(),
            Value::Int(5)
        );
    }

    #[test]
    fn evaluates_grouped_range() {
        assert_eq!(run("range 1 3 sum", "").unwrap(), Value::Int(6));
    }

    #[test]
    fn evaluates_words_chars_and_int_aliases() {
        assert_eq!(run("input W len", "1 22 333").unwrap(), Value::Int(3));
        assert_eq!(run("input C len", "abc").unwrap(), Value::Int(3));
        assert_eq!(run("input L m i sum", "1\n2\n3\n").unwrap(), Value::Int(6));
    }

    #[test]
    fn evaluates_windows() {
        assert_eq!(
            run("range 1 4 win 2", "").unwrap(),
            Value::List(vec![
                Value::List(vec![Value::Int(1), Value::Int(2)]),
                Value::List(vec![Value::Int(2), Value::Int(3)]),
                Value::List(vec![Value::Int(3), Value::Int(4)]),
            ])
        );
    }

    #[test]
    fn rejects_non_positive_window_size() {
        assert_eq!(
            run("range 1 4 win 0", "").unwrap_err().message,
            "window size must be positive"
        );
    }

    #[test]
    fn evaluates_comparison_sections() {
        assert_eq!(run("1 > 0", "").unwrap(), Value::Bool(true));
        assert_eq!(run("1 < 0", "").unwrap(), Value::Bool(false));
        assert_eq!(run("1 = 1", "").unwrap(), Value::Bool(true));
        assert_eq!(run("1 != 2", "").unwrap(), Value::Bool(true));
    }

    #[test]
    fn evaluates_filter_with_grouped_section() {
        assert_eq!(
            run("input L m i f (> 0) sum", "-1\n2\n0\n3\n").unwrap(),
            Value::Int(5)
        );
    }

    #[test]
    fn evaluates_map_with_grouped_section() {
        assert_eq!(
            run(r#"input L m (sp ",")"#, "a,b\nc,d\n").unwrap(),
            Value::List(vec![
                Value::List(vec![Value::Text("a".into()), Value::Text("b".into())]),
                Value::List(vec![Value::Text("c".into()), Value::Text("d".into())]),
            ])
        );
    }
}
