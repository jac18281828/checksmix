# MMIX Instruction Quick Reference

MMIX is a 64-bit big-endian RISC machine (Knuth) with 256 general-purpose registers (`$0`–`$255`), a separate special-register file, byte-addressed memory, and fixed 32-bit instructions. Immediates in assembly may be decimal, hexadecimal (`#`-prefixed, or `0x`/`0X`-prefixed — also a checksmix extension), octal (`0`-prefixed — a checksmix extension; Knuth reads a leading `0` as decimal), or character literals; labels and `IS` constants resolve wherever expressions are accepted.

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

## Assembler directives

| Directive | Syntax | Effect |
| --- | --- | --- |
| `LOC` | `LOC expr` | Set the assembly location counter to *expr* |
| `GREG` | `[label] GREG expr` | Allocate a global register initialized to *expr*; optional label becomes a register alias |
| `IS` | `Name IS expr` | Define a numeric or register alias constant |
| `PREFIX` | `PREFIX str` | Qualify subsequent unqualified names as `str<name>`; names beginning with `:` opt out |
| `BYTE` | `BYTE expr,...` | Emit one byte per operand |
| `WYDE` | `WYDE expr` | Emit one 16-bit wyde |
| `TETRA` | `TETRA expr` | Emit one 32-bit tetra |
| `OCTA` | `OCTA expr` | Emit one 64-bit octa |
| `INCLUDE` | `INCLUDE file` | Assemble the named file as if inserted here, resolved relative to the including file; recursive, cycles are an error |

`WYDE`, `TETRA` and `OCTA` take exactly one operand. This is a known gap
against Knuth, whose MMIXAL takes a comma-separated list for all four data
directives, `BYTE` included.

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
unaligned access a load through that label rounds back down past the datum.

### INCLUDE

`INCLUDE file` (also `.INCLUDE`, case-insensitive) is a **checksmix extension**,
not part of Knuth's MMIXAL. It is a preprocessor stage, not a grammar rule: the
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

Three known limitations:

- **Own line, no label.** `INCLUDE` must occupy its own line; a line whose
  first token is not `INCLUDE`/`.INCLUDE` is left untouched, so a label cannot
  be attached to an `INCLUDE` line.
- **`debug` label collisions.** The `debug` extension's generated `DbgStr_NNNN`
  labels are numbered per preprocessing call, and splitting a host file into
  multiple units around an `INCLUDE` means multiple such calls -- so two units
  that both use `debug` can generate the same label. This is a pre-existing
  limitation (two `debug`-using files on the command line already collide
  today, independent of `INCLUDE`); it surfaces loudly as a `symbol '...'
  redefined` error, never silently.
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

All floating-point instructions use IEEE 754 double precision. Results honor the **rounding mode** in bits 17–16 of special register `rA` (register 21). `rA` is 18 bits wide, so the mode field sits at its top and `PUT rA,$X` rejects any value above `#3FFFF`, leaving `rA` unchanged. `PUTI` cannot reach the field — its `YZ` operand is 16 bits — so selecting a mode needs the register form of `PUT`:

| rA bits 17–16 | Mode | Meaning |
| --- | --- | --- |
| `0` | `ROUND_NEAR` | Round to nearest, ties to even (default) |
| `1` | `ROUND_OFF` | Round toward zero (truncate) |
| `2` | `ROUND_UP` | Round toward +∞ |
| `3` | `ROUND_DOWN` | Round toward −∞ |

Instructions that honor rounding mode: `FADD`, `FSUB`, `FMUL`, `FDIV`, `FSQRT`, `FINT`, `FIX`, `FIXU`, `FLOT`, `FLOTI`, `FLOTU`, `FLOTUI`, `SFLOT`, `SFLOTI`, `SFLOTU`, `SFLOTUI`, `STSF`, `STSFI`.

**Known gap:** MMIX lets `FSQRT`, `FINT` and the conversion instructions override the mode through the `Y` field. checksmix ignores `Y` on all of them and always uses `rA`.

### rA event flags

Arithmetic operations OR event flags into `rA`; they are never cleared automatically. The bit values are Knuth's (`D_BIT` … `X_BIT`, MMIXAL §69).

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

### Epsilon instructions (FCMPE / FUNE / FEQLE)

`FCMPE`, `FUNE`, and `FEQLE` are the "with epsilon" variants of `FCMP`, `FUN`, and `FEQL`. They compare `|$Y − $Z|` against the value in special register `rE` and report equality when the difference is within epsilon. They also raise the `I` flag on NaN operands and `E` (epsilon) comparisons.

## TRAP interface

`TRAP 0, Code, Z` invokes a system call identified by the predefined symbol *Code*. Register `$255` holds the primary argument or return value; additional arguments use `$0`–`$2` as described below.

| Code | Value | Description | Key registers |
| --- | --- | --- | --- |
| `Halt` | 0 | Stop execution, exit code in `$255` | `$255` = exit code |
| `Trip` | 1 | Cause a forced trip | — |
| `Fopen` | 2 | Open a file | `$255` = filename ptr, `$0` = mode; returns fd in `$255` |
| `Fclose` | 3 | Close a file descriptor | `$255` = fd |
| `Fread` | 4 | Read bytes from fd | `$255` = fd, `$0` = buf ptr, `$1` = count; returns bytes read |
| `Fgets` | 5 | Read a line (null-terminated) from fd | `$255` = fd, `$0` = buf ptr, `$1` = max bytes |
| `Fgetws` | 6 | Read a wide string from fd | `$255` = fd, `$0` = buf ptr, `$1` = max wydes |
| `Fwrite` | 7 | Write bytes to fd | `$255` = fd, `$0` = buf ptr, `$1` = count; returns bytes written |
| `Fputs` | 8 | Write null-terminated string to fd | `$255` = fd, `$0` = string ptr; bytes ≥ 0x80 emitted raw |
| `Fputc` | 9 | Write one byte to fd | `$255` = fd, `$0` = byte; high byte of `$0` emitted raw |
| `Fputws` | 10 | Write null-terminated wide string to fd | `$255` = fd, `$0` = string ptr |
| `Fseek` | 11 | Seek within fd | `$255` = fd, `$0` = offset, `$1` = whence |
| `Ftell` | 12 | Get current position in fd | `$255` = fd; returns position in `$255` |
| `Time` | 13 | Current time | returns microseconds since Unix epoch in `$255` |

Standard file descriptors: `StdIn = 0`, `StdOut = 1`, `StdErr = 2` (predefined symbols).

## Instruction table

| Mnemonic | Operands | Description |
| --- | --- | --- |
| `SET` | `SET $X, $Y` / `SET $X, imm` | MMIXAL alias — emits `ORI $X, $Y, 0` for a register, `SETL $X, imm` for a wyde-wide immediate |
| `SETI` | `SETI $X, imm` | checksmix extension, no Knuth counterpart — sets a full 64-bit constant in four tetras, clearing the register |
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
| `FSQRT` | `FSQRT $X, $Y, $Z` | Floating square root (honors rA rounding; Y = mode override) |
| `FINT` | `FINT $X, $Y, $Z` | Round float to integer (honors rA rounding; Y = mode override) |
| `FIX` | `FIX $X, $Y, $Z` | Convert float → signed integer (honors rA rounding) |
| `FIXU` | `FIXU $X, $Y, $Z` | Convert float → unsigned integer, reduced mod 2^64 (honors rA rounding) |
| `FLOT` | `FLOT $X, $Y, $Z` | Convert signed integer → float (honors rA rounding) |
| `FLOTI` | `FLOT $X, $Y, Z` | Convert signed integer → float immediate |
| `FLOTU` | `FLOTU $X, $Y, $Z` | Convert unsigned integer → float (honors rA rounding) |
| `FLOTUI` | `FLOTU $X, $Y, Z` | Convert unsigned integer → float immediate |
| `SFLOT` | `SFLOT $X, $Y, $Z` | Convert signed integer → short float (honors rA rounding) |
| `SFLOTI` | `SFLOT $X, $Y, Z` | Convert signed integer → short float immediate |
| `SFLOTU` | `SFLOTU $X, $Y, $Z` | Convert unsigned integer → short float (honors rA rounding) |
| `SFLOTUI` | `SFLOTU $X, $Y, Z` | Convert unsigned integer → short float immediate |
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
| `JE` | `JE $X, addr` | checksmix extension, no Knuth counterpart — branch if `$X == 0`; encodes as `BZ`/`BZB` |
| `JNE` | `JNE $X, addr` | checksmix extension, no Knuth counterpart — branch if `$X != 0`; encodes as `BNZ`/`BNZB` |
| `JL` | `JL $X, addr` | checksmix extension, no Knuth counterpart — branch if `$X < 0`; encodes as `BN`/`BNB` |
| `JG` | `JG $X, addr` | checksmix extension, no Knuth counterpart — branch if `$X > 0`; encodes as `BP`/`BPB` |
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
| `POP` | `POP X, YZ` | Pop registers and return; X values returned |
| `GO` | `GO $X, $Y, $Z` | Jump to `$Y + $Z`; save next PC in `$X` |
| `GOI` | `GO $X, $Y, Z` | Jump to `$Y + Z`; save next PC in `$X` |
| `GETA` | `GETA $X, addr` | Get relative address into `$X` |
| `GETAB` | `GETAB $X, addr` | Get relative address (backward hint) |
| `GET` | `GET $X, Z` | Read special register Z into `$X` |
| `PUT` | `PUT X, $Z` | Write `$Z` into special register X |
| `PUTI` | `PUT X, Z` | Write immediate Z into special register X |
| `SAVE` | `SAVE $X, 0` | Save register stack to memory |
| `UNSAVE` | `UNSAVE 0, $Z` | Restore register stack from memory |
| `RESUME` | `RESUME XYZ` | Resume after interrupt or trip |
| `TRAP` | `TRAP X, Y, Z` | System call (see TRAP interface above) |
| `HALT` | `HALT` | checksmix extension, no Knuth counterpart — encodes as `TRAP 0,Halt,0` |
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
`SYNCID` as a register. Knuth specifies an immediate byte count there, so
source written to the specification does not assemble.

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
