# MMIX Instruction Quick Reference

MMIX is a 64-bit big-endian RISC machine (Knuth) with 256 general-purpose registers, a separate 256-entry special-register file, byte-addressed memory, and fixed 32-bit instructions. Immediates in assembly may be decimal, octal, hex, or character literals; labels and `IS` constants resolve where expressions are accepted.
This assembler targets the MMIXAL dialect used by `mmixal`/`mmixrun`; the table below lists mnemonics this assembler accepts with their operand forms and a terse description for quick reference.

## Minimal assembly skeleton
```
LOC #100           % code start
GREG @             % global register base (optional)
Main    SETL $0,0  % your code here
        TRAP 0,Halt,0
```

## Instruction table
| Mnemonic | Operands | Description |
| --- | --- | --- |
| `SET` | SET $X, $Y | register copy pseudo-instruction - emits `ORI $X, $Y, 0`|
| `SETI` | SETI $X, immediate | regisger set pseudo-instruction |
| `SETL` | SETL $X, YZ | set low wyde |
| `SETH` | SETH $X, YZ | set high wyde |
| `SETMH` | SETMH $X, YZ | set medium high wyde |
| `SETML` | SETML $X, YZ | set medium low wyde |
| `INCH` | INCH $X, YZ | increment high wyde |
| `INCMH` | INCMH $X, YZ | increment medium high wyde |
| `INCML` | INCML $X, YZ | increment medium low wyde |
| `ORH` | ORH $X, YZ | or high wyde |
| `ORMH` | ORMH $X, YZ | or medium high wyde |
| `ORML` | ORML $X, YZ | or medium low wyde |
| `ORL` | ORL $X, YZ | or low wyde |
| `ANDNH` | ANDNH $X, YZ | and-not high wyde |
| `ANDNMH` | ANDNMH $X, YZ | and-not medium high wyde |
| `ANDNML` | ANDNML $X, YZ | and-not medium low wyde |
| `ANDNL` | ANDNL $X, YZ | and-not low wyde |
| `LDB` | LDB $X, $Y, $Z | load byte signed |
| `LDBI` | LDB $X, $Y, Z | load byte signed (immediate) |
| `LDBU` | LDBU $X, $Y, $Z | load byte unsigned |
| `LDBUI` | LDBU $X, $Y, Z | load byte unsigned (immediate) |
| `LDW` | LDW $X, $Y, $Z | load wyde signed |
| `LDWI` | LDW $X, $Y, Z | load wyde signed (immediate) |
| `LDWU` | LDWU $X, $Y, $Z | load wyde unsigned |
| `LDWUI` | LDWU $X, $Y, Z | load wyde unsigned (immediate) |
| `LDT` | LDT $X, $Y, $Z | load tetra signed |
| `LDTI` | LDT $X, $Y, Z | load tetra signed (immediate) |
| `LDTU` | LDTU $X, $Y, $Z | load tetra unsigned |
| `LDTUI` | LDTU $X, $Y, Z | load tetra unsigned (immediate) |
| `LDO` | LDO $X, $Y, $Z | load octa |
| `LDOI` | LDO $X, $Y, Z | load octa (immediate) |
| `LDOU` | LDOU $X, $Y, $Z | load octa unsigned |
| `LDOUI` | LDOU $X, $Y, Z | load octa unsigned (immediate) |
| `LDUNC` | LDUNC $X, $Y, $Z | load octa uncached |
| `LDUNCI` | LDUNC $X, $Y, Z | load octa uncached (immediate) |
| `LDHT` | LDHT $X, $Y, $Z | load high tetra |
| `LDHTI` | LDHT $X, $Y, Z | load high tetra (immediate) |
| `LDSF` | LDSF $X, $Y, $Z | load short float |
| `LDSFI` | LDSF $X, $Y, Z | load short float (immediate) |
| `LDVTS` | LDVTS $X, $Y, $Z | load virtual translation status |
| `LDVTSI` | LDVTS $X, $Y, Z | load virtual translation status (immediate) |
| `CSWAP` | CSWAP $X, $Y, $Z | compare and swap |
| `CSWAPI` | CSWAP $X, $Y, Z | compare and swap (immediate) |
| `LDA` | LDA $X, $Y, $Z | load address (ADDU) |
| `LDAI` | LDA $X, $Y, Z | load address (immediate) |
| `STB` | STB $X, $Y, $Z | store byte signed |
| `STBI` | STB $X, $Y, Z | store byte signed (immediate) |
| `STBU` | STBU $X, $Y, $Z | store byte unsigned |
| `STBUI` | STBU $X, $Y, Z | store byte unsigned (immediate) |
| `STW` | STW $X, $Y, $Z | store wyde signed |
| `STWI` | STW $X, $Y, Z | store wyde signed (immediate) |
| `STWU` | STWU $X, $Y, $Z | store wyde unsigned |
| `STWUI` | STWU $X, $Y, Z | store wyde unsigned (immediate) |
| `STT` | STT $X, $Y, $Z | store tetra signed |
| `STTI` | STT $X, $Y, Z | store tetra signed (immediate) |
| `STTU` | STTU $X, $Y, $Z | store tetra unsigned |
| `STTUI` | STTU $X, $Y, Z | store tetra unsigned (immediate) |
| `STO` | STO $X, $Y, $Z | store octa |
| `STOI` | STO $X, $Y, Z | store octa (immediate) |
| `STOU` | STOU $X, $Y, $Z | store octa unsigned |
| `STOUI` | STOU $X, $Y, Z | store octa unsigned (immediate) |
| `STUNC` | STUNC $X, $Y, $Z | store octa uncached |
| `STUNCI` | STUNC $X, $Y, Z | store octa uncached (immediate) |
| `STCO` | STCO X, $Y, $Z | store constant octabyte |
| `STCOI` | STCO X, $Y, Z | store constant octabyte (immediate) |
| `STHT` | STHT $X, $Y, $Z | store high tetra |
| `STHTI` | STHT $X, $Y, Z | store high tetra (immediate) |
| `STSF` | STSF $X, $Y, $Z | store short float |
| `STSFI` | STSF $X, $Y, Z | store short float (immediate) |
| `ADD` | ADD $X, $Y, $Z | add with overflow |
| `ADDI` | ADD $X, $Y, Z | add immediate with overflow |
| `ADDU` | ADDU $X, $Y, $Z | add unsigned (same as LDA) |
| `ADDUI` | ADDU $X, $Y, Z | add unsigned immediate |
| `ADDU2` | 2ADDU $X, $Y, $Z | times 2 and add unsigned |
| `ADDU2I` | 2ADDU $X, $Y, Z | times 2 and add unsigned immediate |
| `ADDU4` | 4ADDU $X, $Y, $Z | times 4 and add unsigned |
| `ADDU4I` | 4ADDU $X, $Y, Z | times 4 and add unsigned immediate |
| `ADDU8` | 8ADDU $X, $Y, $Z | times 8 and add unsigned |
| `ADDU8I` | 8ADDU $X, $Y, Z | times 8 and add unsigned immediate |
| `ADDU16` | 16ADDU $X, $Y, $Z | times 16 and add unsigned |
| `ADDU16I` | 16ADDU $X, $Y, Z | times 16 and add unsigned immediate |
| `SUB` | SUB $X, $Y, $Z | subtract with overflow |
| `SUBI` | SUB $X, $Y, Z | subtract immediate with overflow |
| `SUBU` | SUBU $X, $Y, $Z | subtract unsigned |
| `SUBUI` | SUBU $X, $Y, Z | subtract unsigned immediate |
| `NEG` | NEG $X, Y, $Z | negate with overflow (Y is immediate) |
| `NEGI` | NEG $X, Y, Z | negate immediate with overflow |
| `NEGU` | NEGU $X, Y, $Z | negate unsigned |
| `NEGUI` | NEGU $X, Y, Z | negate unsigned immediate |
| `MUL` | MUL $X, $Y, $Z | multiply |
| `MULI` | MUL $X, $Y, Z | multiply immediate |
| `MULU` | MULU $X, $Y, $Z | multiply unsigned |
| `MULUI` | MULU $X, $Y, Z | multiply unsigned immediate |
| `DIV` | DIV $X, $Y, $Z | divide |
| `DIVI` | DIV $X, $Y, Z | divide immediate |
| `DIVU` | DIVU $X, $Y, $Z | divide unsigned |
| `DIVUI` | DIVU $X, $Y, Z | divide unsigned immediate |
| `FCMP` | FCMP $X, $Y, $Z | floating compare |
| `FUN` | FUN $X, $Y, $Z | floating unordered |
| `FEQL` | FEQL $X, $Y, $Z | floating equal |
| `FADD` | FADD $X, $Y, $Z | floating add |
| `FIX` | FIX $X, $Y, $Z | convert float to fixed |
| `FSUB` | FSUB $X, $Y, $Z | floating subtract |
| `FIXU` | FIXU $X, $Y, $Z | convert float to fixed unsigned |
| `FLOT` | FLOT $X, $Y, $Z | convert fixed to float |
| `FLOTI` | FLOTI $X, $Y, Z | convert fixed to float immediate |
| `FLOTU` | FLOTU $X, $Y, $Z | convert fixed unsigned to float |
| `FLOTUI` | FLOTUI $X, $Y, Z | convert fixed unsigned to float immediate |
| `SFLOT` | SFLOT $X, $Y, $Z | convert fixed to short float |
| `SFLOTI` | SFLOTI $X, $Y, Z | convert fixed to short float immediate |
| `SFLOTU` | SFLOTU $X, $Y, $Z | convert fixed unsigned to short float |
| `SFLOTUI` | SFLOTUI $X, $Y, Z | convert fixed unsigned to short float immediate |
| `FMUL` | FMUL $X, $Y, $Z | floating multiply |
| `FDIV` | FDIV $X, $Y, $Z | floating divide |
| `FREM` | FREM $X, $Y, $Z | floating remainder |
| `FSQRT` | FSQRT $X, $Y, $Z | floating square root |
| `FINT` | FINT $X, $Y, $Z | floating round to integer |
| `CMP` | CMP $X, $Y, $Z | compare signed |
| `CMPI` | CMP $X, $Y, Z | compare signed immediate |
| `CMPU` | CMPU $X, $Y, $Z | compare unsigned |
| `CMPUI` | CMPU $X, $Y, Z | compare unsigned immediate |
| `INCL` | INCL $X, $Y, $Z |  |
| `AND` | AND $X, $Y, $Z | bitwise and |
| `ANDI` | AND $X, $Y, Z | bitwise and immediate |
| `OR` | OR $X, $Y, $Z | bitwise or |
| `ORI` | OR $X, $Y, Z | bitwise or immediate |
| `XOR` | XOR $X, $Y, $Z | bitwise exclusive-or |
| `XORI` | XOR $X, $Y, Z | bitwise exclusive-or immediate |
| `ANDN` | ANDN $X, $Y, $Z | bitwise and-not |
| `ANDNI` | ANDN $X, $Y, Z | bitwise and-not immediate |
| `ORN` | ORN $X, $Y, $Z | bitwise or-not |
| `ORNI` | ORN $X, $Y, Z | bitwise or-not immediate |
| `NAND` | NAND $X, $Y, $Z | bitwise not-and |
| `NANDI` | NAND $X, $Y, Z | bitwise not-and immediate |
| `NOR` | NOR $X, $Y, $Z | bitwise not-or |
| `NORI` | NOR $X, $Y, Z | bitwise not-or immediate |
| `NXOR` | NXOR $X, $Y, $Z | bitwise not-exclusive-or |
| `NXORI` | NXOR $X, $Y, Z | bitwise not-exclusive-or immediate |
| `MUX` | MUX $X, $Y, $Z | bitwise multiplex |
| `MUXI` | MUX $X, $Y, Z | bitwise multiplex immediate |
| `BDIF` | BDIF $X, $Y, $Z | byte difference |
| `BDIFI` | BDIF $X, $Y, Z | byte difference immediate |
| `WDIF` | WDIF $X, $Y, $Z | wyde difference |
| `WDIFI` | WDIF $X, $Y, Z | wyde difference immediate |
| `TDIF` | TDIF $X, $Y, $Z | tetra difference |
| `TDIFI` | TDIF $X, $Y, Z | tetra difference immediate |
| `ODIF` | ODIF $X, $Y, $Z | octa difference |
| `ODIFI` | ODIF $X, $Y, Z | octa difference immediate |
| `SADD` | SADD $X, $Y, $Z | sideways add |
| `SADDI` | SADD $X, $Y, Z | sideways add immediate |
| `MOR` | MOR $X, $Y, $Z | multiple or |
| `MORI` | MOR $X, $Y, Z | multiple or immediate |
| `MXOR` | MXOR $X, $Y, $Z | multiple exclusive-or |
| `MXORI` | MXOR $X, $Y, Z | multiple exclusive-or immediate |
| `SL` | SL $X, $Y, $Z | shift left |
| `SLI` | SL $X, $Y, Z | shift left immediate |
| `SLU` | SLU $X, $Y, $Z | shift left unsigned |
| `SLUI` | SLU $X, $Y, Z | shift left unsigned immediate |
| `SR` | SR $X, $Y, $Z | shift right |
| `SRI` | SR $X, $Y, Z | shift right immediate |
| `SRU` | SRU $X, $Y, $Z | shift right unsigned |
| `SRUI` | SRU $X, $Y, Z | shift right unsigned immediate |
| `JMP` | JMP offset (24-bit) |  |
| `JE` | JE $X, offset |  |
| `JNE` | JNE $X, offset |  |
| `JL` | JL $X, offset |  |
| `JG` | JG $X, offset |  |
| `BN` | BN $X, offset | branch if negative |
| `BNB` | BNB $X, offset | branch if negative backward |
| `BZ` | BZ $X, offset | branch if zero |
| `BZB` | BZB $X, offset | branch if zero backward |
| `BP` | BP $X, offset | branch if positive |
| `BPB` | BPB $X, offset | branch if positive backward |
| `BOD` | BOD $X, offset | branch if odd |
| `BODB` | BODB $X, offset | branch if odd backward |
| `BNN` | BNN $X, offset | branch if non-negative |
| `BNNB` | BNNB $X, offset | branch if non-negative backward |
| `BNZ` | BNZ $X, offset | branch if non-zero |
| `BNZB` | BNZB $X, offset | branch if non-zero backward |
| `BNP` | BNP $X, offset | branch if non-positive |
| `BNPB` | BNPB $X, offset | branch if non-positive backward |
| `BEV` | BEV $X, offset | branch if even |
| `BEVB` | BEVB $X, offset | branch if even backward |
| `PBN` | PBN $X, Y, Z | probable branch negative (Y,Z = offset) |
| `PBNB` | PBNB $X, Y, Z | probable branch negative backward |
| `PBZ` | PBZ $X, Y, Z | probable branch zero |
| `PBZB` | PBZB $X, Y, Z | probable branch zero backward |
| `PBP` | PBP $X, Y, Z | probable branch positive |
| `PBPB` | PBPB $X, Y, Z | probable branch positive backward |
| `PBOD` | PBOD $X, Y, Z | probable branch odd |
| `PBODB` | PBODB $X, Y, Z | probable branch odd backward |
| `PBNN` | PBNN $X, Y, Z | probable branch nonnegative |
| `PBNNB` | PBNNB $X, Y, Z | probable branch nonnegative backward |
| `PBNZ` | PBNZ $X, Y, Z | probable branch nonzero |
| `PBNZB` | PBNZB $X, Y, Z | probable branch nonzero backward |
| `PBNP` | PBNP $X, Y, Z | probable branch nonpositive |
| `PBNPB` | PBNPB $X, Y, Z | probable branch nonpositive backward |
| `PBEV` | PBEV $X, Y, Z | probable branch even |
| `PBEVB` | PBEVB $X, Y, Z | probable branch even backward |
| `CSN` | CSN $X, $Y, $Z | conditional set if negative |
| `CSNI` | CSNI $X, $Y, Z | conditional set if negative immediate |
| `CSZ` | CSZ $X, $Y, $Z | conditional set if zero |
| `CSZI` | CSZI $X, $Y, Z | conditional set if zero immediate |
| `CSP` | CSP $X, $Y, $Z | conditional set if positive |
| `CSPI` | CSPI $X, $Y, Z | conditional set if positive immediate |
| `CSOD` | CSOD $X, $Y, $Z | conditional set if odd |
| `CSODI` | CSODI $X, $Y, Z | conditional set if odd immediate |
| `CSNN` | CSNN $X, $Y, $Z | conditional set if non-negative |
| `CSNNI` | CSNNI $X, $Y, Z | conditional set if non-negative immediate |
| `CSNZ` | CSNZ $X, $Y, $Z | conditional set if non-zero |
| `CSNZI` | CSNZI $X, $Y, Z | conditional set if non-zero immediate |
| `CSNP` | CSNP $X, $Y, $Z | conditional set if non-positive |
| `CSNPI` | CSNPI $X, $Y, Z | conditional set if non-positive immediate |
| `CSEV` | CSEV $X, $Y, $Z | conditional set if even |
| `CSEVI` | CSEVI $X, $Y, Z | conditional set if even immediate |
| `ZSN` | ZSN $X, $Y, $Z | zero or set if negative |
| `ZSNI` | ZSNI $X, $Y, Z | zero or set if negative immediate |
| `ZSZ` | ZSZ $X, $Y, $Z | zero or set if zero |
| `ZSZI` | ZSZI $X, $Y, Z | zero or set if zero immediate |
| `ZSP` | ZSP $X, $Y, $Z | zero or set if positive |
| `ZSPI` | ZSPI $X, $Y, Z | zero or set if positive immediate |
| `ZSOD` | ZSOD $X, $Y, $Z | zero or set if odd |
| `ZSODI` | ZSODI $X, $Y, Z | zero or set if odd immediate |
| `ZSNN` | ZSNN $X, $Y, $Z | zero or set if non-negative |
| `ZSNNI` | ZSNNI $X, $Y, Z | zero or set if non-negative immediate |
| `ZSNZ` | ZSNZ $X, $Y, $Z | zero or set if non-zero |
| `ZSNZI` | ZSNZI $X, $Y, Z | zero or set if non-zero immediate |
| `ZSNP` | ZSNP $X, $Y, $Z | zero or set if non-positive |
| `ZSNPI` | ZSNPI $X, $Y, Z | zero or set if non-positive immediate |
| `ZSEV` | ZSEV $X, $Y, $Z | zero or set if even |
| `ZSEVI` | ZSEVI $X, $Y, Z | zero or set if even immediate |
| `TRAP` | TRAP X, Y, Z | trap/system call |
| `TRIP` | TRIP X, Y, Z | trip (forced trap) |
| `PUSHJ` | PUSHJ $X, YZ | push registers and jump |
| `PUSHJB` | PUSHJB $X, YZ | push registers and jump backward |
| `PUSHGO` | PUSHGO $X, $Y, $Z | push registers and go |
| `PUSHGOI` | PUSHGOI $X, $Y, Z | push registers and go (immediate) |
| `POP` | POP X, YZ | pop registers and return |
| `GO` | GO $X, $Y, $Z | go to location |
| `GOI` | GOI $X, $Y, Z | go to location (immediate) |
| `GET` | GET $X, Z | get from special register |
| `PUT` | PUT X, $Z | put into special register |
| `PUTI` | PUTI X, Z | put immediate into special register |
| `SAVE` | SAVE $X, 0 | save context |
| `UNSAVE` | UNSAVE 0, $Z | unsave/restore context |
| `RESUME` | RESUME XYZ | resume after interrupt |
| `SYNC` | SYNC XYZ | synchronize |
| `PRELD` | PRELD X, $Y, $Z | preload data |
| `PRELDI` | PRELDI X, $Y, Z | preload data (immediate) |
| `PREGO` | PREGO X, $Y, $Z | prefetch to go |
| `PREGOI` | PREGOI X, $Y, Z | prefetch to go (immediate) |
| `PREST` | PREST X, $Y, $Z | prestore data |
| `PRESTI` | PRESTI X, $Y, Z | prestore data (immediate) |
| `SYNCD` | SYNCD X, $Y, $Z | synchronize data |
| `SYNCDI` | SYNCDI X, $Y, Z | synchronize data (immediate) |
| `SYNCID` | SYNCID X, $Y, $Z | synchronize instructions and data |
| `SYNCIDI` | SYNCIDI X, $Y, Z | synchronize instructions and data (immediate) |
| `GETA` | GETA $X, $Y, $Z or GETA $X, addr | get address |
| `GETAB` | GETAB $X, $Y, $Z or GETAB $X, addr | get address backward |
| `BYTE` | BYTE | 1 byte of data |
| `WYDE` | WYDE | 2 bytes of data |
| `TETRA` | TETRA | 4 bytes of data |
| `OCTA` | OCTA | 8 bytes of data |
