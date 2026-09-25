# `checksmix`

An assembler, emulator and source-level debugger for Knuth's MMIX, the 64-bit
RISC machine of *The Art of Computer Programming*. Write MMIXAL, run it, read
machine state.

[![crates.io](https://img.shields.io/crates/v/checksmix)](https://crates.io/crates/checksmix)
[![docs.rs](https://img.shields.io/docsrs/checksmix)](https://docs.rs/checksmix)
[![CI/CD Pipeline](https://github.com/jac18281828/checksmix/actions/workflows/ci-cd.yml/badge.svg)](https://github.com/jac18281828/checksmix/actions/workflows/ci-cd.yml)

## Install

```bash
cargo install checksmix
```

This installs `checksmix`, `mmixasm` and `mmixdb`. It needs a stable Rust
toolchain ([rustup.rs](https://rustup.rs)).

To build from source:

```bash
git clone https://github.com/jac18281828/checksmix.git
cd checksmix
cargo build --release          # binaries land in target/release/
```

## Usage

- `checksmix` runs `.mms` source or a `.mmo` object file.
  `checksmix check` assembles without running; `checksmix build` writes a `.mmo`.
- `mmixasm` assembles `.mms` to `.mmo` and lists the symbols, labels and code it produced.
- `mmixdb` steps through `.mms` source with breakpoints, register and memory
  inspection, and Emacs GUD support.

`checksmix` has 256 general-purpose registers, 32 special registers, a sparse 64-bit
address space and the MMIXAL reference's TRAP file I/O.

Assemble once and run the object file:

```bash
checksmix build examples/prime.mms -o prime.mmo
checksmix prime.mmo
```

Set `RUST_LOG=checksmix=debug` to trace instruction decoding and TRAP handling.

## Examples

- [`leapyear.mms`](examples/leapyear.mms): the perpetual leap year below.
- [`hello_halt.mms`](examples/hello_halt.mms): Hello, Halt: the smallest program that runs.
- [`exit_code.mms`](examples/exit_code.mms): return a value to the shell.
- [`hello_world.mms`](examples/hello_world.mms): print a string to standard output.
- [`subroutine.mms`](examples/subroutine.mms): call a subroutine and return its result.
- [`fibonacci.mms`](examples/fibonacci.mms): return fib(20) as the exit code.
- [`big_fib.mms`](examples/big_fib.mms): compute fib(100) in multi-precision arithmetic.
- [`prime.mms`](examples/prime.mms): test a number for primality and print the verdict.
- [`linked_list.mms`](examples/linked_list.mms): walk a linked list and sum its nodes.
- [`time.mms`](examples/time.mms): read the host clock.
- [`all_instructions_test.mms`](examples/all_instructions_test.mms): run every mnemonic as a regression suite.

### Perpetual leap year

The Gregorian rule, which holds for any year: 4 divides it, unless 100 does,
unless 400 does too.

#### Listing — `leapyear.mms`

```mmix
% leapyear.mms -- the first leap year after Year, by the Gregorian rule.

Year    IS      2026

        LOC     Data_Segment
        GREG    @
Digits  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
End     BYTE    0
After   BYTE    "The first leap year after ",0
Is      BYTE    " is ",0
Newline BYTE    10,0

        LOC     #100
Main    SET     $1,Year
        ADDU    $2,$1,1                 % the candidate year
        SET     $6,400
        SET     $7,100
1H      DIVU    $3,$2,$6
        GET     $3,rR
        BZ      $3,Found                % every 400th year is leap
        DIVU    $3,$2,$7
        GET     $3,rR
        BZ      $3,2F                   % any other century is not
        AND     $3,$2,3
        BZ      $3,Found                % otherwise every 4th year is
2H      ADDU    $2,$2,1
        JMP     1B

Found   LDA     $255,After
        TRAP    0,Fputs,StdOut
        SET     $5,$1
        PUSHJ   $4,PrintNum
        LDA     $255,Is
        TRAP    0,Fputs,StdOut
        SET     $5,$2
        PUSHJ   $4,PrintNum
        LDA     $255,Newline
        TRAP    0,Fputs,StdOut
        SET     $255,0
        TRAP    0,Halt,0

% PrintNum: write $0 in decimal, filling Digits from the right.
PrintNum LDA    $1,End
        SET     $2,10
1H      DIVU    $0,$0,$2
        GET     $3,rR
        ADDU    $3,$3,'0'
        SUBU    $1,$1,1
        STBU    $3,$1,0
        PBNZ    $0,1B
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0
```

#### Run

```console
$ checksmix leapyear.mms
=== MMIX Assembler ===
=== Parsing assembly from: leapyear.mms ===

Assembly parsed successfully

=== Initial Machine State ===
MMIX Computer State:
  PC = 0x0000000000000100

General Registers:
  $254 = 0x2000000000000000 (2305843009213693952)
  $255 = 0x0000000000000100 (256)

Special Registers:
  rN   = 0x00000000000007d9 (2009)
  rO   = 0x6000000000000000 (6917529027641081856)
  rS   = 0x6000000000000000 (6917529027641081856)
  rT   = 0x8000000500000000 (-9223372015379939328)
  rTT  = 0x8000000600000000 (-9223372011084972032)
  rK   = 0xffffffffffffffff (-1)
  rV   = 0x369c200400000000 (3935055375966928896)
  rG   = 0x00000000000000fe (254)

Memory: 169 bytes used


=== Executing Program ===
The first leap year after 2026 is 2028
HALT trap at PC=0x0000000000000188, exit code=0
Execution stopped at PC=0x000000000000018c after 106 instructions

Executed 106 instructions

=== Final Machine State ===
MMIX Computer State:
  PC = 0x000000000000018c

General Registers:
  $1   = 0x00000000000007ea (2026)
  $2   = 0x00000000000007ec (2028)
  $254 = 0x2000000000000000 (2305843009213693952)

Special Registers:
  rJ   = 0x0000000000000170 (368)
  rR   = 0x0000000000000002 (2)
  rN   = 0x00000000000007d9 (2009)
  rO   = 0x6000000000000000 (6917529027641081856)
  rS   = 0x6000000000000000 (6917529027641081856)
  rT   = 0x8000000500000000 (-9223372015379939328)
  rTT  = 0x8000000600000000 (-9223372011084972032)
  rK   = 0xffffffffffffffff (-1)
  rV   = 0x369c200400000000 (3935055375966928896)
  rG   = 0x00000000000000fe (254)
  rL   = 0x0000000000000004 (4)

Memory: 178 bytes used


Execution completed.
```

With `Year IS 2099`, the century rule skips 2100: `2104`.

More MMIX, less installation: [playmmix](https://playmmix.2ad.com), the
browser playground built on `checksmix`.

## mmixdb

Step through `.mms` source, set breakpoints and inspect registers and memory, with
the current source line shown as you go.

```bash
mmixdb examples/fibonacci.mms
mmixdb --fullname examples/fibonacci.mms   # Emacs GUD marker mode
```

`mmixdb` debugs `.mms` source; a `.mmo` carries no source map.
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
| print | `p <arg>`, `print <arg>`, `p/f <arg>`, `p/x <arg>` | Print a register (`$N`/`N`), special register (`rJ`, `rA`, ...), label address, IS/GREG symbol, or the memory octa at a hex address's aligned 8-byte base (`0x...`/`#...`). `/f` and `/x`, attached or detached (`p /f <arg>`), print the same octabyte as an IEEE double or in hex instead. |
| set | `set <target> <value>` | Write a register (`$N`/`N`), special register, or the memory octa at a hex address's aligned 8-byte base. A symbol whose type is a register alias (from `GREG` or a register-valued `IS`, e.g. `Sp`) is settable the same way; a label or a symbol whose type is a constant (an `IS` bound to a non-register value) is not; neither names a storage location. `value` is decimal or `0x`/`#`-prefixed hex. |
| state | `bt`, `backtrace`, `info reg`, `info registers` | Print the full register dump. |
| breakpoints | `info break`, `info breakpoints` | List every currently-set breakpoint with its source location. |
| list | `l`, `list` | Print source lines around the current PC. |
| help | `h`, `help`, `?` | Show this help. |
| quit | `q`, `quit`, `exit` | Exit the debugger. |

Blank input repeats the last command; most debugging is stepping. Once the
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

`contrib/mmix-mode.el` is a major mode for `.mms` source written to the dialect
`checksmix` assembles: `%` comments, a label field only on an unindented line,
the explicit immediate mnemonics and `checksmix`'s extensions. It highlights and indents,
colours each use of a label or `IS`/`GREG` name the file defines, shows
the current line's instruction through eldoc, describes any instruction with
`C-c C-d`, and runs the file with `C-c C-c` (`checksmix run`). The instruction
reference is built in, so the file stands alone: copy it into a directory on
`load-path` and require it, with Emacs 29.1 or later:

```elisp
(require 'mmix-mode)
```

Its tests check the mode against the assembler's grammar, so run them from the
repository root:

```sh
emacs --batch -L contrib -l contrib/mmix-mode-test.el -f ert-run-tests-batch-and-exit
```

## Using `checksmix` as a library
`checksmix` is usable as a library, independent of the three binaries above. The
`clap`, `rustyline` and `tracing-subscriber` dependencies the CLIs need live behind
the `cli` feature, which is on by default. A library-only consumer, notably one
targeting `wasm32-unknown-unknown` where `rustyline` does not build, turns it off.
[playmmix](https://playmmix.2ad.com) is built this way, on `checksmix = { version = "0.3",
default-features = false }`, and runs the emulator in the browser as wasm.

```toml
# default — library plus the CLI dependency tree
checksmix = "0.3"

# library only
checksmix = { version = "0.3", default-features = false }
```

Either form gives you the library. The `checksmix`, `mmixasm` and `mmixdb`
executables come from `cargo install checksmix`, not from a `[dependencies]` entry.

### Capturing what a program emits

By default an `MMix` writes to the process's stdout and stderr. Implement
`Host` to intercept what the program writes, the clock behind the `Time` trap,
diagnostics and every recognized `TRAP`:

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

Clone the buffer handle *before* moving the host in; `with_host` consumes it
and hands back no way to reach it again.

`Debugger::load_with_host` takes a host the same way, for programs you want to
step through rather than run straight out. `Debugger::load` installs `StdHost`,
so a debugged program's output goes to the process and never reaches you.

An `MMix` holds its host as `Box<dyn Host>` and so is none of `Send`, `Sync`,
`UnwindSafe` or `RefUnwindSafe`; a `Debugger` holds an `MMix` and inherits that.
Construct one on the thread that runs it, and wrap it in
`std::panic::AssertUnwindSafe` to put it through `catch_unwind`.

## Learning MMIX

MMIX is Knuth's "pretty clean" machine architecture, and he documented it himself:

- [Knuth's MMIX page](https://www-cs-faculty.stanford.edu/~knuth/mmix.html): the
  canonical home: design rationale, current news, and why MIX was retired.
- [*The Art of Computer Programming*](https://www-cs-faculty.stanford.edu/~knuth/taocp.html),
  Volume 1 Fascicle 1, *MMIX: A RISC Computer for the New Millennium* (2005): the
  instruction set as Knuth teaches it, and the shortest path in.

`checksmix` follows the same instruction set, so a program written from any of these
runs here.

## Related projects

- [The MMIX Home Page](http://mmix.cs.hm.edu/): Martin Ruckert's collection at Munich
  University of Applied Sciences: documentation, sources, binaries, worked examples,
  and *The MMIX Supplement*.
- [Instruction Reference](https://mmix.cs.hm.edu/doc/instructions/): the Home Page's
  per-instruction reference.
- [*MMIXware: A RISC Computer for the Third Millennium*](https://www-cs-faculty.stanford.edu/~knuth/mmixware.html):
  the full definition of MMIX, with an assembler and simulator (Springer LNCS 1750, 1999).

## Legacy MIX

`checksmix` began as a MIX emulator. `.mix` and `.mixal` files still run through
`checksmix` ([`example.mix`](examples/example.mix) is one), but MMIX is the target
and new work belongs in `.mms`.

## Contribute

Bug reports and pull requests are welcome; see
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## Tribute

Donald Knuth's art and craft inspire my work. I once reported a "bug" in *The Art
of Computer Programming*, but I never earned my hexadecimal dollar.

This project carries a little of his spirit forward: curiosity, precision and the
belief that programming can be serious fun.

> "*e* is as real as any other number."\
> — Donald E. Knuth
