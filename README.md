# ChecksMix - Blazing Fast MMIX Emulator

MMIX assembler and emulator with fast feedback for learning, experimenting, and debugging Knuth’s 64-bit machine. MIX source still parses and emulates, but the checksmix now focuses on MMIX with `.mms` and `.mmo` workflows.

## What’s inside
- `checksmix`: execute `.mms` assembly directly or run prebuilt `.mmo` object files.
- `mmixasm`: assemble `.mms` to `.mmo` for reuse or distribution.
- `mmixdb`: gdb-style interactive debugger for `.mms` programs (step, breakpoints, print, Emacs GUD mode).
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

- `examples/hello_world.mms`: prints a string via `TRAP 0,Fputs,StdOut`.
- `examples/linked_list.mms`: walks a statically allocated list and sums node values.
- `examples/all_instructions_test.mms`: broad instruction coverage for regression checks.

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

`mmixdb` is a gdb-style debugger for MMIX `.mms` programs: step, breakpoint,
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
| step (into) | `s`, `step` | Execute exactly one instruction, following into calls/branches. |
| next (over) | `n`, `next` | Execute one instruction; if it entered a call, keep stepping until it returns. |
| continue | `c`, `continue` | Resume, single-stepping until a breakpoint or halt. |
| run/reset | `r`, `run` | Reset to the freshly-loaded image, then behave like `continue`. |
| break | `b <line>`, `b <label>`, `break …` | Set a breakpoint at a source line or label. |
| print | `p <arg>`, `print <arg>` | Print a register (`$N`/`N`), special register (`rJ`, `rA`, ...), label address, IS/GREG symbol, or memory octa (`0x...`/`#...`). |
| state | `bt`, `backtrace`, `info reg`, `info registers` | Print the full register dump. |
| list | `l`, `list` | Print source lines around the current PC. |
| help | `h`, `help`, `?` | Show this help. |
| quit | `q`, `quit`, `exit` | Exit the debugger. |

Blank input repeats the last command -- most debugging is stepping.

Emacs users: see `contrib/mmixdb.el` for `M-x mmixdb` under `gud-mode`.

## Legacy MIX support
`.mix` and `.mixal` files still run through `checksmix`, but MMIX is the primary target. Prefer `.mms`/`.mmo` for new work.

## Tribute

Donald Knuth has been one of the formative influences in my career. Early on—as a junior developer just beginning to feel like a mid-level engineer—I implemented his external, file-based merge sort to collate insurance datasets that were far too large for memory. That experience taught me alot about how to think about programming and system design.

Knuth’s blend of rigor, playfulness, and generosity has shaped how I write code and how I view the craft of software.  Some time later, I submitted a “bug” in The Art of Computer Programming- to earn the coveted Knuth “hexadecimal dollar.” His reply was short and perfect:

“e is as real as any other number.”

Evidently!

This project carries a little of that spirit forward: curiosity, precision, and the belief that programming can be serious fun.
