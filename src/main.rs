use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

fn run() -> sled::Result<()> {
    let mut args = env::args().skip(1);
    let mut source = None;
    let mut input_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-e" | "--expr" => {
                source = args.next();
                if source.is_none() {
                    return Err(sled::Diagnostic::new("--expr requires a program"));
                }
            }
            "-i" | "--input" => {
                input_path = args.next();
                if input_path.is_none() {
                    return Err(sled::Diagnostic::new("--input requires a path"));
                }
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            path if source.is_none() => {
                source = Some(fs::read_to_string(path).map_err(|err| {
                    sled::Diagnostic::new(format!("failed to read program {}: {}", path, err))
                })?);
            }
            other => {
                return Err(sled::Diagnostic::new(format!(
                    "unexpected argument: {}",
                    other
                )));
            }
        }
    }

    let source = source
        .ok_or_else(|| sled::Diagnostic::new("missing program; pass a file path or use --expr"))?;

    let input = match input_path {
        Some(path) => fs::read_to_string(&path).map_err(|err| {
            sled::Diagnostic::new(format!("failed to read input {}: {}", path, err))
        })?,
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|err| sled::Diagnostic::new(format!("failed to read stdin: {}", err)))?;
            buffer
        }
    };

    let value = sled::eval(&source, &input)?;
    println!("{}", value.render());
    Ok(())
}

fn print_help() {
    println!("sled");
    println!();
    println!("Usage:");
    println!("  sled <program.sled> [--input input.txt]");
    println!("  sled --expr 'input lines map len sum' < input.txt");
}
