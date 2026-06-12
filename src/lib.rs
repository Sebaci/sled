pub mod analyzer;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod runtime;

pub use diagnostic::{Diagnostic, Result};

pub fn eval(source: &str, input: &str) -> Result<runtime::Value> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(&tokens)?;
    analyzer::analyze(&program)?;
    runtime::eval(&program, input)
}
