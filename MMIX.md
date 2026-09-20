# MMIX Instruction Quick Reference

MMIX is a 64-bit big-endian RISC machine (Knuth, 1999) with 256 general-purpose registers (`$0`–`$255`), a separate special-register file, byte-addressed memory, and fixed 32-bit instructions. Immediates in assembly may be decimal, hexadecimal (`#`-prefixed, or `0x`/`0X`-prefixed — also a checksmix extension), or character literals; every operand is an MMIXAL expression (see "Expressions" below). A leading `0` is an ordinary decimal digit, as in MMIXAL — `SET $1,010` loads 10, and there is no octal spelling.

## Memory access

MMIX has no unaligned access. A wyde, tetra, or octa access at address `A`
resolves to `w·⌊A/w⌋` for its width `w` (2, 4, or 8) — the low `log2(w)` bits
of `A` are ignored. `LDO $X,$Y,$Z` with an address ending in 3 loads the
octabyte at the aligned base below it, not eight bytes straddling two
octabytes. A misaligned address is rounded, never rejected: there is no trap
or diagnostic. Byte access is unaffected — a byte is its own alignment.

## Minimal assembly skeleton

```
        LOC     #100        % set load address to 0x100
        GREG    @           % allocate a base register (optional)
Main    SETL    $0,42       % your code here
        TRAP    0,Halt,0    % halt, exit code in $255
```

A program starts with `$255` holding its entry address — `Main`'s, or the
first instruction's when there is no `Main` (MMIXAL reference).

## Line structure

A line holds a label field, an opcode field and an operand field, each
separated from the next by a blank; every field but the opcode is optional.
A label with nothing but blanks and a comment after it is a statement on its
own; anything else there means the opcode field held a word the assembler
doesn't recognize, reported as an unknown operation with the statement
printed in full.

`;` separates statements — `SETL $1,1; ADD $1,$1,1` assembles both — and
needs no blank on either side. A statement after a `;` is read exactly like
one at the start of a line, label field included.

`%` is the only comment character. It runs to the end of the line and wins
over a later `;`: in `SETL $1,1 % note; ADD` the `; ADD` sits inside the
comment, so no second statement begins.

A line whose first character is not a letter, a digit, `:` or `_` is a
comment in its entirety — `;`, `*`, `#`, `/` and `-` all open one this way.
An indented line is not covered by this rule; its content parses normally.

Once a statement's operands have parsed, text past them is ignored, provided
a blank separates it from the operand field: `ADD $1,$2,$3 sum of the parts`
assembles, the trailing words dropped. Text abutting the operand with no
blank is a syntax error, as is text opening with a digit or with `,` `+` `-`
`*` `/` `~` `&` `|` `^` `<` `>` or `$` — a digit there almost always means a
dropped separator rather than a comment, so `HALT 2 apples` is an error, not
a warning; a bare operand holds no blanks, so `SETL $1,2 + 3`
would otherwise assemble as `2` with `+ 3` dropped in silence. `/` is in
this set: `SET $1,2 / 3` is an error, though `SET $1,2/3` (no blank) still
divides to `0` inside the expression itself.

Whitespace around an operand list's commas stays legal — `TRAP 0, Time, 2`
parses — a checksmix extension over MMIXAL, which ends the operand field at
the first blank.

## Assembler directives

| Directive | Syntax | Effect |
| --- | --- | --- |
| `LOC` | `LOC expr` | Set the assembly location counter to *expr*; a label on the same line names the location *before* the move |
| `GREG` | `[label] GREG expr` | Allocate a global register initialized to *expr*; optional label becomes a register alias |
| `IS` | `Name IS expr` | Define a numeric or register alias constant |
| `PREFIX` | `PREFIX str` | Qualify subsequent unqualified names as `str<name>`; names beginning with `:` opt out |
| `BYTE` | `BYTE expr,...` | Emit one byte per operand |
| `WYDE` | `WYDE expr,...` | Emit one 16-bit wyde per operand |
| `TETRA` | `TETRA expr,...` | Emit one 32-bit tetra per operand |
| `OCTA` | `OCTA expr,...` | Emit one 64-bit octa per operand |
| `INCLUDE` | `INCLUDE file` | Assemble the named file as if inserted here, resolved relative to the including file; recursive, cycles are an error |

A string operand assembles one unit per character. The directive aligns
once, before the first unit; a list does not realign between items.

The assembler aligns before it places an item: it rounds the location counter
up to the item's natural width — 4 for an instruction, 2, 4 or 8 for `WYDE`,
`TETRA` and `OCTA` — and the label on that line takes the rounded address. The
skipped bytes are a gap rather than emitted padding, and load as zero. `BYTE`
is never aligned. Alignment follows the item's kind, not the count of bytes it
emits, so `BYTE "abcd"` is four bytes wide and still lands wherever the counter
stands.

`LOC` sets the counter exactly; it does not align, and the next instruction or
wide datum rounds up from wherever `LOC` left it. Rounding happens when an item
is assembled, never when a label is defined, so a label alone on its own line
keeps the unrounded counter. A bare label followed by `OCTA` can therefore name
an address up to seven bytes below the octabyte, and since MMIX has no
unaligned access a load through that label rounds back down past the datum. A
label on a `LOC` line itself takes the location the counter held *before* the
move: `X LOC @+500` names `X` as the first of the 500 bytes `LOC` skips, and
assembly continues at `X+500`.

### Expressions

Every operand — a register, an immediate, `LOC`'s target, a data item — is an
MMIXAL expression: constants, symbols, `@`, unary operators, and two
left-associative precedence levels of binary operators.

| Level | Operators |
| --- | --- |
| Strong (binds tighter) | `*` `/` `//` `%` `<<` `>>` `&` |
| Weak | `+` `-` `\|` `^` |

`a-b-c` is `(a-b)-c`; `2+3*4` is `14`, since `*` binds tighter than `+`.
Parentheses are the only grouping — MMIXAL has no brackets — and nest freely:
`(2+3)*4` is `20`. Unary operators are `+` (identity), `-` (negate, mod
2⁶⁴), `~` (complement), `$` (cast a pure value to a register number) and `&`
(a symbol's serial number, always rejected — checksmix's object file carries
no symbol table to index).

A bare expression — one with no enclosing parentheses — holds no whitespace:
it is one unbroken run of characters ending at the first space, tab, comma,
`;`, comment character or newline. What follows a bare expression is subject
to the trailing-text rule in "Line structure" above. Write a negative
literal closed up:
`SET $1,-5`, never `SET $1,- 5`. A parenthesized group is the one place an
expression may hold whitespace, a checksmix extension over MMIXAL's own
closed-up syntax and a pure superset of it: `SET $1,(2 + 3)` assembles.

`%` is the remainder operator wherever an expression is being parsed, and a
comment character everywhere else — the same rule MMIXAL itself uses.
`5%3` is `2`, the whole thing one unbroken expression; `5 % 3` is `5`, the
space ending the expression before `% 3` opens a comment; `(5 % 3)` is `2`,
whitespace being ordinary inside a group; `(2 + 3) % sum` is `5`, the group
having already closed before `%` opens the comment (so `sum` need not even be
defined).

Arithmetic is on unsigned octabytes: `+` `-` `*` wrap mod 2⁶⁴; `x/y` is
⌊x/y⌋ and illegal at `y=0`; `x//y` is ⌊2⁶⁴·x/y⌋ and illegal at `x≥y`; `x%y` is
the remainder of that same division; `x<<y` is `(x·2ʸ) mod 2⁶⁴` and `x>>y` is
⌊x/2ʸ⌋, both `0` for `y≥64`; `&` `|` `^` are bitwise.

A symbol's value is pure — a label, an `IS` constant, a predefined constant —
or a register — `IS $n`, a `GREG` label. Unary `$` casts a pure value to a
register. Register arithmetic: register+pure, pure+register and
register−pure give a register; register−register gives a pure value; any
other binary operator with a register operand is an error, as is every unary
operator but `+`. With `x IS $1` and `y IS $10`, `x+3` and `3+x` are `$4`,
and `y-x` is the pure value `9`. A register value may run past 255 inside an
expression, but the final value a register site consumes must fit `0..=255`,
same as a bare `$256` today.

`@` is the current location: for an instruction, its tetra-aligned address;
for a data directive, the aligned address of the directive's first unit — the
same address for every item in its list, since the whole list is evaluated
before any of it assembles.

A forward reference resolves like any label: in most operands it may name a
symbol defined anywhere in the program, and operators apply to it exactly as
to a resolved value — `JMP Later+4` and `OCTA Later-8` both assemble. Applying
an operator to a forward reference at all is a **checksmix extension**: MMIXAL
assembles in one pass and forbids it outright, but every program it accepts
still assembles identically here. `LOC`, `IS`, `GREG` and the two-operand
`LDA`'s size estimate are the exception: they resolve only a symbol already
defined above, an assembler restriction this widens to cover expressions
rather than lifts.

### INCLUDE

`INCLUDE file` (case-insensitive) is a **checksmix extension**, not part of
MMIXAL. It is a preprocessor stage, not a grammar rule: the
named file is inserted as its own translation unit(s), so errors inside it
report *its own* filename and line numbers rather than the includer's. The path
resolves relative to the including file's own directory (like C's
`#include "..."`), each level resolving against its own directory in turn.
Inclusion is recursive; a cycle (a file re-entering itself while already on
the current include chain) is a hard error naming the chain, and an unreadable
file is a hard error naming the file.

```
INCLUDE lib.mms      % pulls lib.mms in as if inserted here
```

Two known limitations:

- **Own line, no label.** `INCLUDE` must occupy its own line; a line whose
  first token is not `INCLUDE` is left untouched, so a label cannot be
  attached to an `INCLUDE` line. `;` cannot separate an `INCLUDE` from
  anything else either: the operand runs to the end of the line, so a `;`
  lands inside it and fails as an unreadable file naming the whole text.
- **`source_text` first-match-on-filename.** Splitting a host file at an
  `INCLUDE` produces multiple units that share the same filename. The
  debug-info API `source_text(file, line)` resolves a unit by the first match
  on filename, so a caller asking for a line in a later host segment (after
  the `INCLUDE`) may get an earlier segment's text instead. This is a
  pre-existing `source_text` limitation surfaced by `INCLUDE`, not something
  `INCLUDE` itself introduces or fixes.

### Global symbols and PREFIX

A label or operand that begins with `:` is a **global** (linkage-visible) symbol; its name is stored verbatim regardless of the current `PREFIX`. Unqualified names are prefixed by the active `PREFIX` string. `PREFIX :` resets to the global namespace.

```
        PREFIX  P_
P_Foo   TRAP    0,Halt,0    % stored as "P_Foo"
:Bar    TRAP    0,Halt,0    % stored as ":Bar" (global, no prefix applied)
```

### Multi-source assembly

`checksmix` and `mmixasm` accept multiple `.mms` inputs in one invocation. All files share one symbol space and one byte stream, assembled as if concatenated in command-line order.

```
checksmix run   main.mms lib.mms
checksmix check main.mms lib.mms
checksmix build -o prog.mmo main.mms lib.mms
mmixasm         main.mms lib.mms -o prog.mmo
```

## Floating-point arithmetic

All floating-point instructions use IEEE 754 double precision. Results honor the **rounding mode** in bits 17–16 of special register `rA` (register 21). `rA` is 18 bits wide, so the mode field sits at its top: `PUT rA,$X` above `#3FFFF` is an illegal-instruction interrupt, and since this VM has no interrupt vector, it halts with a diagnostic. `PUTI` cannot reach the field — its operand is `Z` alone, eight bits — so selecting a mode needs the register form of `PUT`:

| rA bits 17–16 | Mode | Meaning |
| --- | --- | --- |
| `0` | `ROUND_NEAR` | Round to nearest, ties to even (default) |
| `1` | `ROUND_OFF` | Round toward zero (truncate) |
| `2` | `ROUND_UP` | Round toward +∞ |
| `3` | `ROUND_DOWN` | Round toward −∞ |

Instructions that honor rounding mode: `FADD`, `FSUB`, `FMUL`, `FDIV`, `FSQRT`, `FINT`, `FIX`, `FIXU`, `FLOT`, `FLOTI`, `FLOTU`, `FLOTUI`, `SFLOT`, `SFLOTI`, `SFLOTU`, `SFLOTUI`, `STSF`, `STSFI`.

`FIX`, `FIXU`, `FSQRT`, `FINT`, and the `FLOT`/`FLOTI`/`FLOTU`/`FLOTUI`/`SFLOT`/`SFLOTI`/`SFLOTU`/`SFLOTUI` family additionally take a `Y` operand that overrides the mode for that one instruction, numbered independently of rA's own field:

| `Y` | Symbol | rA-equivalent mode |
| --- | --- | --- |
| `0` | `ROUND_CURRENT` | none — uses rA's current mode |
| `1` | `ROUND_OFF` | `1` |
| `2` | `ROUND_UP` | `2` |
| `3` | `ROUND_DOWN` | `3` |
| `4` | `ROUND_NEAR` | `0` |

`Y` is omitted (the two-operand form) or `0` to defer to `rA`; `Y > 4` raises an illegal-instruction interrupt, and since this VM has no interrupt vector, it halts with a diagnostic instead. `STSF`/`STSFI` take no `Y` operand and always use `rA`'s mode.

**Known gap:** none for the twelve mnemonics above — `Y` overrides `rA`'s mode per-instruction; `Y = 0` or the implicit two-operand form falls back to `rA`.

### rA event flags

An arithmetic exception whose enable bit (below) is clear ORs its event flag
into `rA`; event flags are never cleared automatically. One whose enable bit
is set trips to its handler instead, and its event flag stays clear — see
"User trips". The bit values match MMIXAL's predefined symbols `D_BIT` …
`X_BIT`.

| Flag | rA bit | Kind | Raised when |
| --- | --- | --- | --- |
| X | `0x01` | floating | Result is inexact (rounded) |
| Z | `0x02` | floating | Floating division by zero |
| U | `0x04` | floating | Underflow |
| O | `0x08` | floating | Overflow |
| I | `0x10` | floating | Invalid operation (NaN operand, 0/0, ∞−∞, etc.) |
| W | `0x20` | floating | Float-to-integer conversion overflows |
| V | `0x40` | integer | Integer overflow — `ADD`, `SUB`, `MUL`, `NEG`, `DIV` of `#8000000000000000` by −1, `SL`, and the signed stores `STB`/`STW`/`STT` |
| D | `0x80` | integer | Divide check — signed division by zero |

There is no denormalized-operand event: a subnormal operand raises nothing, and an underflow to a subnormal result raises `U`. `FREM` and `FSQRT` raise `U` in no case — the IEEE remainder is exact by definition, and the square root of a nonzero finite operand is neither zero nor subnormal. `DIVU` raises no divide check, because `u($Z) ≤ u(rD)` — which includes a zero divisor — is part of its definition rather than an error.

Read/clear `rA` with `GET $X,rA` / `PUT rA,$X`.

### User trips

`rA`'s low byte holds the eight event flags above; the next byte holds their
enable bits, one per event flag, in the same `D V W I O U Z X` order. `PUT
rA,$X` writes both bytes at once (bounded to 18 bits total, the rounding mode
occupying the top two — see above); enabling an exception is `PUT
rA,$X` with the matching bit set two places left of its event flag. `PUTI`'s
operand is a single byte, too narrow to reach the enable byte — set a
register and use the register form of `PUT`.

`TRIP X,Y,Z` always trips, unconditionally, to the handler at `#00`. An
arithmetic exception trips only when its enable bit is set; instead of the
event flag, control transfers to a fixed handler address, `#10` `#20` `#30`
`#40` `#50` `#60` `#70` `#80` for `D V W I O U Z X` respectively. An
instruction that raises several exceptions at once trips to the leftmost
enabled one, in `D V W I O U Z X` order; every other raised exception whose
enable bit is clear still sets its event flag, but one that is enabled and
not leftmost is dropped entirely — no trip, no event flag.

A trip — explicit or arithmetic — sets `rB ← $255`, `$255 ← rJ`, `rW` to the
address of the instruction after the one that trips, and `rX` to `#80000000`
in the high tetra with that instruction's own opcode/X/Y/Z in the low tetra
(always negative, since the top bit is set). `rY` and `rZ` take the operand
values the instruction used: a register operand's contents, or the literal
field for an immediate or non-register operand — `ADDI $3,$2,5` gives
`rZ = 5`, and `FIX`/`FLOT`'s rounding-mode `Y` and `NEG`'s immediate `Y` are
non-register fields too. A store trip (`STB`/`STW`/`STT`/`STSF`, either form)
sets `rY` to the computed address and `rZ` to the aligned octabyte memory
holds after the store: the stored bytes in place, the rest unchanged. An
arithmetic trip's operands are captured before the instruction's own
destination write, so `ADD $5,$5,$3` overflowing still shows the handler the
pre-`ADD` `$5` in `rY`.

`RESUME 0` returns from a handler: if `rX` is negative — always true right
after a trip — execution continues at `rW`. Otherwise `rX`'s top byte is a
ropcode; `0` reinserts the instruction in `rX`'s low tetra as if it stood at
`rW − 4`, then continues at `rW`. Ropcodes `1`–`3` (operand substitution,
forced-trap emulation, page-table insertion) and `RESUME` with a nonzero `Z`
(the kernel's `RESUME 1`) have nothing in this VM to act on and halt with a
diagnostic instead, PC unmoved.

A trip to a vector nothing ever loaded would read zero, which decodes as a
silent `TRAP 0,0,0` — instead it halts with a diagnostic naming the trip and
the vector, and a nonzero exit code, after every register above is set so a
debugger can see why.

### Epsilon instructions (FCMPE / FUNE / FEQLE)

`FCMPE`, `FUNE`, and `FEQLE` are the "with epsilon" variants of `FCMP`, `FUN`, and `FEQL`. Each compared value `u` has an ε-neighborhood `Nε(u)`, scaled by its own binade: for a normal `u` the radius is `2^(e−1022)·ε`, where `e` is `u`'s raw IEEE-754 biased exponent field; for a denormal it is the fixed `2^−1021·ε`; `Nε(0) = {0}`; `Nε(+∞)` is `{+∞}` when `ε < 1`, every value except `−∞` when `1 ≤ ε < 2`, and every value when `ε ≥ 2` (mirrored for `−∞`). `FCMPE` reports `$Y ≺ $Z` (`-1`), `$Y ∼ $Z` (`0`, meaning `$Y ∈ Nε($Z)` or `$Z ∈ Nε($Y)`), or `$Y ≻ $Z` (`+1`). `FEQLE` reports the stronger `$Y ≈ $Z` (`1`), which requires both memberships to hold, and `0` otherwise.

`FCMPE` and `FEQLE` force their result to `0` and raise `I` when `$Y`, `$Z`, or `rE` is NaN, or `rE` is negative — never on an ordinary inequality. `FUNE` reports `1` on exactly that same exceptional condition and `0` otherwise; it says nothing about proximity, and raises no flag either way.

## TRAP interface

`TRAP 0, Code, Handle` invokes a system call identified by the predefined
symbol *Code*, numbered per the MMIXAL reference. `Handle` (`Z`) names an
open handle, 0–255. A call with one further argument takes it in `$255`
directly; a call with two takes an address in `$255`, the first argument as
the octa there and the second as the octa at `$255+8`. The result replaces
`$255`; a negative result means failure.

| Code | Value | Extra arguments | Result in `$255` |
| --- | --- | --- | --- |
| `Halt` | 0 | `$255` = exit code | — (halts) |
| `Fopen` | 1 | name address, mode | 0, or −1 |
| `Fclose` | 2 | — | 0, or −1 |
| `Fread` | 3 | buffer, size | 0 if all `size` bytes read; `n − size` if end of file after `n`; `−1 − size` on error |
| `Fgets` | 4 | buffer, size | characters stored, or −1 |
| `Fgetws` | 5 | buffer, size | wydes stored, or −1 |
| `Fwrite` | 6 | buffer, size | 0, or `n − size` after writing `n` |
| `Fputs` | 7 | `$255` = string address | bytes written, or −1 |
| `Fputws` | 8 | `$255` = string address | wydes written, or −1 |
| `Fseek` | 9 | `$255` = offset | 0, or −1 |
| `Ftell` | 10 | — | position, or −1 |

`Fopen`'s mode is one of `TextRead` (0), `TextWrite` (1), `BinaryRead` (2),
`BinaryWrite` (3), `BinaryReadWrite` (4). A handle carries four capability
bits: read, write, seek, and read-write. Text modes grant read or write
alone; binary read/write modes add seek; `BinaryReadWrite` grants all four
and switches — a read clears the write capability and a write clears the
read capability, until `Fseek` restores both. The three write modes
truncate an existing file. A call on a handle lacking the needed capability
fails with the table's failure value and touches no file. `Fopen` lets the
program choose the handle; opening one already open closes it first, and a
failed open leaves the handle closed.

`Fgets` reads until `size − 1` characters or a newline, then a zero byte,
returning the count stored (a partial last line at end of file included), or
−1 when `size` is 0 or nothing was read. `Fgetws`/`Fputws` move wyde
characters, two bytes each in memory order, raw to and from the file:
`Fgetws` rounds its buffer address down to even and stops at the wyde
`#000A`, `size − 1` wydes, or end of file; `Fputws` writes up to, not
including, the first zero wyde. `Fputs` writes up to, not including, the
first zero byte, with no byte value translated. `Fseek`'s offset, `≥ 0`,
positions that many bytes from the start; `< 0` positions `−offset − 1`
bytes before the end, so `−1` is the end itself.

Handles 0, 1 and 2 (`StdIn`, `StdOut`, `StdErr`, the predefined symbols'
values) are open at start with `TextRead`, `TextWrite`, `TextWrite`.
**Departure from the reference:** `Fopen`/`Fclose` on any of them return −1
and leave the stream as it was, rather than letting the program rebind them
— checksmix routes handles 1 and 2 through the host's own write, which has
no file underneath to rebind, and a `StdIn` read always fails, since the
host has no read primitive.

### Extensions

Three codes are checksmix's own, numbered `#80`–`#82` so an old binary's
codes 11–13 reach the unhandled-TRAP diagnostic rather than the wrong call:

| Code | Value | Behavior |
| --- | --- | --- |
| `Fputc` | `#80` | Write one byte (`$255`'s low byte) to `Handle`; shares `Fputs`'s capability check and read-write switching. Returns 0, or −1. |
| `Time` | `#81` | `Handle` (`Z`) selects the unit: 0 seconds, 1 milliseconds, 2 microseconds since the Unix epoch. Returns the time in `$255`. |
| `Debug` | `#82` | Backs the `debug "text"` directive, below. |

### `debug "text"`

`debug "text"` is a checksmix extension, not part of MMIXAL. It assembles to
one `TRAP 0,Debug,K` at its own address — labelled with the directive's own
label, if it has one — where `K` is the directive's index, 0-based, in
program order across every translation unit assembled together. `K` is one
byte, so a 257th `debug` directive in one program is an assembly error
naming its file and line.

The directive's text lives in a table outside guest memory: nothing is
written to guest memory and no label is generated. Running the TRAP writes
the string and a newline (`#0A`) to handle 1, changing no register —
`$255` included. A `K` past the table's end prints nothing and reports a
diagnostic instead.

## Register stack

Every register from `rL` through `rG-1` is marginal and reads zero. `PUT rG,z`
accepts a value only for `32 ≤ z ≤ 255` with `z ≥ rL`; any other value is an
illegal-instruction interrupt, and since this VM has no interrupt vector, it
halts with a diagnostic. A legal `PUT rG` zeroes every register between the
old and new `rG` — global to local/marginal when raising, local/marginal to
global when lowering.

`SAVE $X, 0` requires `X` global (`X ≥ rG`); a local `X` halts with a
diagnostic. It pushes a context onto the register stack at the current `rO`,
growing upward, in this order: the `rL` local registers `$0..$(rL-1)`, a
marker octa holding `rL`, the global registers `$rG..$255`, the twelve
special registers `rB rD rE rH rJ rM rR rP rW rX rY rZ`, and one packed octa
holding `rG` in its top byte and `rA` in its low bits. `$X` receives the
packed octa's address; `rO` and `rS` both become the address of the byte
after it, and `rL` becomes 0. `rJ` is saved as data among the specials, never
overwritten — `SAVE` opens no call frame.

`UNSAVE 0, $Z` restores a context whose topmost (packed) octa `$Z` addresses,
validating it whole before changing anything: a packed `rG` outside
`32..=255`, a packed `rA` above the widest legal value, or a saved local
count greater than the packed `rG` all halt with a diagnostic and the
machine unchanged. Otherwise every saved register restores, `rL` becomes the
saved local count, and `rO = rS` land at the address of the first restored
local — where `rO` stood before the matching `SAVE`.

Both instructions ignore their must-be-zero fields (`SAVE`'s `Y` and `Z`,
`UNSAVE`'s `X` and `Y`) rather than rejecting a nonzero value there.

Writing a marginal register `$X` raises `rL` to `X+1` and zeroes `$rL`
through `$X`. For an instruction whose `X` field is a general-register
destination, this rise happens before the instruction runs, so `GET $X,rL`
stores the raised `rL`, not the value it held when the instruction began.
`PUT rL,z` (and `PUTI`) only ever lowers `rL`, to `min(z, rL)`, and zeroes
every register the drop excludes from the local range.

The register stack lives in memory at `rO`: `S[k] = M8[rO+8k]`. `PUSHJ $X,
addr` (and `PUSHJB`, `PUSHGO`, `PUSHGOI`) push the caller's local registers
there and slide the window down. For `X < rG`: `S[0..X] = $0..$X`, with the
marginal slot `S[X]` holding `X` itself — the hole `POP` reads back; `rO`
and `rS` both advance to `rO + 8(X+1)`. The caller's `$(X+1)..$(rL-1)`
become the callee's `$0..$(rL-X-2)`; `rL` becomes `saturating_sub(rL,
X+1)`. For `X ≥ rG`: all of `$0..$(rL-1)` push the same way, followed by
the marker `rL`, and the callee starts with `rL = 0` — the hole for the
later `POP` is `rL`, not `X`.

`POP X, YZ` returns from a `PUSHJ $x` frame whose callee has `rL = L`. It
reads the hole from memory: `x = M8[rO-8] mod 256`. If `X > L`, `X` becomes
`L+1` and the hole gets zero. The caller's `$0..$(x-1)` restore from
`M8[rO-8(x+1)..]`; `$x` gets the callee's `$(X-1)` (the *last* output lands
in the hole), or zero when `X = 0` or the clamp fired; `$(x+1)..$(x+X-1)`
get the callee's `$0..$(X-2)`. `rO` and `rS` retract to `rO - 8(x+1)` —
exactly where the matching `PUSHJ` found them. `rL` becomes `min(x+X, rG)`;
every register from the new `rL` through `rG-1` reads zero. `POP` branches
to `rJ + 4·YZ` and leaves `rJ` unchanged; a subroutine that calls another
saves `rJ` (`GET $k,rJ`) and restores it (`PUT rJ,$k`) before its own
`POP`.

`rO` and `rS` always agree between instructions: every `PUSHJ` and `POP`
stores eagerly and moves both together, where MMIXware may leave `rS`
behind a ring of unspilled registers.

Measured on MMIXware:

| Program | Register | MMIXware |
|---|---|---|
| Caller sets `$1..$5` = 111..555; `PUSHJ $0`; callee sets `$0`=999, `$1`=777; `POP 1,0` | `$0` | 999 |
| | `$1..$5` | 0 |
| | `rL` | 1 |
| Caller sets `$0..$6` = 10..70; `PUSHJ $3`; callee sets `$0,$1,$2` = 801,802,803; `POP 2,0` | `$0..$2` | 10, 20, 30 |
| | `$3` (hole) | 802 |
| | `$4` | 801 |
| | `$5, $6` | 0, 0 |
| | `rL` | 5 |

## Instruction table

| Mnemonic | Operands | Description |
| --- | --- | --- |
| `SET` | `SET $X, $Y` / `SET $X, imm` | MMIXAL alias — emits `ORI $X, $Y, 0` for a register, `SETL $X, imm` for a wyde-wide immediate |
| `SETI` | `SETI $X, imm` | checksmix extension — sets a full 64-bit constant in four tetras, clearing the register |
| `SETL` | `SETL $X, YZ` | Set low wyde; the other 48 bits become zero |
| `SETH` | `SETH $X, YZ` | Set high wyde; the other 48 bits become zero |
| `SETMH` | `SETMH $X, YZ` | Set medium-high wyde; the other 48 bits become zero |
| `SETML` | `SETML $X, YZ` | Set medium-low wyde; the other 48 bits become zero |
| `INCH` | `INCH $X, YZ` | Add into the high wyde; the other 48 bits are preserved |
| `INCMH` | `INCMH $X, YZ` | Add into the medium-high wyde; a carry propagates into the high wyde |
| `INCML` | `INCML $X, YZ` | Add into the medium-low wyde; a carry propagates into the higher wydes |
| `INCL` | `INCL $X, YZ` | Add into the low wyde, unsigned wrapping; a carry propagates into the higher wydes |
| `ORH` | `ORH $X, YZ` | Set bits in the high wyde; the other 48 bits are preserved |
| `ORMH` | `ORMH $X, YZ` | Set bits in the medium-high wyde; the other 48 bits are preserved |
| `ORML` | `ORML $X, YZ` | Set bits in the medium-low wyde; the other 48 bits are preserved |
| `ORL` | `ORL $X, YZ` | Set bits in the low wyde; the other 48 bits are preserved |
| `ANDNH` | `ANDNH $X, YZ` | Clear bits in the high wyde; the other 48 bits are preserved |
| `ANDNMH` | `ANDNMH $X, YZ` | Clear bits in the medium-high wyde; the other 48 bits are preserved |
| `ANDNML` | `ANDNML $X, YZ` | Clear bits in the medium-low wyde; the other 48 bits are preserved |
| `ANDNL` | `ANDNL $X, YZ` | Clear bits in the low wyde; the other 48 bits are preserved |
| `LDB` | `LDB $X, $Y, $Z` | Load byte signed |
| `LDBI` | `LDB $X, $Y, Z` | Load byte signed (immediate) |
| `LDBU` | `LDBU $X, $Y, $Z` | Load byte unsigned |
| `LDBUI` | `LDBU $X, $Y, Z` | Load byte unsigned (immediate) |
| `LDW` | `LDW $X, $Y, $Z` | Load wyde signed |
| `LDWI` | `LDW $X, $Y, Z` | Load wyde signed (immediate) |
| `LDWU` | `LDWU $X, $Y, $Z` | Load wyde unsigned |
| `LDWUI` | `LDWU $X, $Y, Z` | Load wyde unsigned (immediate) |
| `LDT` | `LDT $X, $Y, $Z` | Load tetra signed |
| `LDTI` | `LDT $X, $Y, Z` | Load tetra signed (immediate) |
| `LDTU` | `LDTU $X, $Y, $Z` | Load tetra unsigned |
| `LDTUI` | `LDTU $X, $Y, Z` | Load tetra unsigned (immediate) |
| `LDO` | `LDO $X, $Y, $Z` | Load octa |
| `LDOI` | `LDO $X, $Y, Z` | Load octa (immediate) |
| `LDOU` | `LDOU $X, $Y, $Z` | Load octa unsigned |
| `LDOUI` | `LDOU $X, $Y, Z` | Load octa unsigned (immediate) |
| `LDUNC` | `LDUNC $X, $Y, $Z` | Load octa uncached |
| `LDUNCI` | `LDUNC $X, $Y, Z` | Load octa uncached (immediate) |
| `LDHT` | `LDHT $X, $Y, $Z` | Load high tetra |
| `LDHTI` | `LDHT $X, $Y, Z` | Load high tetra (immediate) |
| `LDSF` | `LDSF $X, $Y, $Z` | Load short float (widen f32 → f64) |
| `LDSFI` | `LDSF $X, $Y, Z` | Load short float (immediate) |
| `LDVTS` | `LDVTS $X, $Y, $Z` | Load virtual translation status |
| `LDVTSI` | `LDVTS $X, $Y, Z` | Load virtual translation status (immediate) |
| `CSWAP` | `CSWAP $X, $Y, $Z` | Compare and swap: if `M8[$Y+$Z] = rP`, store `$X` there and set `$X ← 1`; otherwise `rP ← M8[$Y+$Z]` and `$X ← 0` |
| `CSWAPI` | `CSWAP $X, $Y, Z` | Compare and swap (immediate): if `M8[$Y+Z] = rP`, store `$X` there and set `$X ← 1`; otherwise `rP ← M8[$Y+Z]` and `$X ← 0` |
| `LDA` | `LDA $X, $Y, $Z` / `LDA $X, addr` | Load address of `$Y + $Z` — the `ADDU $X, $Y, $Z` alias; two-operand form described below the table |
| `LDAI` | `LDA $X, $Y, Z` / `LDAI $X, addr` | Load address of `$Y + Z` — the `ADDU $X, $Y, Z` alias; two-operand form described below the table |
| `STB` | `STB $X, $Y, $Z` | Store byte signed |
| `STBI` | `STB $X, $Y, Z` | Store byte signed (immediate) |
| `STBU` | `STBU $X, $Y, $Z` | Store byte unsigned |
| `STBUI` | `STBU $X, $Y, Z` | Store byte unsigned (immediate) |
| `STW` | `STW $X, $Y, $Z` | Store wyde signed |
| `STWI` | `STW $X, $Y, Z` | Store wyde signed (immediate) |
| `STWU` | `STWU $X, $Y, $Z` | Store wyde unsigned |
| `STWUI` | `STWU $X, $Y, Z` | Store wyde unsigned (immediate) |
| `STT` | `STT $X, $Y, $Z` | Store tetra signed |
| `STTI` | `STT $X, $Y, Z` | Store tetra signed (immediate) |
| `STTU` | `STTU $X, $Y, $Z` | Store tetra unsigned |
| `STTUI` | `STTU $X, $Y, Z` | Store tetra unsigned (immediate) |
| `STO` | `STO $X, $Y, $Z` | Store octa |
| `STOI` | `STO $X, $Y, Z` | Store octa (immediate) |
| `STOU` | `STOU $X, $Y, $Z` | Store octa unsigned |
| `STOUI` | `STOU $X, $Y, Z` | Store octa unsigned (immediate) |
| `STUNC` | `STUNC $X, $Y, $Z` | Store octa uncached |
| `STUNCI` | `STUNC $X, $Y, Z` | Store octa uncached (immediate) |
| `STCO` | `STCO X, $Y, $Z` | Store constant octabyte |
| `STCOI` | `STCO X, $Y, Z` | Store constant octabyte (immediate) |
| `STHT` | `STHT $X, $Y, $Z` | Store high tetra |
| `STHTI` | `STHT $X, $Y, Z` | Store high tetra (immediate) |
| `STSF` | `STSF $X, $Y, $Z` | Store short float (narrow f64 → f32, honors rA rounding) |
| `STSFI` | `STSF $X, $Y, Z` | Store short float (immediate) |
| `ADD` | `ADD $X, $Y, $Z` | Add signed (sets overflow) |
| `ADDI` | `ADD $X, $Y, Z` | Add signed immediate |
| `ADDU` | `ADDU $X, $Y, $Z` | Add unsigned (wrapping, same as LDA) |
| `ADDUI` | `ADDU $X, $Y, Z` | Add unsigned immediate |
| `ADDU2` | `2ADDU $X, $Y, $Z` | `$X = 2*$Y + $Z` unsigned |
| `ADDU2I` | `2ADDU $X, $Y, Z` | `$X = 2*$Y + Z` unsigned |
| `ADDU4` | `4ADDU $X, $Y, $Z` | `$X = 4*$Y + $Z` unsigned |
| `ADDU4I` | `4ADDU $X, $Y, Z` | `$X = 4*$Y + Z` unsigned |
| `ADDU8` | `8ADDU $X, $Y, $Z` | `$X = 8*$Y + $Z` unsigned |
| `ADDU8I` | `8ADDU $X, $Y, Z` | `$X = 8*$Y + Z` unsigned |
| `ADDU16` | `16ADDU $X, $Y, $Z` | `$X = 16*$Y + $Z` unsigned |
| `ADDU16I` | `16ADDU $X, $Y, Z` | `$X = 16*$Y + Z` unsigned |
| `SUB` | `SUB $X, $Y, $Z` | Subtract signed (sets overflow) |
| `SUBI` | `SUB $X, $Y, Z` | Subtract signed immediate |
| `SUBU` | `SUBU $X, $Y, $Z` | Subtract unsigned (wrapping) |
| `SUBUI` | `SUBU $X, $Y, Z` | Subtract unsigned immediate |
| `NEG` | `NEG $X, Y, $Z` | `$X = Y − $Z` signed (Y is literal) |
| `NEGI` | `NEG $X, Y, Z` | `$X = Y − Z` signed |
| `NEGU` | `NEGU $X, Y, $Z` | `$X = Y − $Z` unsigned |
| `NEGUI` | `NEGU $X, Y, Z` | `$X = Y − Z` unsigned |
| `MUL` | `MUL $X, $Y, $Z` | Multiply signed |
| `MULI` | `MUL $X, $Y, Z` | Multiply signed immediate |
| `MULU` | `MULU $X, $Y, $Z` | Multiply unsigned (high half in rH) |
| `MULUI` | `MULU $X, $Y, Z` | Multiply unsigned immediate |
| `DIV` | `DIV $X, $Y, $Z` | Divide signed (remainder in rR) |
| `DIVI` | `DIV $X, $Y, Z` | Divide signed immediate |
| `DIVU` | `DIVU $X, $Y, $Z` | Divide unsigned |
| `DIVUI` | `DIVU $X, $Y, Z` | Divide unsigned immediate |
| `FCMP` | `FCMP $X, $Y, $Z` | Floating compare: `$X` = −1/0/+1; unordered operands give 0 and raise I |
| `FUN` | `FUN $X, $Y, $Z` | Floating unordered: `$X` = 1 if NaN |
| `FEQL` | `FEQL $X, $Y, $Z` | Floating equal: `$X` = 1 if equal |
| `FCMPE` | `FCMPE $X, $Y, $Z` | Floating compare with epsilon (rE) |
| `FUNE` | `FUNE $X, $Y, $Z` | Floating unordered with epsilon (rE) |
| `FEQLE` | `FEQLE $X, $Y, $Z` | Floating equivalent with epsilon (rE) |
| `FADD` | `FADD $X, $Y, $Z` | Floating add (honors rA rounding) |
| `FSUB` | `FSUB $X, $Y, $Z` | Floating subtract (honors rA rounding) |
| `FMUL` | `FMUL $X, $Y, $Z` | Floating multiply (honors rA rounding) |
| `FDIV` | `FDIV $X, $Y, $Z` | Floating divide (honors rA rounding) |
| `FREM` | `FREM $X, $Y, $Z` | Floating remainder (IEEE 754 round-half-to-even); a zero remainder takes the dividend's sign |
| `FSQRT` | `FSQRT $X, $Z` / `FSQRT $X, Y, $Z` | Floating square root (honors rA rounding; Y = mode override) |
| `FINT` | `FINT $X, $Z` / `FINT $X, Y, $Z` | Round float to integer (honors rA rounding; Y = mode override) |
| `FIX` | `FIX $X, $Z` / `FIX $X, Y, $Z` | Convert float → signed integer (honors rA rounding; Y = mode override) |
| `FIXU` | `FIXU $X, $Z` / `FIXU $X, Y, $Z` | Convert float → unsigned integer, reduced mod 2^64 (honors rA rounding; Y = mode override) |
| `FLOT` | `FLOT $X, $Z` / `FLOT $X, Y, $Z` | Convert signed integer → float (honors rA rounding; Y = mode override) |
| `FLOTI` | `FLOT $X, Z` / `FLOT $X, Y, Z` | Convert signed integer → float immediate (honors rA rounding; Y = mode override) |
| `FLOTU` | `FLOTU $X, $Z` / `FLOTU $X, Y, $Z` | Convert unsigned integer → float (honors rA rounding; Y = mode override) |
| `FLOTUI` | `FLOTU $X, Z` / `FLOTU $X, Y, Z` | Convert unsigned integer → float immediate (honors rA rounding; Y = mode override) |
| `SFLOT` | `SFLOT $X, $Z` / `SFLOT $X, Y, $Z` | Convert signed integer → short float (honors rA rounding; Y = mode override) |
| `SFLOTI` | `SFLOT $X, Z` / `SFLOT $X, Y, Z` | Convert signed integer → short float immediate (honors rA rounding; Y = mode override) |
| `SFLOTU` | `SFLOTU $X, $Z` / `SFLOTU $X, Y, $Z` | Convert unsigned integer → short float (honors rA rounding; Y = mode override) |
| `SFLOTUI` | `SFLOTU $X, Z` / `SFLOTU $X, Y, Z` | Convert unsigned integer → short float immediate (honors rA rounding; Y = mode override) |
| `CMP` | `CMP $X, $Y, $Z` | Compare signed: `$X` = −1/0/+1 |
| `CMPI` | `CMP $X, $Y, Z` | Compare signed immediate |
| `CMPU` | `CMPU $X, $Y, $Z` | Compare unsigned: `$X` = −1/0/+1 |
| `CMPUI` | `CMPU $X, $Y, Z` | Compare unsigned immediate |
| `AND` | `AND $X, $Y, $Z` | Bitwise AND |
| `ANDI` | `AND $X, $Y, Z` | Bitwise AND immediate |
| `OR` | `OR $X, $Y, $Z` | Bitwise OR |
| `ORI` | `OR $X, $Y, Z` | Bitwise OR immediate |
| `XOR` | `XOR $X, $Y, $Z` | Bitwise XOR |
| `XORI` | `XOR $X, $Y, Z` | Bitwise XOR immediate |
| `ANDN` | `ANDN $X, $Y, $Z` | Bitwise AND-NOT (`$Y & ~$Z`) |
| `ANDNI` | `ANDN $X, $Y, Z` | Bitwise AND-NOT immediate |
| `ORN` | `ORN $X, $Y, $Z` | Bitwise OR-NOT (`$Y | ~$Z`) |
| `ORNI` | `ORN $X, $Y, Z` | Bitwise OR-NOT immediate |
| `NAND` | `NAND $X, $Y, $Z` | Bitwise NAND |
| `NANDI` | `NAND $X, $Y, Z` | Bitwise NAND immediate |
| `NOR` | `NOR $X, $Y, $Z` | Bitwise NOR |
| `NORI` | `NOR $X, $Y, Z` | Bitwise NOR immediate |
| `NXOR` | `NXOR $X, $Y, $Z` | Bitwise XNOR |
| `NXORI` | `NXOR $X, $Y, Z` | Bitwise XNOR immediate |
| `MUX` | `MUX $X, $Y, $Z` | Bitwise multiplex using rM mask |
| `MUXI` | `MUX $X, $Y, Z` | Bitwise multiplex immediate |
| `BDIF` | `BDIF $X, $Y, $Z` | Byte difference (saturating, each byte) |
| `BDIFI` | `BDIF $X, $Y, Z` | Byte difference immediate |
| `WDIF` | `WDIF $X, $Y, $Z` | Wyde difference (saturating) |
| `WDIFI` | `WDIF $X, $Y, Z` | Wyde difference immediate |
| `TDIF` | `TDIF $X, $Y, $Z` | Tetra difference (saturating) |
| `TDIFI` | `TDIF $X, $Y, Z` | Tetra difference immediate |
| `ODIF` | `ODIF $X, $Y, $Z` | Octa difference (saturating) |
| `ODIFI` | `ODIF $X, $Y, Z` | Octa difference immediate |
| `SADD` | `SADD $X, $Y, $Z` | Sideways add (population count of `$Y & ~$Z`) |
| `SADDI` | `SADD $X, $Y, Z` | Sideways add immediate |
| `MOR` | `MOR $X, $Y, $Z` | Matrix OR (boolean 8×8 matrix multiply) |
| `MORI` | `MOR $X, $Y, Z` | Matrix OR immediate |
| `MXOR` | `MXOR $X, $Y, $Z` | Matrix XOR |
| `MXORI` | `MXOR $X, $Y, Z` | Matrix XOR immediate |
| `SL` | `SL $X, $Y, $Z` | Shift left (signed, sets overflow) |
| `SLI` | `SL $X, $Y, Z` | Shift left immediate |
| `SLU` | `SLU $X, $Y, $Z` | Shift left unsigned |
| `SLUI` | `SLU $X, $Y, Z` | Shift left unsigned immediate |
| `SR` | `SR $X, $Y, $Z` | Shift right signed (arithmetic) |
| `SRI` | `SR $X, $Y, Z` | Shift right signed immediate |
| `SRU` | `SRU $X, $Y, $Z` | Shift right unsigned (logical) |
| `SRUI` | `SRU $X, $Y, Z` | Shift right unsigned immediate |
| `JMP` | `JMP addr` | Unconditional jump (24-bit relative offset) |
| `JMPB` | `JMPB addr` | Unconditional jump, backward target required |
| `BN` | `BN $X, addr` | Branch if `$X < 0` |
| `BNB` | `BNB $X, addr` | Branch if `$X < 0` (backward hint) |
| `BZ` | `BZ $X, addr` | Branch if `$X == 0` |
| `BZB` | `BZB $X, addr` | Branch if `$X == 0` (backward hint) |
| `BP` | `BP $X, addr` | Branch if `$X > 0` |
| `BPB` | `BPB $X, addr` | Branch if `$X > 0` (backward hint) |
| `BOD` | `BOD $X, addr` | Branch if `$X` is odd |
| `BODB` | `BODB $X, addr` | Branch if `$X` is odd (backward hint) |
| `BNN` | `BNN $X, addr` | Branch if `$X >= 0` |
| `BNNB` | `BNNB $X, addr` | Branch if `$X >= 0` (backward hint) |
| `BNZ` | `BNZ $X, addr` | Branch if `$X != 0` |
| `BNZB` | `BNZB $X, addr` | Branch if `$X != 0` (backward hint) |
| `BNP` | `BNP $X, addr` | Branch if `$X <= 0` |
| `BNPB` | `BNPB $X, addr` | Branch if `$X <= 0` (backward hint) |
| `BEV` | `BEV $X, addr` | Branch if `$X` is even |
| `BEVB` | `BEVB $X, addr` | Branch if `$X` is even (backward hint) |
| `PBN` | `PBN $X, Y, Z` | Probable branch if negative |
| `PBNB` | `PBNB $X, Y, Z` | Probable branch if negative (backward) |
| `PBZ` | `PBZ $X, Y, Z` | Probable branch if zero |
| `PBZB` | `PBZB $X, Y, Z` | Probable branch if zero (backward) |
| `PBP` | `PBP $X, Y, Z` | Probable branch if positive |
| `PBPB` | `PBPB $X, Y, Z` | Probable branch if positive (backward) |
| `PBOD` | `PBOD $X, Y, Z` | Probable branch if odd |
| `PBODB` | `PBODB $X, Y, Z` | Probable branch if odd (backward) |
| `PBNN` | `PBNN $X, Y, Z` | Probable branch if non-negative |
| `PBNNB` | `PBNNB $X, Y, Z` | Probable branch if non-negative (backward) |
| `PBNZ` | `PBNZ $X, Y, Z` | Probable branch if non-zero |
| `PBNZB` | `PBNZB $X, Y, Z` | Probable branch if non-zero (backward) |
| `PBNP` | `PBNP $X, Y, Z` | Probable branch if non-positive |
| `PBNPB` | `PBNPB $X, Y, Z` | Probable branch if non-positive (backward) |
| `PBEV` | `PBEV $X, Y, Z` | Probable branch if even |
| `PBEVB` | `PBEVB $X, Y, Z` | Probable branch if even (backward) |
| `CSN` | `CSN $X, $Y, $Z` | Conditional set if `$Y < 0` |
| `CSNI` | `CSNI $X, $Y, Z` | Conditional set if `$Y < 0` (immediate) |
| `CSZ` | `CSZ $X, $Y, $Z` | Conditional set if `$Y == 0` |
| `CSZI` | `CSZI $X, $Y, Z` | Conditional set if `$Y == 0` (immediate) |
| `CSP` | `CSP $X, $Y, $Z` | Conditional set if `$Y > 0` |
| `CSPI` | `CSPI $X, $Y, Z` | Conditional set if `$Y > 0` (immediate) |
| `CSOD` | `CSOD $X, $Y, $Z` | Conditional set if `$Y` is odd |
| `CSODI` | `CSODI $X, $Y, Z` | Conditional set if `$Y` is odd (immediate) |
| `CSNN` | `CSNN $X, $Y, $Z` | Conditional set if `$Y >= 0` |
| `CSNNI` | `CSNNI $X, $Y, Z` | Conditional set if `$Y >= 0` (immediate) |
| `CSNZ` | `CSNZ $X, $Y, $Z` | Conditional set if `$Y != 0` |
| `CSNZI` | `CSNZI $X, $Y, Z` | Conditional set if `$Y != 0` (immediate) |
| `CSNP` | `CSNP $X, $Y, $Z` | Conditional set if `$Y <= 0` |
| `CSNPI` | `CSNPI $X, $Y, Z` | Conditional set if `$Y <= 0` (immediate) |
| `CSEV` | `CSEV $X, $Y, $Z` | Conditional set if `$Y` is even |
| `CSEVI` | `CSEVI $X, $Y, Z` | Conditional set if `$Y` is even (immediate) |
| `ZSN` | `ZSN $X, $Y, $Z` | Zero or set `$Z` into `$X` if `$Y < 0` |
| `ZSNI` | `ZSNI $X, $Y, Z` | Zero or set immediate if `$Y < 0` |
| `ZSZ` | `ZSZ $X, $Y, $Z` | Zero or set if `$Y == 0` |
| `ZSZI` | `ZSZI $X, $Y, Z` | Zero or set immediate if `$Y == 0` |
| `ZSP` | `ZSP $X, $Y, $Z` | Zero or set if `$Y > 0` |
| `ZSPI` | `ZSPI $X, $Y, Z` | Zero or set immediate if `$Y > 0` |
| `ZSOD` | `ZSOD $X, $Y, $Z` | Zero or set if `$Y` is odd |
| `ZSODI` | `ZSODI $X, $Y, Z` | Zero or set immediate if `$Y` is odd |
| `ZSNN` | `ZSNN $X, $Y, $Z` | Zero or set if `$Y >= 0` |
| `ZSNNI` | `ZSNNI $X, $Y, Z` | Zero or set immediate if `$Y >= 0` |
| `ZSNZ` | `ZSNZ $X, $Y, $Z` | Zero or set if `$Y != 0` |
| `ZSNZI` | `ZSNZI $X, $Y, Z` | Zero or set immediate if `$Y != 0` |
| `ZSNP` | `ZSNP $X, $Y, $Z` | Zero or set if `$Y <= 0` |
| `ZSNPI` | `ZSNPI $X, $Y, Z` | Zero or set immediate if `$Y <= 0` |
| `ZSEV` | `ZSEV $X, $Y, $Z` | Zero or set if `$Y` is even |
| `ZSEVI` | `ZSEVI $X, $Y, Z` | Zero or set immediate if `$Y` is even |
| `PUSHJ` | `PUSHJ $X, addr` | Push registers and jump; return address in `rJ` |
| `PUSHJB` | `PUSHJB $X, addr` | Push registers and jump (backward hint) |
| `PUSHGO` | `PUSHGO $X, $Y, $Z` | Push registers and jump to `$Y + $Z` |
| `PUSHGOI` | `PUSHGO $X, $Y, Z` | Push registers and jump to `$Y + Z` |
| `POP` | `POP X, YZ` | Pop registers and return; the hole gets the last of the X returned values, the rest land above it in order |
| `GO` | `GO $X, $Y, $Z` | Jump to `$Y + $Z`; save next PC in `$X` |
| `GOI` | `GO $X, $Y, Z` | Jump to `$Y + Z`; save next PC in `$X` |
| `GETA` | `GETA $X, addr` | Get relative address into `$X` |
| `GETAB` | `GETAB $X, addr` | Get relative address (backward hint) |
| `GET` | `GET $X, Z` | Read special register Z into `$X`; `Z ≥ 32` halts |
| `PUT` | `PUT X, $Z` | Write `$Z` into special register X; `X ≥ 32` halts; `rC rN rO rS rI rT rTT rK rQ rU rV` (8–18) are read-only in user mode; `rG` must be 32–255 and at least `rL`; `rA` at most `#3FFFF` |
| `PUTI` | `PUT X, Z` | Write immediate Z into special register X; same rejections as `PUT` |
| `SAVE` | `SAVE $X, 0` | Push a context onto the register stack; `$X` (global) receives its address |
| `UNSAVE` | `UNSAVE 0, $Z` | Restore the context `$Z` addresses from the register stack |
| `RESUME` | `RESUME XYZ` | Resume after interrupt or trip |
| `TRAP` | `TRAP X, Y, Z` | System call (see TRAP interface above) |
| `HALT` | `HALT` | checksmix extension — encodes as `TRAP 0,Halt,0` |
| `TRIP` | `TRIP X, Y, Z` | Forced trip (software interrupt) |
| `SYNC` | `SYNC XYZ` | Synchronize memory/pipeline |
| `SWYM` | `SWYM` / `SWYM X, Y, Z` | Sympathize with your machinery (no-op); operands optional, default to zero |
| `PRELD` | `PRELD $X, $Y, $Z` | Prefetch data into cache |
| `PRELDI` | `PRELD $X, $Y, Z` | Prefetch data (immediate) |
| `PREGO` | `PREGO $X, $Y, $Z` | Prefetch for execution |
| `PREGOI` | `PREGO $X, $Y, Z` | Prefetch for execution (immediate) |
| `PREST` | `PREST $X, $Y, $Z` | Prestore data |
| `PRESTI` | `PREST $X, $Y, Z` | Prestore data (immediate) |
| `SYNCD` | `SYNCD $X, $Y, $Z` | Synchronize data cache |
| `SYNCDI` | `SYNCD $X, $Y, Z` | Synchronize data cache (immediate) |
| `SYNCID` | `SYNCID $X, $Y, $Z` | Synchronize instruction and data cache |
| `SYNCIDI` | `SYNCID $X, $Y, Z` | Synchronize instruction and data cache (immediate) |

checksmix parses the `X` operand of `PRELD`, `PREGO`, `PREST`, `SYNCD` and
`SYNCID` as a register. In MMIX, `X` is an immediate byte count: `PRELD
X,$Y,$Z` covers the `X+1` bytes `M[$Y+$Z]` through `M[$Y+$Z+X]`. So
`PRELD 7,$1,$2` fails to assemble here; write `PRELD $7,$1,$2`, which
emits the same tetra `#9A070102`.

`LDA`/`LDAI $X, addr` resolve at assemble time by whether `addr` fits a byte.
An `addr` of 0 to 255 assembles to a single tetra: `LDAI` correctly emits a
register-immediate `ADDUI $X, $0, addr`, but `LDA` emits the
register-register `ADDU $X, $0, addr` instead — the address ends up in the Z
*register* field, so the assembled instruction adds whatever register `addr`
names rather than the literal value. Both forms also assume register `$0`
holds zero, which nothing in checksmix enforces. This is a known gap against
the code's own intent; it is a code change and out of scope here. An `addr`
above 255 expands to a four-tetra `SETH`/`INCMH`/`INCML`/`INCL` sequence that
clears `$X` and loads the full 64-bit value, correct for both forms.

`GETA`'s `addr` must be 4-byte aligned relative to the current instruction. A
forward target reaches 0 to 262140 bytes ahead (an unsigned count of 0 to
65535 tetras); a backward target is accepted too and is encoded as `GETAB`
automatically, reaching up to 262144 bytes behind (an unsigned count of 1 to
65536 tetras). The two directions don't mirror: a forward delta of zero
tetras takes the field's first value, so `GETAB`'s all-backward field has one
more tetra of reach than `GETA`'s forward-only side. `GETAB` written
directly enforces the same backward-only range and rejects a forward target
outright. Targets that are out of range, misaligned, or (for `GETAB`) not
behind the current instruction are a hard assembly-time error naming the
byte figure; use `LDA` for addresses that don't fit either field.
