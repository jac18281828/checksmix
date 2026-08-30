0.3.6 (2026-08-23)

* **Breaking: `rA`'s event flags are Knuth's bits.** His predefined symbols (MMIXAL §69) are `D_BIT=#80`, `V_BIT=#40`, `W_BIT=#20`, `I_BIT=#10`, `O_BIT=#08`, `U_BIT=#04`, `Z_BIT=#02`, `X_BIT=#01`; checksmix had six of the eight in the wrong place, defined no `V` at all, and read `D` as "denormalized operand" — a class MMIX's `rA` does not have. A program that reads `rA` sees different bits: the inexact flag moves from `#04` to `#01`, invalid operation from `#08` to `#10`, and `#04` now means floating underflow. `MMIX.md`'s event-flag table documented the old layout and is corrected. There is no denormalized-operand event any more; a subnormal operand raises nothing, and an underflow to a subnormal result raises `U` as it always did
* **Breaking: the rounding mode lives in `rA` bits 17–16, not the low two bits.** `rA` is 18 bits wide — which is why Knuth caps a write to it at `#3FFFF` — and the mode field sits at its top. Under the old layout bits 0–1 held an event flag, so raising a float-to-fix overflow silently switched the rounding mode. `PUTI rA,2` no longer selects `ROUND_UP`: `PUTI`'s `YZ` operand is 16 bits and cannot reach the field, so a mode change now needs the register form of `PUT`. `examples/all_instructions_test.mms` Test 190 was written the old way and now builds `#20000` and writes it with `PUT rA,$X`
* **Breaking: `PUT rA,$X` rejects a value above `#3FFFF`.** Knuth defines `rA` as 18 bits and treats a wider write as an error. `PUT` here has no fault channel, so the write is dropped and `rA` keeps its previous value while execution continues; the general user-mode `PUT` restrictions on the other special registers are unchanged and still unenforced
* **Breaking: `DIV` and `DIVI` floor.** Knuth defines `s($X) ← ⌊s($Y)/s($Z)⌋` and `s(rR) ← s($Y) mod s($Z)`, so the remainder takes the divisor's sign; Rust's `/` and `%` truncate toward zero, and every mixed-sign division with a nonzero remainder was wrong. `DIV $3,-7,2` gave quotient −3 remainder −1 and now gives −4 remainder 1
* `DIV $X,#8000000000000000,-1` returns `#8000000000000000` and raises `V` instead of aborting the process. Rust panics on `i64::MIN / -1`, so any MMIX program could kill the emulator with two instructions
* Signed division by zero raises `D`, the integer divide check. `$X ← 0` and `rR ← $Y` as before
* **Breaking: `DIVU` and `DIVUI` honor the `rD` rule.** Knuth defines the quotient only when `u($Z) > u(rD)`; otherwise `$X ← rD` and `rR ← $Y`, because a wider quotient would not fit an octabyte. With `rD` = 5, `$Y` = 0 and `$Z` = 3 the old code divided the full 128-bit dividend and produced `#AAAA...AAAA`; it now yields 5. A zero divisor is that same case and is not an event — the `rD` rule is a definition, so `DIVU` has no divide check and only signed division sets `D`
* **Breaking: `MUL` and `MULI` leave `rH` alone.** Knuth defines `rH` as `MULU`'s output; a signed multiply had been clobbering it, so a `MULU` result read after any intervening `MUL` was wrong
* Integer overflow raises `V`. Ten sites — `ADD`, `SUB`, `MUL`, `SL` and their immediate spellings — raised a bare `#04`, which is Knuth's floating *underflow* bit
* `NEG` and `NEGI` raise `V` on overflow, which they detected and then discarded. `NEG $X,Y,$Z` computes `Y - s($Z)` with `Y` an immediate in `0..255`, so this is not only `NEG $X,0,$Z` with `$Z` at `#8000000000000000`. `NEGU` and `NEGUI` have no overflow and are unchanged
* `SL` and `SLI` overflow exactly when `s($Y)·2^u($Z)` leaves the signed-octabyte range. The old test compared masked high bits and reported an overflow for `SL $X,$Y,$Z` with `$Y` = 2^60 and `$Z` = 2, whose result 2^62 fits. Testing only the bits shifted out is also wrong in the other direction: `$Y` = `#4000000000000000` shifted by 1 shifts out a zero matching the sign, yet the result 2^63 overflows. `SLU` and `SLUI` have no overflow — Knuth defines them modulo 2^64 — and are unchanged
* `STB`, `STW`, `STT` and their immediate spellings raise `V` when the stored value does not fit the destination width signed. All six range checks were present with empty bodies. The stored bytes are unchanged
* `MMIX.md` no longer claims that `FSQRT`, `FINT` and the conversion instructions take a rounding-mode override in the `Y` field, or that `FLOT`, `FLOTI`, `FLOTU` and `FLOTUI` honor the rounding mode. Neither is true of the implementation, which ignores `Y` everywhere and always rounds `FLOT` to nearest; both are now recorded as known gaps
* **Partially reverses `684f71d`.** That commit added "the other 48 bits are preserved" to all twelve `INC*`/`OR*`/`ANDN*` rows. It was right about `OR*` and `ANDN*` — bitwise operations propagate nothing — and wrong about three of the four `INC*`: `INCMH`, `INCML` and `INCL` in `src/mmix.rs` are a `wrapping_add` of a shifted wyde into the whole 64-bit register, so a carry out of the target wyde propagates into the wydes above it. Only `INCH` is exempt, because its wyde is already the top one and a carry there leaves the register instead of propagating. The `0.3.5` entry carries the same false claim in its first bullet; that release is tagged, so it stands uncorrected and this entry supersedes it
* `SWYM` — the only `Opcode` variant with no row in `MMIX.md`'s instruction table — is documented, and the duplicate `LDVTS`/`LDVTSI` pair in the system group is gone. The memory-group pair, next to the other `LD*` instructions, is the one that stays
* The `GETA` prose block described its field as "a signed 16-bit quotient" within "±131068 bytes". `relative_field` in `src/mmixal.rs` treats it as an unsigned tetra count with independent forward and backward arms, and a backward target given to `GETA` silently selects the `GETAB` encoding rather than erroring. The block now states the real reach in each direction, confirmed by assembling all four boundaries: 0 to 262140 bytes forward, up to 262144 bytes backward — the two do not mirror, because a forward delta of zero tetras consumes the field's first value
* The header sentence labeled `#`-prefixed hex "octal". Hex and octal are now named separately, the header notes that `0`-prefixed octal is a checksmix extension — Knuth reads a leading `0` as decimal — and that `0x`/`0X`-prefixed hex is also a checksmix extension
* `WYDE`, `TETRA` and `OCTA` are documented as taking one operand, matching the grammar; only `BYTE` takes a list. Marked a known gap against Knuth, whose MMIXAL takes a list for all four
* `JE`, `JNE`, `JL`, `JG` and `HALT` — five spellings that assemble but appeared nowhere in the docs — are documented as checksmix extensions with no Knuth counterpart, matching the existing `SETI` row's convention
* The two-operand `LDA $X,Label` form (and `LDAI`) is documented, including its two branches: a byte-sized address assembles to one tetra, a larger one expands to a four-tetra `SET`-shaped sequence. The narrow branch's latent defect is flagged, not fixed: `LDA`'s narrow form emits the register-register `ADDU` opcode with the address in the Z *register* field, rather than the register-immediate `ADDUI` its `LDAI` sibling correctly uses, so the assembled instruction adds whatever register the address names rather than the literal value
* New `tests/docs_consistency.rs::every_opcode_appears_once_in_the_instruction_table` parses every `Opcode` variant out of `src/mmixal.rs` and every Mnemonic-column entry out of `MMIX.md`'s instruction table, and fails if an opcode is missing or listed more than once. It does not require the reverse — MMIXAL aliases and checksmix extensions may legitimately name no `Opcode` variant
* **Breaking: memory access is aligned, not raw.** Knuth's memory operators round the address down to a multiple of the access width before reading or writing — `M_w[A] = M_w[w·⌊A/w⌋]` — so `LDO $X,$Y,$Z` with an address ending in 3 loads the octabyte at the aligned base, not eight bytes straddling two octabytes. `read_wyde`/`write_wyde`, `read_tetra`/`write_tetra` and `read_octa`/`write_octa` used the raw address instead; all six now mask their base by width (`& !1`, `& !3`, `& !7`), reads and writes alike, so a program that relied on unaligned access computes differently. `read_byte`/`write_byte` are unchanged — a byte is its own alignment — and instruction fetch, which already read through `read_tetra`, is aligned for free. `mmixdb`'s `print` inherits the same rule and this is user-visible: `print 0x1003` now shows the octabyte at `0x1000`, because the machine cannot read an unaligned octabyte and the old answer described nothing real
* **Breaking: `CSWAP` and `CSWAPI` write `rP` on a failed compare.** Knuth: on a mismatch, `rP ← M8[$Y+$Z]` (or `$Y+Z` for `CSWAPI`) and `$X ← 0`; checksmix set `$X ← 0` and left `rP` untouched, so a compare-and-swap retry loop spun forever reading the same stale `rP`. Both arms now write `rP` from the octabyte already read for the comparison — no second memory read — and the success path is unchanged
* Loading a `.mmo` no longer prints a `Debug: instr@0x370` line ahead of the load report. The load path carried a hardcoded read of address `0x370`, left from tracing a `big_fib` failure, and it fired on every object file whatever the program
* **Breaking: the assembler aligns the location counter.** Knuth's MMIXAL rounds the counter up to an item's natural width before assembling it — 4 for an instruction, 2, 4 or 8 for `WYDE`, `TETRA` and `OCTA` — and the label on that line takes the rounded address; checksmix advanced the counter by the item's raw size and nothing else, in both passes. Any source that mixes `BYTE` data with wider data or with code now assembles to different addresses. Alignment comes from the item's kind, never from the bytes it emits: `BYTE` is never aligned however long its operand list, so `BYTE "abcd"` still lands wherever the counter stands, and a pseudo-instruction that expands to four tetras still aligns to 4. The skipped bytes are a gap rather than emitted padding and load as zero. `LOC` is unchanged — it sets the counter exactly, and the next instruction or wide datum rounds up from there. This closes the hole the aligned-memory-access change above opened: an `OCTA` the assembler had placed at `4 mod 8` was unreachable, because an aligned load rounds down to the octabyte below it. `examples/all_instructions_test.mms` needed a hand-written `TETRA 0` pad to realign its data section; the pad is gone and every address in the file is unchanged
* **A `debug` directive no longer depends on the length of its message.** `preprocess_debug` appends one subroutine per directive, each ending in a `BYTE` string of the user's text, and the next subroutine's `SAVE` followed that string directly — so a source with two `debug` directives assembled only when the first message's length was `0 mod 4`, and otherwise failed outright with `PUSHJ target ... is not 4-byte aligned`. Which text the user typed decided whether the program built. Instruction alignment puts the following `SAVE` on a tetra boundary whatever the message says
* **Sharp edge: a label alone on its own line stays unrounded.** Rounding happens when an item is assembled, not when a label is defined, so a bare label line followed by `OCTA` names an address up to seven bytes below the octabyte — and since memory access rounds down, a load through that label reads the octabyte *before* the datum. This is where Knuth puts alignment, and the alternative cannot be computed in one pass because the following item is not yet known. Attach the label to the directive (`Data OCTA 0`) rather than putting it on the line above

0.3.5 (2026-08-22)

* **Breaking: `SETH`, `SETMH`, `SETML` and `SETL` set the whole register.** Knuth defines each to place its wyde and zero the other 48 bits; checksmix implemented three of the four as merges, so `SETMH $X,#ABCD` on a register holding all ones left `#FFFFABCDFFFFFFFF` where MMIX gives `#0000ABCD00000000`, and `SETL $X,0` — Knuth's one-instruction register clear — cleared only the low wyde. `SETH` alone was right, so the file disagreed with itself. Encodings are unchanged and a `.mmo` still loads, but a program that reached the merge behavior now computes differently. The `INC*`, `OR*` and `ANDN*` wyde operations preserve the other wydes as they always did, which is their defined behavior. No test caught this: the only coverage ran through `SETI`, whose expansion writes all four wydes and therefore cannot observe a merge
* `SETI`'s expansion is now `SETH` + `INCMH` + `INCML` + `INCL`, still four tetras and still a full 64-bit immediate. The old chain of four `SET*` assembled a constant only under merge semantics; leading with `SETH` clears the register, so the sequence lands the same value whatever the register held. `examples/all_instructions_test.mms` Test 2 was that bug written as a test — it chained the four `SET*` into one register and expected the concatenation — and now builds its constant with `SETH` followed by the increments
* `SET $X,imm` assembles, and `SET` is now exactly MMIXAL's alias operation. Knuth specifies two forms: `SET $X,$Y` is `OR $X,$Y,0`, and `SET $X,Y` for a non-register `Y` is `SETL $X,Y`. Both are one tetra, so the immediate form carries 16 bits and an operand above `#FFFF` is an error naming `SETI`; a negative literal wraps into that field, the way `ADDI $1,$2,-1` wraps into its 8-bit one. `SETI` is a checksmix extension with no Knuth counterpart, kept for wide constants — it clears the register and establishes a full 64-bit value in one mnemonic, at four tetras. The two are deliberately different instructions rather than aliases, and the 654 existing `SETI` uses under `examples/` are unchanged
* The sixteen base load and store mnemonics accept an immediate third operand and emit the `*I` variant, joining the six families that already did. `LDO $1,$2,0` and `STO $1,$2,0` now assemble to what `LDOI` and `STOI` produce, `Z` remaining the 8-bit field the encoding gives it. MMIXAL never writes the `I` suffix — one never writes `ADDI` or `JMPB` in source — so a program typed out of Fascicle 1 no longer needs one. The `*I` spellings continue to work unchanged
* **Breaking: `LDA $X,$Y,Z` adds the literal `Z` rather than the contents of register `$Z`.** Knuth defines `LDA` as an alias of `ADDU` in both operand forms — `LDA $X,$Y,$Z` is `ADDU $X,$Y,$Z`, and `LDA $X,$Y,Z` is `ADDU $X,$Y,Z` — and the encoder always matched his opcode table, emitting `#22` for `LDA` and `#23` for `LDAI`. Only the grammar reached the wrong one: the all-register spelling was rejected outright, and the three-operand form that did parse routed to the register opcode, so `LDA $1,$2,12` added whatever `$12` held at execute rather than 12. Source written out of Fascicle 1 now means what it says; source written against the old reading computes differently. The two-operand `LDA $X,Label` and every `LDAI` spelling are unaffected. One consequence is an improvement rather than a regression: `Big IS #300` followed by `LDA $1,$2,Big` used to truncate `#300` to register `$0` in silence and is now an out-of-range error. Any `Z` above `#FF` is rejected the same way, a negative literal included — `LDA $1,$2,-1` assembled to `22 01 02 FF` before and is now an error, so source relying on a wrapped or truncated `Z` fails to assemble rather than changing meaning quietly. `examples/all_instructions_test.mms` Test 221 was that defect written as a test — it set `$12` to 23, wrote `LDA Result,$11,12` and expected 123 — and now pins both forms against the specification
* The last twenty-three base mnemonics select their opcode from the operand, so no family is left where the canonical spelling is rejected. `LDHT`, `LDSF`, `LDUNC`, `LDVTS`, `STHT`, `STSF`, `STUNC`, `CSWAP`, `PREGO`, `PRELD`, `PREST`, `SYNCD`, `SYNCID`, `GO`, `PUSHGO`, `FLOT`, `FLOTU`, `SFLOT` and `SFLOTU` take the ordinary three-operand shape with an immediate `Z`; `STCO X,$Y,Z` keeps `X` an immediate byte and gains an immediate `Z`, which neither spelling accepted before; `NEG $X,Y,Z` and `NEGU $X,Y,Z` keep `Y` immediate; and `PUT rA,Z` writes a literal into a special register without reaching for `PUTI`. Each emits exactly what its `*I` spelling emits, and the `*I` spellings continue to work unchanged, `NEGI` and `NEGUI` excepted — see below
* `JMPB` assembles. `PUSHJB` and `GETAB` each took a backward target and diagnosed a forward one, but no `JMPB` grammar rule existed at all, so `JMPB Back` failed with a syntax error naming an `IS` directive that appeared nowhere in the source — the backward surface contradicted itself. `JMPB` now encodes a backward target and rejects a forward one by naming `JMP`, the way its two siblings do
* `NEGI` and `NEGUI` range-check their `Z` operand. `NEG` and `NEGU` moved to the operand-selected form, which left the `I` spellings as the only route to `NEG`'s three-operand parser, and that parser took `Z` as an unchecked cast: `NEGI $1,0,#300` truncated to `Z=0`, and a symbolic operand that the register-form parser used to diagnose (`BigC IS #300` then `NEGI $1,0,BigC`) became silently wrong code. A negative literal changes the same way — `NEGI $1,0,-1` assembled to `35 01 00 FF` before and is now an error. All three raise the out-of-range error the operand-selected forms raise. The other `*I` spellings are untouched and still truncate or wrap a `Z` outside `0..255`: `ADDI $1,$2,#300` emits `21 01 02 00` and `LDOI $1,$2,-1` emits `8D 01 02 FF`

0.3.4 (2026-08-22)

* Every special register prints under its own name. `impl Display for MMix` indexed an alphabetically ordered name array by slot number, but `SpecialReg`'s discriminants follow Knuth's ISA order, so the two agreed at a handful of indices and disagreed everywhere else: `rN` printed as `rJ`, `rO` as `rK`, `rG` as `rT`, `rL` as `rU`, and the real `rG` never appeared at all. This was the whole of checksmix's special-register presentation — every `checksmix run` state dump, and `mmixdb`'s `state` and `bt`. New `SpecialReg::name` is the single table the dump and the debugger's name lookup now share, so the two cannot drift apart again; the assembler's own predefined-symbol seeding is unchanged and pinned by a test against it

0.3.3 (2026-08-22)

* **Breaking: `Command` is `#[non_exhaustive]` and gained a `Stepi` variant.** A downstream exhaustive match now needs a wildcard arm, the same migration `TrapCode` asked for in 0.3.0
* **Breaking: the `GETAB`, `JMPB` and `PUSHJB` encodings changed.** A `.mmo` built by 0.3.2 or earlier misexecutes those three instructions under this release; rebuild from source
* Every PC-relative instruction field now follows Knuth's encoding. A relative-address instruction comes in a forward/backward opcode pair and its field is unsigned in both: the forward opcode means `@ + 4*YZ`, the backward one `@ + 4*(YZ - 65536)`, with `2^24` in place of `65536` for `JMP`/`JMPB`. The VM read that field three incompatible ways — sign-extending it in `branch_forward`, `branch_backward`, `PUSHJ`, `GETA` and `POP`, and treating it as an unsigned magnitude in `PUSHJB`, `GETAB` and `JMPB` — so any forward displacement of 32768 tetras or more jumped backward, and a genuinely backward `PUSHJB` landed in zeroed memory, decoded `TRAP 0,0,0` and exited 0 without a word. The assembler now picks the opcode from the sign of the displacement, rejects a forward target on an explicitly written `*B` mnemonic, and rejects a misaligned or out-of-range target instead of silently wrapping the field. 0.3.0's claim that `JMPB` was an unsigned magnitude was wrong, and this corrects it
* `INCL $X,YZ` takes the 16-bit immediate its `INCH`/`INCMH`/`INCML` siblings take. It previously required three register operands and packed `$Y` and `$Z`'s register *numbers* into YZ, so `INCL $1,$5,$6` added 1286 and `INCL $1,#203` was a syntax error
* `mmixdb`'s `step` and `next` advance one source line rather than one instruction, matching gdb; the instruction-level behavior they had moves to the new `stepi`/`si`. `MMixAssembler::source_loc` resolves any address inside a statement's expansion to that statement's line instead of only its first address, so stopping inside a `SETI` expansion names its line with the address in front rather than reporting `?? (no source line)`
* `mmixdb` stops at a breakpoint on the entry point when you type `run`, and refuses to resume a program that has exited — `step`, `stepi`, `next` and `continue` answer `The program is not being run.` instead of executing past the end of the image into zeroed memory. `print`, `state` and `list` keep working, and `run` restarts
* The Emacs integration in `contrib/mmixdb.el` works. Its hand-written GUD marker filter returned the empty string for every chunk carrying no marker and never flushed, so the whole session sat unreachable in `gud-marker-acc` and the buffer showed nothing at all; its marker regexp was greedy across the colon, handing `gud-find-file` a path with `:LINE` glued on that it silently dropped. Both are gone: the mode now uses gud's own `gud-gdb-marker-filter` and `gud-gdb-marker-regexp`, which `mmixdb`'s marker was byte-compatible with all along. New `contrib/mmixdb-test.el` covers it, and README and `man/mmixdb.1` now say how to load the mode
* A keyword ends where a symbol could not continue. All 272 mnemonic and directive guards spelled `!ASCII_ALPHANUMERIC`, which is the complement of a symbol's continuation set less the underscore, so a label opening with a keyword and continuing with `_` was claimed by the keyword — `Halt_Loop`, `Swym_x`, `Resume_x` and `Loc_Start` all failed to assemble, and the diagnostic named an `IS` directive appearing nowhere in the source
* `SWYM X,Y,Z` assembles. MMIX's `SWYM` carries X, Y and Z and ignores them at execute; checksmix dropped them a layer early, so `SWYM 1,2,3` was a syntax error while bare `SWYM` worked. The operand group is all or nothing
* All three binaries diagnose an input that contributes no source instead of panicking with an index-out-of-bounds
* README documents every example program, links [playmmix](https://playmmix.2ad.com), and points at Knuth's own MMIX references

0.3.2 (2026-08-16)

* A `GREG` initializer now reaches its register. `GREG value` allocates a global register and records the pair in `MMixAssembler::greg_inits`, but `write_image` wrote only instruction and data bytes into memory — it never applied the initializers, so a program declaring `Base GREG 1000` and then reading `$254` read `0`. Fixing `write_image` fixes every load path that goes through it at once: `checksmix run`/`check`/`build`, `mmixdb`, and any embedder driving `Debugger`. The `.mmo` path is unaffected, and still carries no GREG channel — the object format has nowhere to put one
* `rG` is derived from the GREG allocation rather than hardcoded to `32`. `MMix::initialize` set `rG = 32` unconditionally, which is right only for a program that declares no `GREG` at all. GREG allocates downward from `$254`, so `rG` is now the lowest register actually allocated, floored at the ISA minimum of `32`, and left at `32` when nothing is allocated. This is not a display detail: `rG` is where the register file divides into local and global, so `set_register` now grows `rL` across a wider range, and `push_frame`/`pop_frame` zero a wider range on every `PUSHJ`/`POP`. No shipped example combined a `GREG` with a `PUSHJ`, so that interaction went unexercised until now; a new test covers it
* New `MMix::loaded_extent()`, iterating every address the loaded program occupies paired with its current byte, ascending — zero bytes included. `occupied()` reports nonzero bytes only, because `write_byte` drops a zero from the sparse map, which leaves a zero written as data indistinguishable from an address never written: `BYTE "Hello world!",'\n',0` occupies 14 bytes and `occupied()` reports 13, hiding the NUL terminator `Fputs` depends on. `loaded_extent()` records only what `write_image` writes, so it stays the loaded image rather than accumulating the register-stack and trap-frame traffic a full write journal collects during a run. `occupied()` and `write_byte` are unchanged
* `clap`, `pest`, `pest_derive` and `regex` bumped

0.3.1 (2026-08-15)

* New `MMix::run_bounded(budget) -> (usize, Stop)`, and a `#[non_exhaustive]` `Stop` enum with `Halted`, `BudgetExhausted`, and `Breakpoint(u64)`. `run()` is now `run_bounded(usize::MAX).0`, so its behavior is unchanged, but an embedder can finally execute in slices. This is what makes the interpreter usable from a single-threaded host: a WASM page can run a budget, yield to the event loop, and offer a stop button, where an unbounded `run()` freezes the tab on a student's infinite loop with no way out. `MMix` has no breakpoint concept, so `run_bounded` never returns `Stop::Breakpoint` — that variant exists for `Debugger`, which owns the breakpoint set. On `BudgetExhausted` the machine is left untouched and resuming is another call with the same budget
* `Debugger::do_continue`, `do_run`, and `do_next` are bounded by a one-million-instruction `STEP_BUDGET`. Previously each was an unbounded loop with no way to interrupt it: a program that never reached a halt or a breakpoint spun forever, and no test could catch a regression that caused one — the test hung instead of failing. This surfaced during an adversarial review of `MMix::reset`, where a no-op mutant was killed at ten minutes rather than failing an assertion. `Debugger::report` appends a line when the budget is what stopped the run, so an exhausted budget is distinguishable from a halt
* New `MMix::occupied()`, iterating every address holding a nonzero byte in ascending order. Memory is a sparse `HashMap`, whose iteration order is unspecified; this gives a caller a stable view to display or diff
* New write journal: `MMix::set_journal(bool)` and `MMix::take_journal() -> Vec<u64>`. While enabled, every `write_byte` records its address — including a zero-write, which removes that address from `occupied()` and is itself a state change worth reporting. `take_journal` drains, deduplicated and ascending, matching `occupied()`'s ordering. The enabled flag survives `MMix::reset`; the accumulated addresses do not
* `write_image` and `entry_point` are now public. Both already existed to serve `Debugger`; loading an assembled program into a machine and finding its start address are what any embedder driving `MMix` directly has to do, and reimplementing them meant duplicating the `Main`-label-else-first-text-address rule
* `man/mmixdb.1` documents step-budget exhaustion under `continue` and `next`

0.3.0 (2026-08-15)

* **Breaking: `MMix` and `Debugger` are no longer `Send`, `Sync`, `UnwindSafe`, or `RefUnwindSafe`.** `MMix` now owns its I/O as a `Box<dyn Host>`, and `Debugger` holds an `MMix`. This is deliberate rather than incidental — the intended embedders are single-threaded and capture into `Rc<RefCell<_>>`, which a `Send` bound would forbid — but it is why this release is `0.3.0` and not `0.2.24`. Construct an `MMix` on the thread that runs it, and wrap it in `std::panic::AssertUnwindSafe` to put one through `catch_unwind`. No other public type changed its auto-traits, and nothing lost `Debug` or `Clone`
* New `Host` trait: machine I/O is now injectable. `MMix::with_host` lets an embedder capture what a program writes to stdout and stderr, supply the clock behind the `Time` trap, receive the diagnostics previously printed with `eprintln!`, and observe every recognized `TRAP` through a single hook reporting the trap code, its Z operand, and `$255` both before and after dispatch. `MMix::new()` is backed by `StdHost`, which reproduces the previous process-level behavior exactly. `Host`, `StdHost`, and `TrapCode` are re-exported from the crate root, and `Box<dyn Host>` itself implements `Host`, so a runtime-selected host needs no wrapper. Unhandled trap codes, the register form of `TRAP`, and the `TRIP` instruction report through `diagnostic` rather than the trap hook. File-descriptor traps (`Fopen`/`Fread`/`Fwrite`/`Fseek`/…) continue to use `std::fs` directly and are unaffected
* `TrapCode` is `#[non_exhaustive]`, so a downstream `match` must carry a wildcard arm; it also gained `Hash`
* New `Debugger::load_with_host`, so an embedder can capture what a debugged program writes. `Debugger::load` installs `StdHost` and exposes no way to reach the output afterwards, which left the `Host` trait unreachable from the debugger — the path a browser playground actually drives. Supported by a new `MMix::reset`, which returns the machine to its freshly-constructed state while keeping the installed host; `Command::Run` uses it, so a host that accumulates output sees every run appended rather than losing the one it was built for
* `checksmix` is now usable as a library without its CLI dependencies. `clap`, `rustyline`, and `tracing-subscriber` moved behind a new `cli` feature, on by default, so `cargo install checksmix` and every existing build are unchanged — but a library-only consumer can set `default-features = false` and build for `wasm32-unknown-unknown`, which previously failed because `rustyline` pulls `home`. All three binaries declare `required-features = ["cli"]`; `mmixasm` and `mmixdb` gained explicit `[[bin]]` sections, since auto-discovered binaries cannot carry that key. A new CI job enforces the wasm build
* Mnemonic parsing no longer depends on accidental backtracking. All 264 `mnemonic_*` grammar rules gained an `!ASCII_ALPHANUMERIC` word-boundary guard and atomic marking, so `GET` can no longer shadow `GETA` — 115 shadowing mnemonics across 199 ordered prefix pairs, every one of which now has both members assembled from source, by the instruction corpus or by a unit test. Swapping the pre-transform grammar back in leaves 623 of 626 tests passing, failing exactly the three that depend on the guard, so no currently-correct routing regressed. Known residual: the guard does not cover `_`, so a label like `Halt_Loop` still misparses
* `STCO`/`STCOI` fixed. `parse_inst_stco_rrr` and `_rri` discarded two `parts.next()` calls on the assumption they were comma tokens, but `comma` is a silent grammar rule that emits no pair — so operand fields shifted. The `_rrr` form ran off the end and panicked during assembly; `_rri` rejected valid source, diagnosing the shifted operand rather than the real fault -- "Undefined symbol" for a literal, "cannot use as register" for a numeric `IS` constant, and a panic when it resolved as a register alias
* The VM's `JMP` execution no longer sign-extends its 24-bit field. It is an unsigned forward magnitude, mirroring `JMPB`; the previous handling misdecoded any forward jump of 0x800000 instructions or more. The same pattern remains in `branch_forward`, `PUSHJ`, and `GETA` and is not addressed here
* `examples/all_instructions_test.mms` grew by 940 lines to close corpus coverage gaps, and four vacuous tests within it were fixed — Tests 200 (`NXORI`) and 214 (`MXOR`), which never cleared `Result` before asserting on it, and Tests 259 (`PUSHJ`/`POP`) and 260 (`PUSHGO`/`PUSHGOI`), where three `POP` sites could not fail if `POP` never returned
* New unit test for integer `CMP`/`CMPU` (0x30/0x32), which had no dedicated Rust coverage — only the floating-point `FCMP`/`FCMPE` did. `CMPI`/`CMPUI` are exercised by corpus Tests 206/207 under gate 5 but still have no unit test of their own
* Four unit tests that wrote real bytes to the test runner's stdout now assert the captured bytes instead of only the returned byte count, and a new test asserts that `Halt` routes its diagnostic, its flush, and its trap-hook event through the host. The crate gained its first doctest, on `Host`
* `AGENTS.md` completion gates gained the `wasm32-unknown-unknown` library check and `cargo test --no-default-features`, and gate 5's success condition was corrected: the smoke example *prints* `All tests passed!` and exits 0, it does not end with that line

0.2.23 (2026-07-08)

* New `mmixdb`: an interactive source-level debugger for `.mms` programs (`step`/`next`/`continue`/`run`, breakpoints by source line or label, `print` of registers/special-registers/labels/symbols/memory, `list`, `backtrace`/`info reg`, and a `help`/`h`/`?` command listing all of the above; `quit`/`q`/`exit` to leave). Supports Emacs GUD `--fullname` mode (auto-enabled under `INSIDE_EMACS`) via `contrib/mmixdb.el`
* `MMixAssembler` gained a source-line debug-info substrate (`source_loc`, `addr_for_line`, `source_text`) underlying `mmixdb`'s breakpoints and source display
* New `INCLUDE` directive: `INCLUDE file.mms` expands into an ordered translation unit ahead of parsing, reusing the existing multi-source assembly mechanism rather than text-splicing, so diagnostics from an included file report that file's own name and line numbers. Paths resolve relative to the including file's directory, recursively, with cycle detection
* `GETA`/`GETAB` now validate their target is in range and correctly aligned instead of silently truncating an out-of-range offset to a wrong-but-plausible instruction; out-of-range, misaligned, or (for `GETAB`) forward targets are now a hard assembly-time error pointing at `LDA` as the alternative. Also fixes an independent `GETAB` sign/direction bug where it reused `GETA`'s signed-offset formula despite the VM decoding `GETAB`'s field as an unsigned backward-only magnitude
* Several correctness fixes: `BYTE` escape decoding no longer emits an undocumented auto-terminator; 3-operand `LDA` is sized correctly in the pass-1 forward-reference estimator; branch offsets are divided before casting so they no longer silently truncate; `POP`'s YZ high byte survives parse and encode; backward jumps emit `JMPB` per the MMIX spec; `Program::parse_instruction` returns `Result` instead of panicking
* `regex` bumped 1.12.3 → 1.12.4

0.2.22 (2026-04-30)

* PUSHJ/PUSHGO/POP now implement Knuth's register-stack window slide per MMIXware §1.4. PUSHJ $X spills $0..$X (with the marginal slot at offset X holding the value X), slides caller's $(X+1)..$(rL-1) down to callee's $0..$(rL-X-2), zeroes the freshly-allocated tail, and sets rL := saturating_sub(rL, X+1). POP n reverse-slides the callee's $0..$(n-1) into the caller's $X..$(X+n-1) (the "hole" plus the slots above it), restores the spilled frame, clears the marginal slot, and updates rL to min(rG, max(saved_rL, X+n))
* Standard MMIX calling convention now works: caller stages args at $X+1, $X+2, …; callee reads them at $0, $1, …; POP 1 lands the return value at the caller's $X. Multi-source programs that rely on this convention (the common case for any code written against Knuth's spec) now run correctly
* Local register frame is fully spilled on PUSHJ (`max(X+1, rL) + 1` octas) so the slide-back can reconstruct caller state without a hardware ring buffer; observable behaviour matches the spec at the cost of a larger memory footprint per call
* `set_register` now auto-grows rL when writing to a local at index ≥ rL, matching MMIX hardware semantics — programs that write `$5` no longer have to explicitly bump rL before PUSHJ
* New unit tests cover argument passing via the slide, return-value placement at the hole, freshly-allocated-locals zeroing, multi-value POPs, two-deep nested calls; existing PUSHJ/POP/PUSHGO tests rewritten against the spec
* `examples/fibonacci.mms`, `examples/function.mms`, `examples/remeuclid.mms` rewritten to use the standard convention; the repo-root `big_fib.mms` (committed) computes fib(100) = 354224848179261915075 correctly under the new semantics

0.2.21 (2026-04-30)

* Auto-immediate selection: base mnemonics in the arithmetic, bitwise, bit-fiddle, shift, conditional-set, and zero-or-set families now accept either a register or a 0..255 immediate as their third operand and emit the matching RRR or RRI variant. `ADD $1,$2,5` now assembles to `ADDI`, `AND $1,$2,#FF` to `ANDI`, `SR $1,$2,3` to `SRI`, and so on — matching standard MMIXAL where there is no separate `ADDI` mnemonic
* Existing `*I` mnemonics (`ADDI`, `ANDI`, `SRUI`, …) continue to work unchanged as accepted aliases that always force the immediate encoding, so existing `.mms` sources assemble byte-for-byte identically
* Z-operand resolution: bare symbols at the Z slot are resolved against the symbol and label tables — register aliases (`R IS $4`) keep the RRR form, in-range constants (`K IS 7`) auto-select RRI, label addresses and out-of-range constants are rejected with `<file>:<line>:<col>: immediate operand N out of range 0..255 for <mnem>`
* Strict 0..255 range check on the auto path; the explicit `*I` path retains its existing silent-wrap behaviour for negatives (so `ADDI $1,$2,-1 → ADDI(1,2,0xFF)` still holds)
* Out of scope (separate tasks): loads/stores, `NEG`/`NEGU`, `PUSHGO`/`GO` and other specialty rules, float-immediate forms (`FLOTI`, `SFLOTI`, …), `MMIX.md` documentation, and removal of the `*I` mnemonics
* 47 validation tests added covering every base mnemonic in scope (RRR + RRI), prefix-collision backtracking (ADD/ADDU, AND/ANDN, CSN/CSNN, ZSN/ZSNN, SR/SRU, 2/4/8/16-ADDU), boundary Z values (0, 255, #FF, 256, #100, -1, $255, char literals), symbol/label resolution at Z, cross-family routing, and a comprehensive byte-identical regression pairing every base mnemonic with its explicit `*I` sibling

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
