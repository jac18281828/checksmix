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
use crate::mmo::derive_rg;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The MMIX text/data segment boundary. Mirrors `run_mms`'s fallback
/// (`src/bin/checksmix.rs`): when no `Main` label exists, the entry point is
/// the first instruction address below this boundary.
const SEGMENT_BOUNDARY: u64 = 0x2000000000000000;

/// `print`'s error for a command word with no argument -- plain, attached
/// (`p/f`) and detached (`p /f`) all reach it.
const PRINT_REQUIRES_ARGUMENT: &str = "print requires an argument";

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
    PrintAs(PrintFormat, String),
    Set(String, String),
    State,
    List,
    Help,
    Quit,
    Repeat,
}

/// A `print` output format, gdb's `/f` and `/x` suffixes. Non-exhaustive: a
/// later format is an additive change here, not a breaking one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrintFormat {
    /// The octabyte read as an IEEE 754 double (`p/f`).
    Float,
    /// The octabyte in MMIXAL hex notation (`p/x`).
    Hex,
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
    if let Some(suffix) = head
        .strip_prefix("p/")
        .or_else(|| head.strip_prefix("print/"))
    {
        return parse_print_as(suffix, rest);
    }
    match head {
        "s" | "step" => Ok(Command::Step),
        "si" | "stepi" => Ok(Command::Stepi),
        "n" | "next" => Ok(Command::Next),
        "c" | "continue" => Ok(Command::Continue),
        "r" | "run" => Ok(Command::Run),
        "b" | "break" => {
            if rest.is_empty() {
                Err("break requires a line number, label or address".to_string())
            } else {
                Ok(Command::Break(rest.to_string()))
            }
        }
        "d" | "delete" => Ok(Command::Delete(if rest.is_empty() {
            None
        } else {
            Some(rest.to_string())
        })),
        "p" | "print" => match rest.strip_prefix('/') {
            Some(after_slash) => {
                let (suffix, arg) = split_format_suffix(after_slash);
                parse_print_as(suffix, arg)
            }
            None if rest.is_empty() => Err(PRINT_REQUIRES_ARGUMENT.to_string()),
            None => Ok(Command::Print(rest.to_string())),
        },
        "set" => match rest.split_once(char::is_whitespace) {
            Some((target, value)) if !target.is_empty() && !value.trim().is_empty() => {
                Ok(Command::Set(target.to_string(), value.trim().to_string()))
            }
            _ => Err("set requires a target and a value".to_string()),
        },
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

/// Split a `print` format suffix from what follows it: up to the first blank
/// is the suffix, the rest (trimmed) is the argument. No blank means the
/// whole text is the suffix and the argument is empty.
fn split_format_suffix(text: &str) -> (&str, &str) {
    match text.split_once(char::is_whitespace) {
        Some((suffix, arg)) => (suffix, arg.trim()),
        None => (text, ""),
    }
}

/// Resolve a `print` format suffix to a `Command::PrintAs`, gdb's "Undefined
/// output format" error for any suffix but `f` and `x`, or `print`'s existing
/// no-argument error for a valid format with nothing to print.
fn parse_print_as(suffix: &str, arg: &str) -> Result<Command, String> {
    let format = match suffix {
        "f" => PrintFormat::Float,
        "x" => PrintFormat::Hex,
        _ => return Err(format!("Undefined output format \"{suffix}\".")),
    };
    if arg.is_empty() {
        Err(PRINT_REQUIRES_ARGUMENT.to_string())
    } else {
        Ok(Command::PrintAs(format, arg.to_string()))
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

/// Parse a general-register argument: `$N` or bare `N`, `0 <= N <= 255`.
/// Shared by `Debugger::resolve_print_argument` and `set`'s target
/// resolution.
fn register_index(arg: &str) -> Option<u8> {
    let digits = arg.strip_prefix('$').unwrap_or(arg);
    let n: u16 = digits.parse().ok()?;
    if n > 255 { None } else { Some(n as u8) }
}

fn format_value(value: u64, format: ValueFormat) -> String {
    match format {
        ValueFormat::Signed => (value as i64).to_string(),
        ValueFormat::Unsigned => value.to_string(),
    }
}

/// gdb's error for a `print` argument `resolve_print_argument` cannot
/// resolve, shared by `do_print` and `do_print_as` so plain and formatted
/// printing report an unresolved argument identically.
fn no_symbol_in_context(arg: &str) -> String {
    format!("No symbol \"{}\" in current context.", arg.trim())
}

/// Render an octabyte for `p/f` or `p/x`, independent of `ValueFormat` --
/// `set_format`'s signed/unsigned choice governs plain `print` only.
fn format_as(value: u64, format: PrintFormat) -> String {
    match format {
        PrintFormat::Float => format_float(value),
        PrintFormat::Hex => format_hex(value),
    }
}

/// `p/x`: the MMIXAL constant spelling -- `#` followed by lowercase hex
/// digits, no leading zeros (Rust's `{:x}` already strips them).
fn format_hex(value: u64) -> String {
    format!("#{value:x}")
}

/// `p/f`: the octabyte read as an IEEE 754 double, in shortest round-trip
/// digits. Positional notation for zero and for finite magnitudes in
/// `[1e-4, 1e16)`; scientific otherwise -- Rust's `{}` never switches to
/// scientific notation on its own, so the cutoff is applied here. `{:e}`
/// renders an infinity as `inf`/`-inf`.
fn format_float(bits: u64) -> String {
    let value = f64::from_bits(bits);
    if value.is_nan() {
        return format_nan(bits);
    }
    let magnitude = value.abs();
    if magnitude == 0.0 || (1e-4..1e16).contains(&magnitude) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

/// A NaN's sign and 52-bit fraction field, gdb's `nan(<payload>)` form: the
/// fraction (quiet bit included) in the same hex spelling as `p/x`, prefixed
/// with `-` when the sign bit is set. The fraction and the quiet bit are what
/// distinguish one NaN from another; the exponent field carries nothing.
fn format_nan(bits: u64) -> String {
    const FRACTION_MASK: u64 = (1 << 52) - 1;
    let sign = if bits & (1 << 63) != 0 { "-" } else { "" };
    let fraction = bits & FRACTION_MASK;
    format!("{sign}nan({})", format_hex(fraction))
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

    mmix.set_debug_strings(assembler.debug_strings().to_vec());

    // MMIX starts a program with rG at 255 minus its GREG count: the
    // lowest-numbered register GREG allocated, floored at 32, or 255 with
    // no GREG directive at all. rG must move before a GREG value is written:
    // set_register claims a register below rG as local, and a GREG target
    // can sit below whatever rG the machine already holds on entry.
    mmix.set_special(SpecialReg::RG, derive_rg(&assembler.greg_inits) as u64);

    for &(reg, value) in &assembler.greg_inits {
        mmix.set_register(reg, value);
    }
}

/// Start a program at `entry`: set the PC there, and `$255` to the same
/// address, MMIXware's start state for a running program. `$255` is always
/// global, so this never raises `rL`.
pub fn start_program(mmix: &mut MMix, entry: u64) {
    mmix.set_pc(entry);
    mmix.set_register(255, entry);
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
        start_program(&mut mmix, entry);
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
            Command::PrintAs(format, arg) => vec![self.do_print_as(*format, arg)],
            Command::Set(target, value) => vec![self.do_set(target.clone(), value.clone())],
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
        start_program(&mut self.mmix, self.entry);
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
    /// line, following into calls. Also stops when the PC's tetra holds a
    /// breakpoint, on a halt, or `STEP_BUDGET` (a line that never ends can't
    /// hang this). An address with no source location is not a new line.
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
            if self.breakpoints.contains(&(self.mmix.get_pc() & !3))
                || self.reached_new_line(&origin)
            {
                break;
            }
        }
        self.report_stop(halted, budget_exhausted)
    }

    /// `next`: like `step`, but stepping over calls. The new line only
    /// counts once the call depth is back at or below where it started
    /// (PUSHJ/PUSHGO push a frame; GO does not) -- a callee's first
    /// instruction is on a different source line, so testing the line alone
    /// would stop inside the call. Also stops when the PC's tetra holds a
    /// breakpoint.
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
            if self.breakpoints.contains(&(self.mmix.get_pc() & !3)) {
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

    /// `continue`: single-step from the current PC until the PC's tetra
    /// holds a breakpoint, the program halts, or `STEP_BUDGET` is reached (a
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
            if self.breakpoints.contains(&(self.mmix.get_pc() & !3)) {
                break;
            }
        }
        self.report_stop(halted, budget_exhausted)
    }

    /// `run`/reset: reset the machine to the freshly-loaded image, stop
    /// there if the entry point's tetra holds a breakpoint, and otherwise
    /// behave like `continue`. `continue` executes an instruction before
    /// testing the breakpoint set, so that resuming from a stop does not
    /// re-trigger on the breakpoint it is sitting at; after a reset nothing
    /// has run yet, so the entry breakpoint has to be honored first.
    fn do_run(&mut self) -> Vec<String> {
        self.reset();
        if self.breakpoints.contains(&(self.mmix.get_pc() & !3)) {
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

    /// Resolve a `break`/`delete` argument to the tetra holding its address,
    /// in priority order: a decimal source line in the current file, an
    /// exact label, or a `#`/`0x` hex address. Every path rounds down to its
    /// tetra (`addr & !3`), since MMIX executes instructions only at
    /// multiples of 4: a line, a label and a hex address naming one tetra
    /// key the same breakpoint. A leading ':' (the root-namespace spelling)
    /// is stripped before the label lookup, since `MMixAssembler::labels`
    /// keys a root name without it.
    fn resolve_break_location(&self, arg: &str) -> Option<u64> {
        let arg = arg.trim();
        let addr = if let Ok(line) = arg.parse::<usize>() {
            self.current_file()
                .and_then(|file| self.assembler.addr_for_line(&file, line))
        } else {
            let key = arg.strip_prefix(':').unwrap_or(arg);
            self.assembler
                .labels
                .get(key)
                .copied()
                .or_else(|| self.parse_hex_address(arg))
        };
        addr.map(|addr| addr & !3)
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

    /// `print <arg>` argument resolution, in priority order: `$N`/bare `N`
    /// (general register), a special-register name, a label, an IS/GREG
    /// symbol, a hex address (the memory octa at its aligned 8-byte base),
    /// else unresolved. A leading ':' is stripped before the label/symbol
    /// lookups, since `MMixAssembler::labels`/`symbols` key a root name
    /// without it. Shared by `do_print` and `do_print_as`, so plain and
    /// formatted printing can never disagree about what an argument names.
    fn resolve_print_argument(&self, arg: &str) -> Option<u64> {
        let arg = arg.trim();
        if let Some(n) = register_index(arg) {
            return Some(self.mmix.get_register(n));
        }
        if let Some(reg) = special_reg_from_name(arg) {
            return Some(self.mmix.get_special(reg));
        }
        let key = arg.strip_prefix(':').unwrap_or(arg);
        if let Some(&addr) = self.assembler.labels.get(key) {
            return Some(addr);
        }
        if let Some(sym) = self.assembler.symbols.get(key) {
            return Some(match sym {
                SymbolType::Register(n) => self.mmix.get_register(*n),
                SymbolType::Constant(v) => *v,
            });
        }
        self.parse_hex_address(arg)
            .map(|addr| self.mmix.read_octa(addr))
    }

    /// `print <arg>`: resolve `arg` via `resolve_print_argument` and render
    /// it as `self.format`. Accepts a general register (`$N`/bare `N`), a
    /// special-register name, a label, a register-valued (`GREG`) or
    /// constant-valued (`IS`) symbol, or a hex memory address.
    fn do_print(&self, arg: &str) -> String {
        match self.resolve_print_argument(arg) {
            Some(value) => format_value(value, self.format),
            None => no_symbol_in_context(arg),
        }
    }

    /// `p/f`/`p/x`: the same argument resolution as plain `print`, formatted
    /// as `format` instead of by `self.format`.
    fn do_print_as(&self, format: PrintFormat, arg: &str) -> String {
        match self.resolve_print_argument(arg) {
            Some(value) => format_as(value, format),
            None => no_symbol_in_context(arg),
        }
    }

    /// `set <target> <value>`: writes a general register, a special
    /// register, or the memory octa at a hex address. The value is parsed
    /// first -- an invalid value on an unresolvable target reports the
    /// value error, not the target error. Target resolution, in priority
    /// order: `$N`/bare `N`, a special-register name, a register-aliasing
    /// symbol (a `GREG` or register-valued `IS` label), a hex address.
    /// A plain label and a constant-valued `IS` symbol are not settable --
    /// neither names a storage location. A leading ':' on `target` is
    /// stripped before the symbol lookup, matching `do_print`.
    fn do_set(&mut self, target: String, value: String) -> String {
        let target = target.trim();
        let value = value.trim();
        let Some(parsed) = self.parse_set_value(value) else {
            return format!("Invalid value '{value}'; expected decimal or 0x/#-prefixed hex");
        };
        if let Some(n) = register_index(target) {
            return self.write_register_and_report(n, parsed);
        }
        if let Some(reg) = special_reg_from_name(target) {
            if let Some(err) = self.rejected_special_set(reg, parsed) {
                return err;
            }
            self.mmix.set_special(reg, parsed);
            return format!("{target} = {}", format_value(parsed, self.format));
        }
        let key = target.strip_prefix(':').unwrap_or(target);
        if let Some(SymbolType::Register(n)) = self.assembler.symbols.get(key).copied() {
            return self.write_register_and_report(n, parsed);
        }
        if let Some(addr) = self.parse_hex_address(target) {
            let aligned = addr & !7;
            self.mmix.write_octa(addr, parsed);
            return format!("0x{aligned:x} = {}", format_value(parsed, self.format));
        }
        format!(
            "No settable target \"{target}\" (register, special register, or hex memory address only)"
        )
    }

    /// `set`'s bound on rL and rG, the two special registers that index the
    /// register file: an out-of-range value there panics `save_context` or
    /// corrupts the machine, where every other special register's excess
    /// bits are only data. rL accepts at most rG, raising included; rG
    /// accepts `PUT rG`'s own range, 32-255 and at least rL. `None` when
    /// `value` is in range, or `reg` is neither register.
    fn rejected_special_set(&self, reg: SpecialReg, value: u64) -> Option<String> {
        match reg {
            SpecialReg::RL => {
                let rg = self.mmix.get_special(SpecialReg::RG);
                (value > rg).then(|| format!("Invalid rL {value}: must not exceed rG={rg}"))
            }
            SpecialReg::RG => {
                let rl = self.mmix.get_special(SpecialReg::RL);
                (!(32..=255).contains(&value) || value < rl)
                    .then(|| format!("Invalid rG {value}: must be 32-255 and at least rL={rl}"))
            }
            _ => None,
        }
    }

    fn write_register_and_report(&mut self, n: u8, value: u64) -> String {
        self.mmix.set_register(n, value);
        format!("${n} = {}", format_value(value, self.format))
    }

    /// Parse a `set` value: `0x`/`#`-prefixed as an unsigned hex bit
    /// pattern, else decimal -- signed if it fits `i64` (so a negative
    /// literal stores its two's-complement pattern), else the raw `u64` for
    /// a literal in the upper half of the range `i64` cannot reach.
    fn parse_set_value(&self, value: &str) -> Option<u64> {
        if let Some(addr) = self.parse_hex_address(value) {
            return Some(addr);
        }
        value
            .parse::<i64>()
            .map(|v| v as u64)
            .or_else(|_| value.parse::<u64>())
            .ok()
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
break         b <line>, b <label>, b <addr>, break …   Set a breakpoint on the tetra holding a source line, label, or hex address; it fires when execution reaches any address in that tetra.
delete        d, delete, d <line>, d <label>, d <addr>   Delete one breakpoint, or every breakpoint given no argument.
print         p <arg>, print <arg>             Print a register, special register, label address, IS/GREG symbol, or the memory octa at the address's aligned base. p/f and p/x, attached or detached, print it as an IEEE double or in hex.
set           set <target> <value>             Write a register, special register, or the memory octa at a hex address; a register-aliasing symbol (GREG or register-valued IS) is settable, a label or constant-valued IS symbol is not. rL accepts at most rG and rG accepts 32-255 and at least rL; any other value is rejected and changes nothing.
state         bt, backtrace, info reg, info registers   Print the full register dump.
breakpoints   info break, info breakpoints     List every currently-set breakpoint by its tetra and the source line holding that tetra's first byte.
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
    const GREETING_PROGRAM: &str = "\
\tLOC\t#100
Main\tLDA\t$255,Text
\tTRAP\t0,Fputs,1
\tTRAP\t0,Halt,0
Text\tBYTE\t\"Hi\",0
";

    /// A9's introduction: `T` is line 5 at `#10C`; the `GO` jumps past it to
    /// `#10D`.
    const GO_PROGRAM: &str = "\
\tLOC\t#100
Main\tGETA\t$1,T
\tADDU\t$1,$1,1
\tGO\t$0,$1,0
T\tSET\t$2,5
\tSET\t$255,0
\tTRAP\t0,Halt,0
";

    /// `Loop` is line 4 at `#108`; its `GO` jumps to `#109`, inside the same
    /// tetra, forever.
    const LOOP_PROGRAM: &str = "\
\tLOC\t#100
Main\tGETA\t$1,Loop
\tADDU\t$1,$1,1
Loop\tGO\t$0,$1,0
";

    /// The `BYTE 1` line (4) is at `#108`; `Odd` (line 5) is at `#109`.
    const ODD_PROGRAM: &str = "\
\tLOC\t#100
Main\tSET\t$255,0
\tTRAP\t0,Halt,0
\tBYTE\t1
Odd\tBYTE\t2
";

    /// `Main` sits at `#101`, off a tetra boundary; the tetra there decodes
    /// as `TRAP 1,0,0`, which halts with exit code 1.
    const ENTRY_PROGRAM: &str = "\
\tLOC\t#100
\tBYTE\t0
Main\tBYTE\t1
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
        assert_eq!(
            parse_command("set $0 42"),
            Ok(Command::Set("$0".to_string(), "42".to_string()))
        );
        assert_eq!(
            parse_command("set rJ 0x10"),
            Ok(Command::Set("rJ".to_string(), "0x10".to_string()))
        );
        assert!(parse_command("set").is_err());
        assert!(parse_command("set $0").is_err());
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
    fn parse_command_maps_print_format_suffixes() {
        assert_eq!(
            parse_command("p/f $1"),
            Ok(Command::PrintAs(PrintFormat::Float, "$1".to_string()))
        );
        assert_eq!(
            parse_command("print/x rA"),
            Ok(Command::PrintAs(PrintFormat::Hex, "rA".to_string()))
        );
        assert_eq!(
            parse_command("p /f $1"),
            Ok(Command::PrintAs(PrintFormat::Float, "$1".to_string()))
        );
        assert_eq!(
            parse_command("p/z $1"),
            Err("Undefined output format \"z\".".to_string())
        );
        assert_eq!(
            parse_command("p/2x $1"),
            Err("Undefined output format \"2x\".".to_string())
        );
        assert_eq!(
            parse_command("p/ $1"),
            Err("Undefined output format \"\".".to_string())
        );
        assert_eq!(
            parse_command("p/f"),
            Err("print requires an argument".to_string())
        );
        assert_eq!(parse_command("p $1"), Ok(Command::Print("$1".to_string())));
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
        assert!(joined.lines().any(|l| l.starts_with("set")));
        assert!(
            joined
                .lines()
                .find(|l| l.starts_with("break "))
                .is_some_and(|l| l.contains("<addr>")),
            "the break help line must name the address form: {joined:?}"
        );
        assert!(
            joined
                .lines()
                .find(|l| l.starts_with("delete "))
                .is_some_and(|l| l.contains("<addr>")),
            "the delete help line must name the address form: {joined:?}"
        );
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

    /// A program whose only special statement is a `debug` line, followed
    /// by two ordinary ones. `Main` is line 2, `SETI` line 3, `TRAP` line 4.
    const DEBUG_PROGRAM: &str = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
\tSETI\t$1,7
\tTRAP\t0,Halt,0
";

    /// `next` steps clean over a whole `debug` expansion -- the `JMP`, the
    /// generated subroutine, and the `SWYM` landing pad it jumps back to --
    /// in one call, landing on the next real source line.
    #[test]
    fn next_steps_over_a_debug_line_in_one_go() {
        let recorder = Recorder::default();
        let mut dbg =
            Debugger::load_with_host(assemble(DEBUG_PROGRAM, "debug.mms"), recorder.clone());
        let stop = dbg.execute(Command::Next).join("\n");
        assert!(
            stop.starts_with("debug.mms:3\t"),
            "next must land on line 3 (SETI), got {stop:?}"
        );
        assert_eq!(&*recorder.0.borrow(), b"hi\n");
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

    /// `#`/`0x` both name a hex address, matching `print`'s spellings; the
    /// success message echoes the resolved address beside the argument
    /// text.
    #[test]
    fn break_by_hash_hex_address_stops_there() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let msg = dbg.execute(Command::Break("#118".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x118 (#118)".to_string()]);
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x118);
        assert!(
            stop.starts_with("stack.mms:11\t"),
            "run must stop at the hex breakpoint, got {stop:?}"
        );
    }

    #[test]
    fn break_by_0x_hex_address_stops_there() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let msg = dbg.execute(Command::Break("0x118".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x118 (0x118)".to_string()]);
        dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x118);
    }

    /// A hex address off the tetra boundary rounds down to the instruction
    /// holding it, and `info break` lists it against that instruction's
    /// line, since `source_loc` is an extent lookup.
    #[test]
    fn break_by_hex_address_rounds_down_to_its_tetra() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let msg = dbg.execute(Command::Break("#11B".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x118 (#11B)".to_string()]);
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["0x118  stack.mms:11".to_string()]
        );
    }

    /// A hex address naming a tetra inside a multi-tetra expansion, such as
    /// `SETI`'s, stops there -- no line or label names it.
    #[test]
    fn break_by_hex_address_inside_an_expansion_stops_there() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let msg = dbg.execute(Command::Break("#124".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x124 (#124)".to_string()]);
        dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x124);
    }

    /// `delete` resolves a hex address the same way `break` does, matching
    /// what `info break` lists a line-set breakpoint as.
    #[test]
    fn delete_by_hex_address_removes_a_line_set_breakpoint() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("11".to_string()));
        let msg = dbg.execute(Command::Delete(Some("0x118".to_string())));
        assert_eq!(msg, vec!["Deleted breakpoint at 0x118 (0x118)".to_string()]);
        let stop = dbg.execute(Command::Run).join("\n");
        assert!(
            stop.starts_with("Program exited with code 42."),
            "clearing the only breakpoint must let the program run to completion, got {stop:?}"
        );
    }

    /// An unprefixed number is always a source line, never an address: the
    /// two spellings are disjoint even where the digits would parse as
    /// either.
    #[test]
    fn break_treats_an_unprefixed_number_as_a_line_not_an_address() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let msg = dbg.execute(Command::Break("118".to_string()));
        assert_eq!(
            msg,
            vec!["No location found for '118'; breakpoint not set".to_string()]
        );
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["No breakpoints set.".to_string()]
        );
    }

    /// Text `parse_hex_address` rejects -- a bare prefix, a non-hex digit,
    /// or a value at or above 2^64 -- falls through to the label lookup and
    /// gets the unresolvable-argument message.
    #[test]
    fn break_rejects_malformed_hex_address_text() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        for arg in ["#", "0x", "#11G", "#10000000000000000"] {
            let msg = dbg.execute(Command::Break(arg.to_string()));
            assert_eq!(
                msg,
                vec![format!("No location found for '{arg}'; breakpoint not set")]
            );
        }
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["No breakpoints set.".to_string()]
        );
    }

    #[test]
    fn parse_command_break_requires_an_argument() {
        assert_eq!(
            parse_command("b"),
            Err("break requires a line number, label or address".to_string())
        );
        assert_eq!(
            parse_command("break"),
            Err("break requires a line number, label or address".to_string())
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

    /// A breakpoint on a label bound to a `LOC` line names the location the
    /// counter held before `LOC` moves it -- `Gap` is `#104`, right after
    /// `Main`'s one instruction, not the `#300` `LOC` jumps to -- and that
    /// address has no instruction or data emitted there, so it lists with no
    /// resolvable source line.
    #[test]
    fn breakpoints_listing_shows_no_source_line_for_an_unmapped_address() {
        let source = "\
        LOC     #100
Main    TRAP    0,Halt,0
Gap     LOC     #300
";
        let mut dbg = Debugger::load(assemble(source, "gap.mms"));
        dbg.execute(Command::Break("Gap".to_string()));
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["0x104  (no source line)".to_string()]
        );
    }

    /// `b T`, `b 5` and `b #10C` all name the tetra at `#10C`: a label, a
    /// line and a hex address. The `GO` jumps past `T` to `#10D`; the
    /// breakpoint still fires there, since `#10D` lies in the same tetra.
    #[test]
    fn breakpoint_on_a_label_line_or_hex_address_fires_anywhere_in_its_tetra() {
        for arg in ["T", "5", "#10C"] {
            let mut dbg = Debugger::load(assemble(GO_PROGRAM, "go.mms"));
            dbg.execute(Command::Break(arg.to_string()));
            let stop = dbg.execute(Command::Run);
            assert_eq!(dbg.mmix.get_pc(), 0x10D, "break {arg}");
            assert_eq!(
                stop,
                vec!["0x000000000000010d\tgo.mms:5\tT\tSET\t$2,5".to_string()],
                "break {arg}"
            );
        }
    }

    /// `Loop`'s `GO` jumps from `#108` to `#109`, inside the same tetra;
    /// `run` stops at `#108` first, and `continue` fires the breakpoint
    /// again once the PC returns to that tetra rather than looping forever.
    #[test]
    fn breakpoint_on_a_tetra_the_pc_leaves_and_returns_to_fires_on_continue() {
        let mut dbg = Debugger::load(assemble(LOOP_PROGRAM, "loop.mms"));
        dbg.execute(Command::Break("Loop".to_string()));
        dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x108);
        let stop = dbg.execute(Command::Continue);
        assert_eq!(dbg.mmix.get_pc(), 0x109);
        assert_eq!(
            stop,
            vec!["0x0000000000000109\tloop.mms:4\tLoop\tGO\t$0,$1,0".to_string()]
        );
    }

    /// `step` fires the same breakpoint `continue` does, once the PC returns
    /// to the tetra it left.
    #[test]
    fn breakpoint_on_a_tetra_the_pc_leaves_and_returns_to_fires_on_step() {
        let mut dbg = Debugger::load(assemble(LOOP_PROGRAM, "loop.mms"));
        dbg.execute(Command::Break("Loop".to_string()));
        dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x108);
        let stop = dbg.execute(Command::Step);
        assert_eq!(dbg.mmix.get_pc(), 0x109);
        assert_eq!(
            stop,
            vec!["0x0000000000000109\tloop.mms:4\tLoop\tGO\t$0,$1,0".to_string()]
        );
    }

    /// `next` fires the same breakpoint `continue` does, once the PC returns
    /// to the tetra it left.
    #[test]
    fn breakpoint_on_a_tetra_the_pc_leaves_and_returns_to_fires_on_next() {
        let mut dbg = Debugger::load(assemble(LOOP_PROGRAM, "loop.mms"));
        dbg.execute(Command::Break("Loop".to_string()));
        dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x108);
        let stop = dbg.execute(Command::Next);
        assert_eq!(dbg.mmix.get_pc(), 0x109);
        assert_eq!(
            stop,
            vec!["0x0000000000000109\tloop.mms:4\tLoop\tGO\t$0,$1,0".to_string()]
        );
    }

    /// `entry.mms` starts execution at `Main`, `#101`, off a tetra boundary.
    /// `break` keys the label on its tetra (`#100`); `run`'s reset lands on
    /// `#101` itself, whose tetra already holds the breakpoint, so the
    /// program stops through the exact match before executing anything.
    #[test]
    fn run_stops_at_the_entry_point_even_when_its_label_is_off_the_tetra() {
        let mut dbg = Debugger::load(assemble(ENTRY_PROGRAM, "entry.mms"));
        let msg = dbg.execute(Command::Break("Main".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x100 (Main)".to_string()]);
        let stop = dbg.execute(Command::Run);
        assert_eq!(dbg.mmix.get_pc(), 0x101);
        assert_eq!(stop, vec!["entry.mms:3\tMain\tBYTE\t1".to_string()]);
    }

    /// A label off its tetra sets, lists and deletes as that tetra: `Odd`
    /// (`#109`) keys `0x108`, and `info break` names the `BYTE 1` line that
    /// holds the tetra's first byte, not `Odd`'s own line.
    #[test]
    fn breakpoint_on_a_label_off_its_tetra_lists_and_deletes_by_the_tetra() {
        let mut dbg = Debugger::load(assemble(ODD_PROGRAM, "odd.mms"));
        let msg = dbg.execute(Command::Break("Odd".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x108 (Odd)".to_string()]);
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["0x108  odd.mms:4".to_string()]
        );
        let msg = dbg.execute(Command::Delete(Some("0x109".to_string())));
        assert_eq!(msg, vec!["Deleted breakpoint at 0x108 (0x109)".to_string()]);
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["No breakpoints set.".to_string()]
        );
    }

    /// A breakpoint set by line deletes by the label naming the same tetra.
    #[test]
    fn breakpoint_on_a_line_off_its_tetra_deletes_by_the_label_naming_it() {
        let mut dbg = Debugger::load(assemble(ODD_PROGRAM, "odd.mms"));
        let msg = dbg.execute(Command::Break("5".to_string()));
        assert_eq!(msg, vec!["Breakpoint set at 0x108 (5)".to_string()]);
        let msg = dbg.execute(Command::Delete(Some("Odd".to_string())));
        assert_eq!(msg, vec!["Deleted breakpoint at 0x108 (Odd)".to_string()]);
    }

    /// A label, a hex address and a line all naming one tetra set exactly
    /// one breakpoint: `delete` with no argument clears it alone.
    #[test]
    fn label_hex_and_line_naming_one_tetra_set_one_breakpoint() {
        let mut dbg = Debugger::load(assemble(ODD_PROGRAM, "odd.mms"));
        dbg.execute(Command::Break("Odd".to_string()));
        dbg.execute(Command::Break("#108".to_string()));
        dbg.execute(Command::Break("4".to_string()));
        assert_eq!(
            dbg.execute(Command::Breakpoints),
            vec!["0x108  odd.mms:4".to_string()]
        );
        let msg = dbg.execute(Command::Delete(None));
        assert_eq!(msg, vec!["Deleted 1 breakpoint(s).".to_string()]);
    }

    /// Deleting one of two breakpoints by line number stops it from firing
    /// while the other still does.
    #[test]
    fn delete_by_line_removes_only_that_breakpoint() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("8".to_string()));
        dbg.execute(Command::Break("11".to_string()));
        let msg = dbg.execute(Command::Delete(Some("8".to_string())));
        assert_eq!(msg, vec!["Deleted breakpoint at 0x100 (8)".to_string()]);
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x118);
        assert!(
            stop.starts_with("stack.mms:11\t"),
            "must stop at the remaining breakpoint, not the deleted one, got {stop:?}"
        );
    }

    /// Deleting a breakpoint set on a label, rather than a line number,
    /// stops it from firing.
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

    /// `break :Lib` sets a breakpoint where `break Lib` does: the root
    /// prefix stores `Lib` and `:Lib` under the same key, and `resolve_
    /// break_location` strips the leading `:` before the lookup.
    #[test]
    fn break_with_root_colon_matches_the_plain_label() {
        let mut dbg = Debugger::load(assemble(CALL_PROGRAM, "call.mms"));
        dbg.execute(Command::Break(":Main".to_string()));
        let stop = dbg.execute(Command::Run).join("\n");
        assert_eq!(dbg.mmix.get_pc(), 0x100);
        assert!(
            stop.starts_with("call.mms:"),
            "':Main' must resolve the same breakpoint as 'Main', got {stop:?}"
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

    /// An unresolvable `delete` argument, and a resolvable one with no
    /// breakpoint currently set there, both leave existing breakpoints
    /// untouched.
    #[test]
    fn delete_with_no_effect_leaves_breakpoints_unchanged() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        dbg.execute(Command::Break("11".to_string()));
        // Line 999 has no mapped address: unresolvable.
        let unresolvable = dbg.execute(Command::Delete(Some("999".to_string())));
        assert_eq!(
            unresolvable,
            vec!["No location found for '999'; nothing deleted".to_string()]
        );
        // Line 9 resolves, but no breakpoint sits there.
        let no_breakpoint = dbg.execute(Command::Delete(Some("9".to_string())));
        assert_eq!(
            no_breakpoint,
            vec!["No breakpoint at 0x110 (9)".to_string()]
        );
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

    #[test]
    fn print_x_formats_every_hex_table_row() {
        assert_eq!(format_as(0, PrintFormat::Hex), "#0");
        assert_eq!(format_as(5, PrintFormat::Hex), "#5");
        assert_eq!(
            format_as(0x3FE0000000000000, PrintFormat::Hex),
            "#3fe0000000000000"
        );
        assert_eq!(
            format_as(0xFFFFFFFFFFFFFFFF, PrintFormat::Hex),
            "#ffffffffffffffff"
        );
    }

    #[test]
    fn print_f_formats_every_float_table_row() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "float.mms"));
        let rows: [(u64, &str); 17] = [
            (0x3FE0000000000000, "0.5"),
            (0x3FF0000000000000, "1"),
            (0x4059000000000000, "100"),
            (0xC004000000000000, "-2.5"),
            (0x3F1A36E2EB1C432D, "0.0001"),
            (0x3EE4F8B588E368F1, "1e-5"),
            (0x4341C37937E08000, "1e16"),
            (0x7FEFFFFFFFFFFFFF, "1.7976931348623157e308"),
            (0x0010000000000000, "2.2250738585072014e-308"),
            (0x0000000000000001, "5e-324"),
            (0, "0"),
            (0x8000000000000000, "-0"),
            (0x7FF0000000000000, "inf"),
            (0xFFF0000000000000, "-inf"),
            (0x7FF8000000000000, "nan(#8000000000000)"),
            (0xFFF8000000000001, "-nan(#8000000000001)"),
            (0x7FF0000000000001, "nan(#1)"),
        ];
        for (bits, expected) in rows {
            dbg.mmix.set_register(1, bits);
            assert_eq!(
                dbg.do_print_as(PrintFormat::Float, "$1"),
                expected,
                "mismatch for {bits:#018x}"
            );
        }
    }

    /// A general register, a special register, a label, a register-valued
    /// (`GREG`) symbol, a constant-valued (`IS`) symbol and a hex address --
    /// every form `do_print` resolves.
    const ARG_FORMS_PROGRAM: &str = "\
        LOC     Data_Segment
Cells   OCTA    0
Sp      GREG    Cells
Limit   IS      100
        LOC     #100
Main    TRAP    0,Halt,0
";

    #[test]
    fn print_format_suffixes_apply_to_every_argument_form() {
        let mut dbg = Debugger::load(assemble(ARG_FORMS_PROGRAM, "argforms.mms"));
        let main_addr = *dbg.assembler.labels.get("Main").unwrap();
        dbg.do_set("$1".to_string(), "0x3FE0000000000000".to_string());
        dbg.mmix.set_special(SpecialReg::RJ, 0x2A);
        dbg.do_set("Sp".to_string(), "9".to_string());
        dbg.mmix.write_octa(0x200, 0x3FF0000000000000);

        let cases: [(&str, u64); 6] = [
            ("$1", 0x3FE0000000000000),
            ("rJ", 0x2A),
            ("Main", main_addr),
            ("Sp", 9),
            ("Limit", 100),
            ("0x200", 0x3FF0000000000000),
        ];
        for (arg, expected) in cases {
            assert_eq!(
                dbg.resolve_print_argument(arg),
                Some(expected),
                "resolution mismatch for {arg}"
            );
            assert_eq!(
                dbg.do_print_as(PrintFormat::Hex, arg),
                format_as(expected, PrintFormat::Hex),
                "p/x mismatch for {arg}"
            );
            assert_eq!(
                dbg.do_print_as(PrintFormat::Float, arg),
                format_as(expected, PrintFormat::Float),
                "p/f mismatch for {arg}"
            );
        }

        assert_eq!(
            dbg.do_print_as(PrintFormat::Hex, "Bogus"),
            "No symbol \"Bogus\" in current context."
        );
        assert_eq!(
            dbg.do_print_as(PrintFormat::Float, "Bogus"),
            "No symbol \"Bogus\" in current context."
        );
    }

    /// `set_format` governs `ValueFormat::Signed`/`Unsigned` for plain
    /// `print` only; `p/x` and `p/f` read the same bits either way.
    #[test]
    fn set_format_affects_plain_print_only() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "format.mms"));
        dbg.do_set("$1".to_string(), "-1".to_string());
        let signed = dbg.do_print("$1");
        let hex = dbg.do_print_as(PrintFormat::Hex, "$1");
        let float = dbg.do_print_as(PrintFormat::Float, "$1");

        dbg.set_format(ValueFormat::Unsigned);

        assert_ne!(
            signed,
            dbg.do_print("$1"),
            "set_format must change plain print"
        );
        assert_eq!(
            dbg.do_print_as(PrintFormat::Hex, "$1"),
            hex,
            "p/x must ignore set_format"
        );
        assert_eq!(
            dbg.do_print_as(PrintFormat::Float, "$1"),
            float,
            "p/f must ignore set_format"
        );
    }

    #[test]
    fn blank_repeats_a_formatted_print() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "repeat_fmt.mms"));
        dbg.do_set("$1".to_string(), "0x2A".to_string());
        let cmd = parse_command("p/x $1").unwrap();
        let first = dbg.execute(cmd);
        let repeated = dbg.execute(Command::Repeat);
        assert_eq!(first, repeated);
        assert_eq!(first, vec!["#2a".to_string()]);
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
        dbg.mmix.set_journal(true);
        dbg.execute(Command::Run);
        dbg.mmix.take_journal(); // drain the first run's writes
        // `disable` is never called; `Command::Run` resets the machine.
        dbg.execute(Command::Run);
        assert!(
            !dbg.mmix.take_journal().is_empty(),
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

    /// A program with no `GREG` at all declares no globals, so MMIX starts
    /// it with every register but `$255` local: rG = 255.
    const NO_GREG_PROGRAM: &str = "\
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";

    #[test]
    fn write_image_derives_rg_255_with_no_greg() {
        let dbg = Debugger::load(assemble(NO_GREG_PROGRAM, "no_greg.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 255);
    }

    #[test]
    fn write_image_leaves_rl_at_zero_after_load() {
        let dbg = Debugger::load(assemble(NO_GREG_PROGRAM, "no_greg.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 0);

        let dbg = Debugger::load(assemble(ONE_GREG_PROGRAM, "one_greg.mms"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 0);
    }

    /// `write_image` moves rG before applying a `GREG` value: entering with
    /// rG above the value this program derives, its one `GREG` register
    /// sits below the machine's current rG, and applying the value before
    /// rG drops would wrongly claim that register as local.
    #[test]
    fn write_image_moves_rg_before_applying_greg_values() {
        let asm = assemble(ONE_GREG_PROGRAM, "one_greg.mms");
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RG, 255);
        write_image(&mut mmix, &asm);
        assert_eq!(mmix.get_special(SpecialReg::RG), 254);
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    }

    /// With no `GREG`, rG = 255 puts `$100` below the global threshold: a
    /// write to it grows rL to claim it as a local register.
    #[test]
    fn write_to_a_local_register_raises_rl_under_the_no_greg_default() {
        let mut dbg = Debugger::load(assemble(NO_GREG_PROGRAM, "no_greg.mms"));
        dbg.mmix.set_register(100, 7);
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 101);
    }

    #[test]
    fn start_program_sets_pc_and_dollar_255_to_the_entry() {
        let asm = assemble(MINIMAL_PROGRAM, "minimal.mms");
        let entry = entry_point(&asm);
        let mut mmix = MMix::new();
        start_program(&mut mmix, entry);
        assert_eq!(mmix.get_pc(), entry);
        assert_eq!(mmix.get_register(255), entry);
    }

    #[test]
    fn start_program_uses_the_fallback_entry_with_no_main_label() {
        const NO_MAIN_PROGRAM: &str = "\tLOC\t#100\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(NO_MAIN_PROGRAM, "no_main.mms");
        assert!(!asm.labels.contains_key("Main"));
        let entry = entry_point(&asm);
        let mut mmix = MMix::new();
        start_program(&mut mmix, entry);
        assert_eq!(mmix.get_pc(), entry);
        assert_eq!(mmix.get_register(255), entry);
    }

    #[test]
    fn start_program_leaves_rl_unchanged() {
        let mut mmix = MMix::new();
        mmix.set_register(3, 7); // raises rL to 4
        let rl_before = mmix.get_special(SpecialReg::RL);
        start_program(&mut mmix, 0x100);
        assert_eq!(mmix.get_special(SpecialReg::RL), rl_before);
    }

    #[test]
    fn debugger_load_and_reset_both_hold_dollar_255_at_the_entry() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "minimal.mms"));
        let entry = dbg.entry;
        assert_eq!(dbg.mmix.get_register(255), entry);

        // Disturb $255, then confirm reset restores it.
        dbg.mmix.set_register(255, 0);
        dbg.reset();
        assert_eq!(dbg.mmix.get_register(255), entry);
    }

    #[test]
    fn greg_program_with_pushj_executes_correctly_under_raised_rg() {
        // One GREG directive raises rG to 254 (see the derivation test
        // above); this program then makes a PUSHJ/POP call, modeled on
        // examples/subroutine.mms, to confirm the register-window slide is
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
        // exercised by test_pushj_window_slide_return_value
        // (src/mmix/tests/stack.rs):
        // the two SETIs grow rL to 3 ($1, then $2, each >= the then-current
        // rL); PUSHJ $0 slides caller's $1, $2 (40, 2) down to callee's $0,
        // $1; POP 1 places the callee's $0 (the sum) at the caller's hole
        // $0, and sets rL = min(x + n, rG) = min(0 + 1, 254) = 1.
        assert_eq!(dbg.mmix.get_register(0), 42);
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 1);
        assert_eq!(dbg.mmix.get_register(255), 42);
        assert_eq!(dbg.mmix.get_exit_code(), 42);
    }

    const MINIMAL_PROGRAM: &str = "\tLOC\t#100\nMain\tTRAP\t0,Halt,0\n";

    /// No `GREG`, so `derive_rg` gives rG = 255 and rL starts at 0.
    /// `SAVE` indexes `general_regs` by rL, so an out-of-range `set rL`
    /// that reached it would panic `save_context`.
    const SAVE_PROGRAM: &str = "\tLOC\t#100\nMain\tSAVE\t$255,0\n\tTRAP\t0,Halt,0\n";

    /// `Limit` is a constant-valued `IS` symbol -- not a storage location,
    /// and not settable, distinct from a register-aliasing `GREG`/`IS $N`
    /// symbol.
    const CONSTANT_SYMBOL_PROGRAM: &str = "\
Limit\tIS\t100
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";

    #[test]
    fn set_writes_a_general_register_with_a_decimal_value() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        dbg.do_set("$3".to_string(), "42".to_string());
        assert_eq!(dbg.mmix.get_register(3), 42);
    }

    #[test]
    fn set_writes_a_negative_decimal_as_its_twos_complement_bit_pattern() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        dbg.do_set("$3".to_string(), "-1".to_string());
        assert_eq!(dbg.mmix.get_register(3), u64::MAX);
    }

    /// A decimal literal above `i64::MAX` cannot round-trip through `i64`
    /// parsing alone; the `u64` fallback is what makes this work.
    #[test]
    fn set_round_trips_a_decimal_value_above_i64_max() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        let value = (i64::MAX as u64) + 100;
        dbg.do_set("$3".to_string(), value.to_string());
        assert_eq!(dbg.mmix.get_register(3), value);
    }

    /// Pins that a register-aliasing symbol (here `Sp`, from
    /// `STACK_PROGRAM`'s `GREG` directive) is a settable `set` target,
    /// and writes the same register `print`/`p` reads.
    #[test]
    fn set_writes_through_a_greg_aliased_symbol_name() {
        let mut dbg = Debugger::load(assemble(STACK_PROGRAM, "stack.mms"));
        let reg = match dbg.assembler.symbols.get("Sp").copied() {
            Some(SymbolType::Register(n)) => n,
            other => panic!("Sp must be a register-aliasing symbol, got {other:?}"),
        };
        dbg.do_set("Sp".to_string(), "99".to_string());
        assert_eq!(dbg.mmix.get_register(reg), 99);
        assert_eq!(dbg.do_print("Sp"), format_value(99, dbg.format));
    }

    /// The special-register name is derived from the variant itself, not
    /// hardcoded -- this file has a documented history of special-register
    /// display-name bugs (the alphabetical-vs-discriminant mismatch fixed
    /// 2026-08-22).
    #[test]
    fn set_writes_a_special_register_with_a_hex_value() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        let name = SpecialReg::RJ.name();
        dbg.do_set(name.to_string(), "0x10".to_string());
        assert_eq!(dbg.mmix.get_special(SpecialReg::RJ), 0x10);
    }

    /// `write_octa` masks its address down to an 8-byte base; `set` must
    /// write there too, not at the address as typed.
    #[test]
    fn set_writes_memory_at_the_aligned_base_of_an_unaligned_address() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        let msg = dbg.do_set("0x104".to_string(), "5".to_string());
        assert_eq!(msg, "0x100 = 5");
        assert_eq!(dbg.mmix.read_octa(0x100), 5);
    }

    /// `set` writes `rA` directly through `set_special`, bypassing
    /// `put_special`, which drops a `PUT` of any value above `RA_MAX`
    /// (`#3FFFF`) and leaves rA unchanged -- deliberate: `set` is a raw
    /// debugger poke, not a `PUT` simulation, and this is the one target
    /// where that distinction is actually observable.
    #[test]
    fn set_writes_ra_above_the_put_instructions_clamp() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        let name = SpecialReg::RA.name();
        let above_ra_max = crate::mmix::RA_MAX + 1;
        dbg.do_set(name.to_string(), format!("0x{above_ra_max:x}"));
        assert_eq!(dbg.mmix.get_special(SpecialReg::RA), above_ra_max);
    }

    /// `set rL` above rG is rejected, rL unchanged, and `SAVE` (which
    /// indexes `general_regs` by rL) still runs without a panic.
    #[test]
    fn set_rl_above_rg_is_rejected_and_save_still_runs() {
        let mut dbg = Debugger::load(assemble(SAVE_PROGRAM, "save.mms"));

        let msg = dbg.do_set(SpecialReg::RL.name().to_string(), "300".to_string());
        assert_eq!(msg, "Invalid rL 300: must not exceed rG=255");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 0);

        let msg = dbg.do_set(SpecialReg::RL.name().to_string(), "256".to_string());
        assert_eq!(msg, "Invalid rL 256: must not exceed rG=255");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 0);

        dbg.execute(Command::Stepi); // SAVE $255,0 -- must not panic
        assert_eq!(dbg.mmix.get_pc(), 0x104);
    }

    /// `set rG` outside 32-255, or below rL, is rejected and rG unchanged.
    #[test]
    fn set_rg_outside_put_specials_range_is_rejected() {
        let mut dbg = Debugger::load(assemble(SAVE_PROGRAM, "save.mms"));

        let msg = dbg.do_set(SpecialReg::RG.name().to_string(), "31".to_string());
        assert_eq!(msg, "Invalid rG 31: must be 32-255 and at least rL=0");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 255);

        let msg = dbg.do_set(SpecialReg::RG.name().to_string(), "256".to_string());
        assert_eq!(msg, "Invalid rG 256: must be 32-255 and at least rL=0");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 255);
    }

    /// `set rG` and `set rL` accept a value that keeps rL <= rG, and the
    /// rL rejection above `rG`'s new value names that value.
    #[test]
    fn set_rg_then_rl_accepts_within_the_new_bound() {
        let mut dbg = Debugger::load(assemble(SAVE_PROGRAM, "save.mms"));

        let msg = dbg.do_set(SpecialReg::RG.name().to_string(), "40".to_string());
        assert_eq!(msg, "rG = 40");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RG), 40);

        let msg = dbg.do_set(SpecialReg::RL.name().to_string(), "41".to_string());
        assert_eq!(msg, "Invalid rL 41: must not exceed rG=40");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 0);

        let msg = dbg.do_set(SpecialReg::RL.name().to_string(), "40".to_string());
        assert_eq!(msg, "rL = 40");
        assert_eq!(dbg.mmix.get_special(SpecialReg::RL), 40);
    }

    /// `set` reaches `Command::execute`'s dispatch, not just `do_set`
    /// directly -- the two arguments must land the right way around, and
    /// the returned success message must match the format contract.
    #[test]
    fn set_dispatches_through_execute_and_reports_the_success_message() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        let cmd = parse_command("set $3 42").unwrap();
        let output = dbg.execute(cmd);
        assert_eq!(output, vec!["$3 = 42".to_string()]);
        assert_eq!(dbg.mmix.get_register(3), 42);
    }

    /// `IS $N` also produces a register-aliasing symbol
    /// (`SymbolType::Register`), distinct from a `GREG` label -- both
    /// register-aliasing forms must be settable, not just `GREG`'s.
    #[test]
    fn set_writes_through_an_is_register_alias_symbol_name() {
        let source = "Zero\tIS\t$255\n\tLOC\t#100\nMain\tTRAP\t0,Halt,0\n";
        let mut dbg = Debugger::load(assemble(source, "is.mms"));
        assert_eq!(
            dbg.assembler.symbols.get("Zero").copied(),
            Some(SymbolType::Register(255))
        );
        dbg.do_set("Zero".to_string(), "7".to_string());
        assert_eq!(dbg.mmix.get_register(255), 7);
    }

    #[test]
    fn set_rejects_an_invalid_value_and_mutates_nothing() {
        let mut dbg = Debugger::load(assemble(MINIMAL_PROGRAM, "set.mms"));
        let before = dbg.mmix.get_register(3);
        let result = dbg.do_set("$3".to_string(), "notanumber".to_string());
        assert_eq!(
            result,
            "Invalid value 'notanumber'; expected decimal or 0x/#-prefixed hex"
        );
        assert_eq!(dbg.mmix.get_register(3), before);
    }

    /// The value is validated before the target is resolved: an invalid
    /// value on an unresolvable target reports the value error, not the
    /// not-settable-target error.
    #[test]
    fn set_reports_the_value_error_when_target_and_value_are_both_bad() {
        let mut dbg = Debugger::load(assemble(CALL_PROGRAM, "set.mms"));
        let result = dbg.do_set("Main".to_string(), "notanumber".to_string());
        assert_eq!(
            result,
            "Invalid value 'notanumber'; expected decimal or 0x/#-prefixed hex"
        );
    }

    /// A label is a compile-time address constant, not a storage location:
    /// `set` must reject it, not silently reinterpret it.
    #[test]
    fn set_rejects_a_label_as_a_target() {
        let mut dbg = Debugger::load(assemble(CALL_PROGRAM, "set.mms"));
        let main_addr = *dbg.assembler.labels.get("Main").unwrap();
        let before = dbg.mmix.read_octa(main_addr);
        let result = dbg.do_set("Main".to_string(), "5".to_string());
        assert_eq!(
            result,
            "No settable target \"Main\" (register, special register, or hex memory address only)"
        );
        assert_eq!(dbg.mmix.read_octa(main_addr), before);
    }

    /// A constant-valued `IS` symbol is also not a storage location --
    /// distinct from the `GREG`-symbol test above, this confirms `do_set`
    /// tells `SymbolType::Constant` and `SymbolType::Register` apart rather
    /// than treating every symbol alike.
    #[test]
    fn set_rejects_a_constant_is_symbol_as_a_target() {
        let mut dbg = Debugger::load(assemble(CONSTANT_SYMBOL_PROGRAM, "set.mms"));
        assert_eq!(
            dbg.assembler.symbols.get("Limit").copied(),
            Some(SymbolType::Constant(100))
        );
        let result = dbg.do_set("Limit".to_string(), "5".to_string());
        assert_eq!(
            result,
            "No settable target \"Limit\" (register, special register, or hex memory address only)"
        );
        assert_eq!(
            dbg.assembler.symbols.get("Limit").copied(),
            Some(SymbolType::Constant(100))
        );
    }
}
