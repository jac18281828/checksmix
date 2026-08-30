# ChecksMix - Blazing Fast MMIX Emulator

MMIX assembler and emulator with fast feedback for learning, experimenting, and debugging Knuth’s 64-bit machine. MIX source still parses and emulates, but the checksmix now focuses on MMIX with `.mms` and `.mmo` workflows.

**Try it in your browser: [playmmix.2ad.com](https://playmmix.2ad.com).** A playground built on this crate — edit MMIX assembly, run it, step through with breakpoints, and watch the registers and memory change. No toolchain to install.

## What’s inside
- `checksmix`: execute `.mms` assembly directly or run prebuilt `.mmo` object files.
- `mmixasm`: assemble `.mms` to `.mmo` for reuse or distribution.
- `mmixdb`: interactive source-level debugger for `.mms` programs (step, breakpoints, print, Emacs GUD mode).
- Emulator: 256 general-purpose registers, 32 special registers, sparse 64-bit address space, and basic TRAP support (Halt and Fputs for console output).

## Quick start
1) Install Rust (stable toolchain).  
2) From the repo root, run an example immediately:

```bash
cargo run --bin checksmix -- examples/hello_world.mms
```

Or build once for repeat runs:

```bash
cargo build --release
./target/release/checksmix examples/hello_world.mms
```

Set `RUST_LOG=checksmix=debug` to see instruction decoding and TRAP handling while you experiment.

## Common workflows

### Run MMIX assembly directly
`checksmix` parses and executes `.mms` without producing an object file:

```bash
cargo run --bin checksmix -- examples/linked_list.mms
```

### Assemble to MMO, then emulate
Generate a reusable object file with `mmixasm`, then run it with `checksmix`:

```bash
# Assemble
cargo run --bin mmixasm -- examples/hello_world.mms -o target/hello_world.mmo

# Execute the MMO
cargo run --bin checksmix -- target/hello_world.mmo
```

### Use your own program
1) Write MMIX assembly (see the snippet below).  
2) Run it directly with `checksmix` **or** assemble with `mmixasm` and run the resulting `.mmo`.  
3) Inspect register and memory dumps printed before and after execution.

## Example programs

- `examples/exit_code.mms`: two instructions. `TRAP 0,Halt,0` returns `$255` as the
  process exit status.
- `examples/hello_world.mms`: prints a string via `TRAP 0,Fputs,StdOut`.
- `examples/function.mms`: the MMIX calling convention. `PUSHJ` slides the register window
  so the caller's `$X+1,$X+2` arrive as the callee's `$0,$1`, and `POP 1` returns a value
  to the caller's hole.
- `examples/fibonacci.mms`: that convention applied to an iterative `fib(20)`.
- `examples/remeuclid.mms`: Euclidean remainder over eight cases, accumulating a pass
  bit-field in `$20`.
- `examples/linked_list.mms`: walks a statically allocated list and sums node values.
- `examples/time.mms`: reads the host clock through `TRAP 0,Time,2`.
- `examples/all_instructions_test.mms`: broad instruction coverage for regression checks.
- `examples/example.mix`: a MIXAL program, for the legacy MIX path below.

Hello World (trimmed):

```asm
        LOC     Data_Segment
        GREG    @
Text    BYTE    "Hello world!",'\n',0

        LOC     #100
Main    LDA     $0,Text
        TRAP    0,Fputs,StdOut
        TRAP    0,Halt,0
```

## mmixdb — the interactive debugger

`mmixdb` is an interactive source-level debugger for MMIX `.mms` programs: step, breakpoint,
inspect registers/memory, and see the current source line as you go.

```bash
cargo run --bin mmixdb -- examples/fibonacci.mms
cargo run --bin mmixdb -- --fullname examples/fibonacci.mms   # Emacs GUD marker mode
```

`mmixdb` handles `.mms` sources only -- source-line debugging requires the
original source. `.mmo` object files carry no source map and are out of scope.
`--fullname` is auto-enabled when the `INSIDE_EMACS` environment variable is
set (i.e. when run from Emacs's `gud-mode`).

| Command | Forms | Semantics |
|---|---|---|
| step (into) | `s`, `step` | Execute one source line, following into calls/branches. |
| next (over) | `n`, `next` | Execute one source line, stepping over any call it makes. |
| stepi | `si`, `stepi` | Execute exactly one instruction, following into calls/branches. |
| continue | `c`, `continue` | Resume, single-stepping until a breakpoint or halt. |
| run/reset | `r`, `run` | Reset to the freshly-loaded image, then run on; a breakpoint on the entry point fires. |
| break | `b <line>`, `b <label>`, `break …` | Set a breakpoint at a source line or label. |
| delete | `d`, `delete`, `d <line>`, `d <label>` | Delete one breakpoint, or every breakpoint given no argument. |
| print | `p <arg>`, `print <arg>` | Print a register (`$N`/`N`), special register (`rJ`, `rA`, ...), label address, IS/GREG symbol, or the memory octa at a hex address's aligned 8-byte base (`0x...`/`#...`). |
| set | `set <target> <value>` | Write a register (`$N`/`N`), special register, or the memory octa at a hex address's aligned 8-byte base. A symbol whose type is a register alias (from `GREG` or a register-valued `IS`, e.g. `Sp`) is settable the same way; a label or a symbol whose type is a constant (an `IS` bound to a non-register value) is not -- neither names a storage location. `value` is decimal or `0x`/`#`-prefixed hex. |
| state | `bt`, `backtrace`, `info reg`, `info registers` | Print the full register dump. |
| breakpoints | `info break`, `info breakpoints` | List every currently-set breakpoint with its source location. |
| list | `l`, `list` | Print source lines around the current PC. |
| help | `h`, `help`, `?` | Show this help. |
| quit | `q`, `quit`, `exit` | Exit the debugger. |

Blank input repeats the last command -- most debugging is stepping. Once the
program has exited, `step`, `stepi`, `next` and `continue` are refused; `run`
restarts it.

Emacs users: `contrib/mmixdb.el` provides `M-x mmixdb` under `gud-mode`. Put
`contrib/` on `load-path` and require it:

```elisp
(add-to-list 'load-path "/path/to/checksmix/contrib")
(require 'mmixdb)
```

or autoload it instead:

```elisp
(autoload 'mmixdb "mmixdb" "Run mmixdb under gud-mode." t)
```

## Legacy MIX support
`.mix` and `.mixal` files still run through `checksmix`, but MMIX is the primary target. Prefer `.mms`/`.mmo` for new work.

## Using checksmix as a library
`checksmix` is usable as a library, independent of the three binaries above. The
`clap`, `rustyline`, and `tracing-subscriber` dependencies the CLIs need live behind
the `cli` feature, which is on by default. A library-only consumer — notably one
targeting `wasm32-unknown-unknown`, where `rustyline` does not build — turns it off.
Available from 0.3.0; earlier versions have no features and always pull the CLI tree.
[playmmix](https://playmmix.2ad.com) is built this way, on `checksmix = { version = "0.3",
default-features = false }`, and runs the emulator in the browser as wasm.

```toml
# default — library plus the CLI dependency tree
checksmix = "0.3"

# library only
checksmix = { version = "0.3", default-features = false }
```

Either form gives you the library. The `checksmix`, `mmixasm`, and `mmixdb`
executables come from `cargo install checksmix`, not from a `[dependencies]` entry.

### Capturing what a program emits

By default an `MMix` writes to the process's stdout and stderr. Implement
`Host` to intercept that instead — what the program writes, the clock behind
the `Time` trap, diagnostics, and every recognized `TRAP`:

```rust
use checksmix::{Host, MMix};
use std::cell::RefCell;
use std::rc::Rc;

struct Capture(Rc<RefCell<Vec<u8>>>);

impl Host for Capture {
    fn write(&mut self, _fd: u8, bytes: &[u8]) -> std::io::Result<()> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(())
    }
    fn now_micros(&mut self) -> u64 { 0 }
    fn diagnostic(&mut self, _msg: &str) {}
}

let out = Rc::new(RefCell::new(Vec::new()));
let mut mmix = MMix::with_host(Capture(out.clone()));
```

Clone the buffer handle *before* moving the host in — `with_host` consumes it
and hands back no way to reach it again.

`Debugger::load_with_host` takes a host the same way, for programs you want to
step through rather than run straight out. `Debugger::load` installs `StdHost`,
so a debugged program's output goes to the process and never reaches you.

An `MMix` holds its host as `Box<dyn Host>` and so is none of `Send`, `Sync`,
`UnwindSafe`, or `RefUnwindSafe`; a `Debugger` holds an `MMix` and inherits that — a change from 0.2.23, where it was all
four. Construct one on the thread that runs it, and wrap it in
`std::panic::AssertUnwindSafe` to put it through `catch_unwind`.

## Learning MMIX

MMIX is Knuth's own "pretty clean" machine architecture, and he documented it himself:

- [Knuth's MMIX page](https://www-cs-faculty.stanford.edu/~knuth/mmix.html) — the
  canonical home: design rationale, current news, and why MIX was retired.
- [*MMIXware: A RISC Computer for the Third Millennium*](https://www-cs-faculty.stanford.edu/~knuth/mmixware.html)
  — the complete specification, with Knuth's own assembler and simulator
  (Springer LNCS 1750, 1999).
- [*The Art of Computer Programming*](https://www-cs-faculty.stanford.edu/~knuth/taocp.html),
  Volume 1 Fascicle 1, *MMIX: A RISC Computer for the New Millennium* (2005) — the
  instruction set as Knuth teaches it, and the shortest path in.
- [The MMIX Home Page](http://mmix.cs.hm.edu/) — Martin Ruckert's collection at Munich
  University of Applied Sciences: documentation, sources, binaries, worked examples,
  and *The MMIX Supplement*.

`checksmix` follows the same instruction set, so a program written from any of these
runs here — and in [playmmix](https://playmmix.2ad.com) without installing anything.

## Tribute

Donald Knuth has been one of the formative influences in my career. Early on—as a junior developer just beginning to feel like a mid-level engineer—I implemented his external, file-based merge sort to collate insurance datasets that were far too large for memory. That experience taught me alot about how to think about programming and system design.

Knuth’s blend of rigor, playfulness, and generosity has shaped how I write code and how I view the craft of software.  Some time later, I submitted a “bug” in The Art of Computer Programming- to earn the coveted Knuth “hexadecimal dollar.” His reply was short and perfect:

“e is as real as any other number.”

Evidently!

This project carries a little of that spirit forward: curiosity, precision, and the belief that programming can be serious fun.
