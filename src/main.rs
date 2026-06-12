use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

use clap::Parser;
use json_schema_sort::{ArraySorting, SortOptions};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Input file. Use '-' or omit for stdin.
    input: Option<PathBuf>,

    /// Write the sorted result back to the input file.
    #[arg(short, long)]
    write: bool,

    /// Exit non-zero if the input is not already sorted.
    #[arg(short, long)]
    check: bool,

    /// Do not sort schema-safe arrays like required and type.
    #[arg(long)]
    no_sort_arrays: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let input_path = args.input.as_ref().filter(|path| path.as_os_str() != "-");

    if args.write && input_path.is_none() {
        return Err("--write requires a real input file".into());
    }

    let input = match input_path {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    let mut options = SortOptions::default();
    options.arrays = if args.no_sort_arrays {
        ArraySorting::None
    } else {
        ArraySorting::SchemaSafe
    };

    let output = json_schema_sort::to_string_sorted_with_options(&input, options)?;

    if args.check {
        if normalize_trailing_newline(&input) == output {
            return Ok(());
        }

        return Err("input is not sorted".into());
    }

    if args.write {
        fs::write(input_path.unwrap(), output)?;
    } else {
        print!("{output}");
    }

    Ok(())
}

fn normalize_trailing_newline(input: &str) -> String {
    let mut value = input.trim_end_matches(['\r', '\n']).to_owned();
    value.push('\n');
    value
}
