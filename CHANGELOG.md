0.2.20 (2026-04-26)

* `checksmix` now supports three subcommands: `run` (default, preserves all existing behaviour), `check` (parse + dry-encode one or more `.mms` files; silent on success, `<file>:<line>:<col>: …` on failure, exit 1), and `build` (assemble to `.mmo`, prints only the output path, no verbose debug dump)
* Bare invocation without a subcommand (`checksmix file.mms`) continues to work identically; `--unsigned` remains on `run` only
* `build -o OUT.mmo` / `build --output OUT.mmo` sets the output path; default is the first input's basename with `.mmo`
* Shared `assemble_sources` helper unifies file-reading, `MMixAssembler::new` + `add_source` + `parse` across all three source-touching paths
* CLI parse tests cover subcommand routing, flag scoping (`--unsigned` rejected on `check`/`build`, `-o` rejected on `run`), and multi-file operands
* Integration tests cover: clean two-file `check` (exit 0, no stdout), undefined-symbol `check` (exit 1, `file:line:col` in stderr), duplicate-`:Global` `check` (both filenames in error), `build` round-trip (`.mmo` produced, `run` of result succeeds), bare `.mmo` run regression, and multi-source `run`

0.2.18 (2026-04-26)

* Multi-source assembly: `checksmix a.mms b.mms ...` and `mmixasm a.mms b.mms ... -o out.mmo` load all inputs into one shared symbol space and one byte stream, as if the files were concatenated; symbols, GREG state, and `current_addr` carry across files in command-line order
* Grammar: identifiers may carry a leading `:` marking a linkage-visible (global) symbol; `:Foo` and `Foo` are distinct names, both as label definitions and as operand references
* New `PREFIX` directive: `PREFIX P_` qualifies subsequent unqualified names as `P_<name>`; names starting with `:` opt out and are stored verbatim. PREFIX state persists across files in the same run and resets at the start of each pass
* Predefined symbols (TRAP codes, special registers, segment constants) are reachable as both `Halt` and `:Halt` so they remain available from inside any `PREFIX` region
* Duplicate-symbol detection: redefining `Main`, a global `:Foo`, an IS-bound symbol, or a GREG label now reports `<file>:<line>: symbol '<name>' redefined (first defined at <prev-file>:<prev-line>)`; predefined symbols may still be shadowed by user code
* `mmixasm` accepts one or more inputs; `-o/--output` flag explicit (output defaults to first input's basename with `.mmo`)
* `checksmix` dispatches by extension when given a single input and assembles multiple `.mms` inputs together when given more than one (mixing extensions is an error)

0.2.17 (2026-04-26)

* Fputs/Fputc/Fputws now route to any open file descriptor (previously fds returned by Fopen were silently dropped while $255 still reported a successful byte count); on write failure or unknown fd they return -1
* Output traps emit raw bytes — bytes ≥ 0x80 are no longer widened via `byte as char` into UTF-8 sequences, so a write of 0xFF produces one byte instead of `0xC3 0xBF`
* Halt flushes stdout before returning so buffered Fputs/Fputc output is not discarded when the runner calls `process::exit`
* Fputs/Fputws walk the source string with `wrapping_add` so a string address near `u64::MAX` cannot panic
* Trap-code doc comments from Fclose onward were off-by-one against the `TrapCode` enum and have been corrected
* Existing Fputs/Fputc unit tests used the wrong opcode (Fwrite/Fputs) and stored the string address in $0; corrected, and new tests cover Fputs/Fputc/Fputws to a real Fopen'd fd, raw high-byte output, and -1 returns on unknown fds

0.2.16 (2026-04-25)

* FADD, FSUB, FMUL, FDIV, FSQRT now honor all four rA rounding modes (NEAR / OFF / UP / DOWN) — previously every result used hardware round-to-nearest-even regardless of mode
* Direction is detected via 2Sum (add/sub) and FMA-residual (mul/div/sqrt), giving exact rounding without dropping to a softfloat crate
* Inexact (X) flag now correctly raised on inexact FADD/FSUB/FMUL/FDIV/FSQRT
* Overflow under directed modes clamps to ±MAX (ROUND_OFF, ROUND_UP/DOWN against the wrong infinity sign) instead of always producing ±∞
* Signaling NaN inputs raise rA.I and propagate as quiet NaN; quiet NaN inputs remain silent (per IEEE 754)
* rA.D is now raised only for denormalized **operands** — subnormal results are reported by U as the spec intends
* AGENTS.md release section now points at the actual workflow (`deploy-crate`) and the actual single root `Cargo.toml`
* New unit and `.mms` smoke tests cover sNaN, all four rounding modes on arithmetic, inexact detection, D-flag scope, and overflow clamping

0.2.15 (2026-04-25)

* portable raw-fd / raw-handle setup for Unix and Windows hosts
* AGENTS.md release process and cleanup
* FCMPE/FUNE/FEQLE now have parser, encoder, and grammar entries — the executor paths existed before but the assembler could not reach them
* Floating-point ops raise rA event flags (I invalid, Z divide-by-zero, O overflow, U underflow, X inexact, D denormalized, W float-to-fix overflow)
* FREM uses IEEE 754 round-half-to-even remainder, replacing Rust's truncated `%`
* FIX, FIXU, SFLOT, SFLOTU, STSF, STSFI honor the rA rounding mode and report inexact/overflow/underflow
* FINT rounding-mode codes 1 and 3 now match MMIXware (1 = ROUND_OFF / toward zero, 3 = ROUND_DOWN / toward −∞)
* Floating-point unit tests cover NaN, infinity, denormals, zero divide, all four rounding modes, and assembler emission of the new opcodes
* `examples/all_instructions_test.mms` exercises FCMPE/FUNE/FEQLE end-to-end via the smoke test

0.2.14 (2026-01-01)

* add register symbols

0.2.13 (2026-01-01)

* Add trap for Time - seconds/millis/micros since epoch

0.2.12 (2026-01-01)

* bug fixes - print $255
* halt with error code properly

0.2.11 (2025-12-31)

* implementing the traps

0.2.10 (2025-12-27)

* bug fixing, pop instruction

0.2.8 (2025-12-27)

* print signed values

0.2.7 (2025-12-27)

* negative constants

0.2.6 (2025-12-26)

* byte literals

0.2.5 (2025-12-26)

* Fixed: PUSHJ/POP now correctly restore caller's rJ register in nested function calls
* Fixed: rG (global threshold register) now defaults to 32 per MMIX specification
* This enables proper execution of programs with nested subroutines and return values

0.2.4 (2025-12-25)

* support for pushj/pop

0.2.1 (2025-12-17)

* fix deployment

0.2.0 (2025-12-17)

* full mmix implementation
* massive refactor
* works - so many improvements

0.1.0 (2025-11-19)

* initial working version
