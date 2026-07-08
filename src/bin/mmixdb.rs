use checksmix::{Command, Debugger, MMixAssembler, parse_command};
use clap::Parser;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser, Debug)]
#[command(
    name = "mmixdb",
    about = "A gdb-style interactive debugger for MMIX .mms programs",
    version,
    author
)]
struct Cli {
    /// MMIX assembly source file(s) to debug (.mms only; multiple files are
    /// assembled into one shared symbol space, like `checksmix run`)
    #[arg(required = true, num_args = 1.., value_name = "FILE.mms")]
    program_files: Vec<String>,

    /// Emit Emacs GUD `--fullname` stop markers (auto-enabled under Emacs;
    /// see `contrib/mmixdb.el`)
    #[arg(long, alias = "emacs")]
    fullname: bool,
}

fn main() {
    let cli = Cli::parse();

    for f in &cli.program_files {
        let ext = Path::new(f).extension().and_then(|s| s.to_str());
        if ext != Some("mms") {
            eprintln!(
                "mmixdb: '{}' is not a .mms source file -- mmixdb debugs MMIX assembly sources \
                 only (source-line debugging requires source; .mmo object files are out of scope)",
                f
            );
            process::exit(1);
        }
    }

    let assembler = assemble_sources(&cli.program_files).unwrap_or_else(|e| {
        eprintln!("mmixdb: {e}");
        process::exit(1);
    });

    let mut debugger = Debugger::load(assembler);
    let fullname = cli.fullname || std::env::var_os("INSIDE_EMACS").is_some();
    debugger.set_fullname(fullname);

    for line in debugger.initial_report() {
        print_line(&line);
    }

    let mut rl = DefaultEditor::new().unwrap_or_else(|e| {
        eprintln!("mmixdb: failed to initialize line editor: {e}");
        process::exit(1);
    });

    loop {
        match rl.readline("(mmixdb) ") {
            Ok(line) => {
                if !line.trim().is_empty() {
                    let _ = rl.add_history_entry(line.as_str());
                }
                match parse_command(&line) {
                    Ok(Command::Quit) => {
                        println!("Quit");
                        break;
                    }
                    Ok(cmd) => {
                        for out in debugger.execute(cmd) {
                            print_line(&out);
                        }
                    }
                    Err(e) => println!("{e}"),
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("mmixdb: {e}");
                break;
            }
        }
    }
}

fn print_line(line: &str) {
    if line.ends_with('\n') {
        print!("{line}");
    } else {
        println!("{line}");
    }
}

/// Replicates `run_mms`'s assembly step (`src/bin/checksmix.rs:assemble_sources`):
/// read each file, resolve INCLUDE directives, assemble the first as the
/// primary source, add the rest as additional translation units in one
/// shared symbol space.
fn assemble_sources(filenames: &[String]) -> Result<MMixAssembler, String> {
    let reader = |p: &Path| fs::read_to_string(p);
    let paths: Vec<PathBuf> = filenames.iter().map(PathBuf::from).collect();
    let mut sources: Vec<(String, String)> = Vec::new();
    for path in &paths {
        let src = fs::read_to_string(path)
            .map_err(|err| format!("error reading '{}': {}", path.display(), err))?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let units = MMixAssembler::resolve_includes(&src, &path.to_string_lossy(), base, &reader)?;
        sources.extend(units);
    }
    let (first_name, first_src) = &sources[0];
    let mut asm = MMixAssembler::new(first_src, first_name);
    for (name, src) in sources.iter().skip(1) {
        asm.add_source(src, name);
    }
    asm.parse()?;
    Ok(asm)
}
