use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::Parser;
use json_schema_sort::{ArraySorting, PropertyOrdering, SortOptions};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Inputs to sort: files, directories, or glob patterns. Directories are
    /// searched recursively for `*.json`. Use '-' or omit for stdin.
    inputs: Vec<String>,

    /// Write the sorted result back to each input file.
    #[arg(short, long)]
    write: bool,

    /// Exit non-zero if any input is not already sorted.
    #[arg(short, long)]
    check: bool,

    /// Do not sort schema-safe arrays like required and type.
    #[arg(long)]
    no_sort_arrays: bool,

    /// Keep property and `$defs` names in their original order instead of
    /// sorting them alphabetically.
    #[arg(long)]
    preserve_property_order: bool,

    /// Include git-ignored files when expanding directories and globs.
    /// Explicitly named files are always processed regardless.
    #[arg(long)]
    no_ignore: bool,
}

/// Where the schemas to sort come from. Stdin and files are mutually exclusive,
/// and `--write` only makes sense for files; modelling them as one enum keeps
/// those rules in one place instead of scattered guard clauses.
enum Source {
    Stdin,
    Files(Vec<PathBuf>),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let options = build_options(&args);

    match resolve_sources(&args.inputs, !args.no_ignore)? {
        Source::Stdin => {
            if args.write {
                return Err("--write requires a real input file".into());
            }
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let output = json_schema_sort::to_string_sorted_with_options(&input, options)?;

            if args.check {
                if normalize_trailing_newline(&input) != output {
                    return Err("input is not sorted".into());
                }
                return Ok(());
            }
            print!("{output}");
        }
        Source::Files(files) => {
            if !args.write && !args.check && files.len() > 1 {
                return Err(
                    "refusing to print multiple files to stdout; pass --write or --check".into(),
                );
            }

            let mut unsorted = Vec::new();
            for file in &files {
                let input = fs::read_to_string(file)?;
                let output = json_schema_sort::to_string_sorted_with_options(&input, options)?;
                let already_sorted = normalize_trailing_newline(&input) == output;

                if args.check {
                    if !already_sorted {
                        unsorted.push(file.display().to_string());
                    }
                } else if args.write {
                    // Only touch the file when it actually changes, so mtimes
                    // and build caches stay stable for already-sorted inputs.
                    if !already_sorted {
                        fs::write(file, output)?;
                    }
                } else {
                    print!("{output}");
                }
            }

            if !unsorted.is_empty() {
                for label in &unsorted {
                    eprintln!("not sorted: {label}");
                }
                return Err(format!("{} file(s) not sorted", unsorted.len()).into());
            }
        }
    }

    Ok(())
}

fn build_options(args: &Args) -> SortOptions {
    let mut options = SortOptions::default();
    options.arrays = if args.no_sort_arrays {
        ArraySorting::None
    } else {
        ArraySorting::SchemaSafe
    };
    options.properties = if args.preserve_property_order {
        PropertyOrdering::Preserve
    } else {
        PropertyOrdering::Sorted
    };
    options
}

/// Expand the positional inputs into a concrete [`Source`]. Files are gathered
/// into a sorted, de-duplicated set so output is deterministic regardless of
/// argument order, glob ordering, or directory traversal order.
///
/// When `respect_gitignore` is set, files discovered by expanding a directory
/// or glob are filtered through git's ignore rules. Files the user named
/// explicitly are always kept — naming a path is an explicit override.
fn resolve_sources(inputs: &[String], respect_gitignore: bool) -> Result<Source, Box<dyn Error>> {
    if inputs.is_empty() || (inputs.len() == 1 && inputs[0] == "-") {
        return Ok(Source::Stdin);
    }
    if inputs.iter().any(|input| input == "-") {
        return Err("cannot mix stdin ('-') with file inputs".into());
    }

    let mut explicit = BTreeSet::new();
    let mut expanded = BTreeSet::new();
    for input in inputs {
        let path = Path::new(input);
        if path.is_dir() {
            collect_glob(&format!("{input}/**/*.json"), &mut expanded)?;
        } else if path.is_file() {
            explicit.insert(path.to_path_buf());
        } else {
            collect_glob(input, &mut expanded)?;
        }
    }

    if respect_gitignore {
        remove_gitignored(&mut expanded)?;
    }

    let mut files = explicit;
    files.append(&mut expanded);
    if files.is_empty() {
        return Err(format!("no files matched: {}", inputs.join(", ")).into());
    }
    Ok(Source::Files(files.into_iter().collect()))
}

/// Drop paths that git ignores, using `git check-ignore` so the full ignore
/// hierarchy (nested `.gitignore`, excludes, global config) is honored exactly.
/// If git is unavailable or the inputs are not inside a repository, nothing is
/// filtered — there is no ignore information to act on.
fn remove_gitignored(files: &mut BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
    if files.is_empty() {
        return Ok(());
    }

    let mut child = match Command::new("git")
        .args(["check-ignore", "--stdin", "-z"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok(()),
    };

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        for file in files.iter() {
            stdin.write_all(file.as_os_str().as_encoded_bytes())?;
            stdin.write_all(&[0])?;
        }
    }

    let output = child.wait_with_output()?;
    // 0 = some paths ignored, 1 = none ignored; anything else (e.g. 128 outside
    // a repo) means there is no ignore data to apply, so leave the set as-is.
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Ok(());
    }

    let ignored: BTreeSet<&[u8]> = output
        .stdout
        .split(|&byte| byte == 0)
        .filter(|path| !path.is_empty())
        .collect();
    files.retain(|file| !ignored.contains(file.as_os_str().as_encoded_bytes()));
    Ok(())
}

fn collect_glob(pattern: &str, out: &mut BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in glob::glob(pattern)? {
        let path = entry?;
        if path.is_file() {
            out.insert(path);
        }
    }
    Ok(())
}

fn normalize_trailing_newline(input: &str) -> String {
    let mut value = input.trim_end_matches(['\r', '\n']).to_owned();
    value.push('\n');
    value
}
