//! `mmixdb` debugger core.
//!
//! This module holds all state and command logic for the gdb-style MMIX
//! debugger. It has no TTY dependency: every command is a method that
//! mutates a `Debugger` and returns rendered text, so the whole thing is
//! unit-testable without a terminal. `src/bin/mmixdb.rs` is a thin shell
//! that reads lines (via `rustyline`), calls `parse_command` and
//! `Debugger::execute`, and prints the result.

use crate::mmix::{MMix, SpecialReg, ValueFormat};
use crate::mmixal::{MMixAssembler, SymbolType};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The MMIX text/data segment boundary. Mirrors `run_mms`'s fallback
/// (`src/bin/checksmix.rs`): when no `Main` label exists, the entry point is
/// the first instruction address below this boundary.
const SEGMENT_BOUNDARY: u64 = 0x2000000000000000;

/// A parsed debugger command. One variant per command in the command table;
/// `Repeat` represents blank input, which re-runs the last executed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Step,
    Next,
    Continue,
    Run,
    Break(String),
    Print(String),
    State,
    List,
    Help,
    Quit,
    Repeat,
}

/// Parse one line of debugger input into a `Command`.
///
/// Supports both the short letter and the gdb long word for each command
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
            "" => Err("info requires a subcommand (reg|registers)".to_string()),
            other => Err(format!("unknown info subcommand: {other}")),
        },
        "l" | "list" => Ok(Command::List),
        "h" | "help" | "?" => Ok(Command::Help),
        "q" | "quit" => Ok(Command::Quit),
        other => Err(format!("unknown command: {other}")),
    }
}

/// Map a special-register name to its `SpecialReg`, from the SAME
/// name/discriminant pairs the assembler pre-seeds at `src/mmixal.rs:1154-1189`
/// (`("rJ", 4)`, `("rA", 21)`, ...). Do NOT build this from the display's
/// `special_names` array (`src/mmix.rs` ~:4020) -- that array is alphabetically
/// ordered and does not align with `SpecialReg`'s real discriminants except at
/// a few coincidental indices.
fn special_reg_from_name(name: &str) -> Option<SpecialReg> {
    let num: u8 = match name {
        "rB" => 0,
        "rD" => 1,
        "rE" => 2,
        "rH" => 3,
        "rJ" => 4,
        "rM" => 5,
        "rR" => 6,
        "rBB" => 7,
        "rC" => 8,
        "rN" => 9,
        "rO" => 10,
        "rS" => 11,
        "rI" => 12,
        "rT" => 13,
        "rTT" => 14,
        "rK" => 15,
        "rQ" => 16,
        "rU" => 17,
        "rV" => 18,
        "rG" => 19,
        "rL" => 20,
        "rA" => 21,
        "rF" => 22,
        "rP" => 23,
        "rW" => 24,
        "rX" => 25,
        "rY" => 26,
        "rZ" => 27,
        "rWW" => 28,
        "rXX" => 29,
        "rYY" => 30,
        "rZZ" => 31,
        _ => return None,
    };
    SpecialReg::from_u8(num)
}

fn format_value(value: u64, format: ValueFormat) -> String {
    match format {
        ValueFormat::Signed => (value as i64).to_string(),
        ValueFormat::Unsigned => value.to_string(),
    }
}

fn write_image(mmix: &mut MMix, assembler: &MMixAssembler) {
    for (addr, inst) in &assembler.instructions {
        let bytes = assembler.encode_instruction_bytes(inst);
        for (offset, &byte) in bytes.iter().enumerate() {
            mmix.write_byte(addr + offset as u64, byte);
        }
    }
}

/// Replicates `run_mms`'s entry-point selection (`src/bin/checksmix.rs:213-273`):
/// the `Main` label if present, else the first code address below the
/// text/data segment boundary.
fn entry_point(assembler: &MMixAssembler) -> u64 {
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

/// The gdb-style debugger core: owns the loaded `MMix`, the `MMixAssembler`
/// (for the source map and symbol tables), breakpoints, and REPL state.
pub struct Debugger {
    mmix: MMix,
    assembler: MMixAssembler,
    entry: u64,
    primary_file: Option<String>,
    breakpoints: BTreeSet<u64>,
    last_command: Option<Command>,
    fullname: bool,
    format: ValueFormat,
}

impl Debugger {
    /// Load an assembled program: run the `run_mms` load sequence (write
    /// every instruction's bytes to memory, then resolve the entry point)
    /// and set PC there.
    pub fn load(assembler: MMixAssembler) -> Debugger {
        let mut mmix = MMix::new();
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
            Command::Next => self.do_next(),
            Command::Continue => self.do_continue(),
            Command::Run => self.do_run(),
            Command::Break(arg) => vec![self.do_break(arg.clone())],
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
        self.mmix = MMix::new();
        write_image(&mut self.mmix, &self.assembler);
        self.mmix.set_pc(self.entry);
    }

    fn do_step(&mut self) -> Vec<String> {
        let running = self.mmix.execute_instruction();
        self.report(!running)
    }

    /// `next`: execute one instruction; if it entered a call (call depth
    /// increased -- PUSHJ/PUSHGO push a frame; GO does not), keep
    /// single-stepping until the depth returns to the pre-call level, a
    /// breakpoint is hit, or the program halts. Otherwise stop after the one
    /// instruction, same as `step`.
    fn do_next(&mut self) -> Vec<String> {
        let d0 = self.mmix.call_depth();
        let mut halted = !self.mmix.execute_instruction();
        if !halted {
            while self.mmix.call_depth() > d0 {
                if self.breakpoints.contains(&self.mmix.get_pc()) {
                    break;
                }
                if !self.mmix.execute_instruction() {
                    halted = true;
                    break;
                }
            }
        }
        self.report(halted)
    }

    /// `continue`: single-step from the current PC until a breakpoint
    /// address is hit or the program halts.
    fn do_continue(&mut self) -> Vec<String> {
        let mut halted = false;
        loop {
            if !self.mmix.execute_instruction() {
                halted = true;
                break;
            }
            if self.breakpoints.contains(&self.mmix.get_pc()) {
                break;
            }
        }
        self.report(halted)
    }

    /// `run`/reset: reset the machine to the freshly-loaded image, then
    /// behave like `continue`.
    fn do_run(&mut self) -> Vec<String> {
        self.reset();
        self.do_continue()
    }

    fn do_break(&mut self, arg: String) -> String {
        let arg = arg.trim();
        let resolved = if let Ok(line) = arg.parse::<usize>() {
            self.current_file()
                .and_then(|file| self.assembler.addr_for_line(&file, line))
        } else {
            self.assembler.labels.get(arg).copied()
        };
        match resolved {
            Some(addr) => {
                self.breakpoints.insert(addr);
                format!("Breakpoint set at 0x{addr:x} ({arg})")
            }
            None => format!("No location found for '{arg}'; breakpoint not set"),
        }
    }

    /// `print <arg>` resolution, in priority order: `$N`/bare `N` (general
    /// register), a special-register name, a label, an IS/GREG symbol, a hex
    /// address (memory octa), else an error.
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

    fn current_file(&self) -> Option<String> {
        self.assembler
            .source_loc(self.mmix.get_pc())
            .map(|loc| loc.file.clone())
            .or_else(|| self.primary_file.clone())
    }

    /// The report shown on every stop: the Emacs GUD marker (if `fullname`
    /// mode is on and the current PC has a known source location) followed
    /// by the gdb-style current-line display, or a halt message.
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

    fn location_line(&self) -> String {
        let pc = self.mmix.get_pc();
        match self.assembler.source_loc(pc) {
            Some(loc) => {
                let text = self
                    .assembler
                    .source_text(&loc.file, loc.line)
                    .unwrap_or("");
                format!("{}:{}\t{}", loc.file, loc.line, text)
            }
            None => format!("0x{pc:016x} in ?? (no source line)"),
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
step (into)   s, step                          Execute exactly one instruction, following into calls/branches.
next (over)   n, next                          Execute one instruction; if it entered a call, keep stepping until it returns.
continue      c, continue                      Resume, single-stepping until a breakpoint or halt.
run/reset     r, run                           Reset to the freshly-loaded image, then behave like continue.
break         b <line>, b <label>, break …     Set a breakpoint at a source line or label.
print         p <arg>, print <arg>             Print a register, special register, label address, IS/GREG symbol, or memory octa.
state         bt, backtrace, info reg, info registers   Print the full register dump.
list          l, list                          Print source lines around the current PC.
help          h, help, ?                       Show this help.
quit          q, quit                          Exit the debugger.

Blank input repeats the last command.";
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

    #[test]
    fn parse_command_maps_all_forms() {
        assert_eq!(parse_command("s"), Ok(Command::Step));
        assert_eq!(parse_command("step"), Ok(Command::Step));
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
        assert_eq!(parse_command("l"), Ok(Command::List));
        assert_eq!(parse_command("list"), Ok(Command::List));
        assert_eq!(parse_command("q"), Ok(Command::Quit));
        assert_eq!(parse_command("quit"), Ok(Command::Quit));
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
        assert!(joined.contains("break"));
        assert!(joined.contains("print"));
        assert!(joined.contains("quit"));
        assert!(joined.contains("help"));
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
        dbg.execute(Command::Next);
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
        dbg.execute(Command::Continue);
        assert_eq!(dbg.mmix.get_pc(), target_addr);
        assert_eq!(dbg.current_file().as_deref(), Some("bp.mms"));
        let loc = dbg.assembler.source_loc(dbg.mmix.get_pc()).unwrap();
        assert_eq!(loc.line, target_line);
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
        // SET $3,42 assembles to 4 real instructions (SETH/SETMH/SETML/SETL);
        // step through all of them.
        for _ in 0..4 {
            dbg.execute(Command::Step);
        }
        assert_eq!(dbg.do_print("$3"), "42");
        assert_eq!(dbg.do_print("Main"), format_value(main_addr, dbg.format));
    }

    #[test]
    fn print_special_register_uses_the_correct_table_not_special_names() {
        let source = "\tLOC\t#100\nMain\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "special.mms");
        let mut dbg = Debugger::load(asm);
        // rJ's real discriminant is 4; `special_names`'s alphabetical order
        // puts "rJ" at index 9, not 4 -- a mapping built from that array
        // would read the wrong slot.
        dbg.mmix.set_special(SpecialReg::RJ, 0xDEAD_BEEF_1234);
        assert_eq!(
            dbg.do_print("rJ"),
            format_value(dbg.mmix.get_special(SpecialReg::RJ), dbg.format)
        );
        assert_eq!(dbg.do_print("rJ"), "244837814047284");
    }

    #[test]
    fn blank_repeats_last_command() {
        let source = "\tLOC\t#100\nMain\tSETI\t$1,1\n\tSETI\t$2,2\n\tTRAP\t0,Halt,0\n";
        let asm = assemble(source, "repeat.mms");
        let mut dbg = Debugger::load(asm);
        let pc0 = dbg.mmix.get_pc();
        dbg.execute(Command::Step);
        let pc1 = dbg.mmix.get_pc();
        assert_ne!(pc0, pc1, "first step must advance the PC");
        dbg.execute(Command::Repeat);
        let pc2 = dbg.mmix.get_pc();
        assert_ne!(pc1, pc2, "blank repeat must advance the PC again");
    }
}
