//! `mmixdb` debugger core.
//!
//! This module holds all state and command logic for the interactive MMIX
//! debugger. It has no TTY dependency: every command is a method that
//! mutates a `Debugger` and returns rendered text, so the whole thing is
//! unit-testable without a terminal. `src/bin/mmixdb.rs` is a thin shell
//! that reads lines (via `rustyline`), calls `parse_command` and
//! `Debugger::execute`, and prints the result.

use crate::mmix::{Host, MMix, SpecialReg, ValueFormat};
use crate::mmixal::{MMixAssembler, SymbolType};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The MMIX text/data segment boundary. Mirrors `run_mms`'s fallback
/// (`src/bin/checksmix.rs`): when no `Main` label exists, the entry point is
/// the first instruction address below this boundary.
const SEGMENT_BOUNDARY: u64 = 0x2000000000000000;

/// Per-instruction cap on every multi-instruction step loop (`do_step`,
/// `do_next`, `do_continue`), so a subroutine or program that never
/// returns/halts can't hang the debugger. Not configurable from the public
/// API.
const STEP_BUDGET: usize = 1_000_000;

/// A parsed debugger command. One variant per command in the command table;
/// `Repeat` represents blank input, which re-runs the last executed command.
///
/// Non-exhaustive: a new command is an additive change here, not a breaking
/// one, so match on it with a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Command {
    Step,
    Stepi,
    Next,
    Continue,
    Run,
    Break(String),
    Breakpoints,
    Delete(Option<String>),
    Print(String),
    State,
    List,
    Help,
    Quit,
    Repeat,
}

/// Parse one line of debugger input into a `Command`.
///
/// Supports both the short letter and the long word for each command
/// (the long words matter: Emacs GUD sends them). Blank input is `Repeat`.
/// Unknown input returns an error string for the REPL to print and continue.
pub fn parse_command(input: &str) -> Result<Command, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Command::Repeat);
    }
    let (head, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((h, r)) => (h, r.trim()),
        None => (trimmed, ""),
    };
    match head {
        "s" | "step" => Ok(Command::Step),
        "si" | "stepi" => Ok(Command::Stepi),
        "n" | "next" => Ok(Command::Next),
        "c" | "continue" => Ok(Command::Continue),
        "r" | "run" => Ok(Command::Run),
        "b" | "break" => {
            if rest.is_empty() {
                Err("break requires a line number or label".to_string())
            } else {
                Ok(Command::Break(rest.to_string()))
            }
        }
        "d" | "delete" => Ok(Command::Delete(if rest.is_empty() {
            None
        } else {
            Some(rest.to_string())
        })),
        "p" | "print" => {
            if rest.is_empty() {
                Err("print requires an argument".to_string())
            } else {
                Ok(Command::Print(rest.to_string()))
            }
        }
        "bt" | "backtrace" => Ok(Command::State),
        "info" => match rest {
            "reg" | "registers" => Ok(Command::State),
            "break" | "breakpoints" => Ok(Command::Breakpoints),
            "" => Err("info requires a subcommand (reg|registers|break|breakpoints)".to_string()),
            other => Err(format!("unknown info subcommand: {other}")),
        },
        "l" | "list" => Ok(Command::List),
        "h" | "help" | "?" => Ok(Command::Help),
        "q" | "quit" | "exit" => Ok(Command::Quit),
        other => Err(format!("unknown command: {other}")),
    }
}

/// Resolve a special-register name against `SpecialReg::name`, the single
/// table the state dump and the assembler's predefined symbols also spell
/// registers from.
fn special_reg_from_name(name: &str) -> Option<SpecialReg> {
    (0u8..32)
        .filter_map(SpecialReg::from_u8)
        .find(|reg| reg.name() == name)
}

fn format_value(value: u64, format: ValueFormat) -> String {
    match format {
        ValueFormat::Signed => (value as i64).to_string(),
        ValueFormat::Unsigned => value.to_string(),
    }
}

/// Write every assembled instruction's encoded bytes into `mmix`'s memory,
/// apply every `GREG` initializer to its allocated register, and raise `rG`
/// to mark where the global register range actually starts.
pub fn write_image(mmix: &mut MMix, assembler: &MMixAssembler) {
    for (addr, inst) in &assembler.instructions {
        let bytes = assembler.encode_instruction_bytes(inst);
        for (offset, &byte) in bytes.iter().enumerate() {
            mmix.write_loaded_byte(addr + offset as u64, byte);
        }
    }

    for &(reg, value) in &assembler.greg_inits {
        mmix.set_register(reg, value);
    }

    // GREG allocates downward from $254, so the lowest-numbered allocated
    // register is where the global range starts. MMIX requires rG >= 32
    // (set_register relies on it to keep local-window growth confined to
    // registers below rG), so the derived value is floored there, never
    // clamped down. With no GREG directive, rG keeps MMix::initialize's
    // default of 32.
    if let Some(min_reg) = assembler.greg_inits.iter().map(|&(reg, _)| reg).min() {
        mmix.set_special(SpecialReg::RG, std::cmp::max(min_reg as u64, 32));
    }
}

/// The program's entry point: the `Main` label if present, else the first
/// code address below the text/data segment boundary.
pub fn entry_point(assembler: &MMixAssembler) -> u64 {
    if let Some(&main_addr) = assembler.labels.get("Main") {
        return main_addr;
    }
    assembler
        .instructions
        .iter()
        .find(|(addr, _)| *addr < SEGMENT_BOUNDARY)
        .map(|(addr, _)| *addr)
        .unwrap_or(0x100)
}

/// The interactive debugger core: owns the loaded `MMix`, the `MMixAssembler`
/// (for the source map and symbol tables), breakpoints, and REPL state.
///
/// Holding an `MMix` makes `Debugger` none of `Send`, `Sync`, `UnwindSafe`,
/// or `RefUnwindSafe` — see the [`MMix`] docs.
pub struct Debugger {
    mmix: MMix,
    assembler: MMixAssembler,
    entry: u64,
    primary_file: Option<String>,
    breakpoints: BTreeSet<u64>,
    /// Set when a resume saw the program halt, so a later resume refuses
    /// rather than executing past the end of the image. `reset` clears it.
    exited: bool,
    last_command: Option<Command>,
    fullname: bool,
    format: ValueFormat,
}

impl Debugger {
    /// Load an assembled program: run the `run_mms` load sequence (write
    /// every instruction's bytes to memory, then resolve the entry point)
    /// and set PC there.
    pub fn load(assembler: MMixAssembler) -> Debugger {
        Self::with_machine(MMix::new(), assembler)
    }

    /// Load an assembled program into a machine whose process-level effects
    /// go to `host` rather than the process — the entry point an embedder
    /// needs, since [`Debugger::load`] installs [`crate::StdHost`] and offers
    /// no way to reach the output afterwards.
    ///
    /// `Command::Run` resets the machine between runs but keeps the host, so
    /// a host that accumulates output sees every run appended. Clear the
    /// host's buffers between runs if that is not what you want.
    pub fn load_with_host<H: Host + 'static>(assembler: MMixAssembler, host: H) -> Debugger {
        Self::with_machine(MMix::with_host(host), assembler)
    }

    fn with_machine(mut mmix: MMix, assembler: MMixAssembler) -> Debugger {
        write_image(&mut mmix, &assembler);
        let entry = entry_point(&assembler);
        mmix.set_pc(entry);
        let primary_file = assembler.source_loc(entry).map(|loc| loc.file.clone());
        Debugger {
            mmix,
            assembler,
            entry,
            primary_file,
            breakpoints: BTreeSet::new(),
            exited: false,
            last_command: None,
            fullname: false,
            format: ValueFormat::Signed,
        }
    }

    /// The loaded machine, for reading final register or memory state after
    /// a run without parsing the command output.
    pub fn machine(&self) -> &MMix {
        &self.mmix
    }

    /// The loaded machine, mutably — the route to the installed [`Host`] via
    /// [`MMix::host_mut`].
    pub fn machine_mut(&mut self) -> &mut MMix {
        &mut self.mmix
    }

    pub fn set_fullname(&mut self, on: bool) {
        self.fullname = on;
    }

    pub fn fullname(&self) -> bool {
        self.fullname
    }

    pub fn set_format(&mut self, format: ValueFormat) {
        self.format = format;
    }

    /// The report to show at startup, before any command has run.
    pub fn initial_report(&self) -> Vec<String> {
        self.report(false)
    }

    /// Execute a parsed command, returning the rendered output lines.
    /// `Command::Repeat` re-executes the last executed command; if there is
    /// none, an explanatory message is returned instead.
    pub fn execute(&mut self, cmd: Command) -> Vec<String> {
        let resolved = match cmd {
            Command::Repeat => match self.last_command.clone() {
                Some(c) => c,
                None => return vec!["No previous command.".to_string()],
            },
            other => other,
        };
        let output = match &resolved {
            Command::Step => self.do_step(),
            Command::Stepi => self.do_stepi(),
            Command::Next => self.do_next(),
            Command::Continue => self.do_continue(),
            Command::Run => self.do_run(),
            Command::Break(arg) => vec![self.do_break(arg.clone())],
            Command::Breakpoints => self.do_breakpoints(),
            Command::Delete(arg) => vec![self.do_delete(arg.clone())],
            Command::Print(arg) => vec![self.do_print(arg)],
            Command::State => self.do_state(),
            Command::List => self.do_list(),
            Command::Help => self.do_help(),
            Command::Quit => vec!["Quit".to_string()],
            Command::Repeat => unreachable!("resolved above"),
        };
        self.last_command = Some(resolved);
        output
    }

    fn reset(&mut self) {
        self.mmix.reset();
        write_image(&mut self.mmix, &self.assembler);
        self.mmix.set_pc(self.entry);
        self.exited = false;
    }

    /// gdb's refusal for a resume command issued after the program has
    /// exited. `run` never sees it: it resets first.
    fn refuse_when_exited(&self) -> Option<Vec<String>> {
        self.exited
            .then(|| vec!["The program is not being run.".to_string()])
    }

    /// `stepi`: execute exactly one instruction, following into calls and
    /// branches. The stop lands wherever the instruction left the PC, which
    /// for a pseudo-op is usually mid-expansion.
    fn do_stepi(&mut self) -> Vec<String> {
        if let Some(refusal) = self.refuse_when_exited() {
            return refusal;
        }
        let running = self.mmix.execute_instruction();
        self.report_stop(!running, false)
    }

    /// `step`: run until the PC reaches an address on a different source
    /// line, following into calls. Also stops on a breakpoint, a halt, or
    /// `STEP_BUDGET` (a line that never ends can't hang this). An address
    /// with no source location is not a new line.
    fn do_step(&mut self) -> Vec<String> {
        if let Some(refusal) = self.refuse_when_exited() {
            return refusal;
        }
        let origin = self.origin_line();
        let mut halted = false;
        let mut budget_exhausted = false;
        let mut steps = 0usize;
        loop {
            if steps >= STEP_BUDGET {
                budget_exhausted = true;
                break;
            }
            if !self.mmix.execute_instruction() {
                halted = true;
                break;
            }
            steps += 1;
            if self.breakpoints.contains(&self.mmix.get_pc()) || self.reached_new_line(&origin) {
                break;
            }
        }
        self.report_stop(halted, budget_exhausted)
    }

    /// `next`: like `step`, but stepping over calls. The new line only
    /// counts once the call depth is back at or below where it started
    /// (PUSHJ/PUSHGO push a frame; GO does not) -- a callee's first
    /// instruction is on a different source line, so testing the line alone
    /// would stop inside the call.
    fn do_next(&mut self) -> Vec<String> {
        if let Some(refusal) = self.refuse_when_exited() {
            return refusal;
        }
        let origin = self.origin_line();
        let depth = self.mmix.call_depth();
        let mut halted = false;
        let mut budget_exhausted = false;
        let mut steps = 0usize;
        loop {
            if steps >= STEP_BUDGET {
                budget_exhausted = true;
                break;
            }
            if !self.mmix.execute_instruction() {
                halted = true;
                break;
            }
            steps += 1;
            if self.breakpoints.contains(&self.mmix.get_pc()) {
                break;
            }
            if self.mmix.call_depth() <= depth && self.reached_new_line(&origin) {
                break;
            }
        }
        self.report_stop(halted, budget_exhausted)
    }

    /// The source line the PC sits on, owned so a stepping loop can keep it
    /// across the machine mutations it makes.
    fn origin_line(&self) -> Option<(String, usize)> {
        self.assembler
            .source_loc(self.mmix.get_pc())
            .map(|loc| (loc.file.clone(), loc.line))
    }

    /// Whether the PC has reached a source line other than `origin`. An
    /// address with no source location answers false: it is inside no line,
    /// so it is not a new one.
    fn reached_new_line(&self, origin: &Option<(String, usize)>) -> bool {
        let Some(loc) = self.assembler.source_loc(self.mmix.get_pc()) else {
            return false;
        };
        match origin {
            Some((file, line)) => loc.line != *line || loc.file != *file,
            None => true,
        }
    }

    /// `continue`: single-step from the current PC until a breakpoint
    /// address is hit, the program halts, or `STEP_BUDGET` is reached (a
    /// program that never halts can't hang this).
    fn do_continue(&mut self) -> Vec<String> {
        if let Some(refusal) = self.refuse_when_exited() {
            return refusal;
        }
        let mut halted = false;
        let mut budget_exhausted = false;
        let mut steps = 0usize;
        loop {
            if steps >= STEP_BUDGET {
                budget_exhausted = true;
                break;
            }
            if !self.mmix.execute_instruction() {
                halted = true;
                break;
            }
            steps += 1;
            if self.breakpoints.contains(&self.mmix.get_pc()) {
                break;
            }
        }
        self.report_stop(halted, budget_exhausted)
    }

    /// `run`/reset: reset the machine to the freshly-loaded image, stop
    /// there if a breakpoint sits on the entry point, and otherwise behave
    /// like `continue`. `continue` executes an instruction before testing
    /// the breakpoint set, so that resuming from a stop does not re-trigger
    /// on the breakpoint it is sitting at; after a reset nothing has run
    /// yet, so the entry breakpoint has to be honored first.
    fn do_run(&mut self) -> Vec<String> {
        self.reset();
        if self.breakpoints.contains(&self.mmix.get_pc()) {
            return self.report(false);
        }
        self.do_continue()
    }

    fn do_break(&mut self, arg: String) -> String {
        let arg = arg.trim();
        match self.resolve_break_location(arg) {
            Some(addr) => {
                self.breakpoints.insert(addr);
                format!("Breakpoint set at 0x{addr:x} ({arg})")
            }
            None => format!("No location found for '{arg}'; breakpoint not set"),
        }
    }

    /// Resolve a `break`/`delete` argument to an address, in priority order:
    /// a decimal source line in the current file, else an exact label.
    fn resolve_break_location(&self, arg: &str) -> Option<u64> {
        let arg = arg.trim();
        if let Ok(line) = arg.parse::<usize>() {
            self.current_file()
                .and_then(|file| self.assembler.addr_for_line(&file, line))
        } else {
            self.assembler.labels.get(arg).copied()
        }
    }

    fn do_delete(&mut self, arg: Option<String>) -> String {
        match arg {
            None => {
                let n = self.breakpoints.len();
                self.breakpoints.clear();
                format!("Deleted {n} breakpoint(s).")
            }
            Some(arg) => {
                let arg = arg.trim();
                match self.resolve_break_location(arg) {
                    Some(addr) => {
                        if self.breakpoints.remove(&addr) {
                            format!("Deleted breakpoint at 0x{addr:x} ({arg})")
                        } else {
                            format!("No breakpoint at 0x{addr:x} ({arg})")
                        }
                    }
                    None => format!("No location found for '{arg}'; nothing deleted"),
                }
            }
        }
    }

    /// `print <arg>` resolution, in priority order: `$N`/bare `N` (general
    /// register), a special-register name, a label, an IS/GREG symbol, a hex
    /// address (the memory octa at its aligned 8-byte base), else an error.
    fn do_print(&self, arg: &str) -> String {
        let arg = arg.trim();
        if let Some(value) = self.print_register(arg) {
            return value;
        }
        if let Some(reg) = special_reg_from_name(arg) {
            return format_value(self.mmix.get_special(reg), self.format);
        }
        if let Some(&addr) = self.assembler.labels.get(arg) {
            return format_value(addr, self.format);
        }
        if let Some(sym) = self.assembler.symbols.get(arg) {
            return match sym {
                SymbolType::Register(n) => format_value(self.mmix.get_register(*n), self.format),
                SymbolType::Constant(v) => format_value(*v, self.format),
            };
        }
        if let Some(addr) = self.parse_hex_address(arg) {
            return format_value(self.mmix.read_octa(addr), self.format);
        }
        format!("No symbol \"{arg}\" in current context.")
    }

    fn print_register(&self, arg: &str) -> Option<String> {
        let digits = arg.strip_prefix('$').unwrap_or(arg);
        let n: u16 = digits.parse().ok()?;
        if n > 255 {
            return None;
        }
        Some(format_value(self.mmix.get_register(n as u8), self.format))
    }

    fn parse_hex_address(&self, arg: &str) -> Option<u64> {
        let digits = arg.strip_prefix("0x").or_else(|| arg.strip_prefix('#'))?;
        u64::from_str_radix(digits, 16).ok()
    }

    fn do_state(&self) -> Vec<String> {
        format!("{}", self.mmix.display_with(self.format))
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn do_list(&self) -> Vec<String> {
        let pc = self.mmix.get_pc();
        match self.assembler.source_loc(pc) {
            Some(loc) => {
                let start = loc.line.saturating_sub(2).max(1);
                let end = loc.line + 2;
                (start..=end)
                    .filter_map(|line| {
                        self.assembler.source_text(&loc.file, line).map(|text| {
                            let marker = if line == loc.line { ">" } else { " " };
                            format!("{marker} {line}\t{text}")
                        })
                    })
                    .collect()
            }
            None => vec!["No source line for the current location.".to_string()],
        }
    }

    fn do_breakpoints(&self) -> Vec<String> {
        if self.breakpoints.is_empty() {
            return vec!["No breakpoints set.".to_string()];
        }
        self.breakpoints
            .iter()
            .map(|&addr| match self.assembler.source_loc(addr) {
                Some(loc) => format!("0x{addr:x}  {}:{}", loc.file, loc.line),
                None => format!("0x{addr:x}  (no source line)"),
            })
            .collect()
    }

    fn current_file(&self) -> Option<String> {
        self.assembler
            .source_loc(self.mmix.get_pc())
            .map(|loc| loc.file.clone())
            .or_else(|| self.primary_file.clone())
    }

    /// The report shown on every stop: the Emacs GUD marker (if `fullname`
    /// mode is on and the current PC has a known source location) followed
    /// by the current-line display, or a halt message.
    fn report(&self, halted: bool) -> Vec<String> {
        if halted {
            return vec![format!(
                "Program exited with code {}.",
                self.mmix.get_exit_code()
            )];
        }
        let mut lines = Vec::new();
        if self.fullname
            && let Some(marker) = self.emacs_marker()
        {
            lines.push(marker);
        }
        lines.push(self.location_line());
        lines
    }

    /// [`Debugger::report`], with one line appended when `STEP_BUDGET` —
    /// not a halt or a breakpoint — is what stopped the step loop.
    /// Appending rather than replacing keeps `report`'s shape intact for
    /// every other stop reason. Records a halt, which is what makes the
    /// next resume refuse.
    fn report_stop(&mut self, halted: bool, budget_exhausted: bool) -> Vec<String> {
        self.exited |= halted;
        let mut lines = self.report(halted);
        if budget_exhausted {
            lines.push("still running (step budget exhausted)".to_string());
        }
        lines
    }

    /// The current-line display: `file:line<TAB>text`, prefixed with
    /// `0x<ADDR><TAB>` when the PC sits inside a line rather than at its
    /// first address. An address no statement emitted has no line to name.
    fn location_line(&self) -> String {
        let pc = self.mmix.get_pc();
        let Some(loc) = self.assembler.source_loc(pc) else {
            return format!("0x{pc:016x} in ?? (no source line)");
        };
        let text = self
            .assembler
            .source_text(&loc.file, loc.line)
            .unwrap_or("");
        let line = format!("{}:{}\t{}", loc.file, loc.line, text);
        if self.assembler.addr_for_line(&loc.file, loc.line) == Some(pc) {
            line
        } else {
            format!("0x{pc:016x}\t{line}")
        }
    }

    /// The Emacs GUD marker for the current stop: `\x1a\x1a<ABSOLUTE-PATH>:<LINE>:0:beg:0x<ADDR>\n`.
    /// `None` when the current PC has no known source location.
    fn emacs_marker(&self) -> Option<String> {
        let pc = self.mmix.get_pc();
        let loc = self.assembler.source_loc(pc)?;
        let path = absolute_path(&loc.file);
        Some(format!(
            "\x1a\x1a{}:{}:0:beg:0x{:x}\n",
            path.display(),
            loc.line,
            pc
        ))
    }

    /// Keep this in sync with README.md's mmixdb command table
    /// -- there is no shared source between the two.
    fn do_help(&self) -> Vec<String> {
        const HELP_TEXT: &str = "\
step (into)   s, step                          Execute one source line, following into calls/branches.
next (over)   n, next                          Execute one source line, stepping over any call it makes.
stepi         si, stepi                        Execute exactly one instruction, following into calls/branches.
continue      c, continue                      Resume, single-stepping until a breakpoint or halt.
run/reset     r, run                           Reset to the freshly-loaded image, then run on; a breakpoint on the entry point fires.
break         b <line>, b <label>, break …     Set a breakpoint at a source line or label.
delete        d, delete, d <line>, d <label>   Delete one breakpoint, or every breakpoint given no argument.
print         p <arg>, print <arg>             Print a register, special register, label address, IS/GREG symbol, or the memory octa at the address's aligned base.
state         bt, backtrace, info reg, info registers   Print the full register dump.
breakpoints   info break, info breakpoints     List every currently-set breakpoint with its source location.
list          l, list                          Print source lines around the current PC.
help          h, help, ?                       Show this help.
quit          q, quit, exit                    Exit the debugger.

Blank input repeats the last command. Once the program has exited, step,
stepi, next and continue are refused; run restarts it.";
        HELP_TEXT.lines().map(str::to_string).collect()
    }
}

/// The canonicalized absolute path of `file`, when it exists on disk; falls
/// back to joining it onto the current directory (without resolving `..` or
/// symlinks) when it does not, so the marker format is still well-defined for
/// in-memory sources that have no backing file.
fn absolute_path(file: &str) -> PathBuf {
    std::fs::canonicalize(file).unwrap_or_else(|_| {
        let path = Path::new(file);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assemble(source: &str, filename: &str) -> MMixAssembler {
        let mut asm = MMixAssembler::new(source, filename);
        asm.parse().expect("test source must assemble");
        asm
    }

    const CALL_PROGRAM: &str = "\
\tLOC\t#100
Main\tPUSHJ\t$0,Sub
\tSETI\t$1,7
\tTRAP\t0,Halt,0
Sub\tSETI\t$0,3
\tPOP\t0,0
";

    /// A stack program whose every statement is a pseudo-op or a plain
    /// instruction, so `SETI`'s four-tetra expansion sits between two
    /// single-tetra lines. `Main` is line 8 at 0x100; line 9 begins at 0x110.
    const STACK_PROGRAM: &str = "\
        LOC     Data_Segment
Cells   OCTA    0
        OCTA    0
        OCTA    0
Sp      GREG    Cells

        LOC     #100
Main    SETI    $1,7
        STOI    $1,Sp,0
        ADDUI   Sp,Sp,8
        SETI    $1,35
        STOI    $1,Sp,0
        LDOI    $2,Sp,0
        SUBUI   Sp,Sp,8
        LDOI    $3,Sp,0
        ADDU    $255,$2,$3
        TRAP    0,Halt,0
";

    /// Writes `Hi` to fd 1, then halts.
    ///
    /// Uses the literal 1 rather than the `StdOut` symbol: `StdOut` resolves
    /// through `stdio_raw_identifiers`, which yields a raw handle on Windows
    /// rather than 1, and only fd 1 and 2 reach the host. These tests are
    /// about host routing, not symbol resolution.
    const GREETING_PROGRAM: &str = "\
\tLOC\t#100
Main\tLDA\t$255,Text
\tTRAP\t0,Fputs,1
\tTRAP\t0,Halt,0
Text\tBYTE\t\"Hi\",0
";

    /// Records what a program writes to stdout, shared with the test.
    #[derive(Clone, Default)]
    struct Recorder(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Host for Recorder {
        fn write(&mut self, _fd: u8, bytes: &[u8]) -> std::io::Result<()> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(())
        }
        fn now_micros(&mut self) -> u64 {
            0
        }
        fn diagnostic(&mut self, _msg: &str) {}
    }

    #[test]
    fn load_with_host_routes_program_output_to_the_host() {
        let recorder = Recorder::default();
        let mut debugger =
            Debugger::load_with_host(assemble(GREETING_PROGRAM, "hi.mms"), recorder.clone());
        debugger.execute(Command::Run);
        assert_eq!(&*recorder.0.borrow(), b"Hi");
    }

    #[test]
    fn a_second_run_reaches_the_same_host() {
        let recorder = Recorder::default();
        let mut debugger =
            Debugger::load_with_host(assemble(GREETING_PROGRAM, "hi.mms"), recorder.clone());
        debugger.execute(Command::Run);
        debugger.execute(Command::Run);
        // A second run reaches the same host rather than a fresh StdHost.
        assert_eq!(&*recorder.0.borrow(), b"HiHi");
    }

    #[test]
    fn parse_command_maps_all_forms() {
        assert_eq!(parse_command("s"), Ok(Command::Step));
        assert_eq!(parse_command("step"), Ok(Command::Step));
        assert_eq!(parse_command("si"), Ok(Command::Stepi));
        assert_eq!(parse_command("stepi"), Ok(Command::Stepi));
        assert_eq!(parse_command("n"), Ok(Command::Next));
        assert_eq!(parse_command("next"), Ok(Command::Next));
        assert_eq!(parse_command("c"), Ok(Command::Continue));
        assert_eq!(parse_command("continue"), Ok(Command::Continue));
        assert_eq!(parse_command("r"), Ok(Command::Run));
        assert_eq!(parse_command("run"), Ok(Command::Run));
        assert_eq!(parse_command("b 10"), Ok(Command::Break("10".to_string())));
        assert_eq!(
            parse_command("break Main"),
            Ok(Command::Break("Main".to_string()))
        );
        assert_eq!(parse_command("p $0"), Ok(Command::Print("$0".to_string())));
        assert_eq!(
            parse_command("print rJ"),
            Ok(Command::Print("rJ".to_string()))
        );
        assert_eq!(parse_command("bt"), Ok(Command::State));
        assert_eq!(parse_command("backtrace"), Ok(Command::State));
        assert_eq!(parse_command("info reg"), Ok(Command::State));
        assert_eq!(parse_command("info registers"), Ok(Command::State));
        assert_eq!(
            parse_command("d 10"),
            Ok(Command::Delete(Some("10".to_string())))
        );
        assert_eq!(
            parse_command("delete 10"),
            Ok(Command::Delete(Some("10".to_string())))
        );
        assert_eq!(parse_command("d"), Ok(Command::Delete(None)));
        assert_eq!(parse_command("delete"), Ok(Command::Delete(None)));
        assert_eq!(parse_command("info break"), Ok(Command::Breakpoints));
        assert_eq!(parse_command("info breakpoints"), Ok(Command::Breakpoints));
        assert_eq!(parse_command("l"), Ok(Command::List));
        assert_eq!(parse_command("list"), Ok(Command::List));
        assert_eq!(parse_command("q"), Ok(Command::Quit));
        assert_eq!(parse_command("quit"), Ok(Command::Quit));
        assert_eq!(parse_command("exit"), Ok(Command::Quit));
        assert_eq!(parse_command("h"), Ok(Command::Help));
        assert_eq!(parse_command("help"), Ok(Command::Help));
        assert_eq!(parse_command("?"), Ok(Command::Help));
        assert_eq!(parse_command(""), Ok(Command::Repeat));
        assert_eq!(parse_command("   "), Ok(Command::Repeat));
        assert!(parse_command("bogus").is_err());
    }

    #[test]
    fn help_command_lists_every_command() {
        let asm = assemble(CALL_PROGRAM, "call.mms");
        let mut dbg = Debugger::load(asm);
        let output = dbg.execute(Command::Help);
        let joined = output.join("\n");
        assert!(joined.contains("step"));
        assert!(joined.contains("stepi"));
        assert!(joined.contains("break"));
        assert!(joined.contains("print"));
        assert!(joined.contains("quit"));
        assert!(joined.contains("help"));
        assert!(joined.contains("delete"));
        assert!(joined.contains("info break"));
    }

    /// `next` lands on the head of the next source line every time, never
    /// mid-expansion: line 8's `SETI` occupies four tetras, and one `next`
    /// crosses all of them.
    #[test]
    fn next_advances_one_source_line_across_an_expansion() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let stops: Vec<String> = (0..5)
            .map(|_| dbg.execute(Command::Next).join("\n"))
            .collect();

        for (taken, stop) in stops.iter().enumerate() {
            let line = 9 + taken;
            assert!(
                stop.starts_with(&format!("stack.mms:{line}\t")),
                "next #{} must stop at the head of line {line}, got {stop:?}",
                taken + 1
            );
        }
    }

    /// `stepi` advances one instruction and still names the line it is
    /// inside, with the address in front.
    #[test]
    fn stepi_advances_one_instruction_inside_a_line() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let stop = dbg.execute(Command::Stepi);
        assert_eq!(dbg.mmix.get_pc(), 0x104);
        assert_eq!(
            stop,
            vec!["0x0000000000000104\tstack.mms:8\tMain    SETI    $1,7".to_string()]
        );
    }

    #[test]
    fn next_steps_over_a_call_step_steps_into_it() {
        let asm = assemble(CALL_PROGRAM, "call.mms");
        let mut dbg = Debugger::load(asm);
        // At Main: PUSHJ $0, Sub.
        let depth0 = dbg.mmix.call_depth();

        // `step` follows into the call.
        dbg.execute(Command::Step);
        assert!(
            dbg.mmix.call_depth() > depth0,
            "step across PUSHJ must increase call depth"
        );

        // Reset and take the `next` path instead.
        let asm = assemble(CALL_PROGRAM, "call.mms");
        let mut dbg = Debugger::load(asm);
        let depth0 = dbg.mmix.call_depth();
        let return_pc = dbg.mmix.get_pc().wrapping_add(4);
        let lines = dbg.execute(Command::Next);
        assert_eq!(
            dbg.mmix.call_depth(),
            depth0,
            "next across PUSHJ must return to the pre-call depth"
        );
        assert_eq!(
            dbg.mmix.get_pc(),
            return_pc,
            "next across PUSHJ must land back at the return address"
        );
        assert!(
            !lines.iter().any(|l| l.contains("step budget exhausted")),
            "a normal call-depth return must not be reported as budget-exhausted: {lines:?}"
        );
    }

    #[test]
    fn breakpoint_by_line_stops_there() {
        let source = "\tLOC\t#100\nMain\tSETI\t$1,1\n\tSETI\t$2,2\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "bp.mms");
        let target_line = 3; // "\tSET\t$2,2"
        let target_addr = asm
            .addr_for_line("bp.mms", target_line)
            .expect("line 3 must have an address");
        let mut dbg = Debugger::load(asm);
        dbg.execute(Command::Break(target_line.to_string()));
        let lines = dbg.execute(Command::Continue);
        assert_eq!(dbg.mmix.get_pc(), target_addr);
        assert_eq!(dbg.current_file().as_deref(), Some("bp.mms"));
        let loc = dbg.assembler.source_loc(dbg.mmix.get_pc()).unwrap();
        assert_eq!(loc.line, target_line);
        assert!(
            !lines.iter().any(|l| l.contains("step budget exhausted")),
            "a breakpoint stop must not be reported as budget-exhausted: {lines:?}"
        );
    }

    /// `SETI $X,imm` expands to four tetras. The first address renders the
    /// bare source line; the three inside it name the same line with the
    /// address in front, gdb's `stepi` shape.
    #[test]
    fn location_line_prefixes_the_address_inside_a_line() {
        let source = "\tLOC\t#100\nMain\tSETI\t$1,7\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "expand.mms");
        let mut dbg = Debugger::load(asm);
        assert_eq!(dbg.location_line(), "expand.mms:2\tMain\tSETI\t$1,7");

        for offset in [4, 8, 12] {
            dbg.mmix.set_pc(0x100 + offset);
            assert_eq!(
                dbg.location_line(),
                format!("0x{:016x}\texpand.mms:2\tMain\tSETI\t$1,7", 0x100 + offset)
            );
        }
    }

    /// A breakpoint on the entry point fires on `run`: the reset means
    /// nothing has executed yet, so `continue`'s execute-then-test order
    /// would run straight past it.
    #[test]
    fn run_stops_at_a_breakpoint_on_the_entry_point() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("8".to_string()));
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x100);
        assert!(
            stop.starts_with("stack.mms:8\t"),
            "run must stop at the entry breakpoint, got {stop:?}"
        );
    }

    /// A breakpoint away from the entry point still fires, so the entry
    /// case is not bought by short-circuiting the normal path.
    #[test]
    fn run_stops_at_a_breakpoint_away_from_the_entry_point() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("11".to_string()));
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x118);
        assert!(
            stop.starts_with("stack.mms:11\t"),
            "run must stop at the line-11 breakpoint, got {stop:?}"
        );
    }

    #[test]
    fn breakpoints_listing_reports_none_then_both() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["No breakpoints set.".to_string()]
        );
        dbg.execute(Command::Break("8".to_string()));
        dbg.execute(Command::Break("11".to_string()));
        let listing = dbg.execute(Command::Breakpoints);
        assert_eq!(
            listing,
            vec![
                "0x100  stack.mms:8".to_string(),
                "0x118  stack.mms:11".to_string(),
            ]
        );
    }

    /// Deleting one of two breakpoints by line number stops it from firing
    /// while the other still does — the test that goes red on revert.
    #[test]
    fn delete_by_line_removes_only_that_breakpoint() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("8".to_string()));
        dbg.execute(Command::Break("11".to_string()));
        dbg.execute(Command::Delete(Some("8".to_string())));
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x118);
        assert!(
            stop.starts_with("stack.mms:11\t"),
            "must stop at the remaining breakpoint, not the deleted one, got {stop:?}"
        );
    }

    /// `delete`'s label path, otherwise unproven by the line-number tests.
    #[test]
    fn delete_by_label_removes_the_breakpoint() {
        let mut dbg = Debugger::load(assemble(CALL_PROGRAM, "call.mms"));
        dbg.execute(Command::Break("Main".to_string()));
        dbg.execute(Command::Delete(Some("Main".to_string())));
        let stop = dbg.execute(Command::Run).join("\n");
        assert!(
            stop.starts_with("Program exited"),
            "a deleted label breakpoint must no longer fire, got {stop:?}"
        );
    }

    #[test]
    fn delete_with_no_argument_clears_every_breakpoint() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("8".to_string()));
        dbg.execute(Command::Break("11".to_string()));
        let msg = dbg.execute(Command::Delete(None));
        assert_eq!(msg, vec!["Deleted 2 breakpoint(s).".to_string()]);
        let stop = dbg.execute(Command::Run).join("\n");
        assert!(
            stop.starts_with("Program exited"),
            "clearing every breakpoint must let the program run to completion, got {stop:?}"
        );
    }

    /// An unresolvable argument and a resolvable one with no breakpoint set
    /// there both leave `self.breakpoints` untouched, proven by a subsequent
    /// run still stopping at the real breakpoint.
    #[test]
    fn delete_with_no_effect_leaves_breakpoints_unchanged() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("11".to_string()));
        // Line 999 has no mapped address: unresolvable.
        dbg.execute(Command::Delete(Some("999".to_string())));
        // Line 9 resolves, but no breakpoint sits there.
        dbg.execute(Command::Delete(Some("9".to_string())));
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x118);
        assert!(
            stop.starts_with("stack.mms:11\t"),
            "no-op deletes must not disturb the real breakpoint, got {stop:?}"
        );
    }

    /// `continue` does not re-trigger on the breakpoint it is sitting at:
    /// the instruction there runs first. This is what forces `run` to test
    /// the entry breakpoint itself.
    #[test]
    fn continue_does_not_retrigger_on_the_breakpoint_it_is_sitting_at() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("8".to_string()));
        dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x100);
        let stop = dbg.execute(Command::Continue).join("\n");
        assert_ne!(dbg.mmix.get_pc(), 0x100);
        assert!(stop.starts_with("Program exited with code 42."), "{stop:?}");
    }

    /// After the program exits, a resume is refused rather than executing
    /// the zeroed memory past the end of the image.
    #[test]
    fn resuming_after_exit_is_refused() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Run);
        let pc = dbg.mmix.get_pc();
        for cmd in [
            Command::Step,
            Command::Stepi,
            Command::Next,
            Command::Continue,
        ] {
            assert_eq!(
                dbg.execute(cmd.clone()),
                vec!["The program is not being run.".to_string()],
                "{cmd:?} after exit must be refused"
            );
            assert_eq!(dbg.mmix.get_pc(), pc, "{cmd:?} after exit must not run");
        }
    }

    /// `print`, `state` and `list` keep answering after exit: the register
    /// file is what the operator ran the program to see.
    #[test]
    fn inspection_still_works_after_exit() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Run);
        assert_eq!(dbg.do_print("$255"), "42");
        assert!(!dbg.execute(Command::State).is_empty());
        assert!(!dbg.execute(Command::List).is_empty());
    }

    /// `run` always works: it resets, which clears the exited state.
    #[test]
    fn run_after_exit_restarts_the_program() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Run);
        dbg.execute(Command::Break("11".to_string()));
        let stop = dbg.execute(Command::Run).join("\n");
        assert!(stop.starts_with("stack.mms:11\t"), "{stop:?}");
        assert_eq!(dbg.execute(Command::Step).len(), 1);
    }

    #[test]
    fn fullname_marker_bytes_are_exact() {
        let source = "\tLOC\t#100\nMain\tSETI\t$1,1\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "marker.mms");
        let mut dbg = Debugger::load(asm);
        dbg.set_fullname(true);
        let loc = dbg.assembler.source_loc(dbg.mmix.get_pc()).unwrap().clone();
        let expected_path = absolute_path(&loc.file);
        let expected = format!(
            "\x1a\x1a{}:{}:0:beg:0x{:x}\n",
            expected_path.display(),
            loc.line,
            dbg.mmix.get_pc()
        );
        let report = dbg.initial_report();
        assert_eq!(report[0], expected);
    }

    #[test]
    fn print_returns_register_value_and_label_address() {
        let source = "\tLOC\t#100\nMain\tSETI\t$3,42\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "print.mms");
        let main_addr = *asm.labels.get("Main").unwrap();
        let mut dbg = Debugger::load(asm);
        // SETI expands to four tetras; step through all of them.
        for _ in 0..4 {
            dbg.execute(Command::Stepi);
        }
        assert_eq!(dbg.do_print("$3"), "42");
        assert_eq!(dbg.do_print("Main"), format_value(main_addr, dbg.format));
    }

    #[test]
    fn print_resolves_a_special_register_by_its_discriminant() {
        let source = "\tLOC\t#100\nMain\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "special.mms");
        let mut dbg = Debugger::load(asm);
        // rJ's discriminant is 4, so `print` must read slot 4 whatever
        // index "rJ" occupies in any other ordering of the names.
        dbg.mmix.set_special(SpecialReg::RJ, 0xDEAD_BEEF_1234);
        assert_eq!(
            dbg.do_print("rJ"),
            format_value(dbg.mmix.get_special(SpecialReg::RJ), dbg.format)
        );
        assert_eq!(dbg.do_print("rJ"), "244837814047284");
    }

    /// Every register number resolves to the variant with that discriminant,
    /// and that variant's name resolves back to it. A duplicate name would
    /// round-trip the higher number to the lower variant.
    #[test]
    fn special_register_numbers_names_and_variants_round_trip() {
        for n in 0u8..32 {
            let reg = SpecialReg::from_u8(n).expect("0..32 are all special registers");
            assert_eq!(reg as u8, n);
            assert_eq!(special_reg_from_name(reg.name()), Some(reg));
        }
    }

    /// The assembler seeds its predefined special-register symbols from a list
    /// of its own; each name must belong to the register it is seeded with.
    #[test]
    fn assembler_predefined_symbols_agree_with_special_reg_names() {
        let asm = MMixAssembler::new("", "symbols.mms");
        for n in 0u8..32 {
            let reg = SpecialReg::from_u8(n).expect("0..32 are all special registers");
            assert_eq!(
                asm.symbols.get(reg.name()),
                Some(&SymbolType::Constant(u64::from(n))),
                "the assembler must predefine {} as special register {n}",
                reg.name()
            );
        }
    }

    #[test]
    fn blank_repeats_last_command() {
        let source = "\tLOC\t#100\nMain\tSETI\t$1,1\n\tSETI\t$2,2\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "repeat.mms");
        let mut dbg = Debugger::load(asm);
        let pc0 = dbg.mmix.get_pc();
        dbg.execute(Command::Stepi);
        let pc1 = dbg.mmix.get_pc();
        assert_ne!(pc0, pc1, "first step must advance the PC");
        dbg.execute(Command::Repeat);
        let pc2 = dbg.mmix.get_pc();
        assert_ne!(pc1, pc2, "blank repeat must advance the PC again");
    }

    const INFINITE_LOOP_PROGRAM: &str = "\
\tLOC\t#100
Main\tJMP\tMain
";

    #[test]
    fn command_run_on_a_program_that_never_halts_reports_budget_exhaustion() {
        let asm = assemble(INFINITE_LOOP_PROGRAM, "loop.mms");
        let mut dbg = Debugger::load(asm);
        let output = dbg.execute(Command::Run);
        assert!(!output.iter().any(|line| line.starts_with("Program exited")));
        assert_eq!(
            output.last().map(String::as_str),
            Some("still running (step budget exhausted)")
        );
    }

    #[test]
    fn command_next_on_a_call_that_never_returns_reports_budget_exhaustion() {
        let source = "\
\tLOC\t#100
Main\tPUSHJ\t$0,Loop
\tTRAP\t0,Halt,0
Loop\tJMP\tLoop
";
        let asm = assemble(source, "loopcall.mms");
        let mut dbg = Debugger::load(asm);
        let output = dbg.execute(Command::Next);
        assert!(!output.iter().any(|line| line.starts_with("Program exited")));
        assert_eq!(
            output.last().map(String::as_str),
            Some("still running (step budget exhausted)")
        );
    }

    #[test]
    fn journal_enabled_flag_survives_debugger_runs_reset() {
        let asm = assemble(CALL_PROGRAM, "call.mms");
        let mut dbg = Debugger::load(asm);
        dbg.machine_mut().set_journal(true);
        dbg.execute(Command::Run);
        dbg.machine_mut().take_journal(); // drain the first run's writes
        // `disable` is never called; `Command::Run` resets the machine.
        dbg.execute(Command::Run);
        assert!(
            !dbg.machine_mut().take_journal().is_empty(),
            "the enabled flag must survive do_run's reset()"
        );
    }

    const ONE_GREG_PROGRAM: &str = "\
Base\tGREG\t1000
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";

    #[test]
    fn write_image_applies_greg_initializer_to_its_register() {
        let dbg = Debugger::load(assemble(ONE_GREG_PROGRAM, "one_greg.mms"));
        let &(reg, value) = dbg
            .assembler
            .greg_inits
            .first()
            .expect("one GREG directive");
        assert_eq!(value, 1000);
        assert_eq!(dbg.mmix.get_register(reg), 1000);
    }

    #[test]
    fn write_image_derives_rg_from_one_greg() {
        // One GREG: rG becomes that register.
        let dbg = Debugger::load(assemble(ONE_GREG_PROGRAM, "one_greg.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 254);
    }

    #[test]
    fn write_image_derives_rg_from_two_gregs_takes_lower() {
        // Two GREGs: rG becomes the lower of the two allocated registers.
        const TWO_GREG_PROGRAM: &str = "\
A\tGREG\t1
B\tGREG\t2
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";
        let dbg = Debugger::load(assemble(TWO_GREG_PROGRAM, "two_greg.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 253);
    }

    #[test]
    fn write_image_derives_rg_stays_32_with_no_greg() {
        // No GREG: rG stays at MMix::initialize's default.
        const NO_GREG_PROGRAM: &str = "\
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";
        let dbg = Debugger::load(assemble(NO_GREG_PROGRAM, "no_greg.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 32);
    }

    #[test]
    fn greg_program_with_pushj_executes_correctly_under_raised_rg() {
        // One GREG directive raises rG to 254 (see the derivation test
        // above); this program then makes a PUSHJ/POP call, modeled on
        // examples/function.mms, to confirm the register-window slide is
        // unaffected by push_frame zeroing the wider `new_rl..rG` range.
        const PROGRAM: &str = "\
Base\tGREG\t1000
\tLOC\t#100
Main\tSETI\t$1,40
\tSETI\t$2,2
\tPUSHJ\t$0,AddFunc
\tSET\t$255,$0
\tTRAP\t0,Halt,0
AddFunc\tADDU\t$0,$0,$1
\tPOP\t1,0
";
        let mut dbg = Debugger::load(assemble(PROGRAM, "greg_pushj.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 254);

        dbg.execute(Command::Run);

        // Expected values from the same push_frame/pop_frame slide already
        // exercised by test_pushj_window_slide_return_value (src/mmix.rs):
        // the two SETIs grow rL to 3 ($1, then $2, each >= the then-current
        // rL); PUSHJ $0 slides caller's $1, $2 (40, 2) down to callee's $0,
        // $1; POP 1 places the callee's $0 (the sum) at the caller's hole
        // $0, and restores rL to max(saved_rl, saved_x + n) = max(3, 0 + 1)
        // = 3.
        assert_eq!(dbg.mmix.get_register(0), 42);
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 3);
        assert_eq!(dbg.mmix.get_register(255), 42);
        assert_eq!(dbg.mmix.get_exit_code(), 42);
    }
}
