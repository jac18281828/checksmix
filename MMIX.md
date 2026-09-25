# MMIX Instruction Quick Reference

MMIX is a 64-bit big-endian RISC machine (Knuth, 1999) with 256 general-purpose registers (`$0`–`$255`), a separate special-register file, byte-addressed memory, and fixed 32-bit instructions. Immediates in assembly may be decimal, hexadecimal (`#`-prefixed, or `0x`/`0X`-prefixed — also a checksmix extension), or character literals — one quote, one character, one quote, the character possibly a quote itself, so `'''` is the apostrophe; a character literal's value is that character's Unicode scalar value: `'é'` is `#E9`, `'算'` is `#7B97`. Every operand is an MMIXAL expression (see "Expressions" below). A leading `0` is an ordinary decimal digit, as in MMIXAL — `SET $1,010` loads 10, and there is no octal spelling. A string literal has no escape mechanism either: its content is exactly what it spells, one unit per character, each its character's constant (a data directive's own per-character rule for a string is in "Assembler directives" below).

## Memory access

MMIX has no unaligned access. A wyde, tetra, or octa access at address `A`
resolves to `w·⌊A/w⌋` for its width `w` (2, 4, or 8) — the low `log2(w)` bits
of `A` are ignored. `LDO $X,$Y,$Z` with an address ending in 3 loads the
octabyte at the aligned base below it, not eight bytes straddling two
octabytes. A misaligned address is rounded, never rejected: there is no trap
or diagnostic. Byte access is unaffected — a byte is its own alignment.

Every load, store, `GO` and cache instruction that auto-selects its register
or immediate opcode also takes a two-operand form: `LDO $X,$Y` fills Z with
`0`, a register `$Y` read as an offset of zero. A pure `$Y` is instead an
address: the assembler resolves it against the largest `GREG`-allocated base
register, among those declared earlier in the source with a nonzero initial
value, whose value is no more than 255 below it, and emits the three-operand
form with that base in Y and the remaining offset in Z. No such base is an
error: `no GREG before this instruction holds a base address 0 to 255 bytes
below {addr:#x}`. The form always assembles one tetra. `LDA` does not take
this path; its two-operand form is the address-loading alias described
below.

## Minimal assembly skeleton

```
        LOC     #100        % set load address to 0x100
        GREG    @           % allocate a base register (optional)
Main    SETL    $0,42       % your code here
        TRAP    0,Halt,0    % halt, exit code in $255
```

A program starts with `$255` holding its entry address — `Main`'s, or the
first instruction's when there is no `Main` (MMIXAL reference). rG starts at
255 minus the number of `GREG`s the program declares — every register but
`$255` is local with none — and rL at 0, `$0` and `$1` zero (MMIX passes argc
and argv there; checksmix's command line passes no arguments). rK, rT, rTT
and rV start `#FFFFFFFFFFFFFFFF`, `#8000000500000000`, `#8000000600000000`
and `#369C200400000000` (TAOCP Vol. 1 Fascicle 1, p. 90). A byte two
statements assemble to the same address loads as their XOR, not the second
overwriting the first.

## Line structure

A statement holds a LABEL field, an OP field and an EXPR field, each
separated from the next by a blank; every field but OP is optional (Knuth,
*The Art of Computer Programming*, Volume 1, Fascicle 1, §1.3.2′ "The MMIX
Assembly Language", p. 34). A label with nothing but blanks and a comment
after it is a statement on its own; anything else there means the OP field
held a word the assembler doesn't recognize, reported as an unknown
operation with the statement printed in full.

`;` separates statements — `SETL $1,1; ADD $1,$1,1` assembles both — and
needs no blank on either side. A statement after a `;` is read exactly like
one at the start of a line, label field included.

`%` is the only comment character. It runs to the end of the line and wins
over a later `;`: in `SETL $1,1 % note; ADD` the `; ADD` sits inside the
comment, so no second statement begins.

A line whose first character is not a letter, a digit, `:` or `_` is a
comment in its entirety — `;`, `*`, `#`, `/` and `-` all open one this way.
An indented line has no label field: its first word is always the OP field,
whatever it spells. A statement after `;` keeps its label field regardless
of indentation. Mnemonics and directives match in upper case only; a
lower-case or mixed-case spelling is an ordinary symbol.

Text past EXPR is a **remark** — Knuth's own word, from his listings'
Remarks column, for the commentary his prose permits there. Two rules
govern it: EXPR is greedy, taking the longest operand field the grammar
reads; and whatever it leaves behind is a remark unless it is mistakable
for part of the statement. Text disqualifies itself from being a remark by:

- abutting the statement, with no blank marking where the statement ended;
- opening with one of `, + - * / ~ & | ^ < > $`, any of which could extend
  an expression or an operand list — `/` is in this set, so `SET $1,2 / 3`
  is an error though `SET $1,2/3` (no blank) still divides to `0` inside
  the expression itself;
- opening with a digit, almost always a dropped operand separator rather
  than commentary, so `HALT 2 apples` is an error, not a warning.

`ADD $1,$2,$3 sum of the parts` assembles, the trailing words a permitted
remark; `SETL $1,2 + 3` is a syntax error, `+ 3` read as continuing the
expression rather than dropped in silence. Text with no statement ahead of
it passes the same test: ignored when it passes (`    # note` assembles),
an unknown operation when it fails (`SETL $1,1;9foo` reports
`unknown operation: 9foo`).

Whitespace around an operand list's commas stays legal — `TRAP 0, Time, 2`
parses — a checksmix extension over MMIXAL, which ends the operand field at
the first blank.

## Assembler directives

| Directive | Syntax | Effect |
| --- | --- | --- |
| `LOC` | `LOC expr` | Set the assembly location counter to *expr*; a label on the same line names the location *before* the move |
| `GREG` | `[label] GREG expr` / `[label] GREG` | Allocate a global register initialized to *expr*, or to `0` with the operand omitted; optional label becomes a register alias; a nonzero value is a base address for the two-operand memory form |
| `IS` | `Name IS expr` | Define a numeric or register alias constant |
| `PREFIX` | `PREFIX str` | Qualify subsequent unqualified names as `str<name>`; names beginning with `:` opt out |
| `BYTE` | `BYTE expr,...` | Emit one byte per operand |
| `WYDE` | `WYDE expr,...` | Emit one 16-bit wyde per operand |
| `TETRA` | `TETRA expr,...` | Emit one 32-bit tetra per operand |
| `OCTA` | `OCTA expr,...` | Emit one 64-bit octa per operand |
| `LOCAL` | `LOCAL expr` | Declare register *expr* local; checked against the global threshold at the close of assembly |
| `BSPEC` | `BSPEC expr` | Open special mode |
| `ESPEC` | `ESPEC` | Close special mode |
| `INCLUDE` | `INCLUDE file` | Assemble the named file as if inserted here, resolved relative to the including file; recursive, cycles are an error |

A string operand assembles one unit per character. The directive aligns
once, before the first unit; a list does not realign between items. A bare
`""`, the whole item, assembles as one zero unit and warns; the same empty
string beside an operator or inside parentheses is an error.

A string also stands inside a data-list item's own expression, abbreviating
its characters as comma-separated character constants: an operator before
the string applies to its first character and one after it to its last, so
`BYTE 1+"ace"+2,0` is `BYTE 1+'a','c','e'+2,0` — four bytes, `b`, `c`, `g`,
`0`. A string alone as an item keeps the rule above. This expansion reaches
data-list items only; a string is not a valid instruction operand.

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

`BSPEC expr` opens special mode; `ESPEC` closes it. Inside, only `IS`,
`PREFIX`, `GREG`, `LOCAL` and the four data directives are legal — an
instruction or any other directive between them is an error, and `BSPEC`
does not nest. `IS`, `PREFIX`, `GREG` and `LOCAL` keep their full effect
there; the location counter does not move, and a `BYTE`/`WYDE`/`TETRA`/`OCTA`
list inside emits nothing to the object file at all — no MMIX loader would
have loaded it anyway. A label after `ESPEC` has exactly the address it would
have if the whole block were deleted.

### Expressions

Every operand — a register, an immediate, `LOC`'s target, a data item — is an
MMIXAL expression: constants, symbols, `@`, unary operators, and two
left-associative precedence levels of binary operators. A decimal or
hexadecimal constant always has a value, however many digits it spells: one
of 2⁶⁴ or more reduces mod 2⁶⁴, so `OCTA #112233445566778899` assembles
`#2233445566778899` and `OCTA 18446744073709551621` assembles `5`. Every
field an operand fills has a range; see
[Operand ranges](#operand-ranges).

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
to the remark rules in "Line structure" above. Write a negative
literal closed up:
`SETI $1,-5`, never `SETI $1,- 5`. A parenthesized group is the one place an
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
expression, but the final value a register site consumes must fit `0..255`,
same as a bare `$256` today.

A program may redefine a predefined symbol with a label, `IS` or `GREG`
before any use of it, and its definition then holds at every reference; a
symbol may be defined again only with the same value.

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

### Local symbols

A decimal digit followed by `H` defines a local label; the same digit
followed by `B` or `F` references it backward or forward in an operand. `H`
is legal only in the label field, `B`/`F` only in an operand, and all three
are upper case only. Ten counters run independently, one per digit.

`dB` is the address of the last `dH` of that digit at or before the
referencing statement, or `0` when none has appeared yet — never an error.
`dF` is the address of the first `dH` of that digit after the referencing
statement, and an error when none follows. Resolution follows source order,
not address, so a `LOC` that moves the counter backward does not change what
a later `dB`/`dF` sees. `dH` is redefinable: a second `2H` is not a
redefinition error, which is what makes `9H IS 9B+1` a running counter.

```
2H      JMP     2F      % forward: to the second 2H below
        JMP     TestFail
2H      JMP     2B      % backward: to the first 2H above
```

### INCLUDE

`INCLUDE file` is a **checksmix extension**, not part of
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

The active prefix starts at `:`, the root namespace; `PREFIX :` returns to
it. `x` and `:x` name the same symbol there, and `MMixAssembler::labels`/
`symbols` key a root name without its colon. A label or operand that begins
with `:` opts out of the active `PREFIX`; unqualified names are prefixed by
the active `PREFIX` string instead.

A symbol may carry interior colons: an operand may name a qualified symbol
directly, `Foo:Bar`, and the active `PREFIX` still applies to it unless it
begins with `:`. A symbol never ends with a colon, so the legacy `Label:`
spelling still defines `Label` — but a blank must follow the colon, since
`Label:SET` reads as one qualified name rather than a label and a
mnemonic.

```
        PREFIX  P_
Foo     TRAP    0,Halt,0    % stored as "P_Foo"
:Bar    TRAP    0,Halt,0    % stored as "Bar" (root, PREFIX not applied)
        PREFIX  Lib:
Sub     TRAP    0,Halt,0    % stored as "Lib:Sub"
        PREFIX  :
        SET     $1,Lib:Sub  % a qualified reference names a symbol directly
```

### Multi-source assembly

`checksmix` and `mmixasm` accept multiple `.mms` inputs in one invocation. All files share one symbol space and one byte stream, assembled as if concatenated in command-line order.

```
checksmix run   main.mms lib.mms
checksmix check main.mms lib.mms
checksmix build -o prog.mmo main.mms lib.mms
mmixasm         main.mms lib.mms -o prog.mmo
```

### The `.mmo` object format

`build` writes the MMIXAL reference's object format: a preamble with a
zero timestamp (so the same source always builds the same bytes), one
location record and its data tetras per contiguous run of assembled bytes, a
`debug` string table, and a postamble carrying every `GREG`-initialized
register through `$255`'s entry point. `run` on a `.mmo` reads exactly that
shape back and rejects anything else — a stray record, a foreign-shaped one,
or an unrecognized preamble version — rather than risk loading it wrong. A
`.mmo` built by checksmix 0.3.12 or earlier must be rebuilt.

## Floating-point arithmetic

All floating-point instructions use IEEE 754 double precision. Positive
infinity is the predefined constant `Inf`, rather than a bit pattern the
reader must spell out. Results honor the **rounding mode** in bits 17–16 of special register `rA` (register 21). `rA` is 18 bits wide, so the mode field sits at its top: `PUT rA,$X` above `#3FFFF` is an illegal-instruction interrupt, and since this VM has no interrupt vector, it halts with a diagnostic and exits 1 — every halt but the `TRAP 0,Halt,0` trap does. `PUTI` cannot reach the field — its operand is `Z` alone, eight bits — so selecting a mode needs the register form of `PUT`:

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

**NaN results.** `FADD`, `FSUB`, `FMUL`, `FDIV`, `FREM`, `FSQRT`, and `FINT`
build every NaN result themselves rather than take whatever the host CPU's
arithmetic produces, so a result is identical on every target. A signaling
NaN operand raises I; every NaN operand is quieted (its fraction's top bit
set) before it can appear in a result. For a binary operation the result is
`$Z` if `$Z` is a NaN, otherwise `$Y`; `FSUB` negates `$Z` only when `$Z` is
not a NaN, since it computes `$Y + (−$Z)`. `FIX` and `FIXU` instead copy an
infinite or NaN operand through unchanged, raising I alone.

Each invalid operation (a NaN operand aside) yields `NaN(1/2)`
(`#7FF8000000000000`, or with the sign bit set) and raises I: `FADD`'s
∞ + (−∞) is signed as `$Z`'s; `FSUB`'s ∞ − ∞ as the negated `$Z`'s; `FMUL`'s
`0 × ∞` and `FDIV`'s `0/0` and `∞/∞` by the operands' sign product; `FREM`
with an infinite `$Y` or a zero `$Z` as `$Y`'s; `FSQRT` of a negative
operand (∞ included) always negative.

**Signed zero.** An exactly-zero `FADD`/`FSUB` result is `+0` in every
rounding mode but ROUND_DOWN, except `(−0) + (−0) = −0`. In ROUND_DOWN it is
`−0`, except `(+0) + (+0) = +0`.

### rA event flags

An arithmetic exception whose enable bit (below) is clear ORs its event flag
into `rA`; event flags are never cleared automatically. One whose enable bit
is set trips to its handler instead, and its event flag stays clear — see
"User trips". The bit values are the predefined symbols `D_BIT` … `X_BIT`,
rather than spellings a program must supply itself.

| Flag | rA bit | Kind | Raised when |
| --- | --- | --- | --- |
| X | `0x01` | floating | Result is inexact (rounded) |
| Z | `0x02` | floating | A finite nonzero dividend divided by zero (`FDIV`); alone, never with O or X |
| U | `0x04` | floating | Underflow |
| O | `0x08` | floating | Overflow: the value, rounded in rA's current mode with the exponent unbounded, exceeds the largest finite number — the largest short float for `STSF`/`STSFI`, the largest double for `FADD`/`FSUB`/`FMUL`/`FDIV`; always with X |
| I | `0x10` | floating | A signaling NaN operand; a quiet one does not raise it. Also an invalid operation (0/0, ∞−∞, etc.), or `FIX`/`FIXU` of an infinite or NaN operand |
| W | `0x20` | floating | `FIX`'s rounded result falls below `−2^63` or above `2^63 − 1`; `FIXU` never raises it |
| V | `0x40` | integer | Integer overflow — `ADD`, `SUB`, `MUL`, `NEG`, `DIV` of `#8000000000000000` by −1, `SL`, and the signed stores `STB`/`STW`/`STT` |
| D | `0x80` | integer | Divide check — signed division by zero |

On overflow the delivered result depends on rA's mode: ROUND_NEAR gives
±∞; ROUND_OFF gives ±the format's largest finite value; ROUND_UP gives +∞
for a positive result and −the largest finite value for a negative one;
ROUND_DOWN mirrors it, +the largest finite value and −∞. O follows the
value rounded in rA's mode with the exponent unbounded: rounding to
exactly the largest finite value is not itself overflow — ROUND_OFF of the
largest double plus half its ulp rounds, exponent unbounded, back down to
exactly that value and raises X alone, not O — but rounding, exponent
unbounded, past that value does raise O, even where the ordinary
(exponent-bounded) round-to-nearest result was itself still finite.

There is no denormalized-operand event: a subnormal operand raises nothing.
A rounded result below the normal range raises `U` only when the exact
result is not itself an exact subnormal, or when `U`'s enable bit is set (an
enabled `U` trips on a subnormal or zero result, exact or not, except an
`FADD`/`FSUB` with a zero operand or a zero sum); an underflow with the bit
clear always raises `U` and `X` together. `FREM` and
`FSQRT` raise `U` in no case — the IEEE remainder is exact by definition,
and the square root of a nonzero finite operand is neither zero nor
subnormal. `DIVU` raises no divide check, because `u($Z) ≤ u(rD)` — which
includes a zero divisor — is part of its definition rather than an error.
`±∞` divided by `±0` raises nothing: the result is an exact infinity.

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
event flag, control transfers to a fixed handler address, the predefined
symbols `D_Handler` … `X_Handler` (`#10` `#20` `#30` `#40` `#50` `#60` `#70`
`#80`) for `D V W I O U Z X` respectively. An
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

`Fopen`'s name is the bytes at its address up to the first zero byte,
passed to the host unchanged, capped at the same per-call length as
`Fputs` below. A name with no zero within the bound, or one that is not
valid UTF-8, fails with −1 and touches no file.

`Fgets` reads until `size − 1` characters or a newline, then a zero byte,
returning the count stored (a partial last line at end of file included), or
−1 when `size` is 0 or nothing was read. `Fgetws`/`Fputws` move wyde
characters, two bytes each in memory order, raw to and from the file:
`Fgetws` rounds its buffer address down to even and stops at the wyde
`#000A`, `size − 1` wydes, or end of file; `Fputws` writes up to, not
including, the first zero wyde. `Fputs` writes up to, not including, the
first zero byte, with no byte value translated. `Fputs` and `Fputws` cap
each call at 1,048,576 bytes and 524,288 wydes; a string of exactly the
cap, followed by its zero, writes whole, and a longer one writes that
many, reports a diagnostic, and returns the count actually written in
`$255`. `Fseek`'s offset, `≥ 0`, positions that many bytes from the start;
`< 0` positions `−offset − 1` bytes before the end, so `−1` is the end
itself.

Handles 0, 1 and 2 (`StdIn`, `StdOut`, `StdErr`, the predefined symbols'
values) are open at start with `TextRead`, `TextWrite`, `TextWrite`.
**Departure from the reference:** `Fopen`/`Fclose` on any of them return −1
and leave the stream as it was, rather than letting the program rebind them
— checksmix routes handles 1 and 2 through the host's own write, which has
no file underneath to rebind, and a `StdIn` read always fails, since the
host has no read primitive.

**Departure from the reference:** the reference places no length limit on
`Fopen`'s name, `Fputs`, or `Fputws`, and accepts any name the host
filesystem does; checksmix caps the three calls as above and rejects an
`Fopen` name that is not valid UTF-8.

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

The directive's text is taken exactly as written between the quotes, as its
own UTF-8 bytes, and lives in a table outside guest memory: nothing is
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

`UNSAVE 0, $Z`, or its one-operand spelling `UNSAVE $Z`, restores a context
whose topmost (packed) octa `$Z` addresses, validating it whole before
changing anything: a packed `rG` outside
`32..255`, a packed `rA` above the widest legal value, or a saved local
count greater than the packed `rG` all halt with a diagnostic and the
machine unchanged. Otherwise every saved register restores, `rL` becomes the
saved local count, and `rO = rS` land at the address of the first restored
local — where `rO` stood before the matching `SAVE`.

A nonzero must-be-zero field — `SAVE`'s `Y` and `Z`, `UNSAVE`'s `X` and `Y` —
is an illegal-instruction interrupt, which halts. `GET`, `PUT` and `RESUME`
carry the same rule: `GET`'s and `PUT`'s `Y`, and `RESUME`'s `X` and `Y`, must
also be zero.

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

## Operand counts

MMIXAL's general rule: three operands fill X, Y and Z; two fill X and YZ;
one fills XYZ; an empty operand field is the single operand `0`. Most
mnemonics take a fixed count. These take the range the reference allows,
each field a pure byte or a register unless noted:

| Form | Fields |
| --- | --- |
| `TRAP x,y,z` / `TRIP` / `SWYM` | X, Y, Z |
| `TRAP x,yz` / `TRIP` / `SWYM` | X=x, Y=yz>>8, Z=yz&255 |
| `TRAP xyz` / `TRIP` / `SWYM` | X=xyz>>16, Y=(xyz>>8)&255, Z=xyz&255 |
| bare `TRAP` / `TRIP` / `SWYM` | every field 0 |
| `POP p,yz` | X=p, YZ=yz |
| `POP xyz` | XYZ=xyz |
| bare `POP` | every field 0 |
| bare `RESUME` / `SYNC` | XYZ=0; the one-operand form is unchanged |
| `UNSAVE $Z` | the one-operand spelling of `UNSAVE 0,$Z` |
| bare `UNSAVE` / bare `SAVE` | error |
| `NEG $X,z` / `NEGU $X,z` | `NEG $X,0,z`: Y omitted is 0 |
| `label GREG` with no operand | a global register holding 0 |
| `PUSHJ`, `PUSHJB`, `PUSHGO` X | a pure byte or a register, same bytes; `GO`'s X stays a register |
| `PRELD`, `PREGO`, `PREST`, `SYNCD`, `SYNCID`, `STCO` X | a pure byte or a register, same bytes |

The longest matching form wins, so `TRAP 0,1,2` fills every field rather
than leaving `,2` behind. A partial list — `TRAP 0,`, `POP 1,` — is still an
error.

## Operand ranges

Every value that fills an instruction field must fit that field's range or
assembly fails at the operand:

| Field | Range | Sites |
| --- | --- | --- |
| Special register | `0..31` | `GET`'s `Z`; `PUT`'s and `PUTI`'s `X` |
| Byte | `0..255` | `Z` of every explicit `*I` three-operand spelling, `STCO`'s `X` included; `Y` of `NEG`/`NEGU` and of the float rounding-mode forms; `PUTI`'s `Z`; `SAVE`'s `Z`; `UNSAVE`'s `X`; `POP p,yz`'s `X` |
| Wyde | `0..65535` | the sixteen wyde immediates `SETL`, `SETH`, `SETMH`, `SETML`, `INCL`, `INCH`, `INCMH`, `INCML`, `ORH`, `ORMH`, `ORML`, `ORL`, `ANDNH`, `ANDNMH`, `ANDNML`, `ANDNL`; `yz` of `TRAP`/`TRIP`/`SWYM` and `POP` |
| Three bytes | `0..16777215` | `xyz` of `TRAP`/`TRIP`/`SWYM` and `POP`; `RESUME`; `SYNC` |

A negative value is its 64-bit two's complement and fails every field
narrower than 64 bits: `ADDI $1,$2,-1` is an error, as is `SET $1,-1`
(naming `SETI` and `NEG`). `RESUME` and `SYNC` take the MMIXAL definition's
24-bit `XYZ`, all three bytes reaching the encoding; the machine halts on a
`SYNC` code above 7 regardless.

This departs from the MMIXAL reference, which warns on an out-of-range
instruction field and keeps its low bits rather than rejecting it; a
truncated field would otherwise assemble a different instruction.

A data directive's value follows the reference instead: `BYTE`, `WYDE` and
`TETRA` warn and keep the value's low byte, wyde or tetra when it overflows
that width; `OCTA` never overflows. A string's characters are data values
like any other. A bare `""` — the whole item — assembles as one zero unit
of the directive's width and warns; the same empty string beside an
operator or inside parentheses stays an error.

## Instruction table

| Mnemonic | Operands | Description |
| --- | --- | --- |
| `SET` | `SET $X, $Y` / `SET $X, imm` | MMIXAL alias — emits `ORI $X, $Y, 0` for a register, `SETL $X, imm` for a wyde-wide immediate; an immediate outside `0..#FFFF` is an error (see [Operand ranges](#operand-ranges)) |
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
| `LDB` | `LDB $X, $Y, $Z` / `LDB $X, $Y` | Load byte signed (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDBI` | `LDB $X, $Y, Z` | Load byte signed (immediate) |
| `LDBU` | `LDBU $X, $Y, $Z` / `LDBU $X, $Y` | Load byte unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDBUI` | `LDBU $X, $Y, Z` | Load byte unsigned (immediate) |
| `LDW` | `LDW $X, $Y, $Z` / `LDW $X, $Y` | Load wyde signed (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDWI` | `LDW $X, $Y, Z` | Load wyde signed (immediate) |
| `LDWU` | `LDWU $X, $Y, $Z` / `LDWU $X, $Y` | Load wyde unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDWUI` | `LDWU $X, $Y, Z` | Load wyde unsigned (immediate) |
| `LDT` | `LDT $X, $Y, $Z` / `LDT $X, $Y` | Load tetra signed (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDTI` | `LDT $X, $Y, Z` | Load tetra signed (immediate) |
| `LDTU` | `LDTU $X, $Y, $Z` / `LDTU $X, $Y` | Load tetra unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDTUI` | `LDTU $X, $Y, Z` | Load tetra unsigned (immediate) |
| `LDO` | `LDO $X, $Y, $Z` / `LDO $X, $Y` | Load octa (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDOI` | `LDO $X, $Y, Z` | Load octa (immediate) |
| `LDOU` | `LDOU $X, $Y, $Z` / `LDOU $X, $Y` | Load octa unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDOUI` | `LDOU $X, $Y, Z` | Load octa unsigned (immediate) |
| `LDUNC` | `LDUNC $X, $Y, $Z` / `LDUNC $X, $Y` | Load octa uncached (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDUNCI` | `LDUNC $X, $Y, Z` | Load octa uncached (immediate) |
| `LDHT` | `LDHT $X, $Y, $Z` / `LDHT $X, $Y` | Load high tetra (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDHTI` | `LDHT $X, $Y, Z` | Load high tetra (immediate) |
| `LDSF` | `LDSF $X, $Y, $Z` / `LDSF $X, $Y` | Load short float (widen f32 → f64) (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDSFI` | `LDSF $X, $Y, Z` | Load short float (immediate) |
| `LDVTS` | `LDVTS $X, $Y, $Z` / `LDVTS $X, $Y` | Load virtual translation status (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `LDVTSI` | `LDVTS $X, $Y, Z` | Load virtual translation status (immediate) |
| `CSWAP` | `CSWAP $X, $Y, $Z` / `CSWAP $X, $Y` | Compare and swap: if `M8[$Y+$Z] = rP`, store `$X` there and set `$X ← 1`; otherwise `rP ← M8[$Y+$Z]` and `$X ← 0` (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `CSWAPI` | `CSWAP $X, $Y, Z` | Compare and swap (immediate): if `M8[$Y+Z] = rP`, store `$X` there and set `$X ← 1`; otherwise `rP ← M8[$Y+Z]` and `$X ← 0` |
| `LDA` | `LDA $X, $Y, $Z` / `LDA $X, addr` | Load address of `$Y + $Z` — the `ADDU $X, $Y, $Z` alias; two-operand form described below the table |
| `LDAI` | `LDA $X, $Y, Z` / `LDAI $X, addr` | Load address of `$Y + Z` — the `ADDU $X, $Y, Z` alias; two-operand form described below the table |
| `STB` | `STB $X, $Y, $Z` / `STB $X, $Y` | Store byte signed (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STBI` | `STB $X, $Y, Z` | Store byte signed (immediate) |
| `STBU` | `STBU $X, $Y, $Z` / `STBU $X, $Y` | Store byte unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STBUI` | `STBU $X, $Y, Z` | Store byte unsigned (immediate) |
| `STW` | `STW $X, $Y, $Z` / `STW $X, $Y` | Store wyde signed (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STWI` | `STW $X, $Y, Z` | Store wyde signed (immediate) |
| `STWU` | `STWU $X, $Y, $Z` / `STWU $X, $Y` | Store wyde unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STWUI` | `STWU $X, $Y, Z` | Store wyde unsigned (immediate) |
| `STT` | `STT $X, $Y, $Z` / `STT $X, $Y` | Store tetra signed (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STTI` | `STT $X, $Y, Z` | Store tetra signed (immediate) |
| `STTU` | `STTU $X, $Y, $Z` / `STTU $X, $Y` | Store tetra unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STTUI` | `STTU $X, $Y, Z` | Store tetra unsigned (immediate) |
| `STO` | `STO $X, $Y, $Z` / `STO $X, $Y` | Store octa (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STOI` | `STO $X, $Y, Z` | Store octa (immediate) |
| `STOU` | `STOU $X, $Y, $Z` / `STOU $X, $Y` | Store octa unsigned (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STOUI` | `STOU $X, $Y, Z` | Store octa unsigned (immediate) |
| `STUNC` | `STUNC $X, $Y, $Z` / `STUNC $X, $Y` | Store octa uncached (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STUNCI` | `STUNC $X, $Y, Z` | Store octa uncached (immediate) |
| `STCO` | `STCO X, $Y, $Z` / `STCO X, $Y` | Store constant octabyte, or to the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `STCOI` | `STCO X, $Y, Z` | Store constant octabyte, immediate address; `X` is a byte or a register holding one |
| `STHT` | `STHT $X, $Y, $Z` / `STHT $X, $Y` | Store high tetra (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
| `STHTI` | `STHT $X, $Y, Z` | Store high tetra (immediate) |
| `STSF` | `STSF $X, $Y, $Z` / `STSF $X, $Y` | Store short float (narrow f64 → f32, honors rA rounding) (the two-operand form's `$Y` is a register, an offset of zero, or a base-relative address) |
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
| `NEG` | `NEG $X, Y, $Z` / `NEG $X, $Z` | `$X = Y − $Z` signed (Y is literal; omitted Y is 0) |
| `NEGI` | `NEG $X, Y, Z` | `$X = Y − Z` signed |
| `NEGU` | `NEGU $X, Y, $Z` / `NEGU $X, $Z` | `$X = Y − $Z` unsigned (omitted Y is 0) |
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
| `BDIFI` | `BDIF $X, $Y, Z` | Byte difference immediate; Z is the octabyte `#00…0Z`, so only the low byte subtracts |
| `WDIF` | `WDIF $X, $Y, $Z` | Wyde difference (saturating) |
| `WDIFI` | `WDIF $X, $Y, Z` | Wyde difference immediate; Z is the octabyte `#00…0Z`, so only the low wyde subtracts |
| `TDIF` | `TDIF $X, $Y, $Z` | Tetra difference (saturating) |
| `TDIFI` | `TDIF $X, $Y, Z` | Tetra difference immediate; Z is the octabyte `#00…0Z`, so only the low tetra subtracts |
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
| `PUSHJ` | `PUSHJ X, addr` | Push registers and jump; return address in `rJ`; `X` is a byte or a register holding one |
| `PUSHJB` | `PUSHJB X, addr` | Push registers and jump (backward hint); `X` is a byte or a register holding one |
| `PUSHGO` | `PUSHGO X, $Y, $Z` / `PUSHGO X, $Y` | Push registers and jump to `$Y + $Z`, or to the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `PUSHGOI` | `PUSHGO X, $Y, Z` | Push registers and jump to `$Y + Z`; `X` is a byte or a register holding one |
| `POP` | `POP X, YZ` / `POP xyz` / `POP` | Pop registers and return; the hole gets the last of the X returned values, the rest land above it in order; `XYZ=xyz` for the one-operand form; bare `POP` is `POP 0,0` |
| `GO` | `GO $X, $Y, $Z` / `GO $X, $Y` | Jump to `$Y + $Z`, or to the base address `$Y` alone resolves to; save next PC in `$X` |
| `GOI` | `GO $X, $Y, Z` | Jump to `$Y + Z`; save next PC in `$X` |
| `GETA` | `GETA $X, addr` | Get relative address into `$X` |
| `GETAB` | `GETAB $X, addr` | Get relative address (backward hint) |
| `GET` | `GET $X, Z` | Read special register Z into `$X`; `Z ≥ 32` or `Y != 0` halts |
| `PUT` | `PUT X, $Z` | Write `$Z` into special register X; `X ≥ 32` or `Y != 0` halts; `rC rN rO rS rI rT rTT rK rQ rU rV` (8–18) are read-only in user mode; `rG` must be 32–255 and at least `rL`; `rA` at most `#3FFFF` |
| `PUTI` | `PUT X, Z` | Write immediate Z into special register X; same rejections as `PUT` |
| `SAVE` | `SAVE $X, 0` | Push a context onto the register stack; `$X` (global) receives its address; a nonzero `Y` or `Z` halts; bare `SAVE` is an error |
| `UNSAVE` | `UNSAVE 0, $Z` / `UNSAVE $Z` | Restore the context `$Z` addresses from the register stack; a nonzero `X` or `Y` halts; bare `UNSAVE` is an error |
| `RESUME` | `RESUME XYZ` / `RESUME` | Resume after interrupt or trip; a nonzero `X` or `Y` halts; bare `RESUME` is `RESUME 0` |
| `TRAP` | `TRAP X, Y, Z` / `TRAP X, YZ` / `TRAP XYZ` / `TRAP` | System call (see TRAP interface above); `X`, `Y` and `Z` (or `X`) are each a pure byte or a register; bare `TRAP` is `TRAP 0,0,0` |
| `HALT` | `HALT` | checksmix extension — encodes as `TRAP 0,Halt,0` |
| `TRIP` | `TRIP X, Y, Z` / `TRIP X, YZ` / `TRIP XYZ` / `TRIP` | Forced trip (software interrupt); `X`, `Y` and `Z` (or `X`) are each a pure byte or a register; bare `TRIP` is `TRIP 0,0,0` |
| `SYNC` | `SYNC XYZ` / `SYNC` | Synchronize memory/pipeline; `XYZ` 0–3 is a no-op, 4–7 a privileged-operation interrupt, above 7 an illegal-instruction interrupt; bare `SYNC` is `SYNC 0` |
| `SWYM` | `SWYM` / `SWYM X` / `SWYM X, YZ` / `SWYM X, Y, Z` | Sympathize with your machinery (no-op); operands optional, default to zero; `X`, `Y` and `Z` are each a pure byte or a register |
| `PRELD` | `PRELD X, $Y, $Z` / `PRELD X, $Y` | Prefetch data into cache, or the range the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `PRELDI` | `PRELD X, $Y, Z` | Prefetch data (immediate); `X` is a byte or a register holding one |
| `PREGO` | `PREGO X, $Y, $Z` / `PREGO X, $Y` | Prefetch for execution, or for the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `PREGOI` | `PREGO X, $Y, Z` | Prefetch for execution (immediate); `X` is a byte or a register holding one |
| `PREST` | `PREST X, $Y, $Z` / `PREST X, $Y` | Prestore data, or the range the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `PRESTI` | `PREST X, $Y, Z` | Prestore data (immediate); `X` is a byte or a register holding one |
| `SYNCD` | `SYNCD X, $Y, $Z` / `SYNCD X, $Y` | Synchronize data cache, or the range the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `SYNCDI` | `SYNCD X, $Y, Z` | Synchronize data cache (immediate); `X` is a byte or a register holding one |
| `SYNCID` | `SYNCID X, $Y, $Z` / `SYNCID X, $Y` | Synchronize instruction and data cache, or the range the base address `$Y` alone resolves to; `X` is a byte or a register holding one |
| `SYNCIDI` | `SYNCID X, $Y, Z` | Synchronize instruction and data cache (immediate); `X` is a byte or a register holding one |

`X` in `PRELD`, `PREGO`, `PREST`, `SYNCD`, `SYNCID` and `STCO` takes either
spelling: a pure byte or a register holding one, both assembling the same
tetra. In MMIX, `X` is an immediate byte count: `PRELD X,$Y,$Z` covers the
`X+1` bytes `M[$Y+$Z]` through `M[$Y+$Z+X]`. `PRELD 7,$1,$2` and
`PRELD $7,$1,$2` both emit `#9A070102`.

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
