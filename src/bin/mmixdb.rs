use checksmix::{Command, Debugger, Host, MMixAssembler, StdHost, TrapCode, parse_command};
use clap::Parser;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;

#[derive(Parser, Debug)]
#[command(
    name = "mmixdb",
    about = "An interactive source-level debugger for MMIX .mms programs",
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

    /// Feed a guest's StdIn reads from FILE, read whole at startup and
    /// replayed from the start on every `run` (mmixdb's own stdin carries
    /// debugger commands, never the guest's input)
    #[arg(long, value_name = "FILE")]
    stdin: Option<PathBuf>,
}

/// The bytes behind a guest's `StdIn`, handed out to `Fread`/`Fgets`/
/// `Fgetws` one `Host::read` call at a time and rewound to the start before
/// every run.
struct GuestInput {
    bytes: Vec<u8>,
    pos: usize,
}

impl GuestInput {
    fn read(&mut self, buf: &mut [u8]) -> usize {
        let remaining = &self.bytes[self.pos..];
        let n = remaining.len().min(buf.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        n
    }

    fn rewind(&mut self) {
        self.pos = 0;
    }
}

/// `mmixdb`'s own `Host`: every process-level effect but `read` goes to
/// `StdHost`. `read` never touches the process's real stdin, which
/// `rustyline` owns for debugger commands; with `--stdin` it serves
/// `input`'s bytes instead, and without it a guest `StdIn` read fails, as it
/// does under the default `Host::read`.
struct DebuggerHost {
    stdio: StdHost,
    input: Option<Rc<RefCell<GuestInput>>>,
}

impl Host for DebuggerHost {
    fn write(&mut self, fd: u8, bytes: &[u8]) -> io::Result<()> {
        self.stdio.write(fd, bytes)
    }

    fn read(&mut self, fd: u8, buf: &mut [u8]) -> io::Result<usize> {
        match (&self.input, fd) {
            (Some(input), 0) => Ok(input.borrow_mut().read(buf)),
            _ => Err(io::Error::from(io::ErrorKind::Unsupported)),
        }
    }

    fn flush(&mut self) {
        self.stdio.flush()
    }

    fn now_micros(&mut self) -> u64 {
        self.stdio.now_micros()
    }

    fn diagnostic(&mut self, msg: &str) {
        self.stdio.diagnostic(msg)
    }

    fn trap(&mut self, code: TrapCode, arg: u8, arg255: u64, result255: u64) {
        self.stdio.trap(code, arg, arg255, result255)
    }
}

/// Reads `path` whole into the shared buffer a `DebuggerHost` serves a
/// guest's `StdIn` from. Exits the process on a read failure, in the form
/// `assemble_sources` uses for a source file.
fn load_guest_input(path: Option<PathBuf>) -> Option<Rc<RefCell<GuestInput>>> {
    path.map(|path| {
        let bytes = fs::read(&path).unwrap_or_else(|e| {
            eprintln!("mmixdb: error reading '{}': {}", path.display(), e);
            process::exit(1);
        });
        Rc::new(RefCell::new(GuestInput { bytes, pos: 0 }))
    })
}

/// Mirrors `Debugger::execute`'s own `Command::Repeat` resolution just
/// closely enough to know whether `cmd` is `run` itself, or a blank line
/// repeating it -- the two cases the guest's `StdIn` must rewind for.
/// `Debugger` exposes no way to ask it directly. `last_was_run` is whether
/// the previous non-repeat command was `Run`.
fn command_resolves_to_run(cmd: &Command, last_was_run: bool) -> bool {
    matches!(cmd, Command::Run) || (matches!(cmd, Command::Repeat) && last_was_run)
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

    let guest_input = load_guest_input(cli.stdin);

    let host = DebuggerHost {
        stdio: StdHost,
        input: guest_input.clone(),
    };
    let mut debugger = Debugger::load_with_host(assembler, host);
    let fullname = cli.fullname || std::env::var_os("INSIDE_EMACS").is_some();
    debugger.set_fullname(fullname);

    for line in debugger.initial_report() {
        print_line(&line);
    }

    let mut rl = DefaultEditor::new().unwrap_or_else(|e| {
        eprintln!("mmixdb: failed to initialize line editor: {e}");
        process::exit(1);
    });

    let mut last_was_run = false;

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
                        if command_resolves_to_run(&cmd, last_was_run)
                            && let Some(input) = &guest_input
                        {
                            input.borrow_mut().rewind();
                        }
                        if !matches!(cmd, Command::Repeat) {
                            last_was_run = matches!(cmd, Command::Run);
                        }
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
        // A trimmed-away segment (blank space, or an INCLUDE resolving to
        // nothing) leaves this input with no translation unit at all.
        if units.is_empty() {
            return Err(format!("'{}' contributed no source", path.display()));
        }
        sources.extend(units);
    }
    let (first_name, first_src) = &sources[0];
    let mut asm = MMixAssembler::new(first_src, first_name);
    for (name, src) in sources.iter().skip(1) {
        asm.add_source(src, name);
    }
    asm.parse()?;
    for warning in asm.warnings() {
        eprintln!("{}", warning);
    }
    Ok(asm)
}
