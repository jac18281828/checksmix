use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

use crate::mmix::{STACK_SEGMENT_START, TrapCode};
use pest_derive::Parser;
use regex::Regex;

const DATA_SEGMENT_START: u64 = 0x2000000000000000;

const POOL_SEGMENT_START: u64 = 0x4000000000000000;

#[derive(Parser)]
#[grammar = "mmixal.pest"]
struct MMixalParser;

/// Type of symbol defined by IS directive
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolType {
    Register(u8),  // Register alias: "Zero IS $255" -> Register(255)
    Constant(u64), // Numeric constant: "MAXLIMBS IS 32" -> Constant(32)
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolType::Register(reg) => write!(f, "${}", reg),
            SymbolType::Constant(val) => write!(f, "{}", val),
        }
    }
}

/// MMIX Assembly Language Parser
/// Parses MMIX assembly language into binary object code (.mmo)
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq)]
pub enum MMixInstruction {
    // Immediate load instructions
    SET(u8, u64),    // SET $X, value - pseudo-instruction
    SETRR(u8, u8),   // SET $X, $Y - register copy (emits ORI $X, $Y, 0)
    SETL(u8, u16),   // SETL $X, YZ - set low wyde
    SETH(u8, u16),   // SETH $X, YZ - set high wyde
    SETMH(u8, u16),  // SETMH $X, YZ - set medium high wyde
    SETML(u8, u16),  // SETML $X, YZ - set medium low wyde
    INCH(u8, u16),   // INCH $X, YZ - increment high wyde
    INCMH(u8, u16),  // INCMH $X, YZ - increment medium high wyde
    INCML(u8, u16),  // INCML $X, YZ - increment medium low wyde
    ORH(u8, u16),    // ORH $X, YZ - or high wyde
    ORMH(u8, u16),   // ORMH $X, YZ - or medium high wyde
    ORML(u8, u16),   // ORML $X, YZ - or medium low wyde
    ORL(u8, u16),    // ORL $X, YZ - or low wyde
    ANDNH(u8, u16),  // ANDNH $X, YZ - and-not high wyde
    ANDNMH(u8, u16), // ANDNMH $X, YZ - and-not medium high wyde
    ANDNML(u8, u16), // ANDNML $X, YZ - and-not medium low wyde
    ANDNL(u8, u16),  // ANDNL $X, YZ - and-not low wyde

    // Load instructions
    LDB(u8, u8, u8),    // LDB $X, $Y, $Z - load byte signed
    LDBI(u8, u8, u8),   // LDB $X, $Y, Z - load byte signed (immediate)
    LDBU(u8, u8, u8),   // LDBU $X, $Y, $Z - load byte unsigned
    LDBUI(u8, u8, u8),  // LDBU $X, $Y, Z - load byte unsigned (immediate)
    LDW(u8, u8, u8),    // LDW $X, $Y, $Z - load wyde signed
    LDWI(u8, u8, u8),   // LDW $X, $Y, Z - load wyde signed (immediate)
    LDWU(u8, u8, u8),   // LDWU $X, $Y, $Z - load wyde unsigned
    LDWUI(u8, u8, u8),  // LDWU $X, $Y, Z - load wyde unsigned (immediate)
    LDT(u8, u8, u8),    // LDT $X, $Y, $Z - load tetra signed
    LDTI(u8, u8, u8),   // LDT $X, $Y, Z - load tetra signed (immediate)
    LDTU(u8, u8, u8),   // LDTU $X, $Y, $Z - load tetra unsigned
    LDTUI(u8, u8, u8),  // LDTU $X, $Y, Z - load tetra unsigned (immediate)
    LDO(u8, u8, u8),    // LDO $X, $Y, $Z - load octa
    LDOI(u8, u8, u8),   // LDO $X, $Y, Z - load octa (immediate)
    LDOU(u8, u8, u8),   // LDOU $X, $Y, $Z - load octa unsigned
    LDOUI(u8, u8, u8),  // LDOU $X, $Y, Z - load octa unsigned (immediate)
    LDUNC(u8, u8, u8),  // LDUNC $X, $Y, $Z - load octa uncached
    LDUNCI(u8, u8, u8), // LDUNC $X, $Y, Z - load octa uncached (immediate)
    LDHT(u8, u8, u8),   // LDHT $X, $Y, $Z - load high tetra
    LDHTI(u8, u8, u8),  // LDHT $X, $Y, Z - load high tetra (immediate)
    LDSF(u8, u8, u8),   // LDSF $X, $Y, $Z - load short float
    LDSFI(u8, u8, u8),  // LDSF $X, $Y, Z - load short float (immediate)
    LDVTS(u8, u8, u8),  // LDVTS $X, $Y, $Z - load virtual translation status
    LDVTSI(u8, u8, u8), // LDVTS $X, $Y, Z - load virtual translation status (immediate)
    CSWAP(u8, u8, u8),  // CSWAP $X, $Y, $Z - compare and swap
    CSWAPI(u8, u8, u8), // CSWAP $X, $Y, Z - compare and swap (immediate)
    LDA(u8, u8, u8),    // LDA $X, $Y, $Z - load address (ADDU)
    LDAI(u8, u8, u8),   // LDA $X, $Y, Z - load address (immediate)

    // Store instructions
    STB(u8, u8, u8),    // STB $X, $Y, $Z - store byte signed
    STBI(u8, u8, u8),   // STB $X, $Y, Z - store byte signed (immediate)
    STBU(u8, u8, u8),   // STBU $X, $Y, $Z - store byte unsigned
    STBUI(u8, u8, u8),  // STBU $X, $Y, Z - store byte unsigned (immediate)
    STW(u8, u8, u8),    // STW $X, $Y, $Z - store wyde signed
    STWI(u8, u8, u8),   // STW $X, $Y, Z - store wyde signed (immediate)
    STWU(u8, u8, u8),   // STWU $X, $Y, $Z - store wyde unsigned
    STWUI(u8, u8, u8),  // STWU $X, $Y, Z - store wyde unsigned (immediate)
    STT(u8, u8, u8),    // STT $X, $Y, $Z - store tetra signed
    STTI(u8, u8, u8),   // STT $X, $Y, Z - store tetra signed (immediate)
    STTU(u8, u8, u8),   // STTU $X, $Y, $Z - store tetra unsigned
    STTUI(u8, u8, u8),  // STTU $X, $Y, Z - store tetra unsigned (immediate)
    STO(u8, u8, u8),    // STO $X, $Y, $Z - store octa
    STOI(u8, u8, u8),   // STO $X, $Y, Z - store octa (immediate)
    STOU(u8, u8, u8),   // STOU $X, $Y, $Z - store octa unsigned
    STOUI(u8, u8, u8),  // STOU $X, $Y, Z - store octa unsigned (immediate)
    STUNC(u8, u8, u8),  // STUNC $X, $Y, $Z - store octa uncached
    STUNCI(u8, u8, u8), // STUNC $X, $Y, Z - store octa uncached (immediate)
    STCO(u8, u8, u8),   // STCO X, $Y, $Z - store constant octabyte
    STCOI(u8, u8, u8),  // STCO X, $Y, Z - store constant octabyte (immediate)
    STHT(u8, u8, u8),   // STHT $X, $Y, $Z - store high tetra
    STHTI(u8, u8, u8),  // STHT $X, $Y, Z - store high tetra (immediate)
    STSF(u8, u8, u8),   // STSF $X, $Y, $Z - store short float
    STSFI(u8, u8, u8),  // STSF $X, $Y, Z - store short float (immediate)

    // Arithmetic - Add and Subtract
    ADD(u8, u8, u8),     // ADD $X, $Y, $Z - add with overflow
    ADDI(u8, u8, u8),    // ADD $X, $Y, Z - add immediate with overflow
    ADDU(u8, u8, u8),    // ADDU $X, $Y, $Z - add unsigned (same as LDA)
    ADDUI(u8, u8, u8),   // ADDU $X, $Y, Z - add unsigned immediate
    ADDU2(u8, u8, u8),   // 2ADDU $X, $Y, $Z - times 2 and add unsigned
    ADDU2I(u8, u8, u8),  // 2ADDU $X, $Y, Z - times 2 and add unsigned immediate
    ADDU4(u8, u8, u8),   // 4ADDU $X, $Y, $Z - times 4 and add unsigned
    ADDU4I(u8, u8, u8),  // 4ADDU $X, $Y, Z - times 4 and add unsigned immediate
    ADDU8(u8, u8, u8),   // 8ADDU $X, $Y, $Z - times 8 and add unsigned
    ADDU8I(u8, u8, u8),  // 8ADDU $X, $Y, Z - times 8 and add unsigned immediate
    ADDU16(u8, u8, u8),  // 16ADDU $X, $Y, $Z - times 16 and add unsigned
    ADDU16I(u8, u8, u8), // 16ADDU $X, $Y, Z - times 16 and add unsigned immediate
    SUB(u8, u8, u8),     // SUB $X, $Y, $Z - subtract with overflow
    SUBI(u8, u8, u8),    // SUB $X, $Y, Z - subtract immediate with overflow
    SUBU(u8, u8, u8),    // SUBU $X, $Y, $Z - subtract unsigned
    SUBUI(u8, u8, u8),   // SUBU $X, $Y, Z - subtract unsigned immediate
    NEG(u8, u8, u8),     // NEG $X, Y, $Z - negate with overflow (Y is immediate)
    NEGI(u8, u8, u8),    // NEG $X, Y, Z - negate immediate with overflow
    NEGU(u8, u8, u8),    // NEGU $X, Y, $Z - negate unsigned
    NEGUI(u8, u8, u8),   // NEGU $X, Y, Z - negate unsigned immediate

    MUL(u8, u8, u8),   // MUL $X, $Y, $Z - multiply
    MULI(u8, u8, u8),  // MUL $X, $Y, Z - multiply immediate
    MULU(u8, u8, u8),  // MULU $X, $Y, $Z - multiply unsigned
    MULUI(u8, u8, u8), // MULU $X, $Y, Z - multiply unsigned immediate
    DIV(u8, u8, u8),   // DIV $X, $Y, $Z - divide
    DIVI(u8, u8, u8),  // DIV $X, $Y, Z - divide immediate
    DIVU(u8, u8, u8),  // DIVU $X, $Y, $Z - divide unsigned
    DIVUI(u8, u8, u8), // DIVU $X, $Y, Z - divide unsigned immediate

    // Floating point instructions
    FCMP(u8, u8, u8),    // FCMP $X, $Y, $Z - floating compare
    FUN(u8, u8, u8),     // FUN $X, $Y, $Z - floating unordered
    FEQL(u8, u8, u8),    // FEQL $X, $Y, $Z - floating equal
    FCMPE(u8, u8, u8),   // FCMPE $X, $Y, $Z - floating compare with epsilon (rE)
    FUNE(u8, u8, u8),    // FUNE $X, $Y, $Z - floating unordered with epsilon (rE)
    FEQLE(u8, u8, u8),   // FEQLE $X, $Y, $Z - floating equivalent with epsilon (rE)
    FADD(u8, u8, u8),    // FADD $X, $Y, $Z - floating add
    FIX(u8, u8, u8),     // FIX $X, $Y, $Z - convert float to fixed
    FSUB(u8, u8, u8),    // FSUB $X, $Y, $Z - floating subtract
    FIXU(u8, u8, u8),    // FIXU $X, $Y, $Z - convert float to fixed unsigned
    FLOT(u8, u8, u8),    // FLOT $X, $Y, $Z - convert fixed to float
    FLOTI(u8, u8, u8),   // FLOTI $X, $Y, Z - convert fixed to float immediate
    FLOTU(u8, u8, u8),   // FLOTU $X, $Y, $Z - convert fixed unsigned to float
    FLOTUI(u8, u8, u8),  // FLOTUI $X, $Y, Z - convert fixed unsigned to float immediate
    SFLOT(u8, u8, u8),   // SFLOT $X, $Y, $Z - convert fixed to short float
    SFLOTI(u8, u8, u8),  // SFLOTI $X, $Y, Z - convert fixed to short float immediate
    SFLOTU(u8, u8, u8),  // SFLOTU $X, $Y, $Z - convert fixed unsigned to short float
    SFLOTUI(u8, u8, u8), // SFLOTUI $X, $Y, Z - convert fixed unsigned to short float immediate
    FMUL(u8, u8, u8),    // FMUL $X, $Y, $Z - floating multiply
    FDIV(u8, u8, u8),    // FDIV $X, $Y, $Z - floating divide
    FREM(u8, u8, u8),    // FREM $X, $Y, $Z - floating remainder
    FSQRT(u8, u8, u8),   // FSQRT $X, $Y, $Z - floating square root
    FINT(u8, u8, u8),    // FINT $X, $Y, $Z - floating round to integer

    // Comparison instructions
    CMP(u8, u8, u8),   // CMP $X, $Y, $Z - compare signed
    CMPI(u8, u8, u8),  // CMP $X, $Y, Z - compare signed immediate
    CMPU(u8, u8, u8),  // CMPU $X, $Y, $Z - compare unsigned
    CMPUI(u8, u8, u8), // CMPU $X, $Y, Z - compare unsigned immediate

    INCL(u8, u16), // INCL $X, YZ - increment low wyde

    // Bitwise operations
    AND(u8, u8, u8),   // AND $X, $Y, $Z - bitwise and
    ANDI(u8, u8, u8),  // AND $X, $Y, Z - bitwise and immediate
    OR(u8, u8, u8),    // OR $X, $Y, $Z - bitwise or
    ORI(u8, u8, u8),   // OR $X, $Y, Z - bitwise or immediate
    XOR(u8, u8, u8),   // XOR $X, $Y, $Z - bitwise exclusive-or
    XORI(u8, u8, u8),  // XOR $X, $Y, Z - bitwise exclusive-or immediate
    ANDN(u8, u8, u8),  // ANDN $X, $Y, $Z - bitwise and-not
    ANDNI(u8, u8, u8), // ANDN $X, $Y, Z - bitwise and-not immediate
    ORN(u8, u8, u8),   // ORN $X, $Y, $Z - bitwise or-not
    ORNI(u8, u8, u8),  // ORN $X, $Y, Z - bitwise or-not immediate
    NAND(u8, u8, u8),  // NAND $X, $Y, $Z - bitwise not-and
    NANDI(u8, u8, u8), // NAND $X, $Y, Z - bitwise not-and immediate
    NOR(u8, u8, u8),   // NOR $X, $Y, $Z - bitwise not-or
    NORI(u8, u8, u8),  // NOR $X, $Y, Z - bitwise not-or immediate
    NXOR(u8, u8, u8),  // NXOR $X, $Y, $Z - bitwise not-exclusive-or
    NXORI(u8, u8, u8), // NXOR $X, $Y, Z - bitwise not-exclusive-or immediate
    MUX(u8, u8, u8),   // MUX $X, $Y, $Z - bitwise multiplex
    MUXI(u8, u8, u8),  // MUX $X, $Y, Z - bitwise multiplex immediate

    // Bit fiddling operations
    BDIF(u8, u8, u8),  // BDIF $X, $Y, $Z - byte difference
    BDIFI(u8, u8, u8), // BDIF $X, $Y, Z - byte difference immediate
    WDIF(u8, u8, u8),  // WDIF $X, $Y, $Z - wyde difference
    WDIFI(u8, u8, u8), // WDIF $X, $Y, Z - wyde difference immediate
    TDIF(u8, u8, u8),  // TDIF $X, $Y, $Z - tetra difference
    TDIFI(u8, u8, u8), // TDIF $X, $Y, Z - tetra difference immediate
    ODIF(u8, u8, u8),  // ODIF $X, $Y, $Z - octa difference
    ODIFI(u8, u8, u8), // ODIF $X, $Y, Z - octa difference immediate
    SADD(u8, u8, u8),  // SADD $X, $Y, $Z - sideways add
    SADDI(u8, u8, u8), // SADD $X, $Y, Z - sideways add immediate
    MOR(u8, u8, u8),   // MOR $X, $Y, $Z - multiple or
    MORI(u8, u8, u8),  // MOR $X, $Y, Z - multiple or immediate
    MXOR(u8, u8, u8),  // MXOR $X, $Y, $Z - multiple exclusive-or
    MXORI(u8, u8, u8), // MXOR $X, $Y, Z - multiple exclusive-or immediate

    // Shift instructions
    SL(u8, u8, u8),   // SL $X, $Y, $Z - shift left
    SLI(u8, u8, u8),  // SL $X, $Y, Z - shift left immediate
    SLU(u8, u8, u8),  // SLU $X, $Y, $Z - shift left unsigned
    SLUI(u8, u8, u8), // SLU $X, $Y, Z - shift left unsigned immediate
    SR(u8, u8, u8),   // SR $X, $Y, $Z - shift right
    SRI(u8, u8, u8),  // SR $X, $Y, Z - shift right immediate
    SRU(u8, u8, u8),  // SRU $X, $Y, $Z - shift right unsigned
    SRUI(u8, u8, u8), // SRU $X, $Y, Z - shift right unsigned immediate

    // Branch instructions
    JMP(u32),          // JMP XYZ (24-bit), jump to @ + 4*XYZ
    JMPB(u32),         // JMPB XYZ (24-bit), jump to @ + 4*(XYZ - 2^24)
    BN(u8, u16),       // BN $X, offset - branch if negative
    BNB(u8, u16),      // BNB $X, offset - branch if negative backward
    BZ(u8, u16),       // BZ $X, offset - branch if zero
    BZB(u8, u16),      // BZB $X, offset - branch if zero backward
    BP(u8, u16),       // BP $X, offset - branch if positive
    BPB(u8, u16),      // BPB $X, offset - branch if positive backward
    BOD(u8, u16),      // BOD $X, offset - branch if odd
    BODB(u8, u16),     // BODB $X, offset - branch if odd backward
    BNN(u8, u16),      // BNN $X, offset - branch if non-negative
    BNNB(u8, u16),     // BNNB $X, offset - branch if non-negative backward
    BNZ(u8, u16),      // BNZ $X, offset - branch if non-zero
    BNZB(u8, u16),     // BNZB $X, offset - branch if non-zero backward
    BNP(u8, u16),      // BNP $X, offset - branch if non-positive
    BNPB(u8, u16),     // BNPB $X, offset - branch if non-positive backward
    BEV(u8, u16),      // BEV $X, offset - branch if even
    BEVB(u8, u16),     // BEVB $X, offset - branch if even backward
    PBN(u8, u8, u8),   // PBN $X, Y, Z - probable branch negative (Y,Z = offset)
    PBNB(u8, u8, u8),  // PBNB $X, Y, Z - probable branch negative backward
    PBZ(u8, u8, u8),   // PBZ $X, Y, Z - probable branch zero
    PBZB(u8, u8, u8),  // PBZB $X, Y, Z - probable branch zero backward
    PBP(u8, u8, u8),   // PBP $X, Y, Z - probable branch positive
    PBPB(u8, u8, u8),  // PBPB $X, Y, Z - probable branch positive backward
    PBOD(u8, u8, u8),  // PBOD $X, Y, Z - probable branch odd
    PBODB(u8, u8, u8), // PBODB $X, Y, Z - probable branch odd backward
    PBNN(u8, u8, u8),  // PBNN $X, Y, Z - probable branch nonnegative
    PBNNB(u8, u8, u8), // PBNNB $X, Y, Z - probable branch nonnegative backward
    PBNZ(u8, u8, u8),  // PBNZ $X, Y, Z - probable branch nonzero
    PBNZB(u8, u8, u8), // PBNZB $X, Y, Z - probable branch nonzero backward
    PBNP(u8, u8, u8),  // PBNP $X, Y, Z - probable branch nonpositive
    PBNPB(u8, u8, u8), // PBNPB $X, Y, Z - probable branch nonpositive backward
    PBEV(u8, u8, u8),  // PBEV $X, Y, Z - probable branch even
    PBEVB(u8, u8, u8), // PBEVB $X, Y, Z - probable branch even backward

    // Conditional set instructions
    CSN(u8, u8, u8),   // CSN $X, $Y, $Z - conditional set if negative
    CSNI(u8, u8, u8),  // CSNI $X, $Y, Z - conditional set if negative immediate
    CSZ(u8, u8, u8),   // CSZ $X, $Y, $Z - conditional set if zero
    CSZI(u8, u8, u8),  // CSZI $X, $Y, Z - conditional set if zero immediate
    CSP(u8, u8, u8),   // CSP $X, $Y, $Z - conditional set if positive
    CSPI(u8, u8, u8),  // CSPI $X, $Y, Z - conditional set if positive immediate
    CSOD(u8, u8, u8),  // CSOD $X, $Y, $Z - conditional set if odd
    CSODI(u8, u8, u8), // CSODI $X, $Y, Z - conditional set if odd immediate
    CSNN(u8, u8, u8),  // CSNN $X, $Y, $Z - conditional set if non-negative
    CSNNI(u8, u8, u8), // CSNNI $X, $Y, Z - conditional set if non-negative immediate
    CSNZ(u8, u8, u8),  // CSNZ $X, $Y, $Z - conditional set if non-zero
    CSNZI(u8, u8, u8), // CSNZI $X, $Y, Z - conditional set if non-zero immediate
    CSNP(u8, u8, u8),  // CSNP $X, $Y, $Z - conditional set if non-positive
    CSNPI(u8, u8, u8), // CSNPI $X, $Y, Z - conditional set if non-positive immediate
    CSEV(u8, u8, u8),  // CSEV $X, $Y, $Z - conditional set if even
    CSEVI(u8, u8, u8), // CSEVI $X, $Y, Z - conditional set if even immediate

    // Zero or set instructions
    ZSN(u8, u8, u8),   // ZSN $X, $Y, $Z - zero or set if negative
    ZSNI(u8, u8, u8),  // ZSNI $X, $Y, Z - zero or set if negative immediate
    ZSZ(u8, u8, u8),   // ZSZ $X, $Y, $Z - zero or set if zero
    ZSZI(u8, u8, u8),  // ZSZI $X, $Y, Z - zero or set if zero immediate
    ZSP(u8, u8, u8),   // ZSP $X, $Y, $Z - zero or set if positive
    ZSPI(u8, u8, u8),  // ZSPI $X, $Y, Z - zero or set if positive immediate
    ZSOD(u8, u8, u8),  // ZSOD $X, $Y, $Z - zero or set if odd
    ZSODI(u8, u8, u8), // ZSODI $X, $Y, Z - zero or set if odd immediate
    ZSNN(u8, u8, u8),  // ZSNN $X, $Y, $Z - zero or set if non-negative
    ZSNNI(u8, u8, u8), // ZSNNI $X, $Y, Z - zero or set if non-negative immediate
    ZSNZ(u8, u8, u8),  // ZSNZ $X, $Y, $Z - zero or set if non-zero
    ZSNZI(u8, u8, u8), // ZSNZI $X, $Y, Z - zero or set if non-zero immediate
    ZSNP(u8, u8, u8),  // ZSNP $X, $Y, $Z - zero or set if non-positive
    ZSNPI(u8, u8, u8), // ZSNPI $X, $Y, Z - zero or set if non-positive immediate
    ZSEV(u8, u8, u8),  // ZSEV $X, $Y, $Z - zero or set if even
    ZSEVI(u8, u8, u8), // ZSEVI $X, $Y, Z - zero or set if even immediate

    // System instructions
    TRAP(u8, u8, u8),    // TRAP X, Y, Z - trap/system call
    TRIP(u8, u8, u8),    // TRIP X, Y, Z - trip (forced trap)
    PUSHJ(u8, u8, u8),   // PUSHJ $X, YZ - push registers and jump
    PUSHJB(u8, u8, u8),  // PUSHJB $X, YZ - push registers and jump backward
    PUSHGO(u8, u8, u8),  // PUSHGO $X, $Y, $Z - push registers and go
    PUSHGOI(u8, u8, u8), // PUSHGOI $X, $Y, Z - push registers and go (immediate)
    POP(u8, u8, u8),     // POP X, Y, Z - pop registers and return (YZ combined is a 16-bit field)
    GO(u8, u8, u8),      // GO $X, $Y, $Z - go to location
    GOI(u8, u8, u8),     // GOI $X, $Y, Z - go to location (immediate)
    GET(u8, u8),         // GET $X, Z - get from special register
    PUT(u8, u8),         // PUT X, $Z - put into special register
    PUTI(u8, u8),        // PUTI X, Z - put immediate into special register
    SAVE(u8, u8),        // SAVE $X, 0 - save context
    UNSAVE(u8, u8),      // UNSAVE 0, $Z - unsave/restore context
    RESUME(u32),         // RESUME XYZ - resume after interrupt, 24-bit XYZ
    SYNC(u32),           // SYNC XYZ - synchronize, 24-bit XYZ
    SWYM(u8, u8, u8),    // SWYM X, Y, Z - sympathize with your machinery (nop)
    PRELD(u8, u8, u8),   // PRELD X, $Y, $Z - preload data
    PRELDI(u8, u8, u8),  // PRELDI X, $Y, Z - preload data (immediate)
    PREGO(u8, u8, u8),   // PREGO X, $Y, $Z - prefetch to go
    PREGOI(u8, u8, u8),  // PREGOI X, $Y, Z - prefetch to go (immediate)
    PREST(u8, u8, u8),   // PREST X, $Y, $Z - prestore data
    PRESTI(u8, u8, u8),  // PRESTI X, $Y, Z - prestore data (immediate)
    SYNCD(u8, u8, u8),   // SYNCD X, $Y, $Z - synchronize data
    SYNCDI(u8, u8, u8),  // SYNCDI X, $Y, Z - synchronize data (immediate)
    SYNCID(u8, u8, u8),  // SYNCID X, $Y, $Z - synchronize instructions and data
    SYNCIDI(u8, u8, u8), // SYNCIDI X, $Y, Z - synchronize instructions and data (immediate)
    GETA(u8, u8, u8),    // GETA $X, $Y, $Z or GETA $X, addr - get address
    GETAB(u8, u8, u8),   // GETAB $X, $Y, $Z or GETAB $X, addr - get address backward

    // Data directives
    BYTE(u8),   // BYTE - 1 byte of data
    WYDE(u16),  // WYDE - 2 bytes of data
    TETRA(u32), // TETRA - 4 bytes of data
    OCTA(u64),  // OCTA - 8 bytes of data

    // Control
    HALT, // HALT - stop execution
}

/// MMIX Operation Codes
/// This enum represents just the opcode byte (not the full instruction with operands)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum Opcode {
    // Floating Point instructions (0x00-0x17)
    TRAP = 0x00,
    FCMP = 0x01,
    FUN = 0x02,
    FEQL = 0x03,
    FADD = 0x04,
    FIX = 0x05,
    FSUB = 0x06,
    FIXU = 0x07,
    FLOT = 0x08,
    FLOTI = 0x09,
    FLOTU = 0x0A,
    FLOTUI = 0x0B,
    SFLOT = 0x0C,
    SFLOTI = 0x0D,
    SFLOTU = 0x0E,
    SFLOTUI = 0x0F,
    FMUL = 0x10,
    FCMPE = 0x11,
    FUNE = 0x12,
    FEQLE = 0x13,
    FDIV = 0x14,
    FSQRT = 0x15,
    FREM = 0x16,
    FINT = 0x17,

    // Multiplication and Division (0x18-0x1F)
    MUL = 0x18,
    MULI = 0x19,
    MULU = 0x1A,
    MULUI = 0x1B,
    DIV = 0x1C,
    DIVI = 0x1D,
    DIVU = 0x1E,
    DIVUI = 0x1F,

    // Addition and Subtraction (0x20-0x3F)
    ADD = 0x20,
    ADDI = 0x21,
    ADDU = 0x22,
    ADDUI = 0x23,
    SUB = 0x24,
    SUBI = 0x25,
    SUBU = 0x26,
    SUBUI = 0x27,
    ADDU2 = 0x28,
    ADDU2I = 0x29,
    ADDU4 = 0x2A,
    ADDU4I = 0x2B,
    ADDU8 = 0x2C,
    ADDU8I = 0x2D,
    ADDU16 = 0x2E,
    ADDU16I = 0x2F,
    CMP = 0x30,
    CMPI = 0x31,
    CMPU = 0x32,
    CMPUI = 0x33,
    NEG = 0x34,
    NEGI = 0x35,
    NEGU = 0x36,
    NEGUI = 0x37,
    SL = 0x38,
    SLI = 0x39,
    SLU = 0x3A,
    SLUI = 0x3B,
    SR = 0x3C,
    SRI = 0x3D,
    SRU = 0x3E,
    SRUI = 0x3F,

    // Branch instructions (0x40-0x5F)
    BN = 0x40,
    BNB = 0x41,
    BZ = 0x42,
    BZB = 0x43,
    BP = 0x44,
    BPB = 0x45,
    BOD = 0x46,
    BODB = 0x47,
    BNN = 0x48,
    BNNB = 0x49,
    BNZ = 0x4A,
    BNZB = 0x4B,
    BNP = 0x4C,
    BNPB = 0x4D,
    BEV = 0x4E,
    BEVB = 0x4F,
    PBN = 0x50,
    PBNB = 0x51,
    PBZ = 0x52,
    PBZB = 0x53,
    PBP = 0x54,
    PBPB = 0x55,
    PBOD = 0x56,
    PBODB = 0x57,
    PBNN = 0x58,
    PBNNB = 0x59,
    PBNZ = 0x5A,
    PBNZB = 0x5B,
    PBNP = 0x5C,
    PBNPB = 0x5D,
    PBEV = 0x5E,
    PBEVB = 0x5F,

    // Conditional set (0x60-0x6F)
    CSN = 0x60,
    CSNI = 0x61,
    CSZ = 0x62,
    CSZI = 0x63,
    CSP = 0x64,
    CSPI = 0x65,
    CSOD = 0x66,
    CSODI = 0x67,
    CSNN = 0x68,
    CSNNI = 0x69,
    CSNZ = 0x6A,
    CSNZI = 0x6B,
    CSNP = 0x6C,
    CSNPI = 0x6D,
    CSEV = 0x6E,
    CSEVI = 0x6F,

    // Zero or set (0x70-0x7F)
    ZSN = 0x70,
    ZSNI = 0x71,
    ZSZ = 0x72,
    ZSZI = 0x73,
    ZSP = 0x74,
    ZSPI = 0x75,
    ZSOD = 0x76,
    ZSODI = 0x77,
    ZSNN = 0x78,
    ZSNNI = 0x79,
    ZSNZ = 0x7A,
    ZSNZI = 0x7B,
    ZSNP = 0x7C,
    ZSNPI = 0x7D,
    ZSEV = 0x7E,
    ZSEVI = 0x7F,

    // Load instructions (0x80-0x9F)
    LDB = 0x80,
    LDBI = 0x81,
    LDBU = 0x82,
    LDBUI = 0x83,
    LDW = 0x84,
    LDWI = 0x85,
    LDWU = 0x86,
    LDWUI = 0x87,
    LDT = 0x88,
    LDTI = 0x89,
    LDTU = 0x8A,
    LDTUI = 0x8B,
    LDO = 0x8C,
    LDOI = 0x8D,
    LDOU = 0x8E,
    LDOUI = 0x8F,
    LDSF = 0x90,
    LDSFI = 0x91,
    LDHT = 0x92,
    LDHTI = 0x93,
    CSWAP = 0x94,
    CSWAPI = 0x95,
    LDUNC = 0x96,
    LDUNCI = 0x97,
    LDVTS = 0x98,
    LDVTSI = 0x99,
    PRELD = 0x9A,
    PRELDI = 0x9B,
    PREGO = 0x9C,
    PREGOI = 0x9D,
    GO = 0x9E,
    GOI = 0x9F,

    // Store instructions (0xA0-0xBF)
    STB = 0xA0,
    STBI = 0xA1,
    STBU = 0xA2,
    STBUI = 0xA3,
    STW = 0xA4,
    STWI = 0xA5,
    STWU = 0xA6,
    STWUI = 0xA7,
    STT = 0xA8,
    STTI = 0xA9,
    STTU = 0xAA,
    STTUI = 0xAB,
    STO = 0xAC,
    STOI = 0xAD,
    STOU = 0xAE,
    STOUI = 0xAF,
    STSF = 0xB0,
    STSFI = 0xB1,
    STHT = 0xB2,
    STHTI = 0xB3,
    STCO = 0xB4,
    STCOI = 0xB5,
    STUNC = 0xB6,
    STUNCI = 0xB7,
    SYNCD = 0xB8,
    SYNCDI = 0xB9,
    PREST = 0xBA,
    PRESTI = 0xBB,
    SYNCID = 0xBC,
    SYNCIDI = 0xBD,
    PUSHGO = 0xBE,
    PUSHGOI = 0xBF,

    // Bitwise operations (0xC0-0xCF)
    OR = 0xC0,
    ORI = 0xC1,
    ORN = 0xC2,
    ORNI = 0xC3,
    NOR = 0xC4,
    NORI = 0xC5,
    XOR = 0xC6,
    XORI = 0xC7,
    AND = 0xC8,
    ANDI = 0xC9,
    ANDN = 0xCA,
    ANDNI = 0xCB,
    NAND = 0xCC,
    NANDI = 0xCD,
    NXOR = 0xCE,
    NXORI = 0xCF,

    // Bit manipulation (0xD0-0xDF)
    BDIF = 0xD0,
    BDIFI = 0xD1,
    WDIF = 0xD2,
    WDIFI = 0xD3,
    TDIF = 0xD4,
    TDIFI = 0xD5,
    ODIF = 0xD6,
    ODIFI = 0xD7,
    MUX = 0xD8,
    MUXI = 0xD9,
    SADD = 0xDA,
    SADDI = 0xDB,
    MOR = 0xDC,
    MORI = 0xDD,
    MXOR = 0xDE,
    MXORI = 0xDF,

    // SET family (0xE0-0xEF)
    SETH = 0xE0,
    SETMH = 0xE1,
    SETML = 0xE2,
    SETL = 0xE3,
    INCH = 0xE4,
    INCMH = 0xE5,
    INCML = 0xE6,
    INCL = 0xE7,
    ORH = 0xE8,
    ORMH = 0xE9,
    ORML = 0xEA,
    ORL = 0xEB,
    ANDNH = 0xEC,
    ANDNMH = 0xED,
    ANDNML = 0xEE,
    ANDNL = 0xEF,

    // System operations (0xF0-0xFF)
    JMP = 0xF0,
    JMPB = 0xF1,
    PUSHJ = 0xF2,
    PUSHJB = 0xF3,
    GETA = 0xF4,
    GETAB = 0xF5,
    PUT = 0xF6,
    PUTI = 0xF7,
    POP = 0xF8,
    RESUME = 0xF9,
    SAVE = 0xFA,
    UNSAVE = 0xFB,
    SYNC = 0xFC,
    SWYM = 0xFD,
    GET = 0xFE,
    TRIP = 0xFF,
}

impl TryFrom<u8> for Opcode {
    type Error = String;

    #[allow(unreachable_patterns)]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Opcode::TRAP),
            0x01 => Ok(Opcode::FCMP),
            0x02 => Ok(Opcode::FUN),
            0x03 => Ok(Opcode::FEQL),
            0x04 => Ok(Opcode::FADD),
            0x05 => Ok(Opcode::FIX),
            0x06 => Ok(Opcode::FSUB),
            0x07 => Ok(Opcode::FIXU),
            0x08 => Ok(Opcode::FLOT),
            0x09 => Ok(Opcode::FLOTI),
            0x0A => Ok(Opcode::FLOTU),
            0x0B => Ok(Opcode::FLOTUI),
            0x0C => Ok(Opcode::SFLOT),
            0x0D => Ok(Opcode::SFLOTI),
            0x0E => Ok(Opcode::SFLOTU),
            0x0F => Ok(Opcode::SFLOTUI),
            0x10 => Ok(Opcode::FMUL),
            0x11 => Ok(Opcode::FCMPE),
            0x12 => Ok(Opcode::FUNE),
            0x13 => Ok(Opcode::FEQLE),
            0x14 => Ok(Opcode::FDIV),
            0x15 => Ok(Opcode::FSQRT),
            0x16 => Ok(Opcode::FREM),
            0x17 => Ok(Opcode::FINT),
            0x18 => Ok(Opcode::MUL),
            0x19 => Ok(Opcode::MULI),
            0x1A => Ok(Opcode::MULU),
            0x1B => Ok(Opcode::MULUI),
            0x1C => Ok(Opcode::DIV),
            0x1D => Ok(Opcode::DIVI),
            0x1E => Ok(Opcode::DIVU),
            0x1F => Ok(Opcode::DIVUI),
            0x20 => Ok(Opcode::ADD),
            0x21 => Ok(Opcode::ADDI),
            0x22 => Ok(Opcode::ADDU),
            0x23 => Ok(Opcode::ADDUI),
            0x24 => Ok(Opcode::SUB),
            0x25 => Ok(Opcode::SUBI),
            0x26 => Ok(Opcode::SUBU),
            0x27 => Ok(Opcode::SUBUI),
            0x28 => Ok(Opcode::ADDU2),
            0x29 => Ok(Opcode::ADDU2I),
            0x2A => Ok(Opcode::ADDU4),
            0x2B => Ok(Opcode::ADDU4I),
            0x2C => Ok(Opcode::ADDU8),
            0x2D => Ok(Opcode::ADDU8I),
            0x2E => Ok(Opcode::ADDU16),
            0x2F => Ok(Opcode::ADDU16I),
            0x30 => Ok(Opcode::CMP),
            0x31 => Ok(Opcode::CMPI),
            0x32 => Ok(Opcode::CMPU),
            0x33 => Ok(Opcode::CMPUI),
            0x34 => Ok(Opcode::NEG),
            0x35 => Ok(Opcode::NEGI),
            0x36 => Ok(Opcode::NEGU),
            0x37 => Ok(Opcode::NEGUI),
            0x38 => Ok(Opcode::SL),
            0x39 => Ok(Opcode::SLI),
            0x3A => Ok(Opcode::SLU),
            0x3B => Ok(Opcode::SLUI),
            0x3C => Ok(Opcode::SR),
            0x3D => Ok(Opcode::SRI),
            0x3E => Ok(Opcode::SRU),
            0x3F => Ok(Opcode::SRUI),
            0x40 => Ok(Opcode::BN),
            0x41 => Ok(Opcode::BNB),
            0x42 => Ok(Opcode::BZ),
            0x43 => Ok(Opcode::BZB),
            0x44 => Ok(Opcode::BP),
            0x45 => Ok(Opcode::BPB),
            0x46 => Ok(Opcode::BOD),
            0x47 => Ok(Opcode::BODB),
            0x48 => Ok(Opcode::BNN),
            0x49 => Ok(Opcode::BNNB),
            0x4A => Ok(Opcode::BNZ),
            0x4B => Ok(Opcode::BNZB),
            0x4C => Ok(Opcode::BNP),
            0x4D => Ok(Opcode::BNPB),
            0x4E => Ok(Opcode::BEV),
            0x4F => Ok(Opcode::BEVB),
            0x50 => Ok(Opcode::PBN),
            0x51 => Ok(Opcode::PBNB),
            0x52 => Ok(Opcode::PBZ),
            0x53 => Ok(Opcode::PBZB),
            0x54 => Ok(Opcode::PBP),
            0x55 => Ok(Opcode::PBPB),
            0x56 => Ok(Opcode::PBOD),
            0x57 => Ok(Opcode::PBODB),
            0x58 => Ok(Opcode::PBNN),
            0x59 => Ok(Opcode::PBNNB),
            0x5A => Ok(Opcode::PBNZ),
            0x5B => Ok(Opcode::PBNZB),
            0x5C => Ok(Opcode::PBNP),
            0x5D => Ok(Opcode::PBNPB),
            0x5E => Ok(Opcode::PBEV),
            0x5F => Ok(Opcode::PBEVB),
            0x60 => Ok(Opcode::CSN),
            0x61 => Ok(Opcode::CSNI),
            0x62 => Ok(Opcode::CSZ),
            0x63 => Ok(Opcode::CSZI),
            0x64 => Ok(Opcode::CSP),
            0x65 => Ok(Opcode::CSPI),
            0x66 => Ok(Opcode::CSOD),
            0x67 => Ok(Opcode::CSODI),
            0x68 => Ok(Opcode::CSNN),
            0x69 => Ok(Opcode::CSNNI),
            0x6A => Ok(Opcode::CSNZ),
            0x6B => Ok(Opcode::CSNZI),
            0x6C => Ok(Opcode::CSNP),
            0x6D => Ok(Opcode::CSNPI),
            0x6E => Ok(Opcode::CSEV),
            0x6F => Ok(Opcode::CSEVI),
            0x70 => Ok(Opcode::ZSN),
            0x71 => Ok(Opcode::ZSNI),
            0x72 => Ok(Opcode::ZSZ),
            0x73 => Ok(Opcode::ZSZI),
            0x74 => Ok(Opcode::ZSP),
            0x75 => Ok(Opcode::ZSPI),
            0x76 => Ok(Opcode::ZSOD),
            0x77 => Ok(Opcode::ZSODI),
            0x78 => Ok(Opcode::ZSNN),
            0x79 => Ok(Opcode::ZSNNI),
            0x7A => Ok(Opcode::ZSNZ),
            0x7B => Ok(Opcode::ZSNZI),
            0x7C => Ok(Opcode::ZSNP),
            0x7D => Ok(Opcode::ZSNPI),
            0x7E => Ok(Opcode::ZSEV),
            0x7F => Ok(Opcode::ZSEVI),
            0x80 => Ok(Opcode::LDB),
            0x81 => Ok(Opcode::LDBI),
            0x82 => Ok(Opcode::LDBU),
            0x83 => Ok(Opcode::LDBUI),
            0x84 => Ok(Opcode::LDW),
            0x85 => Ok(Opcode::LDWI),
            0x86 => Ok(Opcode::LDWU),
            0x87 => Ok(Opcode::LDWUI),
            0x88 => Ok(Opcode::LDT),
            0x89 => Ok(Opcode::LDTI),
            0x8A => Ok(Opcode::LDTU),
            0x8B => Ok(Opcode::LDTUI),
            0x8C => Ok(Opcode::LDO),
            0x8D => Ok(Opcode::LDOI),
            0x8E => Ok(Opcode::LDOU),
            0x8F => Ok(Opcode::LDOUI),
            0x90 => Ok(Opcode::LDSF),
            0x91 => Ok(Opcode::LDSFI),
            0x92 => Ok(Opcode::LDHT),
            0x93 => Ok(Opcode::LDHTI),
            0x94 => Ok(Opcode::CSWAP),
            0x95 => Ok(Opcode::CSWAPI),
            0x96 => Ok(Opcode::LDUNC),
            0x97 => Ok(Opcode::LDUNCI),
            0x98 => Ok(Opcode::LDVTS),
            0x99 => Ok(Opcode::LDVTSI),
            0x9A => Ok(Opcode::PRELD),
            0x9B => Ok(Opcode::PRELDI),
            0x9C => Ok(Opcode::PREGO),
            0x9D => Ok(Opcode::PREGOI),
            0x9E => Ok(Opcode::GO),
            0x9F => Ok(Opcode::GOI),
            0xA0 => Ok(Opcode::STB),
            0xA1 => Ok(Opcode::STBI),
            0xA2 => Ok(Opcode::STBU),
            0xA3 => Ok(Opcode::STBUI),
            0xA4 => Ok(Opcode::STW),
            0xA5 => Ok(Opcode::STWI),
            0xA6 => Ok(Opcode::STWU),
            0xA7 => Ok(Opcode::STWUI),
            0xA8 => Ok(Opcode::STT),
            0xA9 => Ok(Opcode::STTI),
            0xAA => Ok(Opcode::STTU),
            0xAB => Ok(Opcode::STTUI),
            0xAC => Ok(Opcode::STO),
            0xAD => Ok(Opcode::STOI),
            0xAE => Ok(Opcode::STOU),
            0xAF => Ok(Opcode::STOUI),
            0xB0 => Ok(Opcode::STSF),
            0xB1 => Ok(Opcode::STSFI),
            0xB2 => Ok(Opcode::STHT),
            0xB3 => Ok(Opcode::STHTI),
            0xB4 => Ok(Opcode::STCO),
            0xB5 => Ok(Opcode::STCOI),
            0xB6 => Ok(Opcode::STUNC),
            0xB7 => Ok(Opcode::STUNCI),
            0xB8 => Ok(Opcode::SYNCD),
            0xB9 => Ok(Opcode::SYNCDI),
            0xBA => Ok(Opcode::PREST),
            0xBB => Ok(Opcode::PRESTI),
            0xBC => Ok(Opcode::SYNCID),
            0xBD => Ok(Opcode::SYNCIDI),
            0xBE => Ok(Opcode::PUSHGO),
            0xBF => Ok(Opcode::PUSHGOI),
            0xC0 => Ok(Opcode::OR),
            0xC1 => Ok(Opcode::ORI),
            0xC2 => Ok(Opcode::ORN),
            0xC3 => Ok(Opcode::ORNI),
            0xC4 => Ok(Opcode::NOR),
            0xC5 => Ok(Opcode::NORI),
            0xC6 => Ok(Opcode::XOR),
            0xC7 => Ok(Opcode::XORI),
            0xC8 => Ok(Opcode::AND),
            0xC9 => Ok(Opcode::ANDI),
            0xCA => Ok(Opcode::ANDN),
            0xCB => Ok(Opcode::ANDNI),
            0xCC => Ok(Opcode::NAND),
            0xCD => Ok(Opcode::NANDI),
            0xCE => Ok(Opcode::NXOR),
            0xCF => Ok(Opcode::NXORI),
            0xD0 => Ok(Opcode::BDIF),
            0xD1 => Ok(Opcode::BDIFI),
            0xD2 => Ok(Opcode::WDIF),
            0xD3 => Ok(Opcode::WDIFI),
            0xD4 => Ok(Opcode::TDIF),
            0xD5 => Ok(Opcode::TDIFI),
            0xD6 => Ok(Opcode::ODIF),
            0xD7 => Ok(Opcode::ODIFI),
            0xD8 => Ok(Opcode::MUX),
            0xD9 => Ok(Opcode::MUXI),
            0xDA => Ok(Opcode::SADD),
            0xDB => Ok(Opcode::SADDI),
            0xDC => Ok(Opcode::MOR),
            0xDD => Ok(Opcode::MORI),
            0xDE => Ok(Opcode::MXOR),
            0xDF => Ok(Opcode::MXORI),
            0xE0 => Ok(Opcode::SETH),
            0xE1 => Ok(Opcode::SETMH),
            0xE2 => Ok(Opcode::SETML),
            0xE3 => Ok(Opcode::SETL),
            0xE4 => Ok(Opcode::INCH),
            0xE5 => Ok(Opcode::INCMH),
            0xE6 => Ok(Opcode::INCML),
            0xE7 => Ok(Opcode::INCL),
            0xE8 => Ok(Opcode::ORH),
            0xE9 => Ok(Opcode::ORMH),
            0xEA => Ok(Opcode::ORML),
            0xEB => Ok(Opcode::ORL),
            0xEC => Ok(Opcode::ANDNH),
            0xED => Ok(Opcode::ANDNMH),
            0xEE => Ok(Opcode::ANDNML),
            0xEF => Ok(Opcode::ANDNL),
            0xF0 => Ok(Opcode::JMP),
            0xF1 => Ok(Opcode::JMPB),
            0xF2 => Ok(Opcode::PUSHJ),
            0xF3 => Ok(Opcode::PUSHJB),
            0xF4 => Ok(Opcode::GETA),
            0xF5 => Ok(Opcode::GETAB),
            0xF6 => Ok(Opcode::PUT),
            0xF7 => Ok(Opcode::PUTI),
            0xF8 => Ok(Opcode::POP),
            0xF9 => Ok(Opcode::RESUME),
            0xFA => Ok(Opcode::SAVE),
            0xFB => Ok(Opcode::UNSAVE),
            0xFC => Ok(Opcode::SYNC),
            0xFD => Ok(Opcode::SWYM),
            0xFE => Ok(Opcode::GET),
            0xFF => Ok(Opcode::TRIP),
            _ => Err(format!("Invalid opcode: {:#04x}", value)),
        }
    }
}

/// Resolved Z operand for an auto-immediate base mnemonic. The parser uses
/// this to choose between the RRR and RRI variants of an instruction.
#[derive(Debug, Clone, Copy)]
enum ZForm {
    Reg(u8),
    Imm(u8),
}

/// What an MMIXAL expression evaluates to: a pure 64-bit value, or a register
/// number. `Register` carries the full 64-bit value unary `$` produced, or
/// register arithmetic derived from one -- range-checking against 0..=255
/// happens where a register value is finally consumed, not here, since an
/// intermediate register-typed value may exceed 255 mid-expression.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExprValue {
    Pure(u64),
    Register(u64),
}

/// `fold_data_atoms`'s `child` parameter: evaluates one `data_term` or
/// `data_primary` into the atoms it contributes.
type EvalOperand = fn(&MMixAssembler, pest::iterators::Pair<Rule>) -> Result<DataAtoms, String>;

/// `fold_data_atoms`'s `apply` parameter, applying a binary operator to two
/// atoms. `apply_weak` and `apply_strong` share this signature.
type ApplyOperator =
    fn(&MMixAssembler, &str, ExprValue, ExprValue, usize, usize) -> Result<ExprValue, String>;

/// One or more values a `data_primary`, `data_term` or `data_expr`
/// contributes: a string's every character is its own atom, so a
/// multi-character string's interior stays free of whatever touches its
/// neighbors while its first and last atom combine with them. Emptiness is
/// unrepresentable, so combining two lists across an operator needs no
/// guard for it.
struct DataAtoms {
    head: ExprValue,
    tail: Vec<ExprValue>,
}

impl DataAtoms {
    fn one(value: ExprValue) -> Self {
        DataAtoms {
            head: value,
            tail: Vec::new(),
        }
    }

    fn from_chars(first: u32, rest: Vec<u32>) -> Self {
        DataAtoms {
            head: ExprValue::Pure(first as u64),
            tail: rest
                .into_iter()
                .map(|ch| ExprValue::Pure(ch as u64))
                .collect(),
        }
    }

    fn last(&self) -> ExprValue {
        *self.tail.last().unwrap_or(&self.head)
    }

    fn last_mut(&mut self) -> &mut ExprValue {
        self.tail.last_mut().unwrap_or(&mut self.head)
    }

    fn into_vec(self) -> Vec<ExprValue> {
        let mut items = vec![self.head];
        items.extend(self.tail);
        items
    }

    /// Merges `self` and `other` across an operator: combines `self`'s last
    /// atom with `other`'s first via `combine`, leaving every other atom in
    /// place.
    fn merge(
        mut self,
        other: DataAtoms,
        combine: impl FnOnce(ExprValue, ExprValue) -> Result<ExprValue, String>,
    ) -> Result<DataAtoms, String> {
        let combined = combine(self.last(), other.head)?;
        *self.last_mut() = combined;
        self.tail.extend(other.tail);
        Ok(self)
    }
}

/// A PC-relative displacement in the instruction encoding: the forward opcode
/// carries `field` directly, the backward opcode carries `2^bits - magnitude`.
#[derive(Debug, Clone, Copy)]
struct RelativeField {
    backward: bool,
    field: u32,
}

/// One input translation unit: its filename, the PREPROCESSED source the
/// parser walks, and the ORIGINAL (un-preprocessed) source `source_text`
/// reads from. `debug "text"` is the only rewrite `preprocess_debug` makes,
/// and it replaces exactly one line with exactly one line, so a preprocessed
/// line and its original always share the same 1-based number — no line map
/// to carry between them.
#[derive(Clone)]
struct SourceUnit {
    filename: String,
    preprocessed: String,
    /// Original, un-preprocessed source text as the user wrote it.
    original: String,
}

pub struct MMixAssembler {
    /// Input translation units in command-line order.
    sources: Vec<SourceUnit>,
    /// Filename of the source currently being walked (set during each pass).
    current_filename: String,
    /// Active PREFIX value applied to unqualified identifiers.
    /// Names starting with ':' bypass the prefix.
    current_prefix: String,
    pub labels: HashMap<String, u64>,
    pub symbols: HashMap<String, SymbolType>, // For IS directive - symbolic names with type
    /// First-definition site for user-defined labels: stored name -> (filename, line).
    /// Predefined symbols are not tracked here, so user code may shadow them.
    label_origins: HashMap<String, (String, usize)>,
    /// First-definition site for user-defined IS/GREG symbols.
    symbol_origins: HashMap<String, (String, usize)>,
    pub instructions: Vec<(u64, MMixInstruction)>,
    current_addr: u64,
    next_greg: u8, // Next global register to allocate (starts at 254, counts down)
    pub greg_inits: Vec<(u8, u64)>, // Global register initialization values: (register, value)
    /// How many of `greg_inits`' entries pass 2 has walked past so far --
    /// the two-operand memory form's base-address search bounds itself to
    /// `greg_inits[..greg_inits_seen]`, the `GREG`s the reference would have
    /// seen by this point in source order. Reset to 0 before pass 2; pass 1
    /// never reads it.
    greg_inits_seen: usize,
    /// Index into `sources` of the translation unit currently being walked
    /// (command-line order), set at the start of each unit in both passes.
    /// Used, rather than `current_filename` alone, to disambiguate two
    /// inputs that share a filename.
    current_unit_index: usize,
    /// Address -> original source location, populated during pass 2.
    debug_info: BTreeMap<u64, SourceLoc>,
    /// The strings every `debug` directive collected so far, `K`-indexed in
    /// program order across every translation unit added. What
    /// `debug_strings()` returns, and what `generate_object_code` hands the
    /// `.mmo` writer.
    debug_strings: Vec<Vec<u8>>,
    /// The file and original line of the first `debug` directive past the
    /// table's 256-entry limit, if the program has one. `parse` turns this
    /// into an assembly error before walking either pass.
    debug_directive_overflow: Option<(String, usize)>,
    /// The ten local-label lists, one per digit: each holds every `dH`
    /// occurrence's bound value, in source order, across the whole program.
    /// Built by pass 1; pass 2 reads them and appends nothing.
    local_labels: [Vec<SymbolType>; 10],
    /// Per-digit count of `dH` occurrences passed so far in the CURRENT
    /// pass. Reset to zero before pass 2 so it replays pass 1's sequence;
    /// `dB` reads index `count - 1` and `dF` reads index `count`, bumped
    /// by one when the referencing statement itself defines `dH` (see
    /// `local_pending_digit`).
    local_occurrence: [usize; 10],
    /// The digit the statement CURRENTLY being walked defines a `dH` for,
    /// if any -- set before its operand is evaluated and cleared after.
    /// A same-line reference to that same digit never resolves to the
    /// statement's own (not yet recorded) occurrence: `2H JMP 2F` on one
    /// line and `2H JMP 2B` on the next must jump to each other, not to
    /// themselves. `dF` reads one past `local_occurrence[digit]` in this
    /// case, since that slot is the statement's own, about to be pushed.
    local_pending_digit: Option<u8>,
    /// Every `LOCAL` declaration seen in pass 1: the declared register, and
    /// the site for the end-of-assembly threshold diagnostic.
    local_declarations: Vec<(u8, String, usize)>,
    /// Whether the walk is currently between a `BSPEC` and its `ESPEC`.
    in_special_mode: bool,
    /// Where the currently open `BSPEC` was written, for the
    /// unterminated-at-end-of-input diagnostic.
    bspec_open_site: Option<(String, usize)>,
    /// Every predefined symbol's root-namespace key, snapshotted right
    /// after `new` seeds them, before any user statement runs.
    predefined_names: HashSet<String>,
    /// First (file, line) a still-predefined name was named in an operand.
    /// A later label/IS/GREG redefining that name is an error exactly when
    /// this is populated: the reference already saw the predefined value.
    predefined_used_at: HashMap<String, (String, usize)>,
}

/// The original (user-facing) source location of an assembled instruction:
/// the file as given on the command line, and the 1-based line in that
/// file's ORIGINAL (un-preprocessed) text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub file: String,
    pub line: usize,
}

/// Why a candidate remark is mistakable for part of the statement rather
/// than commentary a reader could set apart.
enum RemarkAmbiguity {
    /// No blank separates the remark from the statement, so nothing marks
    /// where the statement ended.
    Abutting,
    /// The remark opens with a digit or one of the twelve
    /// [`MMixAssembler::REMARK_CONTINUATION_CHARS`]. A leading digit is a
    /// guard against a dropped operand separator, not a claim that an
    /// expression wanted it; a leading continuation character could extend
    /// the expression or operand list it follows.
    LeadingChar(char),
}

impl MMixAssembler {
    /// Blank out every line whose first character (column 1, before any
    /// leading blank) is not a letter, digit, `:` or `_` -- the MMIXAL
    /// reference's whole-line comment rule. Line-count-preserving, like
    /// `preprocess_debug`, and run before it so a comment line's text can
    /// never be mistaken for a `debug` directive. An indented line is not
    /// covered: its content parses normally, blank or not.
    fn blank_whole_line_comments(source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        for line in source.split_inclusive('\n') {
            let content = line.strip_suffix('\n').unwrap_or(line);
            let is_comment_line = match content.chars().next() {
                Some(' ') | Some('\t') | None => false,
                Some(c) => !(c.is_ascii_alphanumeric() || c == ':' || c == '_'),
            };
            if is_comment_line {
                if content.len() != line.len() {
                    result.push('\n');
                }
            } else {
                result.push_str(line);
            }
        }
        result
    }

    /// Rewrite each `debug "text"` directive in `source` into
    /// `TRAP 0,Debug,K` on the directive's own line and label, and collect
    /// its decoded text. `start_index` is the `K` the first directive here
    /// receives — the count of directives every earlier translation unit
    /// contributed. Nothing is written to guest memory and no label is
    /// generated, so `K` costs one byte and the directive costs one tetra.
    ///
    /// Returns the preprocessed source, the strings this source's
    /// directives contributed (in `K` order), and the file/line of the
    /// first directive to exceed the table's 256-entry limit, if any —
    /// `parse` turns that into an assembly error rather than emitting a `K`
    /// that cannot fit `TRAP`'s one-byte `Z`.
    fn preprocess_debug(
        source: &str,
        filename: &str,
        start_index: usize,
    ) -> (String, Vec<Vec<u8>>, Option<(String, usize)>) {
        // A directive, optionally preceded by its own label. A fixed,
        // valid pattern compiled once per call: infallible.
        let debug_re = Regex::new(r#"(?m)^([^\s]*\s+)?debug\s+"([^"]*)"\s*$"#).unwrap();

        let mut result = String::new();
        let mut strings = Vec::new();
        let mut overflow = None;

        for (index, line) in source.lines().enumerate() {
            match debug_re.captures(line) {
                Some(caps) => {
                    let k = start_index + strings.len();
                    if k > 255 {
                        if overflow.is_none() {
                            overflow = Some((filename.to_string(), index + 1));
                        }
                    } else {
                        let label = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                        result.push_str(label);
                        result.push_str(&format!("\tTRAP\t0,Debug,{k}\n"));
                    }
                    // debug text keeps the source's UTF-8 bytes: it lives
                    // outside guest memory, going straight to the host's
                    // handle 1, not through a data directive's per-character
                    // value.
                    strings.push(caps[2].as_bytes().to_vec());
                }
                None => {
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }

        debug!("Preprocessed source:\n{}", result);
        (result, strings, overflow)
    }

    /// Decode a data directive's string literal content into the values it
    /// represents: one item per character, its Unicode scalar value. Both
    /// passes go through this single function so neither can disagree with
    /// the other on a string's size.
    fn decode_char_values(content: &str) -> Vec<u32> {
        content.chars().map(|ch| ch as u32).collect()
    }

    /// Insert a predefined symbol at its root-namespace key. The root
    /// prefix is the empty string and a leading `:` on a reference is
    /// stripped before lookup (`qualify_name`), so `:name` reaches this
    /// same entry with no second copy needed. Predefined entries are not
    /// tracked in `symbol_origins`, so user code may shadow them without
    /// triggering a redefinition error.
    fn seed_predefined(symbols: &mut HashMap<String, SymbolType>, name: &str, ty: SymbolType) {
        symbols.insert(name.to_string(), ty);
    }

    pub fn new(source: &str, filename: &str) -> Self {
        let mut symbols = HashMap::new();

        // Standard MMIXAL predefined symbols
        // Segment constants
        Self::seed_predefined(
            &mut symbols,
            "Data_Segment",
            SymbolType::Constant(DATA_SEGMENT_START),
        );
        Self::seed_predefined(
            &mut symbols,
            "Pool_Segment",
            SymbolType::Constant(POOL_SEGMENT_START),
        );
        Self::seed_predefined(
            &mut symbols,
            "Stack_Segment",
            SymbolType::Constant(STACK_SEGMENT_START),
        );

        // Standard I/O handles: 0, 1, 2, exactly as the reference numbers
        // them, and as `MMix::initialize` opens them.
        Self::seed_predefined(&mut symbols, "StdIn", SymbolType::Constant(0));
        Self::seed_predefined(&mut symbols, "StdOut", SymbolType::Constant(1));
        Self::seed_predefined(&mut symbols, "StdErr", SymbolType::Constant(2));

        // Fopen's mode argument (MMIXAL reference).
        for (name, mode) in [
            ("TextRead", 0u64),
            ("TextWrite", 1),
            ("BinaryRead", 2),
            ("BinaryWrite", 3),
            ("BinaryReadWrite", 4),
        ] {
            Self::seed_predefined(&mut symbols, name, SymbolType::Constant(mode));
        }

        // TRAP function codes: the reference's eleven, then checksmix's own
        // extensions at #80-#82.
        for (name, code) in [
            ("Halt", TrapCode::Halt),
            ("Fopen", TrapCode::Fopen),
            ("Fclose", TrapCode::Fclose),
            ("Fread", TrapCode::Fread),
            ("Fgets", TrapCode::Fgets),
            ("Fgetws", TrapCode::Fgetws),
            ("Fwrite", TrapCode::Fwrite),
            ("Fputs", TrapCode::Fputs),
            ("Fputws", TrapCode::Fputws),
            ("Fseek", TrapCode::Fseek),
            ("Ftell", TrapCode::Ftell),
            ("Fputc", TrapCode::Fputc),
            ("Time", TrapCode::Time),
            ("Debug", TrapCode::Debug),
        ] {
            Self::seed_predefined(&mut symbols, name, SymbolType::Constant(code as u64));
        }

        // Special register names (for use with GET and PUT instructions)
        for (name, num) in [
            ("rB", 0u64),
            ("rD", 1),
            ("rE", 2),
            ("rH", 3),
            ("rJ", 4),
            ("rM", 5),
            ("rR", 6),
            ("rBB", 7),
            ("rC", 8),
            ("rN", 9),
            ("rO", 10),
            ("rS", 11),
            ("rI", 12),
            ("rT", 13),
            ("rTT", 14),
            ("rK", 15),
            ("rQ", 16),
            ("rU", 17),
            ("rV", 18),
            ("rG", 19),
            ("rL", 20),
            ("rA", 21),
            ("rF", 22),
            ("rP", 23),
            ("rW", 24),
            ("rX", 25),
            ("rY", 26),
            ("rZ", 27),
            ("rWW", 28),
            ("rXX", 29),
            ("rYY", 30),
            ("rZZ", 31),
        ] {
            Self::seed_predefined(&mut symbols, name, SymbolType::Constant(num));
        }

        // Y rounding-mode override values for FIX, FIXU, FSQRT, FINT, and the
        // FLOT/SFLOT families (MMIXAL reference). Numbered independently of
        // rA's own persistent-mode field (`RA_ROUND_SHIFT`): rA's ROUND_NEAR
        // is 0, but Y's is 4, since Y=0 is reserved to mean "no override".
        for (name, mode) in [
            ("ROUND_CURRENT", 0u64),
            ("ROUND_OFF", 1),
            ("ROUND_UP", 2),
            ("ROUND_DOWN", 3),
            ("ROUND_NEAR", 4),
        ] {
            Self::seed_predefined(&mut symbols, name, SymbolType::Constant(mode));
        }

        // Inf: positive floating-point infinity, the reference's two tetras
        // #7ff00000 and 0 read as one octabyte.
        Self::seed_predefined(
            &mut symbols,
            "Inf",
            SymbolType::Constant(0x7FF0000000000000),
        );

        // rA's eight event-flag bits and the eight user-trip handler
        // addresses, in the reference's own D V W I O U Z X order. The bit
        // values match `RA_D` … `RA_X` (`src/mmix/registers.rs`) and the
        // handler addresses match `MMIX.md`'s "User trips" table.
        for (bit_name, bit, handler_name, handler) in [
            ("D_BIT", 0x80u64, "D_Handler", 0x10u64),
            ("V_BIT", 0x40, "V_Handler", 0x20),
            ("W_BIT", 0x20, "W_Handler", 0x30),
            ("I_BIT", 0x10, "I_Handler", 0x40),
            ("O_BIT", 0x08, "O_Handler", 0x50),
            ("U_BIT", 0x04, "U_Handler", 0x60),
            ("Z_BIT", 0x02, "Z_Handler", 0x70),
            ("X_BIT", 0x01, "X_Handler", 0x80),
        ] {
            Self::seed_predefined(&mut symbols, bit_name, SymbolType::Constant(bit));
            Self::seed_predefined(&mut symbols, handler_name, SymbolType::Constant(handler));
        }

        // Every predefined name lives at the root namespace; a program's own
        // definition of one of these is a plain shadow, tracked from here.
        let predefined_names: HashSet<String> = symbols.keys().cloned().collect();

        // Blank whole-line comments, then expand debug directives.
        let blanked_source = Self::blank_whole_line_comments(source);
        let (preprocessed_source, debug_strings, overflow) =
            Self::preprocess_debug(&blanked_source, filename, 0);

        Self {
            sources: vec![SourceUnit {
                filename: filename.to_string(),
                preprocessed: preprocessed_source,
                original: source.to_string(),
            }],
            current_filename: filename.to_string(),
            current_prefix: String::new(),
            labels: HashMap::new(),
            symbols,
            label_origins: HashMap::new(),
            symbol_origins: HashMap::new(),
            instructions: Vec::new(),
            current_addr: 0,
            next_greg: 254, // Start allocating from $254, count down
            greg_inits: Vec::new(),
            greg_inits_seen: 0,
            current_unit_index: 0,
            debug_info: BTreeMap::new(),
            debug_strings,
            debug_directive_overflow: overflow,
            local_labels: Default::default(),
            local_occurrence: [0; 10],
            local_pending_digit: None,
            local_declarations: Vec::new(),
            in_special_mode: false,
            bspec_open_site: None,
            predefined_names,
            predefined_used_at: HashMap::new(),
        }
    }

    /// Append another translation unit. Files are processed in the order they
    /// are added; symbols, labels, GREG state, and `current_addr` carry over,
    /// so the result is identical to assembling the concatenation of inputs.
    pub fn add_source(&mut self, source: &str, filename: &str) {
        let start_index = self.debug_strings.len();
        let blanked_source = Self::blank_whole_line_comments(source);
        let (preprocessed, mut strings, overflow) =
            Self::preprocess_debug(&blanked_source, filename, start_index);
        self.debug_strings.append(&mut strings);
        if self.debug_directive_overflow.is_none() {
            self.debug_directive_overflow = overflow;
        }
        self.sources.push(SourceUnit {
            filename: filename.to_string(),
            preprocessed,
            original: source.to_string(),
        });
    }

    /// The strings every `debug` directive collected, `K`-indexed in
    /// program order across every translation unit: string `K` is at index
    /// `K`, decoded as `BYTE` decodes a string literal, with no trailing
    /// newline added.
    pub fn debug_strings(&self) -> &[Vec<u8>] {
        &self.debug_strings
    }

    /// Apply the active PREFIX to a raw identifier. The root prefix is the
    /// empty string; a name beginning with ':' opts out of the active
    /// PREFIX and is stored at the root, one leading colon stripped -- so
    /// `x` and `:x` name the same root-level symbol.
    fn qualify_name(&self, raw: &str) -> String {
        match raw.strip_prefix(':') {
            Some(rest) => rest.to_string(),
            None => format!("{}{}", self.current_prefix, raw),
        }
    }

    /// The value currently stored under `name`, checking `labels` before
    /// `symbols` -- the order that lets a program's own label win over a
    /// predefined symbol of the same name.
    fn existing_value(&self, name: &str) -> Option<SymbolType> {
        self.labels
            .get(name)
            .map(|&addr| SymbolType::Constant(addr))
            .or_else(|| self.symbols.get(name).copied())
    }

    /// Checks whether `name` may be bound to `candidate`, per the
    /// redefinition rules: an existing user definition must match
    /// `candidate` exactly (a differing one is the ordinary redefinition
    /// error); a still-predefined, not-yet-shadowed name may be redefined
    /// only if no earlier statement has used its predefined value.
    /// `Ok(true)` means the caller should record the new origin and store
    /// `candidate`; `Ok(false)` means an equal redefinition needs no
    /// further action.
    fn check_definable(
        &self,
        name: &str,
        candidate: SymbolType,
        line: usize,
    ) -> Result<bool, String> {
        if let Some((prev_file, prev_line)) = self
            .label_origins
            .get(name)
            .or_else(|| self.symbol_origins.get(name))
        {
            return if self.existing_value(name) == Some(candidate) {
                Ok(false)
            } else {
                Err(format!(
                    "{}:{}: symbol '{}' redefined (first defined at {}:{})",
                    self.current_filename, line, name, prev_file, prev_line
                ))
            };
        }
        if self.predefined_names.contains(name)
            && let Some((used_file, used_line)) = self.predefined_used_at.get(name)
        {
            return Err(format!(
                "{}:{}: predefined symbol '{}' redefined after its value was used at {}:{}",
                self.current_filename, line, name, used_file, used_line
            ));
        }
        Ok(true)
    }

    /// Define a label (instruction/data/standalone) at the current address.
    fn define_label(&mut self, raw: &str, addr: u64, line: usize) -> Result<(), String> {
        let name = self.qualify_name(raw);
        if self.check_definable(&name, SymbolType::Constant(addr), line)? {
            self.label_origins
                .insert(name.clone(), (self.current_filename.clone(), line));
            self.labels.insert(name, addr);
        }
        Ok(())
    }

    /// Define an IS- or GREG-bound symbol.
    fn define_symbol(&mut self, raw: &str, ty: SymbolType, line: usize) -> Result<(), String> {
        let name = self.qualify_name(raw);
        if self.check_definable(&name, ty, line)? {
            self.symbol_origins
                .insert(name.clone(), (self.current_filename.clone(), line));
            self.symbols.insert(name, ty);
        }
        Ok(())
    }

    /// Record `name`'s first use site if it is still an unshadowed
    /// predefined symbol -- the site a later redefinition attempt cites.
    fn mark_predefined_use(&mut self, name: &str, line: usize) {
        if self.predefined_names.contains(name)
            && !self.label_origins.contains_key(name)
            && !self.symbol_origins.contains_key(name)
            && !self.predefined_used_at.contains_key(name)
        {
            self.predefined_used_at
                .insert(name.to_string(), (self.current_filename.clone(), line));
        }
    }

    /// Walk `pair` and every descendant, marking each `global_id` leaf as a
    /// use. Callers scope `pair` to an operand -- never a definition's own
    /// name -- so every `global_id` found here is a reference, not a
    /// binding.
    fn scan_uses_for_redefinition(&mut self, pair: &pest::iterators::Pair<Rule>) {
        if pair.as_rule() == Rule::global_id {
            let (line, _) = pair.line_col();
            let qualified = self.qualify_name(pair.as_str());
            self.mark_predefined_use(&qualified, line);
            return;
        }
        for inner in pair.clone().into_inner() {
            self.scan_uses_for_redefinition(&inner);
        }
    }

    /// Format Pest parse errors in a user-friendly way. Pest reports a line
    /// in the preprocessed text, which is also the line the user wrote it
    /// on (`preprocess_debug` never changes a source's line count).
    /// `source` is the preprocessed text the failed parse walked: an
    /// unterminated group reports as pest expecting more operator content
    /// (`weak_op`/`strong_op`/`group_ws`), never a missing `)`, so naming it
    /// takes a look at the source line rather than at pest's own positives.
    fn format_parse_error(
        error: &pest::error::Error<Rule>,
        filename: &str,
        source: &str,
    ) -> String {
        use pest::error::LineColLocation;

        let (line, col) = match error.line_col {
            LineColLocation::Pos((l, c)) => (l, c),
            LineColLocation::Span((l, c), _) => (l, c),
        };

        if let pest::error::ErrorVariant::ParsingError { positives, .. } = &error.variant
            && Self::expects_more_group_content(positives)
            && Self::line_has_unclosed_paren(source, line)
        {
            return format!(
                "{}:{}:{}: syntax error: unterminated group",
                filename, line, col
            );
        }

        let expected_msg = Self::describe_expected(&error.variant);
        format!(
            "{}:{}:{}: syntax error: expected {}",
            filename, line, col, expected_msg
        )
    }

    /// Render a pest error's `positives` (or custom message) as the
    /// user-facing "expected ..." fragment. Shared by `format_parse_error`
    /// and `format_reparse_error`, which builds the same kind of line from
    /// a sub-parse pest never attempted at the top level.
    fn describe_expected(variant: &pest::error::ErrorVariant<Rule>) -> String {
        match variant {
            pest::error::ErrorVariant::ParsingError { positives, .. } => {
                if positives.is_empty() {
                    "valid MMIX instruction or directive".to_string()
                } else {
                    // Try to make the expected rules more user-friendly
                    let friendly: Vec<String> = positives
                        .iter()
                        .map(|r| match r {
                            Rule::instruction => "instruction".to_string(),
                            Rule::directive => "directive".to_string(),
                            Rule::directive_is => "IS directive (symbol definition)".to_string(),
                            Rule::directive_loc => "LOC directive".to_string(),
                            Rule::expr => "number or expression".to_string(),
                            Rule::global_id => "label or symbol name".to_string(),
                            Rule::identifier => "label or symbol name".to_string(),
                            // A data-list item holding no string parses through
                            // the same grammar as an instruction operand; its
                            // own rule names must never leak into a diagnostic
                            // that `expr` would report identically.
                            Rule::data_term => "data_value".to_string(),
                            Rule::data_primary => "primary".to_string(),
                            Rule::data_group_primary => "group_primary".to_string(),
                            _ => format!("{:?}", r),
                        })
                        .collect();

                    if friendly.len() == 1 {
                        friendly[0].clone()
                    } else {
                        format!("one of: {}", friendly.join(", "))
                    }
                }
            }
            pest::error::ErrorVariant::CustomError { message } => message.clone(),
        }
    }

    /// Format a re-parse `error` (from testing a substring of `source` in
    /// isolation) as a diagnostic in `source`'s own coordinates.
    /// `base_offset` is where that substring began in `source`; pest's own
    /// `error` reports a position relative to the substring, not `source`.
    fn format_reparse_error(
        error: &pest::error::Error<Rule>,
        filename: &str,
        source: &str,
        base_offset: usize,
    ) -> String {
        let sub_pos = match error.location {
            pest::error::InputLocation::Pos(p) => p,
            pest::error::InputLocation::Span((s, _)) => s,
        };
        let (line, col) = pest::Position::new(source, base_offset + sub_pos)
            .map(|p| p.line_col())
            .unwrap_or((1, 1));
        let expected_msg = Self::describe_expected(&error.variant);
        format!("{filename}:{line}:{col}: syntax error: expected {expected_msg}")
    }

    /// True when pest's positives suggest it was still trying to extend an
    /// expression -- the shape an unterminated group's failure takes.
    fn expects_more_group_content(positives: &[Rule]) -> bool {
        positives
            .iter()
            .any(|r| matches!(r, Rule::weak_op | Rule::strong_op | Rule::group_ws))
    }

    /// True when `line` (1-based, in `source`) has more `(` than `)`.
    /// Used only by `format_parse_error`, where pest has already failed to
    /// parse the whole input and there is no statement span to scope to.
    fn line_has_unclosed_paren(source: &str, line: usize) -> bool {
        let Some(text) = source.lines().nth(line - 1) else {
            return false;
        };
        Self::segment_has_unclosed_paren(text)
    }

    /// True when `segment` has more `(` than `)`, skipping the contents of
    /// any string or character literal so a quoted `(` or `)` is never
    /// counted. `diagnose_unrecognized_opcode` scopes `segment` to one
    /// statement's own span, never the whole physical line, so a sibling
    /// statement's parens (on either side of a `;`) can't be blamed on this
    /// one.
    fn segment_has_unclosed_paren(segment: &str) -> bool {
        let mut depth: i32 = 0;
        let mut chars = segment.char_indices();
        while let Some((_, ch)) = chars.next() {
            if Self::skip_literal(&mut chars, ch) {
                continue;
            }
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        depth > 0
    }

    /// Advances `chars` past a string or character literal `ch` opens --
    /// a `"` consumes to the next `"`, a `'` consumes exactly one character
    /// plus the closing quote -- returning whether `ch` opened one. Shared
    /// by every scan that walks a segment's raw text ignoring literal
    /// contents, so a quoted `(`, `)` or `%` is never mistaken for this
    /// release's own syntax.
    fn skip_literal(chars: &mut std::str::CharIndices, ch: char) -> bool {
        match ch {
            '"' => {
                for (_, c) in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                }
                true
            }
            '\'' => {
                chars.next(); // the character
                chars.next(); // the closing quote, if present
                true
            }
            _ => false,
        }
    }

    /// Byte offset of the first `%` in `segment` that sits outside a
    /// string or character literal, or `segment.len()` when there is
    /// none. The unknown-operation diagnostic prints `segment[..cut]` as
    /// the offending statement, so a literal's own `%` (`BYTE "50%"`) is
    /// never mistaken for this release's comment opener and truncated
    /// mid-literal.
    fn segment_comment_start(segment: &str) -> usize {
        let mut chars = segment.char_indices();
        while let Some((idx, ch)) = chars.next() {
            if Self::skip_literal(&mut chars, ch) {
                continue;
            }
            if ch == '%' {
                return idx;
            }
        }
        segment.len()
    }

    /// Characters that could extend a bare expression or an operand list.
    /// A remark opening with one of these, right after the blank that ends
    /// the operand field, would read as continuing the statement rather
    /// than as commentary -- dropping an operand in silence if it were
    /// ignored.
    const REMARK_CONTINUATION_CHARS: [char; 12] =
        [',', '+', '-', '*', '/', '~', '&', '|', '^', '<', '>', '$'];

    /// `None` when `text`, preceded by `blank_before`, reads as a remark;
    /// `Some` naming the ambiguity otherwise. Rule 1 -- `EXPR` is greedy -- has
    /// already run by the time this is called: `text` is only ever what
    /// `EXPR` left behind.
    fn remark_ambiguity(text: &str, blank_before: bool) -> Option<RemarkAmbiguity> {
        if !blank_before {
            return Some(RemarkAmbiguity::Abutting);
        }
        let first = text.chars().next()?;
        (first.is_ascii_digit() || Self::REMARK_CONTINUATION_CHARS.contains(&first))
            .then_some(RemarkAmbiguity::LeadingChar(first))
    }

    /// Formats the diagnostic for `ambiguity`, at `filename:line:col`.
    fn format_remark_ambiguity(
        ambiguity: &RemarkAmbiguity,
        filename: &str,
        line: usize,
        col: usize,
    ) -> String {
        match ambiguity {
            RemarkAmbiguity::Abutting => format!(
                "{filename}:{line}:{col}: syntax error: a remark must be separated from the \
                 statement by a blank"
            ),
            RemarkAmbiguity::LeadingChar(c) => format!(
                "{filename}:{line}:{col}: syntax error: a remark cannot begin with `{c}` — it \
                 reads as part of the statement; start a comment with `%`"
            ),
        }
    }

    /// Formats "unknown operation: {statement}" for a bare word in the OP
    /// field, or for a candidate remark with no statement ahead of it:
    /// `segment`, its `%` comment stripped (raw inside the atomic `remark`
    /// capture, so the grammar never trims it) and the blanks an indented
    /// or post-`;` statement carries trimmed off, is the statement the
    /// reader wrote.
    fn unknown_operation_error(segment: &str, filename: &str, line: usize, col: usize) -> String {
        let statement = segment[..Self::segment_comment_start(segment)].trim();
        format!("{filename}:{line}:{col}: syntax error: unknown operation: {statement}")
    }

    /// Rule 2: what `EXPR` (rule 1) left behind is a remark unless it is
    /// mistakable for part of the statement. `has_statement` is false when
    /// no `Rule::statement` preceded `pair` in its segment; a remark
    /// presupposes a statement to follow, so an ambiguity there reports an
    /// unknown operation instead of a remark diagnostic, and `segment_start` bounds that statement
    /// text to this segment alone -- never a sibling statement's text on
    /// the same line.
    fn check_remark(
        pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
        has_statement: bool,
    ) -> Result<(), String> {
        let text = pair.as_str();
        if text.is_empty() {
            return Ok(());
        }
        let (line, col) = pair.line_col();
        let start = pair.as_span().start();
        let blank_before = start > 0 && matches!(source.as_bytes()[start - 1], b' ' | b'\t');

        let Some(ambiguity) = Self::remark_ambiguity(text, blank_before) else {
            return Ok(());
        };

        if !has_statement {
            let segment = &source[segment_start..pair.as_span().end()];
            return Err(Self::unknown_operation_error(segment, filename, line, col));
        }
        Err(Self::format_remark_ambiguity(
            &ambiguity, filename, line, col,
        ))
    }

    /// A bare label whose statement position holds a word naming no
    /// instruction or directive -- not an ambiguity test on a remark, but a
    /// diagnosis of that word: an unclosed group that swallowed a real
    /// instruction whole, a known mnemonic or directive missing or
    /// malformed its operand, or, failing both, the statement itself so the
    /// reader can place the fault. Naming which specific word is at fault
    /// is ambiguous in general (a valid label followed by a bad mnemonic
    /// and a bad mnemonic swallowed as a label are the same shape).
    /// `segment_start` bounds the unclosed-group check to this statement's
    /// own text, from wherever it began (the line's start, or just past the
    /// previous `;`) to `pair`'s own end -- never a sibling statement's text
    /// on the same line.
    fn diagnose_unrecognized_opcode(
        pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> Result<(), String> {
        let text = pair.as_str();
        if text.is_empty() {
            return Ok(());
        }
        let (line, col) = pair.line_col();

        // An unclosed group makes every instruction alternative fail deep
        // inside its operand, so `statement` falls back to reading the
        // mnemonic as a bare label and leaves the rest for `remark` --
        // trading the real problem for a confusing one unless caught here.
        // A comment after a *successful* match never reaches this
        // function, so a stray `(` in commentary is never mistaken for an
        // unterminated group.
        let segment = &source[segment_start..pair.as_span().end()];
        if Self::segment_has_unclosed_paren(segment) {
            return Err(format!(
                "{filename}:{line}:{col}: syntax error: unterminated group"
            ));
        }
        let remark_start = pair.as_span().start();
        if let Some((error, base_offset)) =
            Self::recognized_keyword_error(text, remark_start, segment, segment_start)
        {
            return Err(Self::format_reparse_error(
                &error,
                filename,
                source,
                base_offset,
            ));
        }
        Err(Self::unknown_operation_error(segment, filename, line, col))
    }

    /// Directive keyword-only rules, paired with the full directive rule
    /// that gives a meaningful "missing/malformed operand" diagnostic once
    /// the keyword itself is confirmed present. `directive_is` isn't here:
    /// it is the one directive whose own grammar folds in the preceding
    /// label, so it needs the whole segment, not `remark_text` alone.
    const DIRECTIVE_KEYWORD_RULES: [(Rule, Rule); 10] = [
        (Rule::directive_loc, Rule::loc_directive),
        (Rule::directive_greg, Rule::greg_directive),
        (Rule::directive_prefix, Rule::prefix_directive),
        (Rule::directive_byte, Rule::data_directive),
        (Rule::directive_wyde, Rule::data_directive),
        (Rule::directive_tetra, Rule::data_directive),
        (Rule::directive_octa, Rule::data_directive),
        (Rule::directive_local, Rule::local_directive),
        (Rule::directive_bspec, Rule::bspec_directive),
        (Rule::directive_espec, Rule::espec_directive),
    ];

    /// When `remark_text` opens with a recognized mnemonic or directive
    /// keyword, re-parse the construct that keyword belongs to and return
    /// its own error, plus the byte offset (into the original source) that
    /// error's position is relative to -- so a known keyword with a
    /// missing or malformed operand (`Foo IS`, `Foo GREG`, `Foo SET`)
    /// reports what pest actually expected there, never a made-up
    /// "unknown operation". Every alternative in `instruction` opens with
    /// a literal mnemonic, so a failure at position 0 there means none
    /// matched even a prefix; a failure past position 0 means a mnemonic
    /// matched and only the operand is missing or malformed. `segment`
    /// (from `segment_start`) is `remark_text`'s own statement, label
    /// included, the only span `directive_is` can be re-parsed against,
    /// since its grammar requires that label as part of the rule itself.
    fn recognized_keyword_error(
        remark_text: &str,
        remark_start: usize,
        segment: &str,
        segment_start: usize,
    ) -> Option<(pest::error::Error<Rule>, usize)> {
        use pest::Parser;

        if MMixalParser::parse(Rule::directive_is, remark_text).is_ok()
            && let Err(e) = MMixalParser::parse(Rule::directive, segment)
        {
            return Some((e, segment_start));
        }
        for (keyword, full_rule) in Self::DIRECTIVE_KEYWORD_RULES {
            if MMixalParser::parse(keyword, remark_text).is_ok()
                && let Err(e) = MMixalParser::parse(full_rule, remark_text)
            {
                return Some((e, remark_start));
            }
        }
        if let Err(e) = MMixalParser::parse(Rule::instruction, remark_text) {
            let pos = match e.location {
                pest::error::InputLocation::Pos(p) => p,
                pest::error::InputLocation::Span((s, _)) => s,
            };
            if pos > 0 {
                return Some((e, remark_start));
            }
        }
        None
    }

    /// True when `pair` (a `Rule::statement`) is a bare label with no
    /// instruction or directive attached -- the one shape whose candidate
    /// remark `diagnose_unrecognized_opcode` diagnoses rather than reading
    /// as a remark.
    fn statement_is_label_only(pair: &pest::iterators::Pair<Rule>) -> bool {
        let mut inner = pair.clone().into_inner();
        matches!(
            inner.next().map(|p| p.as_rule()),
            Some(Rule::label_def | Rule::local_label_def)
        ) && inner.next().is_none()
    }

    /// Whether the line whose `Rule::line` pair starts at byte `start` in
    /// `source` opens with a blank or a tab. `program` is not atomic, so
    /// pest already skipped the run of blanks, tabs and carriage returns
    /// `WHITESPACE` consumes ahead of the pair's own span; walking that run
    /// back from `start` to the previous newline (or the start of file)
    /// finds the line's own first byte, with no rescan of the line's text
    /// beyond its own leading run.
    fn line_opens_indented(source: &str, start: usize) -> bool {
        let bytes = source.as_bytes();
        let mut first = start;
        while first > 0 && matches!(bytes[first - 1], b' ' | b'\t' | b'\r') {
            first -= 1;
        }
        first < bytes.len() && matches!(bytes[first], b' ' | b'\t')
    }

    /// The label-shaped pair that opens `stmt_pair`'s match, and the pair
    /// right after it if the label was not the whole match: `label_def` or
    /// `local_label_def` from `statement`'s own label alternatives, paired
    /// with the instruction or directive that followed; or the local label
    /// / global id `is_directive` reads ahead of its own `IS` keyword --
    /// the one directive whose grammar folds a label into itself -- paired
    /// with `IS`'s own keyword pair. `(None, _)` when `stmt_pair` matched
    /// via `instruction` or a directive that carries no label.
    fn statement_label_split<'i>(
        stmt_pair: &pest::iterators::Pair<'i, Rule>,
    ) -> (
        Option<pest::iterators::Pair<'i, Rule>>,
        Option<pest::iterators::Pair<'i, Rule>>,
    ) {
        let mut inner = stmt_pair.clone().into_inner();
        let Some(first) = inner.next() else {
            return (None, None);
        };
        match first.as_rule() {
            Rule::label_def | Rule::local_label_def => {
                let after = inner.next();
                (Some(first), after)
            }
            Rule::directive => {
                let Some(d) = first.into_inner().next() else {
                    return (None, None);
                };
                if d.as_rule() == Rule::is_directive {
                    let mut is_inner = d.into_inner();
                    let label = is_inner.next();
                    let after = is_inner.next();
                    (label, after)
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        }
    }

    /// The label-shaped pair that opens `stmt_pair`'s match, if any. See
    /// `statement_label_split`.
    fn statement_label_pair<'i>(
        stmt_pair: &pest::iterators::Pair<'i, Rule>,
    ) -> Option<pest::iterators::Pair<'i, Rule>> {
        Self::statement_label_split(stmt_pair).0
    }

    /// The pair right after `stmt_pair`'s opening label, if the label was
    /// not the whole match. See `statement_label_split`.
    fn statement_after_label<'i>(
        stmt_pair: &pest::iterators::Pair<'i, Rule>,
    ) -> Option<pest::iterators::Pair<'i, Rule>> {
        Self::statement_label_split(stmt_pair).1
    }

    /// An indented line has no label field, so a statement that opened by
    /// reading one is an unknown operation even when a real instruction or
    /// directive followed it (`\tFoo SET $2,9`, `\t2H JMP 2F`, an indented
    /// `Foo IS 5`). `{col}` lands on whatever
    /// followed the label -- the instruction, the directive, or `IS`'s own
    /// keyword -- matching where the same diagnostic already lands for a
    /// bare word followed by trailing text.
    fn indented_label_statement_error(
        stmt_pair: &pest::iterators::Pair<Rule>,
        label_pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> String {
        let segment = &source[segment_start..stmt_pair.as_span().end()];
        let (line, col) = Self::statement_after_label(stmt_pair)
            .map(|p| p.line_col())
            .unwrap_or_else(|| label_pair.line_col());
        Self::unknown_operation_error(segment, filename, line, col)
    }

    /// An indented line's lone word, with nothing at all following it, is
    /// an unknown operation rather than a silently defined label. `{col}`
    /// is the word itself, since nothing follows it to point at; `{statement}`
    /// is formed the way the opcode-field diagnosis forms it -- the source
    /// text from the line's start through `remark_pair`'s end, trimmed --
    /// since `label_pair`'s own span can carry trailing blanks that a
    /// following optional token left unconsumed.
    fn indented_bare_label_error(
        label_pair: &pest::iterators::Pair<Rule>,
        remark_pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> String {
        let segment = &source[segment_start..remark_pair.as_span().end()];
        let (line, col) = label_pair.line_col();
        Self::unknown_operation_error(segment, filename, line, col)
    }

    /// `SAVE` and `UNSAVE` are the two of the seven bare mnemonics the
    /// reference's empty-field-is-0 rule does not cover: `SAVE` takes
    /// exactly two operands, so the one implicit operand an empty field
    /// gives is not enough to assemble it, and `UNSAVE`'s one-operand form
    /// reads that implicit operand as a register, which 0 is not.
    /// Both are errors in every position a bare word can
    /// appear -- indented, column 1, or after `;` -- never a silently
    /// defined label. `None` when `label_pair`'s name is neither. `SAVE`'s
    /// diagnostic names the statement as written -- `{statement}`, formed
    /// as `indented_bare_label_error` forms it -- so a trailing colon
    /// shows; `UNSAVE`'s does not name the word at all.
    fn bare_reserved_mnemonic_error(
        label_pair: &pest::iterators::Pair<Rule>,
        remark_pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> Option<String> {
        let name = label_pair
            .clone()
            .into_inner()
            .next()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
        let (line, col) = label_pair.line_col();
        match name.as_str() {
            "SAVE" => {
                let segment = &source[segment_start..remark_pair.as_span().end()];
                Some(Self::unknown_operation_error(segment, filename, line, col))
            }
            "UNSAVE" => Some(format!(
                "{filename}:{line}:{col}: pure value 0 cannot be used where a register is required"
            )),
            _ => None,
        }
    }

    #[instrument(skip(self))]
    pub fn parse(&mut self) -> Result<(), String> {
        if let Some((file, line)) = &self.debug_directive_overflow {
            return Err(format!(
                "{file}:{line}: error: too many `debug` directives in this \
                 program; the string table holds at most 256"
            ));
        }

        debug!("Starting MMIXAL parsing (two-pass)");
        match self.parse_two_pass() {
            Ok(_) => {
                debug!(
                    instruction_count = self.instructions.len(),
                    label_count = self.labels.len(),
                    symbol_count = self.symbols.len(),
                    "Parsing completed successfully"
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Two-pass assembler:
    /// Pass 1: Collect all labels and their addresses, process IS directives
    /// Pass 2: Generate instructions with resolved label references
    ///
    /// Each pass walks every translation unit in command-line order, threading
    /// `current_addr`, `current_prefix`, and the symbol tables across files so
    /// the result matches assembling the concatenation of the inputs. The
    /// PREFIX state is reset at the start of each pass.
    #[instrument(skip(self))]
    fn parse_two_pass(&mut self) -> Result<(), String> {
        use pest::Parser;

        let sources = self.sources.clone();
        debug!("Pass 1: Collecting labels and symbols");
        self.current_prefix.clear();
        self.local_occurrence = [0; 10];
        self.in_special_mode = false;
        self.bspec_open_site = None;

        for (index, unit) in sources.iter().enumerate() {
            self.current_filename = unit.filename.clone();
            self.current_unit_index = index;
            let pairs = MMixalParser::parse(Rule::program, &unit.preprocessed)
                .map_err(|e| Self::format_parse_error(&e, &unit.filename, &unit.preprocessed))?;
            for pair in pairs {
                if pair.as_rule() == Rule::program {
                    for line_pair in pair.into_inner() {
                        if line_pair.as_rule() == Rule::line {
                            self.first_pass_line(line_pair, &unit.preprocessed, &unit.filename)?;
                        }
                    }
                }
            }
        }

        if let Some((file, line)) = &self.bspec_open_site {
            return Err(format!(
                "{file}:{line}: syntax error: BSPEC has no matching ESPEC before end of input"
            ));
        }

        // LOCAL's threshold is fixed only once every GREG has allocated:
        // one above the lowest register GREG handed out, `$31` at the
        // lowest -- $0..$31 are local on every MMIX regardless of GREG
        // activity.
        let threshold = (self.next_greg as u16).saturating_add(1).max(32);
        for (reg, file, line) in &self.local_declarations {
            if u16::from(*reg) >= threshold {
                return Err(format!(
                    "{file}:{line}: LOCAL ${reg} is not below the global threshold ${threshold}"
                ));
            }
        }

        debug!(
            "Pass 1 complete: {} labels, {} symbols",
            self.labels.len(),
            self.symbols.len()
        );

        let saved_addr = self.current_addr;
        self.current_addr = 0;
        self.current_prefix.clear();
        self.local_occurrence = [0; 10];
        self.in_special_mode = false;
        self.greg_inits_seen = 0;

        debug!("Pass 2: Generating instructions");

        for (index, unit) in sources.iter().enumerate() {
            self.current_filename = unit.filename.clone();
            self.current_unit_index = index;
            let pairs = MMixalParser::parse(Rule::program, &unit.preprocessed)
                .map_err(|e| Self::format_parse_error(&e, &unit.filename, &unit.preprocessed))?;
            for pair in pairs {
                if pair.as_rule() == Rule::program {
                    for line_pair in pair.into_inner() {
                        if line_pair.as_rule() == Rule::line {
                            for stmt_pair in line_pair.into_inner() {
                                if stmt_pair.as_rule() == Rule::statement {
                                    self.second_pass_statement(stmt_pair)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        self.current_addr = saved_addr;
        Ok(())
    }

    /// Pass 1's per-line walk. Tracks where the current `;`-delimited
    /// segment began and whether its statement was a bare label, so a
    /// segment's candidate remark is diagnosed by the matching function and
    /// never blamed on a sibling's parens. The rule that an indented line
    /// has no label field applies only to a line's own first segment -- a
    /// statement after `;` keeps reading a label, wherever the physical
    /// line started.
    fn first_pass_line(
        &mut self,
        line_pair: pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
    ) -> Result<(), String> {
        let mut segment_start = line_pair.as_span().start();
        let line_indented = Self::line_opens_indented(source, segment_start);
        let mut is_first_segment = true;
        let mut has_statement = false;
        // `Some(label_pair)` exactly when the current segment's statement
        // is a bare label with no instruction or directive attached.
        let mut bare_label: Option<pest::iterators::Pair<Rule>> = None;

        for stmt_pair in line_pair.into_inner() {
            match stmt_pair.as_rule() {
                Rule::statement => {
                    has_statement = true;
                    if is_first_segment
                        && line_indented
                        && let Some(label_pair) = Self::statement_label_pair(&stmt_pair)
                        && stmt_pair.as_span().end() > label_pair.as_span().end()
                    {
                        return Err(Self::indented_label_statement_error(
                            &stmt_pair,
                            &label_pair,
                            source,
                            filename,
                            segment_start,
                        ));
                    }
                    bare_label = Self::statement_is_label_only(&stmt_pair)
                        .then(|| stmt_pair.clone().into_inner().next())
                        .flatten();
                    self.first_pass_statement(stmt_pair)?;
                }
                Rule::remark => {
                    if let Some(label_pair) = bare_label.as_ref()
                        && stmt_pair.as_str().is_empty()
                    {
                        if let Some(err) = Self::bare_reserved_mnemonic_error(
                            label_pair,
                            &stmt_pair,
                            source,
                            filename,
                            segment_start,
                        ) {
                            return Err(err);
                        }
                        if is_first_segment && line_indented {
                            return Err(Self::indented_bare_label_error(
                                label_pair,
                                &stmt_pair,
                                source,
                                filename,
                                segment_start,
                            ));
                        }
                    }
                    if bare_label.is_some() {
                        Self::diagnose_unrecognized_opcode(
                            &stmt_pair,
                            source,
                            filename,
                            segment_start,
                        )?;
                    } else {
                        Self::check_remark(
                            &stmt_pair,
                            source,
                            filename,
                            segment_start,
                            has_statement,
                        )?;
                    }
                    // Skip the `;` that follows, if any, so the next
                    // segment starts clean.
                    segment_start = stmt_pair.as_span().end() + 1;
                    is_first_segment = false;
                    has_statement = false;
                    bare_label = None;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// First pass: collect labels and process IS/PREFIX directives.
    /// Redefinition errors are reported here; the second pass overwrites
    /// silently because every label collected here will be re-encountered
    /// at the same address.
    #[instrument(skip(self, pair), fields(current_addr = format!("0x{:X}", self.current_addr)))]
    fn first_pass_statement(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut pending_label: Option<(String, usize)> = None;
        let mut pending_local: Option<(u8, usize)> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let (line, _) = inner_pair.line_col();
                    let ident = inner_pair.into_inner().next().unwrap();
                    pending_label = Some((ident.as_str().to_string(), line));
                }
                Rule::local_label_def => {
                    let (line, _) = inner_pair.line_col();
                    let digit = Self::local_digit(inner_pair.as_str());
                    pending_local = Some((digit, line));
                    self.local_pending_digit = Some(digit);
                }
                Rule::instruction => {
                    if self.in_special_mode {
                        return Err(Self::special_mode_content_error(
                            &self.current_filename,
                            &inner_pair,
                            "an instruction",
                        ));
                    }
                    self.align_current_addr(Self::INSTRUCTION_ALIGNMENT);
                    self.scan_uses_for_redefinition(&inner_pair);
                    let inst = self.peek_instruction_type(inner_pair)?;
                    let size = Self::instruction_size(&inst);
                    if let Some((raw, line)) = pending_label.take() {
                        self.define_label(&raw, self.current_addr, line)?;
                    }
                    if let Some((digit, _)) = pending_local.take() {
                        self.record_local_label(
                            digit,
                            SymbolType::Constant(self.current_addr),
                            true,
                        );
                    }
                    self.current_addr += size;
                }
                Rule::directive => {
                    let directive_pair = inner_pair.into_inner().next().unwrap();
                    match directive_pair.as_rule() {
                        Rule::data_directive => {
                            if self.in_special_mode {
                                // Discarded: no bytes, no address movement,
                                // but a label on the line still binds to
                                // the (unmoved) current address.
                                if let Some((raw, line)) = pending_label.take() {
                                    self.define_label(&raw, self.current_addr, line)?;
                                }
                                if let Some((digit, _)) = pending_local.take() {
                                    self.record_local_label(
                                        digit,
                                        SymbolType::Constant(self.current_addr),
                                        true,
                                    );
                                }
                            } else {
                                self.scan_uses_for_redefinition(&directive_pair);
                                let alignment = Self::data_directive_alignment(&directive_pair)?;
                                self.align_current_addr(alignment);
                                let size = self.data_directive_size(directive_pair.clone())?;
                                if let Some((raw, line)) = pending_label.take() {
                                    self.define_label(&raw, self.current_addr, line)?;
                                }
                                if let Some((digit, _)) = pending_local.take() {
                                    self.record_local_label(
                                        digit,
                                        SymbolType::Constant(self.current_addr),
                                        true,
                                    );
                                }
                                self.current_addr += size;
                            }
                        }
                        Rule::loc_directive => {
                            if self.in_special_mode {
                                return Err(Self::special_mode_content_error(
                                    &self.current_filename,
                                    &directive_pair,
                                    "LOC",
                                ));
                            }
                            // A label on a LOC line names the location the
                            // counter held before LOC moves it, per the
                            // MMIXAL reference's `X LOC @+500`. The operand
                            // is evaluated before this line's own local
                            // label (if any) is recorded, so a same-digit
                            // reference in it never resolves to itself.
                            let addr_before = self.current_addr;
                            if let Some((raw, line)) = pending_label.take() {
                                self.define_label(&raw, addr_before, line)?;
                            }
                            self.scan_uses_for_redefinition(&directive_pair);
                            self.parse_loc_directive(directive_pair)?;
                            if let Some((digit, _)) = pending_local.take() {
                                self.record_local_label(
                                    digit,
                                    SymbolType::Constant(addr_before),
                                    true,
                                );
                            }
                        }
                        Rule::greg_directive => {
                            // GREG allocates a global register; an attached
                            // label aliases the register, not an address.
                            let allocated_reg = if self.next_greg == 0 {
                                return Err(
                                    "Too many GREG directives - ran out of global registers"
                                        .to_string(),
                                );
                            } else {
                                let reg = self.next_greg;
                                self.next_greg -= 1;
                                reg
                            };

                            // A GREG with no operand -- the empty field is
                            // 0 -- holds a global register at 0, per the
                            // reference's own reading of an empty operand
                            // field.
                            let mut greg_parts = directive_pair.clone().into_inner();
                            let _directive = greg_parts.next();
                            let value = match greg_parts.next() {
                                Some(operand) => {
                                    self.scan_uses_for_redefinition(&operand);
                                    self.parse_number(operand)?
                                }
                                None => 0,
                            };
                            self.greg_inits.push((allocated_reg, value));

                            if let Some((raw, line)) = pending_label.take() {
                                self.define_symbol(
                                    &raw,
                                    SymbolType::Register(allocated_reg),
                                    line,
                                )?;
                            }
                            if let Some((digit, _)) = pending_local.take() {
                                self.record_local_label(
                                    digit,
                                    SymbolType::Register(allocated_reg),
                                    true,
                                );
                            }
                        }
                        Rule::is_directive => {
                            self.parse_is_directive(directive_pair, true)?;
                            // IS directive doesn't advance current_addr.
                        }
                        Rule::prefix_directive => {
                            self.parse_prefix_directive(directive_pair);
                        }
                        Rule::local_directive => {
                            Self::require_blank_label(
                                &self.current_filename,
                                &directive_pair,
                                "LOCAL",
                                pending_label.is_some() || pending_local.is_some(),
                            )?;
                            self.handle_local_directive(directive_pair)?;
                        }
                        Rule::bspec_directive => {
                            Self::require_blank_label(
                                &self.current_filename,
                                &directive_pair,
                                "BSPEC",
                                pending_label.is_some() || pending_local.is_some(),
                            )?;
                            self.open_special_mode(directive_pair)?;
                        }
                        Rule::espec_directive => {
                            Self::require_blank_label(
                                &self.current_filename,
                                &directive_pair,
                                "ESPEC",
                                pending_label.is_some() || pending_local.is_some(),
                            )?;
                            self.close_special_mode(&directive_pair)?;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Standalone labels (no instruction or directive on the line)
        if let Some((raw, line)) = pending_label {
            self.define_label(&raw, self.current_addr, line)?;
        }
        if let Some((digit, _)) = pending_local {
            self.record_local_label(digit, SymbolType::Constant(self.current_addr), true);
        }
        self.local_pending_digit = None;

        Ok(())
    }

    /// Diagnostic for an instruction or `LOC` found between `BSPEC` and
    /// `ESPEC`: only `IS`, `PREFIX`, `GREG`, `LOCAL` and the four data
    /// directives are legal there.
    fn special_mode_content_error(
        filename: &str,
        pair: &pest::iterators::Pair<Rule>,
        what: &str,
    ) -> String {
        let (line, _) = pair.line_col();
        format!("{filename}:{line}: syntax error: {what} is not allowed inside BSPEC/ESPEC")
    }

    /// `LOCAL`, `BSPEC` and `ESPEC` take no label field.
    fn require_blank_label(
        filename: &str,
        pair: &pest::iterators::Pair<Rule>,
        keyword: &str,
        has_pending_label: bool,
    ) -> Result<(), String> {
        if has_pending_label {
            let (line, _) = pair.line_col();
            return Err(format!(
                "{filename}:{line}: syntax error: {keyword} takes no label"
            ));
        }
        Ok(())
    }

    /// `LOCAL expr`: `expr` must resolve to a register, checked at the
    /// close of assembly against the global threshold `next_greg` derives.
    fn handle_local_directive(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let (line, _) = pair.line_col();
        let mut parts = pair.into_inner();
        let _keyword = parts.next();
        let operand = parts.next().unwrap();
        self.scan_uses_for_redefinition(&operand);
        let reg = self.parse_register(operand)?;
        self.local_declarations
            .push((reg, self.current_filename.clone(), line));
        Ok(())
    }

    /// `BSPEC expr`: opens special mode. `BSPEC` does not nest, and its
    /// operand must fit in two bytes.
    fn open_special_mode(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let (line, _) = pair.line_col();
        if self.in_special_mode {
            return Err(format!(
                "{}:{}: syntax error: BSPEC does not nest",
                self.current_filename, line
            ));
        }
        let mut parts = pair.into_inner();
        let _keyword = parts.next();
        let operand = parts.next().unwrap();
        self.scan_uses_for_redefinition(&operand);
        let value = self.parse_number(operand)?;
        if value > 0xFFFF {
            return Err(format!(
                "{}:{}: syntax error: BSPEC operand {} does not fit in two bytes",
                self.current_filename, line, value
            ));
        }
        self.in_special_mode = true;
        self.bspec_open_site = Some((self.current_filename.clone(), line));
        Ok(())
    }

    /// `ESPEC`: closes special mode; an `ESPEC` with no open `BSPEC` is an
    /// error.
    fn close_special_mode(&mut self, pair: &pest::iterators::Pair<Rule>) -> Result<(), String> {
        let (line, _) = pair.line_col();
        if !self.in_special_mode {
            return Err(format!(
                "{}:{}: syntax error: ESPEC has no matching BSPEC",
                self.current_filename, line
            ));
        }
        self.in_special_mode = false;
        self.bspec_open_site = None;
        Ok(())
    }

    /// Second pass: generate actual instructions with resolved labels.
    /// Labels and IS-bound symbols are re-inserted (overwriting the pass-1
    /// values with the same value) without redefinition checking, since
    /// PREFIX state is replayed identically and produces the same names.
    #[instrument(skip(self, pair), fields(current_addr = format!("0x{:X}", self.current_addr)))]
    fn second_pass_statement(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        // Captured before `into_inner()` consumes `pair`: the statement's
        // line in the ACTIVE translation unit's PREPROCESSED text.
        // `record_debug_info` maps it back to the original source line.
        let (line, _) = pair.line_col();
        let mut label_name: Option<String> = None;
        let mut pending_local: Option<u8> = None;
        let mut inst: Option<MMixInstruction> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let ident = inner_pair.into_inner().next().unwrap();
                    label_name = Some(ident.as_str().to_string());
                }
                Rule::local_label_def => {
                    let digit = Self::local_digit(inner_pair.as_str());
                    pending_local = Some(digit);
                    self.local_pending_digit = Some(digit);
                }
                Rule::instruction => {
                    self.align_current_addr(Self::INSTRUCTION_ALIGNMENT);
                    if let Some(raw) = label_name.take() {
                        let qualified = self.qualify_name(&raw);
                        self.labels.insert(qualified, self.current_addr);
                    }
                    // Evaluated before this line's own local label (if any)
                    // is recorded, so a same-digit reference in an operand
                    // never resolves to itself.
                    inst = Some(self.parse_instruction(inner_pair)?);
                    if let Some(digit) = pending_local.take() {
                        self.record_local_label(digit, SymbolType::Constant(0), false);
                    }
                }
                Rule::directive => {
                    let directive_pair = inner_pair.into_inner().next().unwrap();
                    match directive_pair.as_rule() {
                        Rule::data_directive => {
                            if self.in_special_mode {
                                if let Some(raw) = label_name.take() {
                                    let qualified = self.qualify_name(&raw);
                                    self.labels.insert(qualified, self.current_addr);
                                }
                                if let Some(digit) = pending_local.take() {
                                    self.record_local_label(digit, SymbolType::Constant(0), false);
                                }
                            } else {
                                let alignment = Self::data_directive_alignment(&directive_pair)?;
                                self.align_current_addr(alignment);
                                if let Some(raw) = label_name.take() {
                                    let qualified = self.qualify_name(&raw);
                                    self.labels.insert(qualified, self.current_addr);
                                }
                                let instructions = self.parse_data_directive(directive_pair)?;
                                if let Some(digit) = pending_local.take() {
                                    self.record_local_label(digit, SymbolType::Constant(0), false);
                                }
                                for instruction in instructions {
                                    let size = Self::instruction_size(&instruction);
                                    self.record_debug_info(self.current_addr, line);
                                    self.instructions.push((self.current_addr, instruction));
                                    self.current_addr += size;
                                }
                            }
                        }
                        Rule::loc_directive => {
                            // Mirrors first pass: the label names the
                            // location before LOC moves the counter, and
                            // the operand is evaluated before this line's
                            // own local label is recorded.
                            if let Some(raw) = label_name.take() {
                                let qualified = self.qualify_name(&raw);
                                self.labels.insert(qualified, self.current_addr);
                            }
                            self.parse_loc_directive(directive_pair)?;
                            if let Some(digit) = pending_local.take() {
                                self.record_local_label(digit, SymbolType::Constant(0), false);
                            }
                        }
                        Rule::greg_directive => {
                            // GREG was already processed in first pass. The
                            // two-operand memory form's base-address search
                            // bounds itself to the GREGs seen by this point
                            // in source order, so pass 2 replays the count.
                            self.greg_inits_seen += 1;
                            if let Some(raw) = label_name.take() {
                                let qualified = self.qualify_name(&raw);
                                if !self.symbols.contains_key(&qualified) {
                                    return Err(format!(
                                        "Internal error: GREG label '{}' not found in symbols from first pass",
                                        qualified
                                    ));
                                }
                            }
                            if let Some(digit) = pending_local.take() {
                                self.record_local_label(digit, SymbolType::Constant(0), false);
                            }
                        }
                        Rule::is_directive => {
                            self.parse_is_directive(directive_pair, false)?;
                        }
                        Rule::prefix_directive => {
                            self.parse_prefix_directive(directive_pair);
                        }
                        Rule::local_directive => {}
                        Rule::bspec_directive => {
                            self.in_special_mode = true;
                        }
                        Rule::espec_directive => {
                            self.in_special_mode = false;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some(instruction) = inst {
            let size = Self::instruction_size(&instruction);
            debug!(inst = ?instruction, addr = format!("0x{:X}", self.current_addr), size, "Added instruction");
            self.record_debug_info(self.current_addr, line);
            self.instructions.push((self.current_addr, instruction));
            self.current_addr += size;
        }

        // Standalone labels (no instruction or directive on the line)
        if let Some(raw) = label_name {
            let qualified = self.qualify_name(&raw);
            self.labels.insert(qualified, self.current_addr);
        }
        if let Some(digit) = pending_local {
            self.record_local_label(digit, SymbolType::Constant(0), false);
        }
        self.local_pending_digit = None;

        Ok(())
    }

    /// Record `addr`'s source location in the active translation unit:
    /// `line`, in the preprocessed text pest walked, is also the line the
    /// user wrote it on (`preprocess_debug` never changes a source's line
    /// count).
    fn record_debug_info(&mut self, addr: u64, line: usize) {
        let file = self.sources[self.current_unit_index].filename.clone();
        self.debug_info.insert(addr, SourceLoc { file, line });
    }

    /// Peek at instruction type to determine size without modifying state
    fn peek_instruction_type(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let inner = pair.into_inner().next().ok_or("Empty instruction")?;

        match inner.as_rule() {
            // SET is one tetra whichever variant it selects.
            Rule::inst_set => Ok(MMixInstruction::SETRR(0, 0)),
            Rule::inst_seti => Ok(MMixInstruction::SET(0, 0)), // Placeholder: SETI $X,IMM (will expand)
            Rule::inst_setl_ri => Ok(MMixInstruction::SETL(0, 0)),
            Rule::inst_seth_ri => Ok(MMixInstruction::SETH(0, 0)),
            Rule::inst_setmh_ri => Ok(MMixInstruction::SETMH(0, 0)),
            Rule::inst_setml_ri => Ok(MMixInstruction::SETML(0, 0)),
            Rule::inst_incl_ri => Ok(MMixInstruction::INCL(0, 0)),
            // LDA $X,$Y,Z (3-operand form): parse_inst_lda_rri always emits a
            // real 4-byte LDA regardless of Z's value -- it never expands to
            // SET, unlike the 2-operand form below.
            Rule::inst_lda_rri => Ok(MMixInstruction::LDA(0, 0, 0)),
            Rule::inst_lda_ri => {
                // Check if LDA will expand to SET (address > 0xFF)
                // We need to peek at the operand to determine this
                let mut parts = inner.clone().into_inner();
                let _mnem = parts.next();
                let operands = parts.next().unwrap();
                let mut ops = operands.into_inner();
                let _x = ops.next(); // skip register
                let addr_pair = ops.next().unwrap();

                // Try to resolve the address
                match self.parse_number(addr_pair) {
                    Ok(addr) if addr <= 0xFF => Ok(MMixInstruction::LDA(0, 0, 0)), // 4 bytes
                    _ => Ok(MMixInstruction::SET(0, 0)), // 16 bytes (will expand)
                }
            }
            Rule::inst_halt => Ok(MMixInstruction::HALT),
            // For all other instructions, return a standard 4-byte instruction
            _ => Ok(MMixInstruction::ADDU(0, 0, 0)),
        }
    }

    /// Round the location counter up to `alignment`, the way MMIXAL does
    /// before it assembles an item: a label on that line names the rounded
    /// address, and the skipped bytes are a gap rather than emitted padding.
    ///
    /// Both passes round at the same point, ahead of the item's operands.
    /// `peek_instruction_type` sizes the two-operand `LDA` from its operand's
    /// value, so rounding on opposite sides of operand evaluation would let
    /// the passes disagree about a forward reference with no other symptom.
    fn align_current_addr(&mut self, alignment: u64) {
        self.current_addr = self.current_addr.next_multiple_of(alignment);
    }

    /// Alignment of a data directive, taken from the directive's kind and
    /// never from the number of bytes it emits. `BYTE` stays unaligned
    /// however long its operand list; `WYDE`, `TETRA` and `OCTA` round to
    /// their own width.
    fn data_directive_alignment(pair: &pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let directive = pair
            .clone()
            .into_inner()
            .next()
            .ok_or("Empty data directive")?;

        match directive.as_rule() {
            Rule::directive_byte => Ok(1),
            Rule::directive_wyde => Ok(2),
            Rule::directive_tetra => Ok(4),
            Rule::directive_octa => Ok(8),
            _ => Err(format!("Unknown data directive: {:?}", directive.as_rule())),
        }
    }

    /// Calculate the actual size of a data directive: its unit width times
    /// its unit count. A string contributes one unit per decoded byte;
    /// every other primary contributes one, matching what pass 2's
    /// `eval_data_value_items` emits (`n₁ + … + n_k − k + 1` items for a
    /// value holding k strings, per `MMIX.md`).
    fn data_directive_size(&self, pair: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let mut parts = pair.clone().into_inner();
        let directive = parts.next().ok_or("Empty data directive")?;

        let unit_width = Self::data_directive_unit_width(directive.as_rule())?;
        let values = parts.next().ok_or("Missing data values")?;

        let mut unit_count = 0u64;
        for value in values.into_inner() {
            unit_count += self.data_value_unit_count(value)?;
        }
        let total_size = unit_width * unit_count;
        debug!(
            "Data directive size: {} units x {} bytes = {} bytes",
            unit_count, unit_width, total_size
        );
        Ok(total_size)
    }

    /// The number of values one `data_value` contributes, without
    /// evaluating any of them. A string of n characters replaces its own
    /// primary with n values; every other primary contributes one, matching
    /// pass 2's `eval_data_value_items`. So a value holding k strings of
    /// n₁ … n_k characters contributes n₁ + … + n_k − k + 1, whatever
    /// operators surround them. An empty string is an error at its own
    /// position, the same rule pass 2 applies, unless it is the value's
    /// only content, which contributes zero.
    fn data_value_unit_count(&self, value: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let data_expr = Self::data_expr(value)?;
        if Self::is_bare_empty_string(&data_expr) {
            return Ok(0);
        }

        let mut char_total = 0u64;
        let mut string_count = 0u64;
        for term in data_expr.into_inner().step_by(2) {
            for primary in term.into_inner().step_by(2) {
                self.count_data_primary_strings(primary, &mut char_total, &mut string_count)?;
            }
        }
        Ok(1 + char_total - string_count)
    }

    /// [`Self::data_value_unit_count`]'s walk into one `data_primary`,
    /// descending through a unary wrap to the string or ordinary value it
    /// ultimately holds.
    fn count_data_primary_strings(
        &self,
        primary: pest::iterators::Pair<Rule>,
        char_total: &mut u64,
        string_count: &mut u64,
    ) -> Result<(), String> {
        let mut children = primary.into_inner();
        let first = children
            .next()
            .ok_or_else(|| "data_primary has a child".to_string())?;
        match first.as_rule() {
            Rule::unary_op => {
                let operand = children
                    .next()
                    .ok_or_else(|| "unary operator needs an operand".to_string())?;
                self.count_data_primary_strings(operand, char_total, string_count)
            }
            Rule::string_literal => {
                let (_, rest) = self.decode_data_string_literal(&first)?;
                *char_total += 1 + rest.len() as u64;
                *string_count += 1;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The unit width, in bytes, that a data directive assembles per value.
    fn data_directive_unit_width(directive_kind: Rule) -> Result<u64, String> {
        match directive_kind {
            Rule::directive_byte => Ok(1),
            Rule::directive_wyde => Ok(2),
            Rule::directive_tetra => Ok(4),
            Rule::directive_octa => Ok(8),
            _ => Err(format!("Unknown data directive: {:?}", directive_kind)),
        }
    }

    fn parse_instruction(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let inner = pair.into_inner().next().ok_or("Empty instruction")?;

        match inner.as_rule() {
            Rule::inst_set => self.parse_inst_set(inner),
            Rule::inst_seti => self.parse_inst_seti(inner),
            Rule::inst_setl_ri => self.parse_inst_setl(inner),
            Rule::inst_seth_ri => self.parse_inst_seth(inner),
            Rule::inst_setmh_ri => self.parse_inst_setmh(inner),
            Rule::inst_setml_ri => self.parse_inst_setml(inner),
            Rule::inst_incl_ri => self.parse_inst_incl(inner),
            Rule::inst_inch_ri => self.parse_inst_inch(inner),
            Rule::inst_incmh_ri => self.parse_inst_incmh(inner),
            Rule::inst_incml_ri => self.parse_inst_incml(inner),
            Rule::inst_orh_ri => self.parse_inst_orh(inner),
            Rule::inst_ormh_ri => self.parse_inst_ormh(inner),
            Rule::inst_orml_ri => self.parse_inst_orml(inner),
            Rule::inst_orl_ri => self.parse_inst_orl(inner),
            Rule::inst_andnh_ri => self.parse_inst_andnh(inner),
            Rule::inst_andnmh_ri => self.parse_inst_andnmh(inner),
            Rule::inst_andnml_ri => self.parse_inst_andnml(inner),
            Rule::inst_andnl_ri => self.parse_inst_andnl(inner),
            Rule::inst_load_store_auto => self.parse_inst_load_store_auto(inner),
            Rule::inst_load_store_rri => self.parse_inst_load_store_rri(inner),
            Rule::inst_lda_auto => self.parse_inst_lda_auto(inner),
            Rule::inst_lda_rri => self.parse_inst_lda_rri(inner),
            Rule::inst_lda_ri => self.parse_inst_lda_ri(inner),
            Rule::inst_arith_auto => self.parse_inst_arith_auto(inner),
            Rule::inst_arith_rri => self.parse_inst_arith_rri(inner),
            Rule::inst_flot_round => self.parse_inst_flot_round(inner),
            Rule::inst_flot_auto => self.parse_inst_flot_auto(inner),
            Rule::inst_float_round_rrz => self.parse_inst_float_round_rrz(inner),
            Rule::inst_float_round_rr => self.parse_inst_float_round_rr(inner),
            Rule::inst_float_round_rri => self.parse_inst_float_round_rri(inner),
            Rule::inst_float_rri => self.parse_inst_float_rri(inner),
            Rule::inst_float_rrr => self.parse_inst_float_rrr(inner),
            Rule::inst_neg_auto => self.parse_inst_neg_auto(inner),
            Rule::inst_neg_rri => self.parse_inst_neg_rri(inner),
            Rule::inst_bitwise_auto => self.parse_inst_bitwise_auto(inner),
            Rule::inst_bitwise_rri => self.parse_inst_bitwise_rri(inner),
            Rule::inst_bitfiddle_auto => self.parse_inst_bitfiddle_auto(inner),
            Rule::inst_bitfiddle_rri => self.parse_inst_bitfiddle_rri(inner),
            Rule::inst_shift_auto => self.parse_inst_shift_auto(inner),
            Rule::inst_shift_rri => self.parse_inst_shift_rri(inner),
            Rule::inst_conditional_set_auto => self.parse_inst_conditional_set_auto(inner),
            Rule::inst_conditional_set_rri => self.parse_inst_conditional_set_rri(inner),
            Rule::inst_zero_or_set_auto => self.parse_inst_zero_or_set_auto(inner),
            Rule::inst_zero_or_set_rri => self.parse_inst_zero_or_set_rri(inner),
            Rule::inst_pbranch => self.parse_inst_pbranch(inner),
            Rule::inst_branch => self.parse_inst_branch(inner),
            Rule::inst_jmp => self.parse_inst_jmp(inner),
            Rule::inst_geta => self.parse_inst_geta(inner),
            Rule::inst_getab => self.parse_inst_getab(inner),
            Rule::inst_pushj => self.parse_inst_pushj(inner),
            Rule::inst_pushjb => self.parse_inst_pushjb(inner),
            Rule::inst_go_auto => self.parse_inst_go_auto(inner),
            Rule::inst_pushgo_rri => self.parse_inst_pushgo_rri(inner),
            Rule::inst_pop => self.parse_inst_pop(inner),
            Rule::inst_go_rri => self.parse_inst_go_rri(inner),
            Rule::inst_get => self.parse_inst_get(inner),
            Rule::inst_put_auto => self.parse_inst_put_auto(inner),
            Rule::inst_puti => self.parse_inst_puti(inner),
            Rule::inst_save => self.parse_inst_save(inner),
            Rule::inst_unsave => self.parse_inst_unsave(inner),
            Rule::inst_ldunc_rri => self.parse_inst_ldunc_rri(inner),
            Rule::inst_stunc_rri => self.parse_inst_stunc_rri(inner),
            Rule::inst_ldht_rri => self.parse_inst_ldht_rri(inner),
            Rule::inst_stht_rri => self.parse_inst_stht_rri(inner),
            Rule::inst_ldsf_rri => self.parse_inst_ldsf_rri(inner),
            Rule::inst_stsf_rri => self.parse_inst_stsf_rri(inner),
            Rule::inst_ldvts_rri => self.parse_inst_ldvts_rri(inner),
            Rule::inst_cswap_rri => self.parse_inst_cswap_rri(inner),
            Rule::inst_stco_auto => self.parse_inst_stco_auto(inner),
            Rule::inst_stco_rri => self.parse_inst_stco_rri(inner),
            Rule::inst_cache_auto => self.parse_inst_cache_auto(inner),
            Rule::inst_preld_rri => self.parse_inst_preld_rri(inner),
            Rule::inst_prego_rri => self.parse_inst_prego_rri(inner),
            Rule::inst_prest_rri => self.parse_inst_prest_rri(inner),
            Rule::inst_syncd_rri => self.parse_inst_syncd_rri(inner),
            Rule::inst_syncid_rri => self.parse_inst_syncid_rri(inner),
            Rule::inst_resume => self.parse_inst_resume(inner),
            Rule::inst_trip => self.parse_inst_trip(inner),
            Rule::inst_swym => self.parse_inst_swym(inner),
            Rule::inst_sync => self.parse_inst_sync(inner),
            Rule::inst_trap => self.parse_inst_trap(inner),
            Rule::inst_halt => Ok(MMixInstruction::HALT),
            _ => Err(format!(
                "Unsupported instruction rule: {:?}",
                inner.as_rule()
            )),
        }
    }

    fn parse_inst_set(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next(); // mnemonic_set
        let operands = parts.next().unwrap(); // operand_list_two
        let mut ops = operands.into_inner();
        let dest = self.parse_register(ops.next().unwrap())?;
        self.lower_set_source(dest, ops.next().unwrap())
    }

    /// Resolve `SET`'s source operand into the instruction it selects: a
    /// register value copies, a pure value at most `#FFFF` is `SETL`.
    /// `SET` is one tetra, so the immediate form carries 16 bits; anything
    /// wider is an error naming `SETI` for a wider constant or `SETI`/`NEG`
    /// for a negative one.
    fn lower_set_source(
        &self,
        dest: u8,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();

        match self.eval_expr(pair)? {
            ExprValue::Register(r) => {
                let reg = self.require_register_in_range(r, line, col)?;
                Ok(MMixInstruction::SETRR(dest, reg))
            }
            ExprValue::Pure(value) => {
                if value <= 0xFFFF {
                    return Ok(MMixInstruction::SETL(dest, value as u16));
                }
                let hint = if value >= 0x8000_0000_0000_0000 {
                    "use SETI or NEG for a negative constant"
                } else {
                    "use SETI for a wider constant"
                };
                Err(format!(
                    "{}:{}:{}: immediate operand {} out of range 0..65535 for SET; {}",
                    self.current_filename, line, col, value as i64, hint
                ))
            }
        }
    }

    fn parse_inst_seti(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next(); // mnemonic_seti
        let operands = parts.next().unwrap(); // operand_list_two
        let mut ops = operands.into_inner();
        let dest_reg = self.parse_register(ops.next().unwrap())?;
        let val = self.parse_number(ops.next().unwrap())?;

        Ok(MMixInstruction::SET(dest_reg, val))
    }

    fn parse_inst_setl(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "SETL")?;
        Ok(MMixInstruction::SETL(reg, val))
    }

    fn parse_inst_seth(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "SETH")?;
        Ok(MMixInstruction::SETH(reg, val))
    }

    fn parse_inst_setmh(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "SETMH")?;
        Ok(MMixInstruction::SETMH(reg, val))
    }

    fn parse_inst_setml(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "SETML")?;
        Ok(MMixInstruction::SETML(reg, val))
    }

    fn parse_inst_incl(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "INCL")?;
        Ok(MMixInstruction::INCL(reg, val))
    }

    fn parse_inst_inch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "INCH")?;
        Ok(MMixInstruction::INCH(reg, val))
    }

    fn parse_inst_incmh(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "INCMH")?;
        Ok(MMixInstruction::INCMH(reg, val))
    }

    fn parse_inst_incml(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "INCML")?;
        Ok(MMixInstruction::INCML(reg, val))
    }

    fn parse_inst_orh(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ORH")?;
        Ok(MMixInstruction::ORH(reg, val))
    }

    fn parse_inst_ormh(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ORMH")?;
        Ok(MMixInstruction::ORMH(reg, val))
    }

    fn parse_inst_orml(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ORML")?;
        Ok(MMixInstruction::ORML(reg, val))
    }

    fn parse_inst_orl(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ORL")?;
        Ok(MMixInstruction::ORL(reg, val))
    }

    fn parse_inst_andnh(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ANDNH")?;
        Ok(MMixInstruction::ANDNH(reg, val))
    }

    fn parse_inst_andnmh(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ANDNMH")?;
        Ok(MMixInstruction::ANDNMH(reg, val))
    }

    fn parse_inst_andnml(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ANDNML")?;
        Ok(MMixInstruction::ANDNML(reg, val))
    }

    fn parse_inst_andnl(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ANDNL")?;
        Ok(MMixInstruction::ANDNL(reg, val))
    }

    fn parse_inst_load_store_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let (x, y, z) = match operands.as_rule() {
            Rule::operand_list_three => {
                let mut ops = operands.into_inner();
                let x = self.parse_register(ops.next().unwrap())?;
                let y = self.parse_register(ops.next().unwrap())?;
                let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;
                (x, y, z)
            }
            Rule::operand_list_two => {
                // The two-operand memory form: the second operand is a
                // register (an offset of zero) or a base address resolved
                // against a preceding GREG.
                let mut ops = operands.into_inner();
                let x = self.parse_register(ops.next().unwrap())?;
                let (y, offset) = self.resolve_memory_base_operand(ops.next().unwrap())?;
                (x, y, ZForm::Imm(offset))
            }
            _ => unreachable!("memory auto instructions take two or three operands"),
        };

        match (mnem.as_str(), z) {
            ("LDB", ZForm::Reg(z)) => Ok(MMixInstruction::LDB(x, y, z)),
            ("LDB", ZForm::Imm(z)) => Ok(MMixInstruction::LDBI(x, y, z)),
            ("LDBU", ZForm::Reg(z)) => Ok(MMixInstruction::LDBU(x, y, z)),
            ("LDBU", ZForm::Imm(z)) => Ok(MMixInstruction::LDBUI(x, y, z)),
            ("LDW", ZForm::Reg(z)) => Ok(MMixInstruction::LDW(x, y, z)),
            ("LDW", ZForm::Imm(z)) => Ok(MMixInstruction::LDWI(x, y, z)),
            ("LDWU", ZForm::Reg(z)) => Ok(MMixInstruction::LDWU(x, y, z)),
            ("LDWU", ZForm::Imm(z)) => Ok(MMixInstruction::LDWUI(x, y, z)),
            ("LDT", ZForm::Reg(z)) => Ok(MMixInstruction::LDT(x, y, z)),
            ("LDT", ZForm::Imm(z)) => Ok(MMixInstruction::LDTI(x, y, z)),
            ("LDTU", ZForm::Reg(z)) => Ok(MMixInstruction::LDTU(x, y, z)),
            ("LDTU", ZForm::Imm(z)) => Ok(MMixInstruction::LDTUI(x, y, z)),
            ("LDO", ZForm::Reg(z)) => Ok(MMixInstruction::LDO(x, y, z)),
            ("LDO", ZForm::Imm(z)) => Ok(MMixInstruction::LDOI(x, y, z)),
            ("LDOU", ZForm::Reg(z)) => Ok(MMixInstruction::LDOU(x, y, z)),
            ("LDOU", ZForm::Imm(z)) => Ok(MMixInstruction::LDOUI(x, y, z)),
            ("STB", ZForm::Reg(z)) => Ok(MMixInstruction::STB(x, y, z)),
            ("STB", ZForm::Imm(z)) => Ok(MMixInstruction::STBI(x, y, z)),
            ("STBU", ZForm::Reg(z)) => Ok(MMixInstruction::STBU(x, y, z)),
            ("STBU", ZForm::Imm(z)) => Ok(MMixInstruction::STBUI(x, y, z)),
            ("STW", ZForm::Reg(z)) => Ok(MMixInstruction::STW(x, y, z)),
            ("STW", ZForm::Imm(z)) => Ok(MMixInstruction::STWI(x, y, z)),
            ("STWU", ZForm::Reg(z)) => Ok(MMixInstruction::STWU(x, y, z)),
            ("STWU", ZForm::Imm(z)) => Ok(MMixInstruction::STWUI(x, y, z)),
            ("STT", ZForm::Reg(z)) => Ok(MMixInstruction::STT(x, y, z)),
            ("STT", ZForm::Imm(z)) => Ok(MMixInstruction::STTI(x, y, z)),
            ("STTU", ZForm::Reg(z)) => Ok(MMixInstruction::STTU(x, y, z)),
            ("STTU", ZForm::Imm(z)) => Ok(MMixInstruction::STTUI(x, y, z)),
            ("STO", ZForm::Reg(z)) => Ok(MMixInstruction::STO(x, y, z)),
            ("STO", ZForm::Imm(z)) => Ok(MMixInstruction::STOI(x, y, z)),
            ("STOU", ZForm::Reg(z)) => Ok(MMixInstruction::STOU(x, y, z)),
            ("STOU", ZForm::Imm(z)) => Ok(MMixInstruction::STOUI(x, y, z)),
            ("LDUNC", ZForm::Reg(z)) => Ok(MMixInstruction::LDUNC(x, y, z)),
            ("LDUNC", ZForm::Imm(z)) => Ok(MMixInstruction::LDUNCI(x, y, z)),
            ("STUNC", ZForm::Reg(z)) => Ok(MMixInstruction::STUNC(x, y, z)),
            ("STUNC", ZForm::Imm(z)) => Ok(MMixInstruction::STUNCI(x, y, z)),
            ("LDHT", ZForm::Reg(z)) => Ok(MMixInstruction::LDHT(x, y, z)),
            ("LDHT", ZForm::Imm(z)) => Ok(MMixInstruction::LDHTI(x, y, z)),
            ("STHT", ZForm::Reg(z)) => Ok(MMixInstruction::STHT(x, y, z)),
            ("STHT", ZForm::Imm(z)) => Ok(MMixInstruction::STHTI(x, y, z)),
            ("LDSF", ZForm::Reg(z)) => Ok(MMixInstruction::LDSF(x, y, z)),
            ("LDSF", ZForm::Imm(z)) => Ok(MMixInstruction::LDSFI(x, y, z)),
            ("STSF", ZForm::Reg(z)) => Ok(MMixInstruction::STSF(x, y, z)),
            ("STSF", ZForm::Imm(z)) => Ok(MMixInstruction::STSFI(x, y, z)),
            ("LDVTS", ZForm::Reg(z)) => Ok(MMixInstruction::LDVTS(x, y, z)),
            ("LDVTS", ZForm::Imm(z)) => Ok(MMixInstruction::LDVTSI(x, y, z)),
            ("CSWAP", ZForm::Reg(z)) => Ok(MMixInstruction::CSWAP(x, y, z)),
            ("CSWAP", ZForm::Imm(z)) => Ok(MMixInstruction::CSWAPI(x, y, z)),
            _ => Err(format!("Unknown load/store instruction: {}", mnem)),
        }
    }

    fn parse_inst_load_store_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "LDBI" => Ok(MMixInstruction::LDBI(x, y, z)),
            "LDBUI" => Ok(MMixInstruction::LDBUI(x, y, z)),
            "LDWI" => Ok(MMixInstruction::LDWI(x, y, z)),
            "LDWUI" => Ok(MMixInstruction::LDWUI(x, y, z)),
            "LDTI" => Ok(MMixInstruction::LDTI(x, y, z)),
            "LDTUI" => Ok(MMixInstruction::LDTUI(x, y, z)),
            "LDOI" => Ok(MMixInstruction::LDOI(x, y, z)),
            "LDOUI" => Ok(MMixInstruction::LDOUI(x, y, z)),
            "STBI" => Ok(MMixInstruction::STBI(x, y, z)),
            "STBUI" => Ok(MMixInstruction::STBUI(x, y, z)),
            "STWI" => Ok(MMixInstruction::STWI(x, y, z)),
            "STWUI" => Ok(MMixInstruction::STWUI(x, y, z)),
            "STTI" => Ok(MMixInstruction::STTI(x, y, z)),
            "STTUI" => Ok(MMixInstruction::STTUI(x, y, z)),
            "STOI" => Ok(MMixInstruction::STOI(x, y, z)),
            "STOUI" => Ok(MMixInstruction::STOUI(x, y, z)),
            _ => Err(format!("Unknown load/store instruction: {}", mnem.as_str())),
        }
    }

    /// `LDA $X,$Y,$Z` is `ADDU $X,$Y,$Z` and `LDA $X,$Y,Z` is `ADDU $X,$Y,Z`,
    /// so Z selects the same pair of opcodes ADDU selects.
    fn parse_inst_lda_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;

        match self.lower_z_operand(ops.next().unwrap(), "LDA")? {
            ZForm::Reg(z) => Ok(MMixInstruction::LDA(x, y, z)),
            ZForm::Imm(z) => Ok(MMixInstruction::LDAI(x, y, z)),
        }
    }

    fn parse_inst_lda_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDAI", MMixInstruction::LDAI)
    }

    fn parse_inst_lda_ri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let addr_value = self.parse_number(ops.next().unwrap())?;

        // LDA $X,Label where Label is a full 64-bit address should become SET
        // LDA is really ADDU $X,$0,Z where Z is an 8-bit immediate
        // If the address doesn't fit in 8 bits, use SET instead
        match mnem.as_str().to_uppercase().as_str() {
            "LDA" => {
                if addr_value <= 0xFF {
                    Ok(MMixInstruction::LDA(x, 0, addr_value as u8))
                } else {
                    // Address too large for LDA immediate form - use SET instead
                    debug!("LDA with large address {:#x} converted to SET", addr_value);
                    Ok(MMixInstruction::SET(x, addr_value))
                }
            }
            "LDAI" => {
                if addr_value <= 0xFF {
                    Ok(MMixInstruction::LDAI(x, 0, addr_value as u8))
                } else {
                    Ok(MMixInstruction::SET(x, addr_value))
                }
            }
            _ => Err(format!("Unknown LDA instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_arith_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z_pair = ops.next().unwrap();
        let z = self.lower_z_operand(z_pair, &mnem)?;

        match (mnem.as_str(), z) {
            ("ADD", ZForm::Reg(z)) => Ok(MMixInstruction::ADD(x, y, z)),
            ("ADD", ZForm::Imm(z)) => Ok(MMixInstruction::ADDI(x, y, z)),
            ("ADDU", ZForm::Reg(z)) => Ok(MMixInstruction::ADDU(x, y, z)),
            ("ADDU", ZForm::Imm(z)) => Ok(MMixInstruction::ADDUI(x, y, z)),
            ("2ADDU", ZForm::Reg(z)) => Ok(MMixInstruction::ADDU2(x, y, z)),
            ("2ADDU", ZForm::Imm(z)) => Ok(MMixInstruction::ADDU2I(x, y, z)),
            ("4ADDU", ZForm::Reg(z)) => Ok(MMixInstruction::ADDU4(x, y, z)),
            ("4ADDU", ZForm::Imm(z)) => Ok(MMixInstruction::ADDU4I(x, y, z)),
            ("8ADDU", ZForm::Reg(z)) => Ok(MMixInstruction::ADDU8(x, y, z)),
            ("8ADDU", ZForm::Imm(z)) => Ok(MMixInstruction::ADDU8I(x, y, z)),
            ("16ADDU", ZForm::Reg(z)) => Ok(MMixInstruction::ADDU16(x, y, z)),
            ("16ADDU", ZForm::Imm(z)) => Ok(MMixInstruction::ADDU16I(x, y, z)),
            ("SUB", ZForm::Reg(z)) => Ok(MMixInstruction::SUB(x, y, z)),
            ("SUB", ZForm::Imm(z)) => Ok(MMixInstruction::SUBI(x, y, z)),
            ("SUBU", ZForm::Reg(z)) => Ok(MMixInstruction::SUBU(x, y, z)),
            ("SUBU", ZForm::Imm(z)) => Ok(MMixInstruction::SUBUI(x, y, z)),
            ("MUL", ZForm::Reg(z)) => Ok(MMixInstruction::MUL(x, y, z)),
            ("MUL", ZForm::Imm(z)) => Ok(MMixInstruction::MULI(x, y, z)),
            ("MULU", ZForm::Reg(z)) => Ok(MMixInstruction::MULU(x, y, z)),
            ("MULU", ZForm::Imm(z)) => Ok(MMixInstruction::MULUI(x, y, z)),
            ("DIV", ZForm::Reg(z)) => Ok(MMixInstruction::DIV(x, y, z)),
            ("DIV", ZForm::Imm(z)) => Ok(MMixInstruction::DIVI(x, y, z)),
            ("DIVU", ZForm::Reg(z)) => Ok(MMixInstruction::DIVU(x, y, z)),
            ("DIVU", ZForm::Imm(z)) => Ok(MMixInstruction::DIVUI(x, y, z)),
            ("CMP", ZForm::Reg(z)) => Ok(MMixInstruction::CMP(x, y, z)),
            ("CMP", ZForm::Imm(z)) => Ok(MMixInstruction::CMPI(x, y, z)),
            ("CMPU", ZForm::Reg(z)) => Ok(MMixInstruction::CMPU(x, y, z)),
            ("CMPU", ZForm::Imm(z)) => Ok(MMixInstruction::CMPUI(x, y, z)),
            _ => Err(format!("Unknown arithmetic instruction: {}", mnem)),
        }
    }

    fn parse_inst_arith_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "ADDI" => Ok(MMixInstruction::ADDI(x, y, z)),
            "ADDUI" => Ok(MMixInstruction::ADDUI(x, y, z)),
            "2ADDUI" => Ok(MMixInstruction::ADDU2I(x, y, z)),
            "4ADDUI" => Ok(MMixInstruction::ADDU4I(x, y, z)),
            "8ADDUI" => Ok(MMixInstruction::ADDU8I(x, y, z)),
            "16ADDUI" => Ok(MMixInstruction::ADDU16I(x, y, z)),
            "SUBI" => Ok(MMixInstruction::SUBI(x, y, z)),
            "SUBUI" => Ok(MMixInstruction::SUBUI(x, y, z)),
            "MULI" => Ok(MMixInstruction::MULI(x, y, z)),
            "MULUI" => Ok(MMixInstruction::MULUI(x, y, z)),
            "DIVI" => Ok(MMixInstruction::DIVI(x, y, z)),
            "DIVUI" => Ok(MMixInstruction::DIVUI(x, y, z)),
            "CMPI" => Ok(MMixInstruction::CMPI(x, y, z)),
            "CMPUI" => Ok(MMixInstruction::CMPUI(x, y, z)),
            _ => Err(format!("Unknown arithmetic instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_neg_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap().as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let (x, y, z) = match operands.as_rule() {
            Rule::operand_list_three => {
                let mut ops = operands.into_inner();
                let x = self.parse_register(ops.next().unwrap())?;
                let y = self.imm_byte(ops.next().unwrap(), &mnem)?;
                let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;
                (x, y, z)
            }
            Rule::operand_list_two => {
                // Y omitted: NEG $X,z is NEG $X,0,z.
                let mut ops = operands.into_inner();
                let x = self.parse_register(ops.next().unwrap())?;
                let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;
                (x, 0, z)
            }
            _ => unreachable!("NEG/NEGU take two or three operands"),
        };

        match (mnem.as_str(), z) {
            ("NEG", ZForm::Reg(z)) => Ok(MMixInstruction::NEG(x, y, z)),
            ("NEG", ZForm::Imm(z)) => Ok(MMixInstruction::NEGI(x, y, z)),
            ("NEGU", ZForm::Reg(z)) => Ok(MMixInstruction::NEGU(x, y, z)),
            ("NEGU", ZForm::Imm(z)) => Ok(MMixInstruction::NEGUI(x, y, z)),
            _ => Err(format!("Unknown NEG instruction: {}", mnem)),
        }
    }

    fn parse_inst_neg_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let name = mnem.as_str().to_uppercase();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.imm_byte(ops.next().unwrap(), &name)?;
        let z = self.imm_byte(ops.next().unwrap(), &name)?;

        match name.as_str() {
            "NEGI" => Ok(MMixInstruction::NEGI(x, y, z)),
            "NEGUI" => Ok(MMixInstruction::NEGUI(x, y, z)),
            _ => Err(format!("Unknown NEG instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_float_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.parse_register(ops.next().unwrap())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FCMP" => Ok(MMixInstruction::FCMP(x, y, z)),
            "FUN" => Ok(MMixInstruction::FUN(x, y, z)),
            "FEQL" => Ok(MMixInstruction::FEQL(x, y, z)),
            "FCMPE" => Ok(MMixInstruction::FCMPE(x, y, z)),
            "FUNE" => Ok(MMixInstruction::FUNE(x, y, z)),
            "FEQLE" => Ok(MMixInstruction::FEQLE(x, y, z)),
            "FADD" => Ok(MMixInstruction::FADD(x, y, z)),
            "FSUB" => Ok(MMixInstruction::FSUB(x, y, z)),
            "FMUL" => Ok(MMixInstruction::FMUL(x, y, z)),
            "FDIV" => Ok(MMixInstruction::FDIV(x, y, z)),
            "FREM" => Ok(MMixInstruction::FREM(x, y, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FIX`/`FIXU`/`FSQRT`/`FINT`, 3-operand form: `Y` is a rounding-mode
    /// value (`0..=4`, `ROUND_CURRENT`/`ROUND_OFF`/`ROUND_UP`/`ROUND_DOWN`/
    /// `ROUND_NEAR`). Only its byte field is checked here, the same as
    /// `NEG`'s own value-typed `Y`; `Y > 4` halts at run time.
    fn parse_inst_float_round_rrz(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;
        let z = self.parse_register(ops.next().unwrap())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FIX" => Ok(MMixInstruction::FIX(x, y, z)),
            "FIXU" => Ok(MMixInstruction::FIXU(x, y, z)),
            "FSQRT" => Ok(MMixInstruction::FSQRT(x, y, z)),
            "FINT" => Ok(MMixInstruction::FINT(x, y, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FIX`/`FIXU`/`FSQRT`/`FINT`, 2-operand form: `Y` is implicitly 0
    /// (`ROUND_CURRENT` — no override, use rA's mode).
    fn parse_inst_float_round_rr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let z = self.parse_register(ops.next().unwrap())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FIX" => Ok(MMixInstruction::FIX(x, 0, z)),
            "FIXU" => Ok(MMixInstruction::FIXU(x, 0, z)),
            "FSQRT" => Ok(MMixInstruction::FSQRT(x, 0, z)),
            "FINT" => Ok(MMixInstruction::FINT(x, 0, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FLOT`/`FLOTU`/`SFLOT`/`SFLOTU`, 3-operand form: `Y` forces a
    /// rounding mode, `Z` auto-selects register or immediate.
    fn parse_inst_flot_round(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap().as_str().to_uppercase();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.imm_byte(ops.next().unwrap(), &mnem)?;
        let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;

        match (mnem.as_str(), z) {
            ("FLOT", ZForm::Reg(z)) => Ok(MMixInstruction::FLOT(x, y, z)),
            ("FLOT", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTI(x, y, z)),
            ("FLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::FLOTU(x, y, z)),
            ("FLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTUI(x, y, z)),
            ("SFLOT", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOT(x, y, z)),
            ("SFLOT", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTI(x, y, z)),
            ("SFLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOTU(x, y, z)),
            ("SFLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTUI(x, y, z)),
            _ => Err(format!("Unknown float conversion instruction: {}", mnem)),
        }
    }

    /// `FLOT`/`FLOTU`/`SFLOT`/`SFLOTU`, 2-operand form: `Y` implicitly 0.
    fn parse_inst_flot_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap().as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;

        match (mnem.as_str(), z) {
            ("FLOT", ZForm::Reg(z)) => Ok(MMixInstruction::FLOT(x, 0, z)),
            ("FLOT", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTI(x, 0, z)),
            ("FLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::FLOTU(x, 0, z)),
            ("FLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTUI(x, 0, z)),
            ("SFLOT", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOT(x, 0, z)),
            ("SFLOT", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTI(x, 0, z)),
            ("SFLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOTU(x, 0, z)),
            ("SFLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTUI(x, 0, z)),
            _ => Err(format!("Unknown float conversion instruction: {}", mnem)),
        }
    }

    /// `FLOTI`/`FLOTUI`/`SFLOTI`/`SFLOTUI`, 3-operand form: `Y` forces a
    /// rounding mode, `Z` stays immediate-only.
    fn parse_inst_float_round_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FLOTI" => Ok(MMixInstruction::FLOTI(x, y, z)),
            "FLOTUI" => Ok(MMixInstruction::FLOTUI(x, y, z)),
            "SFLOTI" => Ok(MMixInstruction::SFLOTI(x, y, z)),
            "SFLOTUI" => Ok(MMixInstruction::SFLOTUI(x, y, z)),
            _ => Err(format!(
                "Unknown floating point immediate instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FLOTI`/`FLOTUI`/`SFLOTI`/`SFLOTUI`, 2-operand form: `Y` implicitly 0.
    fn parse_inst_float_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FLOTI" => Ok(MMixInstruction::FLOTI(x, 0, z)),
            "FLOTUI" => Ok(MMixInstruction::FLOTUI(x, 0, z)),
            "SFLOTI" => Ok(MMixInstruction::SFLOTI(x, 0, z)),
            "SFLOTUI" => Ok(MMixInstruction::SFLOTUI(x, 0, z)),
            _ => Err(format!(
                "Unknown floating point immediate instruction: {}",
                mnem.as_str()
            )),
        }
    }

    fn parse_inst_bitwise_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z_pair = ops.next().unwrap();
        let z = self.lower_z_operand(z_pair, &mnem)?;

        match (mnem.as_str(), z) {
            ("AND", ZForm::Reg(z)) => Ok(MMixInstruction::AND(x, y, z)),
            ("AND", ZForm::Imm(z)) => Ok(MMixInstruction::ANDI(x, y, z)),
            ("OR", ZForm::Reg(z)) => Ok(MMixInstruction::OR(x, y, z)),
            ("OR", ZForm::Imm(z)) => Ok(MMixInstruction::ORI(x, y, z)),
            ("XOR", ZForm::Reg(z)) => Ok(MMixInstruction::XOR(x, y, z)),
            ("XOR", ZForm::Imm(z)) => Ok(MMixInstruction::XORI(x, y, z)),
            ("ANDN", ZForm::Reg(z)) => Ok(MMixInstruction::ANDN(x, y, z)),
            ("ANDN", ZForm::Imm(z)) => Ok(MMixInstruction::ANDNI(x, y, z)),
            ("ORN", ZForm::Reg(z)) => Ok(MMixInstruction::ORN(x, y, z)),
            ("ORN", ZForm::Imm(z)) => Ok(MMixInstruction::ORNI(x, y, z)),
            ("NAND", ZForm::Reg(z)) => Ok(MMixInstruction::NAND(x, y, z)),
            ("NAND", ZForm::Imm(z)) => Ok(MMixInstruction::NANDI(x, y, z)),
            ("NOR", ZForm::Reg(z)) => Ok(MMixInstruction::NOR(x, y, z)),
            ("NOR", ZForm::Imm(z)) => Ok(MMixInstruction::NORI(x, y, z)),
            ("NXOR", ZForm::Reg(z)) => Ok(MMixInstruction::NXOR(x, y, z)),
            ("NXOR", ZForm::Imm(z)) => Ok(MMixInstruction::NXORI(x, y, z)),
            ("MUX", ZForm::Reg(z)) => Ok(MMixInstruction::MUX(x, y, z)),
            ("MUX", ZForm::Imm(z)) => Ok(MMixInstruction::MUXI(x, y, z)),
            _ => Err(format!("Unknown bitwise instruction: {}", mnem)),
        }
    }

    fn parse_inst_bitwise_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "ANDI" => Ok(MMixInstruction::ANDI(x, y, z)),
            "ORI" => Ok(MMixInstruction::ORI(x, y, z)),
            "XORI" => Ok(MMixInstruction::XORI(x, y, z)),
            "ANDNI" => Ok(MMixInstruction::ANDNI(x, y, z)),
            "ORNI" => Ok(MMixInstruction::ORNI(x, y, z)),
            "NANDI" => Ok(MMixInstruction::NANDI(x, y, z)),
            "NORI" => Ok(MMixInstruction::NORI(x, y, z)),
            "NXORI" => Ok(MMixInstruction::NXORI(x, y, z)),
            "MUXI" => Ok(MMixInstruction::MUXI(x, y, z)),
            _ => Err(format!("Unknown bitwise instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_bitfiddle_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z_pair = ops.next().unwrap();
        let z = self.lower_z_operand(z_pair, &mnem)?;

        match (mnem.as_str(), z) {
            ("BDIF", ZForm::Reg(z)) => Ok(MMixInstruction::BDIF(x, y, z)),
            ("BDIF", ZForm::Imm(z)) => Ok(MMixInstruction::BDIFI(x, y, z)),
            ("WDIF", ZForm::Reg(z)) => Ok(MMixInstruction::WDIF(x, y, z)),
            ("WDIF", ZForm::Imm(z)) => Ok(MMixInstruction::WDIFI(x, y, z)),
            ("TDIF", ZForm::Reg(z)) => Ok(MMixInstruction::TDIF(x, y, z)),
            ("TDIF", ZForm::Imm(z)) => Ok(MMixInstruction::TDIFI(x, y, z)),
            ("ODIF", ZForm::Reg(z)) => Ok(MMixInstruction::ODIF(x, y, z)),
            ("ODIF", ZForm::Imm(z)) => Ok(MMixInstruction::ODIFI(x, y, z)),
            ("SADD", ZForm::Reg(z)) => Ok(MMixInstruction::SADD(x, y, z)),
            ("SADD", ZForm::Imm(z)) => Ok(MMixInstruction::SADDI(x, y, z)),
            ("MOR", ZForm::Reg(z)) => Ok(MMixInstruction::MOR(x, y, z)),
            ("MOR", ZForm::Imm(z)) => Ok(MMixInstruction::MORI(x, y, z)),
            ("MXOR", ZForm::Reg(z)) => Ok(MMixInstruction::MXOR(x, y, z)),
            ("MXOR", ZForm::Imm(z)) => Ok(MMixInstruction::MXORI(x, y, z)),
            _ => Err(format!("Unknown bit fiddling instruction: {}", mnem)),
        }
    }

    fn parse_inst_bitfiddle_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "BDIFI" => Ok(MMixInstruction::BDIFI(x, y, z)),
            "WDIFI" => Ok(MMixInstruction::WDIFI(x, y, z)),
            "TDIFI" => Ok(MMixInstruction::TDIFI(x, y, z)),
            "ODIFI" => Ok(MMixInstruction::ODIFI(x, y, z)),
            "SADDI" => Ok(MMixInstruction::SADDI(x, y, z)),
            "MORI" => Ok(MMixInstruction::MORI(x, y, z)),
            "MXORI" => Ok(MMixInstruction::MXORI(x, y, z)),
            _ => Err(format!(
                "Unknown bit fiddling instruction: {}",
                mnem.as_str()
            )),
        }
    }

    fn parse_inst_shift_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z_pair = ops.next().unwrap();
        let z = self.lower_z_operand(z_pair, &mnem)?;

        match (mnem.as_str(), z) {
            ("SL", ZForm::Reg(z)) => Ok(MMixInstruction::SL(x, y, z)),
            ("SL", ZForm::Imm(z)) => Ok(MMixInstruction::SLI(x, y, z)),
            ("SLU", ZForm::Reg(z)) => Ok(MMixInstruction::SLU(x, y, z)),
            ("SLU", ZForm::Imm(z)) => Ok(MMixInstruction::SLUI(x, y, z)),
            ("SR", ZForm::Reg(z)) => Ok(MMixInstruction::SR(x, y, z)),
            ("SR", ZForm::Imm(z)) => Ok(MMixInstruction::SRI(x, y, z)),
            ("SRU", ZForm::Reg(z)) => Ok(MMixInstruction::SRU(x, y, z)),
            ("SRU", ZForm::Imm(z)) => Ok(MMixInstruction::SRUI(x, y, z)),
            _ => Err(format!("Unknown shift instruction: {}", mnem)),
        }
    }

    fn parse_inst_shift_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "SLI" => Ok(MMixInstruction::SLI(x, y, z)),
            "SLUI" => Ok(MMixInstruction::SLUI(x, y, z)),
            "SRI" => Ok(MMixInstruction::SRI(x, y, z)),
            "SRUI" => Ok(MMixInstruction::SRUI(x, y, z)),
            _ => Err(format!("Unknown shift instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_conditional_set_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z_pair = ops.next().unwrap();
        let z = self.lower_z_operand(z_pair, &mnem)?;

        match (mnem.as_str(), z) {
            ("CSN", ZForm::Reg(z)) => Ok(MMixInstruction::CSN(x, y, z)),
            ("CSN", ZForm::Imm(z)) => Ok(MMixInstruction::CSNI(x, y, z)),
            ("CSZ", ZForm::Reg(z)) => Ok(MMixInstruction::CSZ(x, y, z)),
            ("CSZ", ZForm::Imm(z)) => Ok(MMixInstruction::CSZI(x, y, z)),
            ("CSP", ZForm::Reg(z)) => Ok(MMixInstruction::CSP(x, y, z)),
            ("CSP", ZForm::Imm(z)) => Ok(MMixInstruction::CSPI(x, y, z)),
            ("CSOD", ZForm::Reg(z)) => Ok(MMixInstruction::CSOD(x, y, z)),
            ("CSOD", ZForm::Imm(z)) => Ok(MMixInstruction::CSODI(x, y, z)),
            ("CSNN", ZForm::Reg(z)) => Ok(MMixInstruction::CSNN(x, y, z)),
            ("CSNN", ZForm::Imm(z)) => Ok(MMixInstruction::CSNNI(x, y, z)),
            ("CSNZ", ZForm::Reg(z)) => Ok(MMixInstruction::CSNZ(x, y, z)),
            ("CSNZ", ZForm::Imm(z)) => Ok(MMixInstruction::CSNZI(x, y, z)),
            ("CSNP", ZForm::Reg(z)) => Ok(MMixInstruction::CSNP(x, y, z)),
            ("CSNP", ZForm::Imm(z)) => Ok(MMixInstruction::CSNPI(x, y, z)),
            ("CSEV", ZForm::Reg(z)) => Ok(MMixInstruction::CSEV(x, y, z)),
            ("CSEV", ZForm::Imm(z)) => Ok(MMixInstruction::CSEVI(x, y, z)),
            _ => Err(format!("Unknown conditional set instruction: {}", mnem)),
        }
    }

    fn parse_inst_conditional_set_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "CSNI" => Ok(MMixInstruction::CSNI(x, y, z)),
            "CSZI" => Ok(MMixInstruction::CSZI(x, y, z)),
            "CSPI" => Ok(MMixInstruction::CSPI(x, y, z)),
            "CSODI" => Ok(MMixInstruction::CSODI(x, y, z)),
            "CSNNI" => Ok(MMixInstruction::CSNNI(x, y, z)),
            "CSNZI" => Ok(MMixInstruction::CSNZI(x, y, z)),
            "CSNPI" => Ok(MMixInstruction::CSNPI(x, y, z)),
            "CSEVI" => Ok(MMixInstruction::CSEVI(x, y, z)),
            _ => Err(format!(
                "Unknown conditional set immediate instruction: {}",
                mnem.as_str()
            )),
        }
    }

    fn parse_inst_zero_or_set_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem_pair = parts.next().unwrap();
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z_pair = ops.next().unwrap();
        let z = self.lower_z_operand(z_pair, &mnem)?;

        match (mnem.as_str(), z) {
            ("ZSN", ZForm::Reg(z)) => Ok(MMixInstruction::ZSN(x, y, z)),
            ("ZSN", ZForm::Imm(z)) => Ok(MMixInstruction::ZSNI(x, y, z)),
            ("ZSZ", ZForm::Reg(z)) => Ok(MMixInstruction::ZSZ(x, y, z)),
            ("ZSZ", ZForm::Imm(z)) => Ok(MMixInstruction::ZSZI(x, y, z)),
            ("ZSP", ZForm::Reg(z)) => Ok(MMixInstruction::ZSP(x, y, z)),
            ("ZSP", ZForm::Imm(z)) => Ok(MMixInstruction::ZSPI(x, y, z)),
            ("ZSOD", ZForm::Reg(z)) => Ok(MMixInstruction::ZSOD(x, y, z)),
            ("ZSOD", ZForm::Imm(z)) => Ok(MMixInstruction::ZSODI(x, y, z)),
            ("ZSNN", ZForm::Reg(z)) => Ok(MMixInstruction::ZSNN(x, y, z)),
            ("ZSNN", ZForm::Imm(z)) => Ok(MMixInstruction::ZSNNI(x, y, z)),
            ("ZSNZ", ZForm::Reg(z)) => Ok(MMixInstruction::ZSNZ(x, y, z)),
            ("ZSNZ", ZForm::Imm(z)) => Ok(MMixInstruction::ZSNZI(x, y, z)),
            ("ZSNP", ZForm::Reg(z)) => Ok(MMixInstruction::ZSNP(x, y, z)),
            ("ZSNP", ZForm::Imm(z)) => Ok(MMixInstruction::ZSNPI(x, y, z)),
            ("ZSEV", ZForm::Reg(z)) => Ok(MMixInstruction::ZSEV(x, y, z)),
            ("ZSEV", ZForm::Imm(z)) => Ok(MMixInstruction::ZSEVI(x, y, z)),
            _ => Err(format!("Unknown zero or set instruction: {}", mnem)),
        }
    }

    fn parse_inst_zero_or_set_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "ZSNI" => Ok(MMixInstruction::ZSNI(x, y, z)),
            "ZSZI" => Ok(MMixInstruction::ZSZI(x, y, z)),
            "ZSPI" => Ok(MMixInstruction::ZSPI(x, y, z)),
            "ZSODI" => Ok(MMixInstruction::ZSODI(x, y, z)),
            "ZSNNI" => Ok(MMixInstruction::ZSNNI(x, y, z)),
            "ZSNZI" => Ok(MMixInstruction::ZSNZI(x, y, z)),
            "ZSNPI" => Ok(MMixInstruction::ZSNPI(x, y, z)),
            "ZSEVI" => Ok(MMixInstruction::ZSEVI(x, y, z)),
            _ => Err(format!(
                "Unknown zero or set immediate instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// Resolve a PC-relative target into the field a `bits`-wide operand
    /// carries. Forward reaches `0..=2^bits - 1` tetras and backward
    /// `1..=2^bits`, so a displacement of zero takes the forward opcode. A
    /// mnemonic spelled with a trailing `B` asserts a backward target and
    /// rejects a forward one; every other mnemonic takes its direction from
    /// the sign of the displacement. `absolute_alternative` names an absolute
    /// instruction to suggest when the target is unreachable, or is empty.
    fn relative_field(
        &self,
        mnemonic: &str,
        target: u64,
        bits: u32,
        (line, col): (usize, usize),
        absolute_alternative: &str,
    ) -> Result<RelativeField, String> {
        let delta = target.wrapping_sub(self.current_addr) as i64;
        if delta % 4 != 0 {
            return Err(format!(
                "{}:{}:{}: {} target 0x{:X} is not 4-byte aligned relative to the current instruction (byte delta {})",
                self.current_filename, line, col, mnemonic, target, delta
            ));
        }
        let tetras = delta / 4;
        if let Some(forward_sibling) = mnemonic.strip_suffix('B')
            && tetras >= 0
        {
            return Err(format!(
                "{}:{}:{}: {} target 0x{:X} is not behind the current instruction ({} only encodes backward addresses; use {} instead)",
                self.current_filename, line, col, mnemonic, target, mnemonic, forward_sibling
            ));
        }
        let span = 1i64 << bits;
        let out_of_range = |direction: &str, reach_tetras: i64| {
            format!(
                "{}:{}:{}: {} target 0x{:X} is out of range: {} byte delta {} exceeds {}'s {}-byte {} reach{}",
                self.current_filename,
                line,
                col,
                mnemonic,
                target,
                direction,
                delta.unsigned_abs(),
                mnemonic,
                reach_tetras * 4,
                direction,
                absolute_alternative
            )
        };
        if tetras >= 0 {
            if tetras >= span {
                return Err(out_of_range("forward", span - 1));
            }
            return Ok(RelativeField {
                backward: false,
                field: tetras as u32,
            });
        }
        if -tetras > span {
            return Err(out_of_range("backward", span));
        }
        Ok(RelativeField {
            backward: true,
            field: (span + tetras) as u32,
        })
    }

    fn parse_inst_branch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let target = self.parse_number(ops.next().unwrap())?;

        // Each mnemonic names the variant to emit for a forward target and
        // the one for a backward target.
        type Branch = fn(u8, u16) -> MMixInstruction;
        let mnem = mnem.as_str().to_uppercase();
        let (forward, backward): (Branch, Branch) = match mnem.as_str() {
            "BN" | "BNB" => (MMixInstruction::BN, MMixInstruction::BNB),
            "BZ" | "BZB" => (MMixInstruction::BZ, MMixInstruction::BZB),
            "BP" | "BPB" => (MMixInstruction::BP, MMixInstruction::BPB),
            "BOD" | "BODB" => (MMixInstruction::BOD, MMixInstruction::BODB),
            "BNN" | "BNNB" => (MMixInstruction::BNN, MMixInstruction::BNNB),
            "BNZ" | "BNZB" => (MMixInstruction::BNZ, MMixInstruction::BNZB),
            "BNP" | "BNPB" => (MMixInstruction::BNP, MMixInstruction::BNPB),
            "BEV" | "BEVB" => (MMixInstruction::BEV, MMixInstruction::BEVB),
            _ => return Err(format!("Unknown branch instruction: {mnem}")),
        };

        let resolved = self.relative_field(&mnem, target, 16, (line, col), "")?;
        let field = resolved.field as u16;
        Ok(if resolved.backward {
            backward(x, field)
        } else {
            forward(x, field)
        })
    }

    fn parse_inst_jmp(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let target = self.parse_number(ops.next().unwrap())?;
        let mnem = mnem.map_or_else(|| "JMP".to_string(), |m| m.as_str().to_uppercase());
        let resolved = self.relative_field(&mnem, target, 24, (line, col), " (use GO instead)")?;
        Ok(if resolved.backward {
            MMixInstruction::JMPB(resolved.field)
        } else {
            MMixInstruction::JMP(resolved.field)
        })
    }

    fn parse_inst_pbranch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let target = self.parse_number(ops.next().unwrap())?;

        type ProbableBranch = fn(u8, u8, u8) -> MMixInstruction;
        let mnem = mnem.as_str().to_uppercase();
        let (forward, backward): (ProbableBranch, ProbableBranch) = match mnem.as_str() {
            "PBN" | "PBNB" => (MMixInstruction::PBN, MMixInstruction::PBNB),
            "PBZ" | "PBZB" => (MMixInstruction::PBZ, MMixInstruction::PBZB),
            "PBP" | "PBPB" => (MMixInstruction::PBP, MMixInstruction::PBPB),
            "PBOD" | "PBODB" => (MMixInstruction::PBOD, MMixInstruction::PBODB),
            "PBNN" | "PBNNB" => (MMixInstruction::PBNN, MMixInstruction::PBNNB),
            "PBNZ" | "PBNZB" => (MMixInstruction::PBNZ, MMixInstruction::PBNZB),
            "PBNP" | "PBNPB" => (MMixInstruction::PBNP, MMixInstruction::PBNPB),
            "PBEV" | "PBEVB" => (MMixInstruction::PBEV, MMixInstruction::PBEVB),
            _ => return Err(format!("Unknown probable branch instruction: {mnem}")),
        };

        let resolved = self.relative_field(&mnem, target, 16, (line, col), "")?;
        let y = (resolved.field >> 8) as u8;
        let z = (resolved.field & 0xFF) as u8;
        Ok(if resolved.backward {
            backward(x, y, z)
        } else {
            forward(x, y, z)
        })
    }

    fn parse_inst_geta(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let _mnem = parts.next(); // Skip mnemonic
        let operand = parts.next().unwrap(); // Get operand_list_two

        let mut operand_parts = operand.into_inner();
        let reg_pair = operand_parts.next().unwrap();
        let addr_pair = operand_parts.next().unwrap();

        let x = self.parse_register(reg_pair)?;
        let addr = self.parse_number(addr_pair)?;

        debug!(
            "GETA: current_addr=0x{:X}, target_addr=0x{:X}",
            self.current_addr, addr
        );

        // GETA reaches 65535 tetras forward; a backward target takes GETAB.
        let resolved = self.relative_field(
            "GETA",
            addr,
            16,
            (line, col),
            " (use LDA for longer-range addresses)",
        )?;
        let y = (resolved.field >> 8) as u8;
        let z = (resolved.field & 0xFF) as u8;

        debug!(
            "GETA: backward={}, field=0x{:X}, y=0x{:X}, z=0x{:X}",
            resolved.backward, resolved.field, y, z
        );

        Ok(if resolved.backward {
            MMixInstruction::GETAB(x, y, z)
        } else {
            MMixInstruction::GETA(x, y, z)
        })
    }

    fn parse_inst_getab(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let _mnem = parts.next(); // Skip mnemonic
        let operand = parts.next().unwrap(); // Get operand_list_two

        let mut operand_parts = operand.into_inner();
        let reg_pair = operand_parts.next().unwrap();
        let addr_pair = operand_parts.next().unwrap();

        let x = self.parse_register(reg_pair)?;
        let addr = self.parse_number(addr_pair)?;

        let resolved = self.relative_field(
            "GETAB",
            addr,
            16,
            (line, col),
            " (use LDA for longer-range addresses)",
        )?;
        let y = (resolved.field >> 8) as u8;
        let z = (resolved.field & 0xFF) as u8;

        Ok(MMixInstruction::GETAB(x, y, z))
    }

    /// Splits a pure 16-bit value into its high and low bytes: Y and Z for
    /// the two-operand forms `TRAP`'s family, `POP`, `PUSHJ` and `PUSHJB`
    /// each resolve a combined field into.
    fn split_hi_lo_byte(value: u16) -> (u8, u8) {
        ((value >> 8) as u8, (value & 0xFF) as u8)
    }

    /// Splits a pure 24-bit value into X, Y and Z: the one-operand form
    /// `TRAP`'s family and `POP` both resolve a combined field into.
    fn split_xyz_bytes(value: u32) -> (u8, u8, u8) {
        (
            (value >> 16) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        )
    }

    /// `TRAP`/`TRIP`/`SWYM`'s shared operand shapes: three fields (each a
    /// register or a pure byte), two fields (X alone that way, YZ a pure
    /// 16-bit value split into Y and Z), one field (a pure 24-bit value
    /// split into X, Y and Z), or none (every field 0).
    fn parse_trap_family_operands(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<(u8, u8, u8), String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let Some(operands) = parts.next() else {
            return Ok((0, 0, 0));
        };
        match operands.as_rule() {
            Rule::operand_list_three => {
                let mut ops = operands.into_inner();
                let x = self.parse_reg_or_byte(ops.next().unwrap(), mnem)?;
                let y = self.parse_reg_or_byte(ops.next().unwrap(), mnem)?;
                let z = self.parse_reg_or_byte(ops.next().unwrap(), mnem)?;
                Ok((x, y, z))
            }
            Rule::operand_list_two => {
                let mut ops = operands.into_inner();
                let x = self.parse_reg_or_byte(ops.next().unwrap(), mnem)?;
                let yz = self.imm_wyde(ops.next().unwrap(), mnem)?;
                let (y, z) = Self::split_hi_lo_byte(yz);
                Ok((x, y, z))
            }
            Rule::operand_list_one => {
                let mut ops = operands.into_inner();
                let xyz = self.imm_three_bytes(ops.next().unwrap(), mnem)?;
                let (x, y, z) = Self::split_xyz_bytes(xyz);
                Ok((x, y, z))
            }
            _ => unreachable!("TRAP/TRIP/SWYM take zero, one, two or three operands"),
        }
    }

    fn parse_inst_trap(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (x, y, z) = self.parse_trap_family_operands(pair, "TRAP")?;
        Ok(MMixInstruction::TRAP(x, y, z))
    }

    // Helper: parse instruction with format (reg, reg, imm)
    fn parse_rri<F>(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
        f: F,
    ) -> Result<MMixInstruction, String>
    where
        F: FnOnce(u8, u8, u8) -> MMixInstruction,
    {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), mnem)?;
        Ok(f(x, y, z))
    }

    // PUSHJ/PUSHJB: format (reg-or-byte, imm) where imm is 16-bit offset
    fn parse_inst_pushj(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operand = parts.next().unwrap();
        let mut ops = operand.into_inner();
        let x = self.parse_reg_or_byte(ops.next().unwrap(), "PUSHJ")?;
        let addr = self.parse_number(ops.next().unwrap())?;
        let resolved = self.relative_field("PUSHJ", addr, 16, (line, col), "")?;
        let (y, z) = Self::split_hi_lo_byte(resolved.field as u16);
        Ok(if resolved.backward {
            MMixInstruction::PUSHJB(x, y, z)
        } else {
            MMixInstruction::PUSHJ(x, y, z)
        })
    }

    fn parse_inst_pushjb(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operand = parts.next().unwrap();
        let mut ops = operand.into_inner();
        let x = self.parse_reg_or_byte(ops.next().unwrap(), "PUSHJB")?;
        let addr = self.parse_number(ops.next().unwrap())?;
        let resolved = self.relative_field("PUSHJB", addr, 16, (line, col), "")?;
        let (y, z) = Self::split_hi_lo_byte(resolved.field as u16);
        Ok(MMixInstruction::PUSHJB(x, y, z))
    }

    /// `GO`'s X stays a register; `PUSHGO`'s X is also a pure byte, the
    /// same bytes as the register spelling.
    fn parse_inst_go_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap().as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let is_three = operands.as_rule() == Rule::operand_list_three;
        let mut ops = operands.into_inner();
        let x_pair = ops.next().unwrap();
        let x = if mnem == "PUSHGO" {
            self.parse_reg_or_byte(x_pair, &mnem)?
        } else {
            self.parse_register(x_pair)?
        };
        let (y, z) = if is_three {
            let y = self.parse_register(ops.next().unwrap())?;
            let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;
            (y, z)
        } else {
            // The two-operand memory form: the second operand is a register
            // (an offset of zero) or a base address resolved against a
            // preceding GREG.
            let (y, offset) = self.resolve_memory_base_operand(ops.next().unwrap())?;
            (y, ZForm::Imm(offset))
        };

        match (mnem.as_str(), z) {
            ("GO", ZForm::Reg(z)) => Ok(MMixInstruction::GO(x, y, z)),
            ("GO", ZForm::Imm(z)) => Ok(MMixInstruction::GOI(x, y, z)),
            ("PUSHGO", ZForm::Reg(z)) => Ok(MMixInstruction::PUSHGO(x, y, z)),
            ("PUSHGO", ZForm::Imm(z)) => Ok(MMixInstruction::PUSHGOI(x, y, z)),
            _ => Err(format!("Unknown GO instruction: {}", mnem)),
        }
    }

    fn parse_inst_pushgo_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PUSHGOI", MMixInstruction::PUSHGOI)
    }

    /// `POP p,yz`: X=p, YZ=yz. `POP xyz`: XYZ=xyz, so `POP 1` is
    /// `POP(0,0,1)`. Bare `POP`: every field 0.
    fn parse_inst_pop(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let Some(operands) = parts.next() else {
            return Ok(MMixInstruction::POP(0, 0, 0));
        };
        match operands.as_rule() {
            Rule::operand_list_two => {
                let mut ops = operands.into_inner();
                let x = self.imm_byte(ops.next().unwrap(), "POP")?;
                let yz = self.imm_wyde(ops.next().unwrap(), "POP")?;
                let (y, z) = Self::split_hi_lo_byte(yz);
                Ok(MMixInstruction::POP(x, y, z))
            }
            Rule::operand_list_one => {
                let mut ops = operands.into_inner();
                let xyz = self.imm_three_bytes(ops.next().unwrap(), "POP")?;
                let (x, y, z) = Self::split_xyz_bytes(xyz);
                Ok(MMixInstruction::POP(x, y, z))
            }
            _ => unreachable!("POP takes zero, one or two operands"),
        }
    }

    fn parse_inst_go_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "GOI", MMixInstruction::GOI)
    }

    fn parse_inst_get(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let z = self.special_register(ops.next().unwrap(), "GET")?;
        Ok(MMixInstruction::GET(x, z))
    }

    fn parse_inst_put_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.special_register(ops.next().unwrap(), "PUT")?;
        match self.lower_z_operand(ops.next().unwrap(), "PUT")? {
            ZForm::Reg(z) => Ok(MMixInstruction::PUT(x, z)),
            ZForm::Imm(z) => Ok(MMixInstruction::PUTI(x, z)),
        }
    }

    fn parse_inst_puti(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.special_register(ops.next().unwrap(), "PUTI")?;
        let z = self.imm_byte(ops.next().unwrap(), "PUTI")?;
        Ok(MMixInstruction::PUTI(x, z))
    }

    fn parse_inst_save(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), "SAVE")?;
        Ok(MMixInstruction::SAVE(x, z))
    }

    /// `UNSAVE X,Z`: `X` must be 0 (checked by the emulator, not here).
    /// `UNSAVE $Z` is the one-operand spelling of `UNSAVE 0,$Z`.
    fn parse_inst_unsave(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        match operands.as_rule() {
            Rule::operand_list_two => {
                let mut ops = operands.into_inner();
                let x = self.imm_byte(ops.next().unwrap(), "UNSAVE")?;
                let z = self.parse_register(ops.next().unwrap())?;
                Ok(MMixInstruction::UNSAVE(x, z))
            }
            Rule::operand_list_one => {
                let mut ops = operands.into_inner();
                let z = self.parse_register(ops.next().unwrap())?;
                Ok(MMixInstruction::UNSAVE(0, z))
            }
            _ => unreachable!("UNSAVE takes one or two operands"),
        }
    }

    fn parse_inst_ldunc_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDUNCI", MMixInstruction::LDUNCI)
    }

    fn parse_inst_stunc_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "STUNCI", MMixInstruction::STUNCI)
    }

    fn parse_inst_ldht_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDHTI", MMixInstruction::LDHTI)
    }

    fn parse_inst_stht_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "STHTI", MMixInstruction::STHTI)
    }

    fn parse_inst_ldsf_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDSFI", MMixInstruction::LDSFI)
    }

    fn parse_inst_stsf_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "STSFI", MMixInstruction::STSFI)
    }

    fn parse_inst_ldvts_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDVTSI", MMixInstruction::LDVTSI)
    }

    fn parse_inst_cswap_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "CSWAPI", MMixInstruction::CSWAPI)
    }

    /// STCO's X is a pure byte or a register, the same bytes; only Z
    /// auto-selects register or immediate.
    fn parse_inst_stco_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let is_three = operands.as_rule() == Rule::operand_list_three;
        let mut ops = operands.into_inner();
        let x = self.parse_reg_or_byte(ops.next().unwrap(), "STCO")?;
        let (y, z) = if is_three {
            let y = self.parse_register(ops.next().unwrap())?;
            let z = self.lower_z_operand(ops.next().unwrap(), "STCO")?;
            (y, z)
        } else {
            let (y, offset) = self.resolve_memory_base_operand(ops.next().unwrap())?;
            (y, ZForm::Imm(offset))
        };
        match z {
            ZForm::Reg(z) => Ok(MMixInstruction::STCO(x, y, z)),
            ZForm::Imm(z) => Ok(MMixInstruction::STCOI(x, y, z)),
        }
    }

    fn parse_inst_stco_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.imm_byte(ops.next().unwrap(), "STCOI")?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.imm_byte(ops.next().unwrap(), "STCOI")?;
        Ok(MMixInstruction::STCOI(x, y, z))
    }

    /// `PRELD`/`PREGO`/`PREST`/`SYNCD`/`SYNCID`'s X is a pure byte or a
    /// register, the same bytes; only Z auto-selects register or immediate.
    fn parse_inst_cache_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap().as_str().to_uppercase();
        let operands = parts.next().unwrap();
        let is_three = operands.as_rule() == Rule::operand_list_three;
        let mut ops = operands.into_inner();
        let x = self.parse_reg_or_byte(ops.next().unwrap(), &mnem)?;
        let (y, z) = if is_three {
            let y = self.parse_register(ops.next().unwrap())?;
            let z = self.lower_z_operand(ops.next().unwrap(), &mnem)?;
            (y, z)
        } else {
            let (y, offset) = self.resolve_memory_base_operand(ops.next().unwrap())?;
            (y, ZForm::Imm(offset))
        };

        match (mnem.as_str(), z) {
            ("PRELD", ZForm::Reg(z)) => Ok(MMixInstruction::PRELD(x, y, z)),
            ("PRELD", ZForm::Imm(z)) => Ok(MMixInstruction::PRELDI(x, y, z)),
            ("PREGO", ZForm::Reg(z)) => Ok(MMixInstruction::PREGO(x, y, z)),
            ("PREGO", ZForm::Imm(z)) => Ok(MMixInstruction::PREGOI(x, y, z)),
            ("PREST", ZForm::Reg(z)) => Ok(MMixInstruction::PREST(x, y, z)),
            ("PREST", ZForm::Imm(z)) => Ok(MMixInstruction::PRESTI(x, y, z)),
            ("SYNCD", ZForm::Reg(z)) => Ok(MMixInstruction::SYNCD(x, y, z)),
            ("SYNCD", ZForm::Imm(z)) => Ok(MMixInstruction::SYNCDI(x, y, z)),
            ("SYNCID", ZForm::Reg(z)) => Ok(MMixInstruction::SYNCID(x, y, z)),
            ("SYNCID", ZForm::Imm(z)) => Ok(MMixInstruction::SYNCIDI(x, y, z)),
            _ => Err(format!("Unknown cache control instruction: {}", mnem)),
        }
    }

    fn parse_inst_preld_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PRELDI", MMixInstruction::PRELDI)
    }

    fn parse_inst_prego_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PREGOI", MMixInstruction::PREGOI)
    }

    fn parse_inst_prest_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PRESTI", MMixInstruction::PRESTI)
    }

    fn parse_inst_syncd_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "SYNCDI", MMixInstruction::SYNCDI)
    }

    fn parse_inst_syncid_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "SYNCIDI", MMixInstruction::SYNCIDI)
    }

    /// Bare `RESUME`: XYZ=0. `RESUME` takes a 24-bit `XYZ`, the MMIXAL
    /// definition's spelling; all three bytes reach the encoding.
    fn parse_inst_resume(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let xyz = match parts.next() {
            Some(op) => self.imm_three_bytes(op.into_inner().next().unwrap(), "RESUME")?,
            None => 0,
        };
        Ok(MMixInstruction::RESUME(xyz))
    }

    fn parse_inst_trip(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (x, y, z) = self.parse_trap_family_operands(pair, "TRIP")?;
        Ok(MMixInstruction::TRIP(x, y, z))
    }

    fn parse_inst_swym(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (x, y, z) = self.parse_trap_family_operands(pair, "SWYM")?;
        Ok(MMixInstruction::SWYM(x, y, z))
    }

    /// Bare `SYNC`: XYZ=0. `SYNC` takes a 24-bit `XYZ`, the MMIXAL
    /// definition's spelling; all three bytes reach the encoding, though
    /// the machine halts on any code above 7.
    fn parse_inst_sync(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let xyz = match parts.next() {
            Some(op) => self.imm_three_bytes(op.into_inner().next().unwrap(), "SYNC")?,
            None => 0,
        };
        Ok(MMixInstruction::SYNC(xyz))
    }

    /// Build the data unit a directive assembles from one value, truncating
    /// to the directive's unit width.
    fn data_directive_unit(directive_kind: Rule, value: u64) -> Result<MMixInstruction, String> {
        match directive_kind {
            Rule::directive_byte => Ok(MMixInstruction::BYTE(value as u8)),
            Rule::directive_wyde => Ok(MMixInstruction::WYDE(value as u16)),
            Rule::directive_tetra => Ok(MMixInstruction::TETRA(value as u32)),
            Rule::directive_octa => Ok(MMixInstruction::OCTA(value)),
            _ => Err(format!("Unknown data directive: {:?}", directive_kind)),
        }
    }

    fn parse_data_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<MMixInstruction>, String> {
        let mut parts = pair.into_inner();
        let directive_kind = parts.next().ok_or("Empty data directive")?.as_rule();
        let values_pair = parts.next().ok_or("Missing data values")?;

        let mut result = Vec::new();
        for value in values_pair.into_inner() {
            let (line, col) = value.line_col();
            for item in self.eval_data_value_items(value)? {
                let val = self.require_pure(item, line, col)?;
                result.push(Self::data_directive_unit(directive_kind, val)?);
            }
        }
        Ok(result)
    }

    fn parse_loc_directive(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut parts = pair.into_inner();
        let _directive = parts.next(); // Skip "LOC" keyword
        let addr = self.parse_number(parts.next().unwrap())?;
        self.current_addr = addr;
        Ok(())
    }

    fn parse_is_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
        checking: bool,
    ) -> Result<(), String> {
        let mut parts = pair.into_inner();
        let lhs = parts.next().unwrap();
        let lhs_rule = lhs.as_rule();
        let (line, _) = lhs.line_col();
        let raw_name = lhs.as_str().to_string();
        let _is_keyword = parts.next(); // Skip "IS" keyword
        let value_pair = parts.next().unwrap();
        let (vline, vcol) = value_pair.line_col();

        self.scan_uses_for_redefinition(&value_pair);
        let symbol_type = match self.eval_expr(value_pair)? {
            ExprValue::Register(r) => {
                SymbolType::Register(self.require_register_in_range(r, vline, vcol)?)
            }
            ExprValue::Pure(value) => SymbolType::Constant(value),
        };

        if lhs_rule == Rule::local_label_def {
            self.record_local_label(Self::local_digit(&raw_name), symbol_type, checking);
        } else if checking {
            self.define_symbol(&raw_name, symbol_type, line)?;
        } else {
            let qualified = self.qualify_name(&raw_name);
            self.symbols.insert(qualified, symbol_type);
        }
        Ok(())
    }

    /// `PREFIX`'s operand is stored with one leading ':' stripped, so
    /// `PREFIX :` is the root (the empty prefix) and `PREFIX :Foo:` equals
    /// `PREFIX Foo:`. checksmix replaces the prefix outright rather than
    /// qualifying a relative operand against the current one.
    fn parse_prefix_directive(&mut self, pair: pest::iterators::Pair<Rule>) {
        let mut parts = pair.into_inner();
        let _directive = parts.next(); // Skip "PREFIX" keyword
        let arg = parts.next().expect("prefix_arg required by grammar");
        let arg_str = arg.as_str();
        self.current_prefix = arg_str.strip_prefix(':').unwrap_or(arg_str).to_string();
    }

    /// Resolve the Z operand of a base mnemonic (auto-immediate path) into
    /// either a register reference or an 8-bit immediate: a register-valued
    /// expression selects the register form, a pure value is range-checked
    /// against `0..=255` for the immediate form.
    fn lower_z_operand(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<ZForm, String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => {
                let reg = self.require_register_in_range(r, line, col)?;
                Ok(ZForm::Reg(reg))
            }
            ExprValue::Pure(v) => self.imm_in_range(v, mnem, line, col),
        }
    }

    /// Range-check a resolved operand value against an instruction field's
    /// width, the one place every site in the range table calls: the value
    /// must fit `0..=max` or assembly fails naming the field's own maximum
    /// and the mnemonic as written. `v` prints as a signed 64-bit integer
    /// when it is `2^63` or more, so a negative literal reports negative.
    fn field_value(
        &self,
        v: u64,
        max: u64,
        mnem: &str,
        line: usize,
        col: usize,
    ) -> Result<u64, String> {
        if v <= max {
            Ok(v)
        } else {
            Err(format!(
                "{}:{}:{}: immediate operand {} out of range 0..{} for {}",
                self.current_filename, line, col, v as i64, max, mnem
            ))
        }
    }

    /// Evaluate `pair` and range-check it as an 8-bit instruction field
    /// (0..=255): every explicit `*I` spelling's own byte-sized operand.
    fn imm_byte(&self, pair: pest::iterators::Pair<Rule>, mnem: &str) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 0xFF, mnem, line, col)
            .map(|v| v as u8)
    }

    /// Evaluate `pair` and range-check it as a 16-bit instruction field
    /// (0..=65535): the wyde immediates and `TRAP`/`TRIP`/`SWYM`/`POP`'s
    /// `yz`.
    fn imm_wyde(&self, pair: pest::iterators::Pair<Rule>, mnem: &str) -> Result<u16, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 0xFFFF, mnem, line, col)
            .map(|v| v as u16)
    }

    /// Evaluate `pair` and range-check it as a 24-bit instruction field
    /// (0..=16777215): `TRAP`/`TRIP`/`SWYM`/`POP`'s `xyz`, `RESUME` and
    /// `SYNC`.
    fn imm_three_bytes(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u32, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 0xFF_FFFF, mnem, line, col)
            .map(|v| v as u32)
    }

    /// Evaluate `pair` and range-check it as a special register number
    /// (0..=31): `GET`'s `Z`, `PUT`'s and `PUTI`'s `X`.
    fn special_register(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 31, mnem, line, col)
            .map(|v| v as u8)
    }

    /// Range-check an already-resolved Z value as an 8-bit immediate
    /// (0..=255), for [`Self::lower_z_operand`], which has already told
    /// register and pure values apart.
    fn imm_in_range(&self, v: u64, mnem: &str, line: usize, col: usize) -> Result<ZForm, String> {
        self.field_value(v, 0xFF, mnem, line, col)
            .map(|v| ZForm::Imm(v as u8))
    }

    /// Evaluate `pair` and accept either a register or a pure value as an
    /// 8-bit field: a register contributes its own number, range-checked
    /// the same as any other register operand; a pure value contributes its
    /// own magnitude, range-checked as an immediate. `TRAP`, `TRIP` and
    /// `SWYM`'s X, Y and Z read this way, as does the X byte `PUSHJ`,
    /// `PUSHGO`, the `PRELD` family and `STCO` take.
    fn parse_reg_or_byte(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => self.require_register_in_range(r, line, col),
            ExprValue::Pure(v) => self.field_value(v, 0xFF, mnem, line, col).map(|v| v as u8),
        }
    }

    /// The two-operand memory form's base-address search: among every
    /// `GREG` seen so far (in source order) whose initial value is nonzero,
    /// choose the largest value `b` with `b <= addr` and `addr - b < 256`,
    /// the earliest allocated on a tie between registers holding the same
    /// value. Returns the matched register and `addr - b`.
    fn resolve_base_address(&self, addr: u64, line: usize, col: usize) -> Result<(u8, u8), String> {
        let mut best: Option<(u8, u64)> = None;
        for &(reg, value) in &self.greg_inits[..self.greg_inits_seen] {
            if value == 0 || value > addr || addr - value >= 256 {
                continue;
            }
            if best.is_none_or(|(_, best_value)| value > best_value) {
                best = Some((reg, value));
            }
        }
        match best {
            Some((reg, value)) => Ok((reg, (addr - value) as u8)),
            None => Err(format!(
                "{}:{}:{}: no GREG before this instruction holds a base address 0 to 255 \
                 bytes below {addr:#x}",
                self.current_filename, line, col
            )),
        }
    }

    /// The two-operand memory form's second operand: a register operand is
    /// an offset of zero, following its value, not its spelling; a pure
    /// operand is a base address, resolved by [`Self::resolve_base_address`].
    /// Returns the register to place in Y and the offset to place in Z.
    fn resolve_memory_base_operand(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(u8, u8), String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => {
                let reg = self.require_register_in_range(r, line, col)?;
                Ok((reg, 0))
            }
            ExprValue::Pure(addr) => self.resolve_base_address(addr, line, col),
        }
    }

    /// Evaluate `pair` (an `expr`, or one of the rules it nests) under the
    /// register/pure-value rules in `MMIX.md`'s Expressions section: `+ - *`
    /// wrap mod 2^64, `/` `//` `%` `<<` `>>` follow the reference's
    /// definitions, `@` is the current location, and a symbol carries
    /// whichever kind its `SymbolType` records. Register arithmetic combines
    /// a register with a pure value into a register (register-pure
    /// subtraction included); register-register subtraction gives a pure
    /// value; every other operator applied to a register is an error, as is
    /// every non-`+` unary operator.
    fn eval_expr(&self, pair: pest::iterators::Pair<Rule>) -> Result<ExprValue, String> {
        match pair.as_rule() {
            // Wraps exactly one `expr`; some call sites hand this container
            // pair straight to the evaluator unwrapped.
            Rule::operand_list_one => self.eval_expr(
                pair.into_inner()
                    .next()
                    .expect("operand wraps exactly one expr"),
            ),
            Rule::expr | Rule::group_expr | Rule::data_group_expr => {
                let mut parts = pair.into_inner();
                let mut acc = self.eval_expr(parts.next().expect("expr has a term"))?;
                while let Some(op) = parts.next() {
                    let (line, col) = op.line_col();
                    let rhs = self.eval_expr(parts.next().expect("weak operator needs a term"))?;
                    acc = self.apply_weak(op.as_str(), acc, rhs, line, col)?;
                }
                Ok(acc)
            }
            Rule::term | Rule::group_term | Rule::data_group_term => {
                let mut parts = pair.into_inner();
                let mut acc = self.eval_expr(parts.next().expect("term has a primary"))?;
                while let Some(op) = parts.next() {
                    let (line, col) = op.line_col();
                    let rhs =
                        self.eval_expr(parts.next().expect("strong operator needs a primary"))?;
                    acc = self.apply_strong(op.as_str(), acc, rhs, line, col)?;
                }
                Ok(acc)
            }
            Rule::primary | Rule::group_primary | Rule::data_group_primary => {
                let (line, col) = pair.line_col();
                let mut parts = pair.into_inner();
                let first = parts.next().expect("primary has a child");
                if first.as_rule() == Rule::unary_op {
                    let operand =
                        self.eval_expr(parts.next().expect("unary operator needs an operand"))?;
                    self.apply_unary(first.as_str(), operand, line, col)
                } else {
                    self.eval_expr(first)
                }
            }
            Rule::group | Rule::data_group => self.eval_expr(
                pair.into_inner()
                    .next()
                    .expect("group has an inner expression"),
            ),
            // Reached only through `data_group_primary`: a parenthesized
            // group always needs a single value.
            Rule::string_literal => self.eval_group_string(pair),
            Rule::at_symbol => Ok(ExprValue::Pure(self.current_addr)),
            Rule::constant => self.eval_literal(
                pair.into_inner()
                    .next()
                    .expect("constant has exactly one literal"),
            ),
            Rule::global_id => {
                let (line, col) = pair.line_col();
                let text = pair.as_str();
                let qualified = self.qualify_name(text);
                // Labels before symbols: a program's own label wins over a
                // predefined symbol of the same name (a user IS/GREG
                // symbol already overwrites the predefined entry directly
                // in `symbols`, so this order alone is what a label needs).
                if let Some(&label_addr) = self.labels.get(&qualified) {
                    Ok(ExprValue::Pure(label_addr))
                } else if let Some(&symbol_type) = self.symbols.get(&qualified) {
                    Ok(match symbol_type {
                        SymbolType::Constant(value) => ExprValue::Pure(value),
                        SymbolType::Register(reg) => ExprValue::Register(reg as u64),
                    })
                } else {
                    Err(format!(
                        "{}:{}:{}: Undefined symbol: {}",
                        self.current_filename, line, col, qualified
                    ))
                }
            }
            Rule::local_ref_back => {
                let digit = Self::local_digit(pair.as_str());
                Ok(self.resolve_local_back(digit))
            }
            Rule::local_ref_fwd => {
                let (line, col) = pair.line_col();
                let digit = Self::local_digit(pair.as_str());
                self.resolve_local_fwd(digit, line, col)
            }
            other => {
                let (line, col) = pair.line_col();
                Err(format!(
                    "Line {}:{}: Expected expression, got: {:?}",
                    line, col, other
                ))
            }
        }
    }

    /// A `string_literal` standing where a parenthesized group needs its
    /// one value: its single character, or an error naming how many
    /// characters it holds. `eval_data_primary_atoms` is the path for a
    /// string outside a group, where it may expand to more than one item.
    fn eval_group_string(&self, pair: pest::iterators::Pair<Rule>) -> Result<ExprValue, String> {
        let (line, col) = pair.line_col();
        let (first, rest) = self.decode_data_string_literal(&pair)?;
        if rest.is_empty() {
            return Ok(ExprValue::Pure(first as u64));
        }
        Err(format!(
            "{}:{}:{}: a {}-character string is not a single value here",
            self.current_filename,
            line,
            col,
            1 + rest.len()
        ))
    }

    /// Whether `data_expr` is `BYTE ""`'s one exempt shape: a bare string,
    /// with no unary wrap, sibling term or primary, and no content. Its
    /// empty string contributes zero items, not the error every other
    /// position gives one.
    fn is_bare_empty_string(data_expr: &pest::iterators::Pair<Rule>) -> bool {
        let mut terms = data_expr.clone().into_inner();
        let Some(term) = terms.next() else {
            return false;
        };
        if terms.next().is_some() {
            return false;
        }
        let mut primaries = term.into_inner();
        let Some(primary) = primaries.next() else {
            return false;
        };
        if primaries.next().is_some() {
            return false;
        }
        let mut children = primary.into_inner();
        let Some(child) = children.next() else {
            return false;
        };
        if children.next().is_some() || child.as_rule() != Rule::string_literal {
            return false;
        }
        let text = child.as_str();
        text.len() == 2
    }

    /// A `data_value`'s inner `data_expr`, present whenever the grammar
    /// built the node.
    fn data_expr(
        value: pest::iterators::Pair<Rule>,
    ) -> Result<pest::iterators::Pair<Rule>, String> {
        value
            .into_inner()
            .next()
            .ok_or_else(|| "Missing data expression".to_string())
    }

    /// A `string_literal`'s content, decoded to its first character's
    /// Unicode scalar value and the rest. Rejects an empty string with an
    /// error at its own position; the one exemption, `BYTE ""` standing
    /// entirely alone, is caught by [`Self::is_bare_empty_string`] before
    /// either pass reaches this.
    fn decode_data_string_literal(
        &self,
        string_pair: &pest::iterators::Pair<Rule>,
    ) -> Result<(u32, Vec<u32>), String> {
        let (line, col) = string_pair.line_col();
        let text = string_pair.as_str();
        let mut values = Self::decode_char_values(&text[1..text.len() - 1]).into_iter();
        let first = values.next().ok_or_else(|| {
            format!(
                "{}:{}:{}: an empty string is not a value inside an expression",
                self.current_filename, line, col
            )
        })?;
        Ok((first, values.collect()))
    }

    /// Evaluate a `data_value` (a `data_expr`) into the values it
    /// contributes to its directive's list. A string stands wherever a
    /// `data_primary` stands: an operator directly on it combines with its
    /// first character (a leading unary or the operator before it) or its
    /// last (the operator after it), and each character between stays its
    /// own item. A string standing alone, with no operator anywhere,
    /// contributes one item per character, none for an empty string.
    fn eval_data_value_items(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<ExprValue>, String> {
        let data_expr = Self::data_expr(pair)?;
        if Self::is_bare_empty_string(&data_expr) {
            return Ok(Vec::new());
        }
        Ok(self.eval_data_expr_atoms(data_expr)?.into_vec())
    }

    /// Folds a `data_expr`'s `data_term`s or a `data_term`'s
    /// `data_primary`s into one [`DataAtoms`]: `child` evaluates each
    /// operand and `apply` combines a pair of atoms across the operator
    /// between them, via [`DataAtoms::merge`].
    fn fold_data_atoms(
        &self,
        pair: pest::iterators::Pair<Rule>,
        child: EvalOperand,
        apply: ApplyOperator,
    ) -> Result<DataAtoms, String> {
        let mut parts = pair.into_inner();
        let mut atoms = child(
            self,
            parts
                .next()
                .ok_or_else(|| "a fold operand list is never empty".to_string())?,
        )?;
        while let Some(op) = parts.next() {
            let (line, col) = op.line_col();
            let rhs = child(
                self,
                parts
                    .next()
                    .ok_or_else(|| "an operator needs a right operand".to_string())?,
            )?;
            atoms = atoms.merge(rhs, |lhs, rhs| {
                apply(self, op.as_str(), lhs, rhs, line, col)
            })?;
        }
        Ok(atoms)
    }

    /// [`Self::eval_data_value_items`]'s fold across a `data_expr`'s weak
    /// operators: each `data_term` contributes one or more atoms, and a
    /// weak operator combines the last atom before it with the first atom
    /// after -- every atom between stays its own item.
    fn eval_data_expr_atoms(&self, pair: pest::iterators::Pair<Rule>) -> Result<DataAtoms, String> {
        self.fold_data_atoms(pair, Self::eval_data_term_atoms, Self::apply_weak)
    }

    /// [`Self::eval_data_expr_atoms`]'s counterpart for a `data_term`'s
    /// strong operators, combining `data_primary` atom lists the same way.
    fn eval_data_term_atoms(&self, pair: pest::iterators::Pair<Rule>) -> Result<DataAtoms, String> {
        self.fold_data_atoms(pair, Self::eval_data_primary_atoms, Self::apply_strong)
    }

    /// A `data_primary`'s atoms: one, for an ordinary value, or one per
    /// character for a string. A unary operator combines with the first
    /// atom of its operand only -- `-"ab"` is `-'a'`, `'b'`, matching
    /// [`DataAtoms::merge`]'s treatment of a binary operator's neighbor.
    fn eval_data_primary_atoms(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<DataAtoms, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let first = parts
            .next()
            .ok_or_else(|| "data_primary has a child".to_string())?;
        match first.as_rule() {
            Rule::unary_op => {
                let operand = parts
                    .next()
                    .ok_or_else(|| "unary operator needs an operand".to_string())?;
                let mut atoms = self.eval_data_primary_atoms(operand)?;
                atoms.head = self.apply_unary(first.as_str(), atoms.head, line, col)?;
                Ok(atoms)
            }
            Rule::string_literal => {
                let (first_char, rest) = self.decode_data_string_literal(&first)?;
                Ok(DataAtoms::from_chars(first_char, rest))
            }
            _ => Ok(DataAtoms::one(self.eval_expr(first)?)),
        }
    }

    /// The digit a local-symbol token (`dH`, `dB` or `dF`) opens with, as
    /// an index into the ten per-digit lists.
    fn local_digit(text: &str) -> u8 {
        text.as_bytes()[0] - b'0'
    }

    fn symbol_type_to_expr_value(ty: SymbolType) -> ExprValue {
        match ty {
            SymbolType::Constant(v) => ExprValue::Pure(v),
            SymbolType::Register(r) => ExprValue::Register(r as u64),
        }
    }

    /// `dB`: the last `dH` of this digit at or before the referencing
    /// statement, or `0` when none has appeared yet -- never an error.
    fn resolve_local_back(&self, digit: u8) -> ExprValue {
        let idx = digit as usize;
        let occurrence = self.local_occurrence[idx];
        if occurrence == 0 {
            ExprValue::Pure(0)
        } else {
            Self::symbol_type_to_expr_value(self.local_labels[idx][occurrence - 1])
        }
    }

    /// `dF`: the first `dH` of this digit after the referencing statement,
    /// an error naming the reference when none follows. In pass 1 the list
    /// for `digit` never holds more than `local_occurrence[digit]` entries
    /// (pass 1 is still building it), so this always reports undefined
    /// there, exactly as an undefined named forward reference does.
    /// `local_pending_digit` bumps the search past the statement's own
    /// not-yet-recorded occurrence, when it defines this same digit.
    fn resolve_local_fwd(&self, digit: u8, line: usize, col: usize) -> Result<ExprValue, String> {
        let idx = digit as usize;
        let mut index = self.local_occurrence[idx];
        if self.local_pending_digit == Some(digit) {
            index += 1;
        }
        self.local_labels[idx]
            .get(index)
            .map(|&ty| Self::symbol_type_to_expr_value(ty))
            .ok_or_else(|| {
                format!(
                    "{}:{}:{}: Undefined symbol: {}F",
                    self.current_filename, line, col, digit
                )
            })
    }

    /// Records one `dH` occurrence. `building` is pass 1's own flag: pass 1
    /// appends `value` to the digit's list, pass 2 only advances the
    /// per-pass occurrence counter that keeps its `dB`/`dF` reads in step
    /// with pass 1's completed lists.
    fn record_local_label(&mut self, digit: u8, value: SymbolType, building: bool) {
        let idx = digit as usize;
        if building {
            self.local_labels[idx].push(value);
        }
        self.local_occurrence[idx] += 1;
    }

    /// Evaluate a leaf numeric literal: hex, octal, decimal or char.
    fn eval_literal(&self, pair: pest::iterators::Pair<Rule>) -> Result<ExprValue, String> {
        let (line, col) = pair.line_col();
        let text = pair.as_str();

        let value = match pair.as_rule() {
            Rule::char_literal => {
                let inner = &text[1..text.len() - 1];
                let ch = inner.chars().next().ok_or_else(|| {
                    "grammar admits exactly one character between the quotes".to_string()
                })?;
                ch as u32 as u64
            }
            // A digit string the grammar matched always has a value: a hex
            // or decimal constant of 2^64 or more reduces mod 2^64, the
            // reference's own rule, computed digit by digit so no width
            // limit on the source spelling is ever reached.
            Rule::hex_literal => {
                let hex_str = if let Some(stripped) = text.strip_prefix('#') {
                    stripped
                } else if let Some(stripped) =
                    text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
                {
                    stripped
                } else {
                    text
                };
                hex_str.chars().try_fold(0u64, |acc, c| {
                    let digit = c
                        .to_digit(16)
                        .ok_or_else(|| "grammar admits only hex digits".to_string())?;
                    Ok::<u64, String>(acc.wrapping_shl(4).wrapping_add(digit as u64))
                })?
            }
            Rule::dec_literal => text.chars().try_fold(0u64, |acc, c| {
                let digit = c
                    .to_digit(10)
                    .ok_or_else(|| "grammar admits only decimal digits".to_string())?;
                Ok::<u64, String>(acc.wrapping_mul(10).wrapping_add(digit as u64))
            })?,
            other => {
                return Err(format!(
                    "Line {}:{}: Expected a literal, got: {:?}",
                    line, col, other
                ));
            }
        };
        Ok(ExprValue::Pure(value))
    }

    /// Unary operators: `+` is the identity, including on a register; `-`
    /// and `~` apply only to a pure value; `$` casts a pure value to a
    /// register; `&` (a symbol's serial number) is always rejected, since
    /// checksmix's object file carries no symbol table to index.
    fn apply_unary(
        &self,
        op: &str,
        value: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        if op == "&" {
            return Err(format!(
                "{}:{}:{}: unary & (a symbol's serial number) is unsupported",
                self.current_filename, line, col
            ));
        }
        match (op, value) {
            ("+", v) => Ok(v),
            ("-", ExprValue::Pure(v)) => Ok(ExprValue::Pure(0u64.wrapping_sub(v))),
            ("~", ExprValue::Pure(v)) => Ok(ExprValue::Pure(!v)),
            ("$", ExprValue::Pure(v)) => Ok(ExprValue::Register(v)),
            (op, ExprValue::Register(_)) => Err(format!(
                "{}:{}:{}: unary {} cannot apply to a register",
                self.current_filename, line, col, op
            )),
            _ => unreachable!("grammar admits only + - ~ $ & as unary_op"),
        }
    }

    /// Weak (lowest-precedence) binary operators: `+` `-` `|` `^`.
    /// Register arithmetic: register+pure, pure+register and register-pure
    /// give a register; register-register gives a pure value; `|` and `^`
    /// never take a register operand.
    fn apply_weak(
        &self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        use ExprValue::{Pure, Register};
        match (op, lhs, rhs) {
            ("+", Pure(a), Pure(b)) => Ok(Pure(a.wrapping_add(b))),
            ("-", Pure(a), Pure(b)) => Ok(Pure(a.wrapping_sub(b))),
            ("|", Pure(a), Pure(b)) => Ok(Pure(a | b)),
            ("^", Pure(a), Pure(b)) => Ok(Pure(a ^ b)),
            ("+", Register(a), Pure(b)) | ("+", Pure(b), Register(a)) => {
                Ok(Register(a.wrapping_add(b)))
            }
            ("-", Register(a), Pure(b)) => Ok(Register(a.wrapping_sub(b))),
            ("-", Register(a), Register(b)) => Ok(Pure(a.wrapping_sub(b))),
            _ => Err(format!(
                "{}:{}:{}: {} cannot apply to a register operand",
                self.current_filename, line, col, op
            )),
        }
    }

    /// Strong (highest-precedence) binary operators: `*` `/` `//` `%` `<<`
    /// `>>` `&`. None takes a register operand. `x/y` is illegal at `y=0`;
    /// `x//y` is illegal at `x>=y` (which subsumes `y=0`, since every `x` is
    /// `>=0`); `x%y` shares `x/y`'s zero-divisor rule, computing the
    /// remainder of the same division. A shift of 64 or more gives `0`.
    fn apply_strong(
        &self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        let (a, b) = match (lhs, rhs) {
            (ExprValue::Pure(a), ExprValue::Pure(b)) => (a, b),
            _ => {
                return Err(format!(
                    "{}:{}:{}: {} cannot apply to a register operand",
                    self.current_filename, line, col, op
                ));
            }
        };
        let value = match op {
            "*" => a.wrapping_mul(b),
            "/" => {
                if b == 0 {
                    return Err(format!(
                        "{}:{}:{}: division by zero in {}/{}",
                        self.current_filename, line, col, a, b
                    ));
                }
                a / b
            }
            "%" => {
                if b == 0 {
                    return Err(format!(
                        "{}:{}:{}: division by zero in {}%{}",
                        self.current_filename, line, col, a, b
                    ));
                }
                a % b
            }
            "//" => {
                if a >= b {
                    return Err(format!(
                        "{}:{}:{}: illegal fraction {}//{} (the dividend must be less than the divisor)",
                        self.current_filename, line, col, a, b
                    ));
                }
                (((a as u128) << 64) / (b as u128)) as u64
            }
            "<<" => {
                if b >= 64 {
                    0
                } else {
                    a << b
                }
            }
            ">>" => {
                if b >= 64 {
                    0
                } else {
                    a >> b
                }
            }
            "&" => a & b,
            _ => unreachable!("grammar admits only * / // % << >> & as strong_op"),
        };
        Ok(ExprValue::Pure(value))
    }

    /// Range-check a register value carried inside an expression (up to
    /// 64 bits, per unary `$`) down to the 0..=255 a register field holds.
    fn require_register_in_range(&self, r: u64, line: usize, col: usize) -> Result<u8, String> {
        u8::try_from(r).map_err(|_| {
            format!(
                "{}:{}:{}: register ${} is out of range 0..255",
                self.current_filename, line, col, r
            )
        })
    }

    /// Evaluate `pair` (an `expr`) and demand a pure value.
    fn parse_number(&self, pair: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let (line, col) = pair.line_col();
        let value = self.eval_expr(pair)?;
        self.require_pure(value, line, col)
    }

    /// Demand a pure value out of an already-evaluated `ExprValue`, at
    /// `line`:`col` for the diagnostic. Shared by [`Self::parse_number`],
    /// which evaluates its own pair first, and `parse_data_directive`, which
    /// already holds one from [`Self::eval_data_value_items`].
    fn require_pure(&self, value: ExprValue, line: usize, col: usize) -> Result<u64, String> {
        match value {
            ExprValue::Pure(v) => Ok(v),
            ExprValue::Register(r) => Err(format!(
                "{}:{}:{}: register ${} cannot be used where a pure value is required",
                self.current_filename, line, col, r
            )),
        }
    }

    /// Evaluate `pair` (an `expr`) and demand a register, range-checked
    /// 0..=255.
    fn parse_register(&self, pair: pest::iterators::Pair<Rule>) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => self.require_register_in_range(r, line, col),
            ExprValue::Pure(v) => Err(format!(
                "{}:{}:{}: pure value {} cannot be used where a register is required",
                self.current_filename, line, col, v
            )),
        }
    }

    /// Every instruction occupies a tetra-aligned slot, including the
    /// pseudo-instructions `instruction_size` reports as wider than one tetra.
    /// Alignment is not the emitted size.
    const INSTRUCTION_ALIGNMENT: u64 = 4;

    fn instruction_size(inst: &MMixInstruction) -> u64 {
        match inst {
            MMixInstruction::SET(_, _) => 16,
            MMixInstruction::SETRR(_, _) => 4, // ORI $X, $Y, 0
            MMixInstruction::BYTE(_) => 1,
            MMixInstruction::WYDE(_) => 2,
            MMixInstruction::TETRA(_) => 4,
            MMixInstruction::OCTA(_) => 8,
            _ => 4,
        }
    }

    /// Encode a single instruction into bytes using the shared encode module
    pub fn encode_instruction_bytes(&self, instruction: &MMixInstruction) -> Vec<u8> {
        crate::encode::encode_instruction_bytes(instruction)
    }

    /// Generate object code in MMO format
    pub fn generate_object_code(&self) -> Vec<u8> {
        crate::mmo::MmoGenerator::new(self.instructions.clone(), self.labels.clone())
            .with_debug_strings(self.debug_strings.clone())
            .generate()
    }

    /// The original source location of the code at `addr`, or `None` for an
    /// address no statement emitted.
    ///
    /// `addr` need not be an entry address: an address inside a multi-tetra
    /// expansion resolves to the statement that emitted it. Each entry
    /// covers exactly the bytes its own item emitted, so an address past the
    /// end of a run maps to nothing -- the gap between the text and data
    /// segments stays unmapped.
    pub fn source_loc(&self, addr: u64) -> Option<&SourceLoc> {
        let (&start, loc) = self.debug_info.range(..=addr).next_back()?;
        (addr - start < self.emitted_size(start)?).then_some(loc)
    }

    /// The byte extent of the item emitted at `addr`, or `None` if no item
    /// starts there.
    fn emitted_size(&self, addr: u64) -> Option<u64> {
        self.instructions
            .iter()
            .find(|(at, _)| *at == addr)
            .map(|(_, inst)| Self::instruction_size(inst))
    }

    /// The lowest instruction address whose source location is `(file,
    /// line)`, or `None` if that line produced no code. Exact-line only.
    ///
    /// If `file` was passed as more than one translation unit (a duplicate
    /// input filename), this still matches by string equality against
    /// whichever unit(s) produced code at that line -- a documented,
    /// deterministic limitation for the pathological duplicate-filename
    /// case; the address returned is always the lowest that matches.
    pub fn addr_for_line(&self, file: &str, line: usize) -> Option<u64> {
        self.debug_info
            .iter()
            .find(|(_, loc)| loc.file == file && loc.line == line)
            .map(|(&addr, _)| addr)
    }

    /// The original (un-preprocessed) text of `file`'s 1-based `line`,
    /// without the trailing newline, or `None` past end-of-file or for an
    /// unknown file. If `file` was passed more than once, resolves to the
    /// FIRST translation unit with that name in command-line order.
    pub fn source_text(&self, file: &str, line: usize) -> Option<&str> {
        let unit = self.sources.iter().find(|u| u.filename == file)?;
        let index = line.checked_sub(1)?;
        unit.original.lines().nth(index)
    }

    /// Expand `INCLUDE <file>` directives into an ordered list of translation
    /// units, ready to feed to `new`/`add_source`. Host files are split at
    /// each `INCLUDE` into segments (each tagged with the host's filename and
    /// blank-padded to keep absolute line numbers); each included file is
    /// resolved recursively and its units inserted at that position. Paths
    /// resolve relative to the including file's directory. `read` supplies
    /// file contents -- injected so this logic is testable without real
    /// filesystem access and reusable by any frontend. A cycle (re-entry on
    /// the current include chain) or an unreadable file is an `Err`.
    pub fn resolve_includes<R>(
        root_source: &str,
        root_filename: &str,
        base_dir: &Path,
        read: &R,
    ) -> Result<Vec<(String, String)>, String>
    where
        R: Fn(&Path) -> std::io::Result<String>,
    {
        let mut chain = Vec::new();
        Self::resolve_includes_chain(root_source, root_filename, base_dir, read, &mut chain)
    }

    /// Recursive worker behind `resolve_includes`. `chain` holds the
    /// lexically-normalized identity of every file currently being expanded
    /// (the ancestors on the path from the top-level call to here), used to
    /// detect a file re-entering itself before it finishes expanding.
    fn resolve_includes_chain<R>(
        root_source: &str,
        root_filename: &str,
        base_dir: &Path,
        read: &R,
        chain: &mut Vec<PathBuf>,
    ) -> Result<Vec<(String, String)>, String>
    where
        R: Fn(&Path) -> std::io::Result<String>,
    {
        let mut units = Vec::new();
        let mut segment = String::new();
        let mut segment_start_line: usize = 1;
        let mut current_line: usize = 0;

        for raw_line in root_source.split_inclusive('\n') {
            current_line += 1;
            let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
            let Some(operand) = Self::parse_include_operand(content) else {
                segment.push_str(raw_line);
                continue;
            };

            if !segment.trim().is_empty() {
                units.push((
                    root_filename.to_string(),
                    Self::pad_source(&segment, segment_start_line),
                ));
            }
            segment.clear();

            let target = Self::normalize_lexically(&base_dir.join(&operand));
            if chain.contains(&target) {
                let mut names: Vec<String> =
                    chain.iter().map(|p| p.display().to_string()).collect();
                names.push(target.display().to_string());
                return Err(format!("include cycle detected: {}", names.join(" -> ")));
            }

            let included_source = read(&target).map_err(|err| {
                format!("cannot read included file '{}': {}", target.display(), err)
            })?;
            let included_filename = target.display().to_string();
            let included_base_dir = target
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(""));

            chain.push(target);
            let included_units = Self::resolve_includes_chain(
                &included_source,
                &included_filename,
                &included_base_dir,
                read,
                chain,
            )?;
            chain.pop();
            units.extend(included_units);

            segment_start_line = current_line + 1;
        }

        if !segment.trim().is_empty() {
            units.push((
                root_filename.to_string(),
                Self::pad_source(&segment, segment_start_line),
            ));
        }

        Ok(units)
    }

    /// If `line`, after stripping a trailing `%` comment and trimming
    /// whitespace, is an `INCLUDE` directive matched in upper case only,
    /// returns its operand (unquoted if wrapped in matching double quotes).
    /// `;` is not a comment character here: it lands inside the operand and
    /// fails as an unreadable file naming the whole text, since `INCLUDE`
    /// occupies its own line.
    fn parse_include_operand(line: &str) -> Option<String> {
        let without_comment = match line.find('%') {
            Some(idx) => &line[..idx],
            None => line,
        };
        let trimmed = without_comment.trim();
        let mut parts = trimmed.splitn(2, |c: char| c.is_whitespace());
        let keyword = parts.next()?;
        if keyword != "INCLUDE" {
            return None;
        }
        let operand = parts.next().unwrap_or("").trim();
        if operand.is_empty() {
            return None;
        }
        let unquoted = if operand.len() >= 2 && operand.starts_with('"') && operand.ends_with('"') {
            &operand[1..operand.len() - 1]
        } else {
            operand
        };
        Some(unquoted.to_string())
    }

    /// Prepend `start_line - 1` newlines to `text` so absolute line numbers
    /// survive being embedded at `start_line` of some larger file (the
    /// grammar ignores leading blank lines).
    fn pad_source(text: &str, start_line: usize) -> String {
        let padding = start_line.saturating_sub(1);
        let mut padded = String::with_capacity(text.len() + padding);
        for _ in 0..padding {
            padded.push('\n');
        }
        padded.push_str(text);
        padded
    }

    /// Lexically collapse `.`/`..` components without touching the
    /// filesystem (unlike `fs::canonicalize`, which would defeat the
    /// injected reader in unit tests and can fail on a nonexistent path).
    fn normalize_lexically(path: &Path) -> PathBuf {
        use std::path::Component;

        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !result.pop() {
                        result.push("..");
                    }
                }
                other => result.push(other.as_os_str()),
            }
        }
        result
    }
}

// Keep all the existing tests - they should work unchanged
#[cfg(test)]
mod tests {
    use super::*;

    // ---- Mnemonic word-boundary guard (adversarial) ------------------
    // Every `mnemonic_*` and `directive_*` rule closes with the shared
    // `word_end` rule: a keyword ends where a symbol could not continue, so a
    // short mnemonic's literal cannot match as a bare prefix of a longer one.
    // `Rule::parse` doesn't require consuming the whole input, so without the
    // guard each of these three calls returns `Ok` (the rule matches only its
    // own shorter literal and stops); with it each must return `Err`.

    #[test]
    fn test_mnemonic_boundary_guard_rejects_prefix_match() {
        use pest::Parser;

        // GET is a literal prefix of GETA/GETAB (the flagged pair).
        assert!(
            MMixalParser::parse(Rule::mnemonic_get, "GETA").is_err(),
            "mnemonic_get must not match a bare prefix of GETA"
        );
        // SET is a literal prefix of SETL/SETH/SETMH/SETML.
        assert!(
            MMixalParser::parse(Rule::mnemonic_set, "SETL").is_err(),
            "mnemonic_set must not match a bare prefix of SETL"
        );
        // SYNC is a literal prefix of SYNCD/SYNCID/SYNCDI/SYNCIDI.
        assert!(
            MMixalParser::parse(Rule::mnemonic_sync, "SYNCD").is_err(),
            "mnemonic_sync must not match a bare prefix of SYNCD"
        );
    }

    // ---- Mnemonic word-boundary guard: whitespace-adjacency behavior -
    // The guard has two known, intentional side effects on whitespace-
    // adjacent constructs. Both are real behavior changes, tested in
    // both directions here rather than left to surface only as an
    // unexplained corpus diff.

    #[test]
    fn test_boundary_guard_rejects_zero_whitespace_before_operand() {
        // `ADDa,b,c` (no space between the mnemonic and its first operand)
        // parsed as ADD with the three IS-aliased register operands before
        // this guard -- an accident of the grammar's implicit whitespace,
        // never legitimate MMIXAL syntax. `ADDa` fails `word_end` ('a' could
        // continue a symbol), so `ADDa,b,c` never matches an instruction;
        // `Main` claims the line as a bare label, but a label statement
        // holds nothing but blanks and a comment, so `ADDa,b,c` -- an
        // unrecognized word in opcode position -- is a syntax error rather
        // than commentary silently dropped.
        let source = "a IS $1\nb IS $2\nc IS $3\nMain ADDa,b,c";
        let mut asm = MMixAssembler::new(source, "<test>");
        assert!(
            asm.parse().is_err(),
            "ADDa,b,c must be rejected as an unknown operation"
        );
    }

    #[test]
    fn test_boundary_guard_accepts_mnemonic_prefixed_label() {
        // `HaltLoop` immediately followed by more source used to fail to
        // parse as a label: unguarded `mnemonic_halt` greedily matched the
        // "Halt" prefix of "HaltLoop" as a complete zero-operand HALT
        // instruction before the grammar ever tried `label_def`, leaving
        // "Loop  ADD $1,$2,$3" unparsed. After the guard,
        // "Halt" immediately followed by 'L' (alphanumeric) fails the
        // boundary check, `instruction` no longer matches at that position,
        // and `label_def` correctly claims `HaltLoop` as a label.
        let source = "HaltLoop  ADD $1,$2,$3\n  HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        assert_eq!(
            asm.labels.get("HaltLoop"),
            Some(&0),
            "HaltLoop must resolve to address 0"
        );
    }

    #[test]
    fn test_keyword_boundary_admits_underscore_in_label() {
        // A symbol continues on '_', so a keyword must not end before one:
        // `Halt_Loop` is a label, not HALT trailing garbage. These cover the
        // three shapes that misparsed -- an operand-less mnemonic, a mnemonic
        // whose operand would absorb the tail, and a directive.
        let source = "Halt_Loop SETL $1,1\n\
                      Swym_x SETL $2,2\n\
                      Resume_x SETL $3,3\n\
                      Loc_Start SETL $4,4\n\
                      Greg_Base SETL $5,5\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        for (name, address) in [
            ("Halt_Loop", 0u64),
            ("Swym_x", 4),
            ("Resume_x", 8),
            ("Loc_Start", 12),
            ("Greg_Base", 16),
        ] {
            assert_eq!(
                asm.labels.get(name),
                Some(&address),
                "{name} must be claimed as a label at {address}"
            );
        }
    }

    #[test]
    fn test_swym_carries_its_operands() {
        let source = "SWYM 1,2,3\nSWYM";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        assert_eq!(asm.instructions.len(), 2, "both SWYM forms must assemble");
        assert_eq!(asm.instructions[0].1, MMixInstruction::SWYM(1, 2, 3));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SWYM(0, 0, 0));
    }

    #[test]
    fn test_swym_rejects_a_partial_operand_list() {
        // SWYM 1,2 is the two-operand form, SWYM(1,0,2). A trailing
        // comma with nothing after it is a partial list: no operand count
        // SWYM takes matches "1,2,", so it falls back to the one-operand
        // form on "1", leaving ",2," -- a leading comma reads as a dropped
        // operand, not commentary, so this is a syntax error rather than
        // silently becoming SWYM 1.
        let mut asm = MMixAssembler::new("SWYM 1,2,", "<test>");
        assert!(
            asm.parse().is_err(),
            "a trailing comma leaves a partial list"
        );
    }

    #[test]
    fn test_parse_simple_label() {
        let mut asm = MMixAssembler::new("LOOP: HALT", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("LOOP"), Some(&0));
        assert_eq!(asm.instructions.len(), 1);
    }

    #[test]
    fn test_parse_octa_directive() {
        let mut asm = MMixAssembler::new("OCTA #123456789ABCDEF0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::OCTA(0x123456789ABCDEF0)
        );
    }

    #[test]
    fn test_parse_node_structure() {
        let mut asm = MMixAssembler::new("NODE: OCTA 42\n      OCTA 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("NODE"), Some(&0));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_parse_seti() {
        let mut asm = MMixAssembler::new("SETI $2, 10", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SET(2, 10));
    }

    #[test]
    fn test_parse_set_register() {
        let mut asm = MMixAssembler::new("SET $1, $7", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETRR(1, 7));
    }

    #[test]
    fn test_parse_negative_literal_seti() {
        let mut asm = MMixAssembler::new("SETI $1, -1", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SET(1, u64::MAX));
    }

    #[test]
    fn test_parse_negative_literal_8bit_is_an_error() {
        assert_eq!(
            assemble_err("ADDI $1, $2, -1"),
            "<test>:1:14: immediate operand -1 out of range 0..255 for ADDI"
        );
    }

    #[test]
    fn test_byte_string_pass1_pass2_agree() {
        // Pass 1 sizes the string and pass 2 expands it; the two must agree.
        // The forward OCTA reads pass 1's counter, because pass 2 resolves it
        // before reaching the label and overwriting the entry; the emitted
        // bytes read pass 2's. The label sits on a BYTE so that no rounding
        // can absorb a disagreement between them. A backslash is an ordinary
        // byte, so "a\nb" is four bytes.
        let mut asm = MMixAssembler::new("OCTA LABEL\nBYTE \"a\\nb\",0\nLABEL BYTE 7", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm.instructions[1..6]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            bytes,
            vec![
                (8, MMixInstruction::BYTE(b'a')),
                (9, MMixInstruction::BYTE(b'\\')),
                (10, MMixInstruction::BYTE(b'n')),
                (11, MMixInstruction::BYTE(b'b')),
                (12, MMixInstruction::BYTE(0)),
            ]
        );
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(13));
    }

    #[test]
    fn test_wyde_list_mixes_numbers_and_string_pass1_pass2_agree() {
        // A leading BYTE leaves the counter unaligned so List's WYDE must
        // round up. The forward OCTA reads pass 1's size for the list;
        // pass 2 must compute the same size or Next's address disagrees.
        let mut asm = MMixAssembler::new(
            "OCTA Next\nBYTE 1\nList WYDE 10,\"ab\",20\nNext BYTE 99",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("List"), Some(&10));
        let list: Vec<_> = asm.instructions[2..6]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            list,
            vec![
                (10, MMixInstruction::WYDE(10)),
                (12, MMixInstruction::WYDE(b'a' as u16)),
                (14, MMixInstruction::WYDE(b'b' as u16)),
                (16, MMixInstruction::WYDE(20)),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&18));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(18));
    }

    #[test]
    fn test_tetra_list_mixes_numbers_and_string_pass1_pass2_agree() {
        let mut asm = MMixAssembler::new(
            "OCTA Next\nBYTE 1\nList TETRA 10,\"ab\",20\nNext BYTE 99",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("List"), Some(&12));
        let list: Vec<_> = asm.instructions[2..6]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            list,
            vec![
                (12, MMixInstruction::TETRA(10)),
                (16, MMixInstruction::TETRA(b'a' as u32)),
                (20, MMixInstruction::TETRA(b'b' as u32)),
                (24, MMixInstruction::TETRA(20)),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&28));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(28));
    }

    #[test]
    fn test_octa_list_mixes_numbers_and_string_pass1_pass2_agree() {
        let mut asm = MMixAssembler::new(
            "OCTA Next\nBYTE 1\nList OCTA 10,\"ab\",20\nNext BYTE 99",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("List"), Some(&16));
        let list: Vec<_> = asm.instructions[2..6]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            list,
            vec![
                (16, MMixInstruction::OCTA(10)),
                (24, MMixInstruction::OCTA(b'a' as u64)),
                (32, MMixInstruction::OCTA(b'b' as u64)),
                (40, MMixInstruction::OCTA(20)),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&48));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(48));
    }

    #[test]
    fn test_byte_string_no_auto_terminator() {
        // MMIXAL appends no terminator to a BYTE string: "Hi" is two bytes and
        // nothing more.
        let mut asm = MMixAssembler::new("BYTE \"Hi\"", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            bytes,
            vec![
                (0, MMixInstruction::BYTE(b'H')),
                (1, MMixInstruction::BYTE(b'i')),
            ]
        );
    }

    #[test]
    fn test_octa_label_rounds_up_after_byte() {
        // MMIXAL rounds the counter to the item's width before assembling it,
        // so the octabyte -- and the label on it -- lands at 8, not 1.
        let mut asm = MMixAssembler::new("BYTE 1\nDATA: OCTA 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("DATA"), Some(&8));
    }

    #[test]
    fn test_wyde_and_tetra_labels_round_to_their_widths() {
        let mut asm = MMixAssembler::new("BYTE 1\nDATA: WYDE 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("DATA"), Some(&2));

        let mut asm = MMixAssembler::new("BYTE 1\nDATA: TETRA 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("DATA"), Some(&4));
    }

    #[test]
    fn test_instruction_label_rounds_up_after_byte() {
        // Instructions align to 4 like any tetra-wide item.
        let mut asm = MMixAssembler::new("BYTE 1\nCODE: HALT", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("CODE"), Some(&4));
    }

    #[test]
    fn test_forward_reference_resolves_to_aligned_address() {
        // Pass 2 overwrites asm.labels with its own addresses, so a pass-1 and
        // pass-2 disagreement survives only in the operand pass 2 encoded from
        // the stale pass-1 entry. DATA rounds to 8, four tetras past the JMP's
        // two.
        let mut asm = MMixAssembler::new("JMP DATA\nBYTE 1\nDATA: OCTA 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::JMP(2));
    }

    #[test]
    fn test_two_debug_directives_assemble_to_one_tetra_each() {
        // Each directive costs exactly one tetra: Main and the second
        // directive's TRAP sit four bytes apart, and Halt follows another
        // four bytes on -- no label, no data, no generated block.
        let source = "        LOC     #100\nMain    debug \"one\"\nSecond  debug \"two\"\n        TRAP    0,Halt,0\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();

        let main_addr = *asm.labels.get("Main").expect("Main label");
        let second_addr = *asm.labels.get("Second").expect("Second label");
        assert_eq!(second_addr, main_addr + 4);
        assert_eq!(asm.instructions.len(), 3);
        assert_eq!(asm.debug_strings(), &[b"one".to_vec(), b"two".to_vec()]);
    }

    /// `K` is one byte: a 257th `debug` directive in one program is an
    /// assembly error naming its file and line, not a wrapped or truncated
    /// index.
    #[test]
    fn test_a_257th_debug_directive_is_an_assembly_error() {
        let mut source = String::new();
        for _ in 0..256 {
            source.push_str("debug \"x\"\n");
        }
        source.push_str("debug \"overflow\"\n"); // line 257
        let mut asm = MMixAssembler::new(&source, "many.mms");
        let err = asm
            .parse()
            .expect_err("a 257th directive must not assemble");
        assert!(
            err.starts_with("many.mms:257:"),
            "must name the overflowing directive's file and line, got {err:?}"
        );
    }

    /// Two translation units contribute to one shared, program-order `K`
    /// space: the second unit's directive picks up where the first left off.
    #[test]
    fn test_debug_strings_span_translation_units_in_program_order() {
        let mut asm = MMixAssembler::new("debug \"first\"\n", "a.mms");
        asm.add_source("debug \"second\"\n", "b.mms");
        asm.parse().unwrap();
        assert_eq!(
            asm.debug_strings(),
            &[b"first".to_vec(), b"second".to_vec()]
        );
    }

    #[test]
    fn test_multibyte_byte_string_is_not_aligned() {
        // Alignment comes from the item's kind, not its size: a four-byte BYTE
        // list is still placed wherever the counter stands. Deriving alignment
        // from data_directive_size would put TEXT at 4.
        let mut asm = MMixAssembler::new("BYTE 1\nTEXT: BYTE \"abcd\"", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("TEXT"), Some(&1));
    }

    #[test]
    fn test_loc_sets_the_counter_exactly() {
        // LOC assigns the counter; it does not align. The next aligned item
        // rounds from wherever LOC left it.
        let mut asm = MMixAssembler::new("LOC #101\nHERE: BYTE 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("HERE"), Some(&0x101));
    }

    #[test]
    fn test_standalone_label_line_is_not_rounded() {
        // Rounding happens when an item is assembled, not when a label is
        // defined alone, so a bare label line names an address up to 7 bytes
        // below the octabyte that follows it.
        let mut asm = MMixAssembler::new("BYTE 1\nDATA\nOCTA #FF", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("DATA"), Some(&1));
        assert_eq!(
            asm.instructions
                .last()
                .map(|(addr, inst)| (*addr, inst.clone())),
            Some((8, MMixInstruction::OCTA(0xFF)))
        );
    }

    #[test]
    fn test_lda_rri_pass1_size_matches_pass2() {
        let mut asm = MMixAssembler::new("JMP LABEL\nLDA $1,$2,4\nLABEL: HALT", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::JMP(2));
    }

    #[test]
    fn test_branch_offset_beyond_i16_bytes_not_truncated() {
        // Byte delta between BZ (at addr 0) and LABEL (at addr 0x10000, i.e.
        // 65536 bytes forward) exceeds i16::MAX (32767), so casting the raw
        // byte delta to i16 before dividing by 4 would silently wrap.
        let source = "BZ $0,LABEL\nLOC #10000\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        let expected_offset = 0x10000i64 / 4;
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::BZ(0, expected_offset as u16)
        );
    }

    #[test]
    fn test_jmp_backward_emits_jmpb() {
        // A JMP whose target is BEHIND the current instruction must assemble
        // to JMPB, whose 24-bit field is 2^24 - magnitude.
        let source = "LOC #100\nBACK: HALT\nJMP BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        // JMP is at addr 0x104 (BACK's HALT is 4 bytes), target 0x100:
        // magnitude = (0x104 - 0x100) / 4 = 1, field = 0x1000000 - 1.
        assert_eq!(asm.instructions[1].1, MMixInstruction::JMPB(0xFFFFFF));
    }

    #[test]
    fn test_geta_offset_beyond_range_errors() {
        // Target is 65536 tetras forward, one past GETA's 0..=65535 reach.
        // Without the range check this source assembles to a wrapped field.
        let source = "GETA $0,LABEL\nLOC #40000\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("GETA"));
        assert!(err.contains("out of range"));
    }

    #[test]
    fn test_geta_misaligned_target_errors() {
        // A bare label line takes the counter unrounded, so LABEL names the
        // byte after the BYTE -- address 5, and 5 bytes forward of the GETA.
        let source = "GETA $0,LABEL\nBYTE 1\nLABEL\nOCTA 0";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("not 4-byte aligned"), "{err}");
    }

    #[test]
    fn test_geta_offset_within_range_succeeds() {
        // 65535 tetras forward is the last target GETA reaches.
        let source = "GETA $0,LABEL\nLOC #3FFFC\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        let MMixInstruction::GETA(_, y, z) = asm.instructions[0].1 else {
            panic!("expected GETA instruction");
        };
        let field = ((y as u16) << 8) | z as u16;
        assert_eq!(field, 65535);
    }

    #[test]
    fn test_getab_forward_target_errors() {
        // Target is FORWARD of the GETAB, which cannot be encoded at all
        // (GETAB is a backward-only unsigned magnitude). Under the unfixed
        // code this assembles successfully with a semantically inverted
        // encoding.
        let source = "GETAB $0,LABEL\nLOC #100\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        assert!(asm.parse().is_err());
    }

    #[test]
    fn test_getab_backward_target_encodes_knuth_field() {
        // GETAB sits at addr 0x104 (BACK's HALT is 4 bytes), target 0x100:
        // magnitude = (0x104 - 0x100) / 4 = 1, field = 65536 - 1 = 0xFFFF.
        let source = "LOC #100\nBACK: HALT\nGETAB $0,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::GETAB(0, 0xFF, 0xFF));
    }

    #[test]
    fn test_geta_forward_reach_extends_past_i16() {
        // 32769 tetras forward is inside GETA's 0..=65535 reach and outside
        // the i16 range the old check enforced.
        let source = "GETA $0,LABEL\nLOC #20004\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::GETA(0, 0x80, 0x01));
    }

    #[test]
    fn test_forward_mnemonic_at_backward_target_emits_backward_sibling() {
        // MMIXAL picks the opcode from the sign of the displacement, so a BNP
        // one tetra behind itself becomes BNPB with field 65536 - 1.
        let source = "LOC #100\nBACK: HALT\nBNP $1,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::BNPB(1, 0xFFFF));
    }

    #[test]
    fn test_pushj_at_backward_target_emits_pushjb() {
        let source = "LOC #100\nBACK: HALT\nPUSHJ $1,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions[1].1,
            MMixInstruction::PUSHJB(1, 0xFF, 0xFF)
        );
    }

    #[test]
    fn test_pbranch_at_backward_target_emits_knuth_field() {
        // parse_inst_pbranch is a separate function from parse_inst_branch and
        // needs its own coverage of both the auto-selection and the field.
        let source = "LOC #100\nBACK: HALT\nPBZ $1,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::PBZB(1, 0xFF, 0xFF));

        let explicit = "LOC #100\nBACK: HALT\nPBZB $1,BACK";
        let mut asm = MMixAssembler::new(explicit, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::PBZB(1, 0xFF, 0xFF));
    }

    #[test]
    fn test_backward_mnemonic_at_forward_target_names_forward_sibling() {
        let source = "BZB $1,LABEL\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("BZB"), "{err}");
        assert!(err.contains("use BZ instead"), "{err}");
    }

    #[test]
    fn test_zero_displacement_takes_the_forward_opcode() {
        // Forward reaches 0..=65535 tetras and backward 1..=65536, so a
        // branch to itself is forward with an empty field.
        let source = "HERE: BZ $1,HERE";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BZ(1, 0));
    }

    #[test]
    fn test_branch_backward_reaches_65536_tetras() {
        // A backward field of 0 means -65536 tetras, the far end of the reach.
        let source = "LOC #0\nBACK: HALT\nLOC #40000\nBZ $1,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::BZB(1, 0));
    }

    #[test]
    fn test_branch_beyond_reach_errors() {
        let forward = "BZ $1,LABEL\nLOC #40000\nLABEL: HALT";
        let mut asm = MMixAssembler::new(forward, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("out of range"), "{err}");

        let backward = "LOC #0\nBACK: HALT\nLOC #40004\nBZ $1,BACK";
        let mut asm = MMixAssembler::new(backward, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("out of range"), "{err}");
    }

    #[test]
    fn test_branch_misaligned_target_errors() {
        // LABEL is a bare label line on data: address 5, not a multiple of 4.
        let source = "BZ $1,LABEL\nBYTE 1\nLABEL\nOCTA 0";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("not 4-byte aligned"), "{err}");
    }

    #[test]
    fn test_jmp_misaligned_target_errors() {
        // LABEL is a bare label line on data: address 5, not a multiple of 4.
        let source = "JMP LABEL\nBYTE 1\nLABEL\nOCTA 0";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("not 4-byte aligned"), "{err}");
    }

    #[test]
    fn test_jmp_beyond_reach_errors() {
        // JMP's field is 24 bits: 0..=16777215 tetras forward, 1..=16777216
        // backward. Without the check the extra bits were masked away.
        let forward = "JMP LABEL\nLOC #4000000\nLABEL: HALT";
        let mut asm = MMixAssembler::new(forward, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("out of range"), "{err}");

        let backward = "LOC #0\nBACK: HALT\nLOC #4000004\nJMP BACK";
        let mut asm = MMixAssembler::new(backward, "<test>");
        let err = asm.parse().unwrap_err();
        assert!(err.contains("out of range"), "{err}");
    }

    #[test]
    fn test_incl_takes_a_16_bit_immediate() {
        // INCL adds YZ to $X, like its INCH/INCMH/INCML siblings.
        let mut asm = MMixAssembler::new("INCL $1,#203", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::INCL(1, 0x203));
    }

    #[test]
    fn test_parse_char_literal_immediate() {
        let mut asm = MMixAssembler::new("ANDI $1, $2, 'A'", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 65));
    }

    #[test]
    fn test_parse_char_literal_multi_char_error() {
        let mut asm = MMixAssembler::new("ANDI $1, $2, 'AB'", "<test>");
        assert!(asm.parse().is_err());
    }

    #[test]
    fn test_char_literal_two_characters_reports_expected_primary() {
        assert_eq!(
            assemble_err("Main\tAND\t$1,$2,'AB'\n\tTRAP\t0,Halt,0"),
            "<test>:1:16: syntax error: expected primary"
        );
    }

    // Bitwise operation tests
    #[test]
    fn test_parse_and() {
        let mut asm = MMixAssembler::new("AND $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::AND(1, 2, 3));
    }

    #[test]
    fn test_parse_andi() {
        let mut asm = MMixAssembler::new("ANDI $1, $2, #FF", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 0xFF));
    }

    #[test]
    fn test_parse_or() {
        let mut asm = MMixAssembler::new("OR $10, $20, $30", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::OR(10, 20, 30));
    }

    #[test]
    fn test_parse_xor() {
        let mut asm = MMixAssembler::new("XOR $5, $6, $7", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::XOR(5, 6, 7));
    }

    #[test]
    fn test_parse_andn() {
        let mut asm = MMixAssembler::new("ANDN $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ANDN(1, 2, 3));
    }

    #[test]
    fn test_parse_nand() {
        let mut asm = MMixAssembler::new("NAND $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::NAND(1, 2, 3));
    }

    #[test]
    fn test_parse_nor() {
        let mut asm = MMixAssembler::new("NOR $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::NOR(1, 2, 3));
    }

    #[test]
    fn test_parse_nxor() {
        let mut asm = MMixAssembler::new("NXOR $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::NXOR(1, 2, 3));
    }

    #[test]
    fn test_parse_mux() {
        let mut asm = MMixAssembler::new("MUX $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MUX(1, 2, 3));
    }

    // Bit fiddling operations tests
    #[test]
    fn test_parse_bdif() {
        let mut asm = MMixAssembler::new("BDIF $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BDIF(1, 2, 3));
    }

    #[test]
    fn test_parse_bdifi() {
        let mut asm = MMixAssembler::new("BDIFI $1, $2, #10", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BDIFI(1, 2, 0x10));
    }

    #[test]
    fn test_parse_wdif() {
        let mut asm = MMixAssembler::new("WDIF $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::WDIF(1, 2, 3));
    }

    #[test]
    fn test_parse_wdifi() {
        let mut asm = MMixAssembler::new("WDIFI $1, $2, 100", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::WDIFI(1, 2, 100));
    }

    #[test]
    fn test_parse_tdif() {
        let mut asm = MMixAssembler::new("TDIF $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::TDIF(1, 2, 3));
    }

    #[test]
    fn test_parse_tdifi() {
        let mut asm = MMixAssembler::new("TDIFI $1, $2, 50", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::TDIFI(1, 2, 50));
    }

    #[test]
    fn test_parse_odif() {
        let mut asm = MMixAssembler::new("ODIF $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ODIF(1, 2, 3));
    }

    #[test]
    fn test_parse_odifi() {
        let mut asm = MMixAssembler::new("ODIFI $1, $2, 255", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ODIFI(1, 2, 255));
    }

    #[test]
    fn test_parse_sadd() {
        let mut asm = MMixAssembler::new("SADD $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SADD(1, 2, 3));
    }

    #[test]
    fn test_parse_saddi() {
        let mut asm = MMixAssembler::new("SADDI $1, $2, 0", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SADDI(1, 2, 0));
    }

    #[test]
    fn test_parse_mor() {
        let mut asm = MMixAssembler::new("MOR $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MOR(1, 2, 3));
    }

    #[test]
    fn test_parse_mori() {
        let mut asm = MMixAssembler::new("MORI $1, $2, 128", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MORI(1, 2, 128));
    }

    #[test]
    fn test_parse_mxor() {
        let mut asm = MMixAssembler::new("MXOR $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MXOR(1, 2, 3));
    }

    #[test]
    fn test_parse_mxori() {
        let mut asm = MMixAssembler::new("MXORI $1, $2, 64", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MXORI(1, 2, 64));
    }

    // Shift instruction parsing tests
    #[test]
    fn test_parse_sl() {
        let mut asm = MMixAssembler::new("SL $3, $1, $2", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SL(3, 1, 2));
    }

    #[test]
    fn test_parse_sli() {
        let mut asm = MMixAssembler::new("SLI $3, $1, 8", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SLI(3, 1, 8));
    }

    #[test]
    fn test_parse_slu() {
        let mut asm = MMixAssembler::new("SLU $10, $20, $30", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SLU(10, 20, 30));
    }

    #[test]
    fn test_parse_slui() {
        let mut asm = MMixAssembler::new("SLUI $1, $2, 16", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SLUI(1, 2, 16));
    }

    #[test]
    fn test_parse_sr() {
        let mut asm = MMixAssembler::new("SR $5, $6, $7", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SR(5, 6, 7));
    }

    #[test]
    fn test_parse_sri() {
        let mut asm = MMixAssembler::new("SRI $3, $1, 4", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRI(3, 1, 4));
    }

    #[test]
    fn test_parse_sru() {
        let mut asm = MMixAssembler::new("SRU $8, $9, $10", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRU(8, 9, 10));
    }

    #[test]
    fn test_parse_srui() {
        let mut asm = MMixAssembler::new("SRUI $3, $1, 1", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRUI(3, 1, 1));
    }

    #[test]
    fn test_parse_fcmpe() {
        let mut asm = MMixAssembler::new("FCMPE $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::FCMPE(1, 2, 3));
    }

    #[test]
    fn test_parse_fune() {
        let mut asm = MMixAssembler::new("FUNE $4, $5, $6", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::FUNE(4, 5, 6));
    }

    #[test]
    fn test_parse_feqle() {
        let mut asm = MMixAssembler::new("FEQLE $7, $8, $9", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::FEQLE(7, 8, 9));
    }

    /// Verify the longest-first grammar still matches the shorter mnemonics.
    #[test]
    fn test_parse_fcmp_after_fcmpe_added() {
        let mut asm = MMixAssembler::new("FCMP $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::FCMP(1, 2, 3));
    }

    #[test]
    fn test_parse_feql_after_feqle_added() {
        let mut asm = MMixAssembler::new("FEQL $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::FEQL(1, 2, 3));
    }

    #[test]
    fn test_parse_fun_after_fune_added() {
        let mut asm = MMixAssembler::new("FUN $1, $2, $3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::FUN(1, 2, 3));
    }

    // ---- Multi-source assembly + global-':' symbol tests ----

    #[test]
    fn test_global_symbol_label_and_operand() {
        // `:Foo` parses both as a label definition and as an operand reference.
        let src = "\
            LOC #100\n\
            Main BNZ $1,:Foo\n\
            :Foo HALT\n";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        // The root prefix is `:`; `:Foo` and `Foo` name one symbol, keyed
        // without the colon.
        assert_eq!(asm.labels.get("Foo").copied(), Some(0x104));
        assert_eq!(asm.labels.get("Main").copied(), Some(0x100));
        assert!(!asm.labels.contains_key(":Foo"));
    }

    #[test]
    fn test_multi_source_main_calls_lib() {
        let main_src = "\
            LOC #100\n\
            Main PUSHJ $0,:Lib\n\
                 HALT\n";
        let lib_src = "\
            LOC #200\n\
            :Lib POP 0,0\n";
        let mut asm = MMixAssembler::new(main_src, "main.mms");
        asm.add_source(lib_src, "lib.mms");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Main").copied(), Some(0x100));
        assert_eq!(asm.labels.get("Lib").copied(), Some(0x200));

        // Two LOC regions both produced instructions.
        let addrs: Vec<u64> = asm.instructions.iter().map(|(a, _)| *a).collect();
        assert!(addrs.contains(&0x100));
        assert!(addrs.contains(&0x200));
    }

    #[test]
    fn test_multi_source_main_redefined() {
        let a = "\
            LOC #100\n\
            Main HALT\n";
        let b = "\
            LOC #200\n\
            Main HALT\n";
        let mut asm = MMixAssembler::new(a, "a.mms");
        asm.add_source(b, "b.mms");
        let err = asm.parse().expect_err("expected redefinition error");
        assert!(err.contains("'Main'"), "err: {}", err);
        assert!(err.contains("a.mms"), "err: {}", err);
        assert!(err.contains("b.mms"), "err: {}", err);
        assert!(err.contains("redefined"), "err: {}", err);
    }

    #[test]
    fn test_multi_source_global_symbol_redefined() {
        let a = "\
            LOC #100\n\
            :Foo HALT\n";
        let b = "\
            LOC #200\n\
            :Foo HALT\n";
        let mut asm = MMixAssembler::new(a, "a.mms");
        asm.add_source(b, "b.mms");
        let err = asm.parse().expect_err("expected redefinition error");
        assert!(err.contains("'Foo'"), "err: {}", err);
        assert!(err.contains("a.mms"), "err: {}", err);
        assert!(err.contains("b.mms"), "err: {}", err);
    }

    #[test]
    fn test_redefinition_across_label_and_is() {
        // A label and an IS-bound symbol with the same qualified name collide.
        let src = "\
            LOC #100\n\
            Foo HALT\n\
            Foo IS 5\n";
        let mut asm = MMixAssembler::new(src, "<test>");
        let err = asm.parse().expect_err("expected redefinition error");
        assert!(err.contains("'Foo'"), "err: {}", err);
        assert!(err.contains("redefined"), "err: {}", err);
    }

    // ---- PREFIX directive tests ----

    #[test]
    fn test_prefix_qualifies_unqualified_symbol() {
        let src = "\
            PREFIX Sub_\n\
            Bar IS 5\n";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.symbols.get("Sub_Bar").copied(),
            Some(SymbolType::Constant(5))
        );
        assert!(!asm.symbols.contains_key("Bar"));
    }

    #[test]
    fn test_prefix_colon_opts_out() {
        let src = "\
            PREFIX Sub_\n\
            :Foo IS 9\n";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        // A leading ':' opts out of the active PREFIX and, at the root, is
        // stored without the colon.
        assert_eq!(
            asm.symbols.get("Foo").copied(),
            Some(SymbolType::Constant(9))
        );
        assert!(!asm.symbols.contains_key(":Foo"));
        assert!(!asm.symbols.contains_key("Sub_:Foo"));
        assert!(!asm.symbols.contains_key("Sub_Foo"));
    }

    #[test]
    fn test_prefix_persists_across_files() {
        // PREFIX set in file A applies to definitions in file B.
        let a = "PREFIX P_\n";
        let b = "\
            LOC #100\n\
            Bar HALT\n";
        let mut asm = MMixAssembler::new(a, "a.mms");
        asm.add_source(b, "b.mms");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("P_Bar").copied(), Some(0x100));
        assert!(!asm.labels.contains_key("Bar"));
    }

    #[test]
    fn test_prefix_reset_to_global() {
        // `PREFIX :` makes unqualified names resolve under the global root.
        let src = "\
            PREFIX P_\n\
            Bar IS 1\n\
            PREFIX :\n\
            Baz IS 2\n";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.symbols.get("P_Bar").copied(),
            Some(SymbolType::Constant(1))
        );
        assert_eq!(
            asm.symbols.get("Baz").copied(),
            Some(SymbolType::Constant(2))
        );
    }

    // -----------------------------------------------------------------
    // Auto-immediate selection for base mnemonics (ADD, AND, SR, ...).
    // -----------------------------------------------------------------
    // The base mnemonic now accepts either a register or an in-range
    // immediate as its third operand and emits the corresponding RRR or
    // RRI MMixInstruction variant. The explicit *I mnemonics still work
    // as before through their original code path.

    #[test]
    fn test_auto_arith_register_form_unchanged() {
        // Regression: ADD with register Z still emits ADD, not ADDI.
        let mut asm = MMixAssembler::new("ADD $1,$2,$3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 3));
    }

    #[test]
    fn test_auto_arith_immediate_swap() {
        // ADD with a literal Z now selects the ADDI variant automatically.
        let mut asm = MMixAssembler::new("ADD $1,$2,5", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 5));
    }

    // -----------------------------------------------------------------
    // SET selects its variant from the operand's kind, and the base
    // load/store mnemonics select RRR/RRI the same way.
    // -----------------------------------------------------------------

    #[test]
    fn test_set_immediate_selects_setl() {
        let mut asm = MMixAssembler::new("SET $1,5", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_set_accepts_a_wyde_wide_immediate() {
        // Above the 8-bit arithmetic Z field, still inside SET's own wyde.
        let mut asm = MMixAssembler::new("SET $1,20000", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 20000));
    }

    #[test]
    fn test_set_rejects_an_immediate_above_a_wyde() {
        let mut asm = MMixAssembler::new("SET $1,#10000", "<test>");
        let err = asm.parse().expect_err("expected out-of-range error");
        assert!(
            err.contains("SETI"),
            "error should name SETI as the wide form, got: {err}"
        );
    }

    #[test]
    fn test_set_rejects_a_wide_symbol_operand() {
        let mut asm = MMixAssembler::new("C IS #12345\nSET $1,C", "<test>");
        let err = asm.parse().expect_err("expected out-of-range error");
        assert!(
            err.contains("SETI"),
            "error should name SETI as the wide form, got: {err}"
        );
    }

    #[test]
    fn test_seti_accepts_an_immediate_above_a_wyde() {
        let mut asm = MMixAssembler::new("SETI $1,#10000", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SET(1, 0x10000));
    }

    #[test]
    fn test_set_negative_literal_is_an_error() {
        assert_eq!(
            assemble_err("SET $1,-1"),
            "<test>:1:8: immediate operand -1 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
        );
    }

    #[test]
    fn test_set_symbol_register_alias_still_copies() {
        let mut asm = MMixAssembler::new("N IS $7\nSET $1,N", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETRR(1, 7));
    }

    #[test]
    fn test_set_is_one_tetra_and_seti_is_four() {
        let mut set = MMixAssembler::new("SET $1,5", "<test>");
        set.parse().unwrap();
        let mut seti = MMixAssembler::new("SETI $1,5", "<test>");
        seti.parse().unwrap();

        assert_eq!(
            set.encode_instruction_bytes(&set.instructions[0].1).len(),
            4
        );
        assert_eq!(
            seti.encode_instruction_bytes(&seti.instructions[0].1).len(),
            16
        );
    }

    #[test]
    fn test_auto_load_store_immediate_swap() {
        let mut base = MMixAssembler::new("LDO $1,$2,0\nSTO $1,$2,0", "<test>");
        base.parse().unwrap();
        let mut explicit = MMixAssembler::new("LDOI $1,$2,0\nSTOI $1,$2,0", "<test>");
        explicit.parse().unwrap();

        assert_eq!(base.instructions[0].1, explicit.instructions[0].1);
        assert_eq!(base.instructions[1].1, explicit.instructions[1].1);
        assert_eq!(base.instructions[0].1, MMixInstruction::LDOI(1, 2, 0));
        assert_eq!(base.instructions[1].1, MMixInstruction::STOI(1, 2, 0));
    }

    #[test]
    fn test_auto_load_store_register_form_unchanged() {
        let mut asm = MMixAssembler::new("LDO $1,$2,$3\nSTBU $1,$2,$3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDO(1, 2, 3));
        assert_eq!(asm.instructions[1].1, MMixInstruction::STBU(1, 2, 3));
    }

    #[test]
    fn test_auto_load_store_immediate_out_of_range() {
        // Z is an 8-bit field for loads and stores.
        let mut asm = MMixAssembler::new("LDO $1,$2,300", "<test>");
        let err = asm.parse().expect_err("expected out-of-range error");
        assert!(
            err.contains("out of range 0..255"),
            "error should mention range, got: {err}"
        );
    }

    #[test]
    fn test_auto_arith_explicit_addi_still_works() {
        // The *I alias path is unchanged.
        let mut asm = MMixAssembler::new("ADDI $1,$2,5", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 5));
    }

    #[test]
    fn test_auto_arith_addi_rejects_register_z() {
        // *I mnemonics must continue to reject a register third operand.
        let mut asm = MMixAssembler::new("ADDI $1,$2,$3", "<test>");
        assert!(asm.parse().is_err(), "ADDI with $Z should not parse");
    }

    #[test]
    fn test_auto_arith_immediate_out_of_range() {
        // A literal Z above 255 produces an out-of-range error.
        let mut asm = MMixAssembler::new("ADD $1,$2,300", "<test>");
        let err = asm.parse().expect_err("expected out-of-range error");
        assert!(
            err.contains("out of range 0..255"),
            "error should mention range, got: {err}"
        );
    }

    #[test]
    fn test_auto_arith_symbol_resolves_to_immediate() {
        // A symbol bound to a small constant should auto-select the RRI form.
        let src = "K IS 7\nADD $1,$2,K";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 7));
    }

    #[test]
    fn test_auto_arith_symbol_resolves_to_register_alias() {
        // A symbol bound to a register alias should keep the RRR form.
        let src = "R IS $4\nADD $1,$2,R";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 4));
    }

    // Family coverage: one representative test per other family.

    #[test]
    fn test_auto_bitwise_and_with_hex_immediate() {
        let mut asm = MMixAssembler::new("AND $1,$2,#FF", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 0xFF));
    }

    #[test]
    fn test_auto_shift_sr_with_decimal_immediate() {
        let mut asm = MMixAssembler::new("SR $1,$2,3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRI(1, 2, 3));
    }

    #[test]
    fn test_auto_bitfiddle_bdif_register_form() {
        // Bit-fiddle family still chooses RRR when Z is a register.
        let mut asm = MMixAssembler::new("BDIF $1,$2,$3", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BDIF(1, 2, 3));
    }

    #[test]
    fn test_auto_conditional_set_csz_with_immediate() {
        let mut asm = MMixAssembler::new("CSZ $1,$2,7", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::CSZI(1, 2, 7));
    }

    #[test]
    fn test_auto_zero_or_set_zsp_with_immediate() {
        let mut asm = MMixAssembler::new("ZSP $1,$2,1", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ZSPI(1, 2, 1));
    }

    // Regression: a program written with base mnemonics must assemble to
    // the exact same bytes as the same program written with explicit *I
    // mnemonics. Touches all six in-scope families.
    #[test]
    fn test_auto_immediate_byte_identical_to_explicit_i() {
        let auto_src = "\
ADD  $1,$2,5
SUBU $3,$4,#10
AND  $5,$6,#FF
OR   $7,$8,1
SR   $1,$2,3
SLU  $3,$4,16
BDIF $5,$6,7
SADD $7,$8,255
CSZ  $1,$2,42
CSNN $3,$4,1
ZSP  $5,$6,8
ZSEV $7,$8,128
";
        let explicit_src = "\
ADDI  $1,$2,5
SUBUI $3,$4,#10
ANDI  $5,$6,#FF
ORI   $7,$8,1
SRI   $1,$2,3
SLUI  $3,$4,16
BDIFI $5,$6,7
SADDI $7,$8,255
CSZI  $1,$2,42
CSNNI $3,$4,1
ZSPI  $5,$6,8
ZSEVI $7,$8,128
";

        let mut auto_asm = MMixAssembler::new(auto_src, "<auto>");
        auto_asm.parse().unwrap();
        let mut explicit_asm = MMixAssembler::new(explicit_src, "<explicit>");
        explicit_asm.parse().unwrap();

        assert_eq!(
            auto_asm.instructions.len(),
            explicit_asm.instructions.len(),
            "auto and explicit forms produced different instruction counts"
        );

        for (i, (auto, explicit)) in auto_asm
            .instructions
            .iter()
            .zip(explicit_asm.instructions.iter())
            .enumerate()
        {
            let auto_bytes = auto_asm.encode_instruction_bytes(&auto.1);
            let explicit_bytes = explicit_asm.encode_instruction_bytes(&explicit.1);
            assert_eq!(
                auto_bytes, explicit_bytes,
                "instruction {i}: auto form {:?} encoded to {:?}, explicit form {:?} encoded to {:?}",
                auto.1, auto_bytes, explicit.1, explicit_bytes
            );
        }
    }

    // -----------------------------------------------------------------
    // Extensive validation for the auto-immediate path.
    // -----------------------------------------------------------------
    // The auto rules introduce backtracking through prefix collisions
    // (e.g. AND vs ANDI, ADD vs ADDU, CSN vs CSNN). The tests below
    // pin down the routing for every base mnemonic in scope, exercise
    // boundary Z values, exercise symbol-Z resolution paths, and
    // confirm cross-family non-interference.

    /// Assemble a snippet whose first instruction is the one under test
    /// and assert it produced the expected enum variant. Snippets are
    /// kept to one logical instruction so failures point at the case.
    fn assert_first_instruction(src: &str, expected: MMixInstruction) {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
        assert!(
            !asm.instructions.is_empty(),
            "no instructions produced for {src:?}"
        );
        assert_eq!(
            asm.instructions[0].1, expected,
            "wrong instruction for {src:?}"
        );
    }

    /// Like `assert_first_instruction`, but for offset-bearing families
    /// (branch/PB-branch offsets, GETA/GETAB and PUSHJ/PUSHJB PC-relative
    /// addresses) where the exact computed value isn't what a prefix-
    /// collision test is proving. Checks only the variant discriminant
    /// (and any un-computed fields the predicate cares to check).
    /// Assert the LAST instruction of `src`, letting a case prefix its source
    /// with a label the instruction under test can reach backward.
    fn assert_last_instruction_matches(src: &str, predicate: impl Fn(&MMixInstruction) -> bool) {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
        let last = asm
            .instructions
            .last()
            .unwrap_or_else(|| panic!("no instructions produced for {src:?}"));
        assert!(
            predicate(&last.1),
            "wrong instruction variant for {src:?}: got {:?}",
            last.1
        );
    }

    fn assert_first_instruction_matches(src: &str, predicate: impl Fn(&MMixInstruction) -> bool) {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
        assert!(
            !asm.instructions.is_empty(),
            "no instructions produced for {src:?}"
        );
        assert!(
            predicate(&asm.instructions[0].1),
            "wrong instruction variant for {src:?}: got {:?}",
            asm.instructions[0].1
        );
    }

    // ---- Family-wide auto-immediate coverage ------------------------

    #[test]
    fn test_auto_arith_full_coverage() {
        // Every arithmetic base mnemonic + register form (RRR) and
        // immediate form (RRI). Z=5 for stability; mnemonic prefixes
        // (ADD/ADDU/2ADDU/...) must each route to their own variant.
        let cases: &[(&str, MMixInstruction)] = &[
            ("ADD $1,$2,$3", MMixInstruction::ADD(1, 2, 3)),
            ("ADD $1,$2,5", MMixInstruction::ADDI(1, 2, 5)),
            ("ADDU $1,$2,$3", MMixInstruction::ADDU(1, 2, 3)),
            ("ADDU $1,$2,5", MMixInstruction::ADDUI(1, 2, 5)),
            ("2ADDU $1,$2,$3", MMixInstruction::ADDU2(1, 2, 3)),
            ("2ADDU $1,$2,5", MMixInstruction::ADDU2I(1, 2, 5)),
            ("4ADDU $1,$2,$3", MMixInstruction::ADDU4(1, 2, 3)),
            ("4ADDU $1,$2,5", MMixInstruction::ADDU4I(1, 2, 5)),
            ("8ADDU $1,$2,$3", MMixInstruction::ADDU8(1, 2, 3)),
            ("8ADDU $1,$2,5", MMixInstruction::ADDU8I(1, 2, 5)),
            ("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3)),
            ("16ADDU $1,$2,5", MMixInstruction::ADDU16I(1, 2, 5)),
            ("SUB $1,$2,$3", MMixInstruction::SUB(1, 2, 3)),
            ("SUB $1,$2,5", MMixInstruction::SUBI(1, 2, 5)),
            ("SUBU $1,$2,$3", MMixInstruction::SUBU(1, 2, 3)),
            ("SUBU $1,$2,5", MMixInstruction::SUBUI(1, 2, 5)),
            ("MUL $1,$2,$3", MMixInstruction::MUL(1, 2, 3)),
            ("MUL $1,$2,5", MMixInstruction::MULI(1, 2, 5)),
            ("MULU $1,$2,$3", MMixInstruction::MULU(1, 2, 3)),
            ("MULU $1,$2,5", MMixInstruction::MULUI(1, 2, 5)),
            ("DIV $1,$2,$3", MMixInstruction::DIV(1, 2, 3)),
            ("DIV $1,$2,5", MMixInstruction::DIVI(1, 2, 5)),
            ("DIVU $1,$2,$3", MMixInstruction::DIVU(1, 2, 3)),
            ("DIVU $1,$2,5", MMixInstruction::DIVUI(1, 2, 5)),
            ("CMP $1,$2,$3", MMixInstruction::CMP(1, 2, 3)),
            ("CMP $1,$2,5", MMixInstruction::CMPI(1, 2, 5)),
            ("CMPU $1,$2,$3", MMixInstruction::CMPU(1, 2, 3)),
            ("CMPU $1,$2,5", MMixInstruction::CMPUI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_bitwise_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("AND $1,$2,$3", MMixInstruction::AND(1, 2, 3)),
            ("AND $1,$2,5", MMixInstruction::ANDI(1, 2, 5)),
            ("OR $1,$2,$3", MMixInstruction::OR(1, 2, 3)),
            ("OR $1,$2,5", MMixInstruction::ORI(1, 2, 5)),
            ("XOR $1,$2,$3", MMixInstruction::XOR(1, 2, 3)),
            ("XOR $1,$2,5", MMixInstruction::XORI(1, 2, 5)),
            ("ANDN $1,$2,$3", MMixInstruction::ANDN(1, 2, 3)),
            ("ANDN $1,$2,5", MMixInstruction::ANDNI(1, 2, 5)),
            ("ORN $1,$2,$3", MMixInstruction::ORN(1, 2, 3)),
            ("ORN $1,$2,5", MMixInstruction::ORNI(1, 2, 5)),
            ("NAND $1,$2,$3", MMixInstruction::NAND(1, 2, 3)),
            ("NAND $1,$2,5", MMixInstruction::NANDI(1, 2, 5)),
            ("NOR $1,$2,$3", MMixInstruction::NOR(1, 2, 3)),
            ("NOR $1,$2,5", MMixInstruction::NORI(1, 2, 5)),
            ("NXOR $1,$2,$3", MMixInstruction::NXOR(1, 2, 3)),
            ("NXOR $1,$2,5", MMixInstruction::NXORI(1, 2, 5)),
            ("MUX $1,$2,$3", MMixInstruction::MUX(1, 2, 3)),
            ("MUX $1,$2,5", MMixInstruction::MUXI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_bitfiddle_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("BDIF $1,$2,$3", MMixInstruction::BDIF(1, 2, 3)),
            ("BDIF $1,$2,5", MMixInstruction::BDIFI(1, 2, 5)),
            ("WDIF $1,$2,$3", MMixInstruction::WDIF(1, 2, 3)),
            ("WDIF $1,$2,5", MMixInstruction::WDIFI(1, 2, 5)),
            ("TDIF $1,$2,$3", MMixInstruction::TDIF(1, 2, 3)),
            ("TDIF $1,$2,5", MMixInstruction::TDIFI(1, 2, 5)),
            ("ODIF $1,$2,$3", MMixInstruction::ODIF(1, 2, 3)),
            ("ODIF $1,$2,5", MMixInstruction::ODIFI(1, 2, 5)),
            ("SADD $1,$2,$3", MMixInstruction::SADD(1, 2, 3)),
            ("SADD $1,$2,5", MMixInstruction::SADDI(1, 2, 5)),
            ("MOR $1,$2,$3", MMixInstruction::MOR(1, 2, 3)),
            ("MOR $1,$2,5", MMixInstruction::MORI(1, 2, 5)),
            ("MXOR $1,$2,$3", MMixInstruction::MXOR(1, 2, 3)),
            ("MXOR $1,$2,5", MMixInstruction::MXORI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_shift_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("SL $1,$2,$3", MMixInstruction::SL(1, 2, 3)),
            ("SL $1,$2,5", MMixInstruction::SLI(1, 2, 5)),
            ("SLU $1,$2,$3", MMixInstruction::SLU(1, 2, 3)),
            ("SLU $1,$2,5", MMixInstruction::SLUI(1, 2, 5)),
            ("SR $1,$2,$3", MMixInstruction::SR(1, 2, 3)),
            ("SR $1,$2,5", MMixInstruction::SRI(1, 2, 5)),
            ("SRU $1,$2,$3", MMixInstruction::SRU(1, 2, 3)),
            ("SRU $1,$2,5", MMixInstruction::SRUI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_conditional_set_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("CSN $1,$2,$3", MMixInstruction::CSN(1, 2, 3)),
            ("CSN $1,$2,5", MMixInstruction::CSNI(1, 2, 5)),
            ("CSZ $1,$2,$3", MMixInstruction::CSZ(1, 2, 3)),
            ("CSZ $1,$2,5", MMixInstruction::CSZI(1, 2, 5)),
            ("CSP $1,$2,$3", MMixInstruction::CSP(1, 2, 3)),
            ("CSP $1,$2,5", MMixInstruction::CSPI(1, 2, 5)),
            ("CSOD $1,$2,$3", MMixInstruction::CSOD(1, 2, 3)),
            ("CSOD $1,$2,5", MMixInstruction::CSODI(1, 2, 5)),
            ("CSNN $1,$2,$3", MMixInstruction::CSNN(1, 2, 3)),
            ("CSNN $1,$2,5", MMixInstruction::CSNNI(1, 2, 5)),
            ("CSNZ $1,$2,$3", MMixInstruction::CSNZ(1, 2, 3)),
            ("CSNZ $1,$2,5", MMixInstruction::CSNZI(1, 2, 5)),
            ("CSNP $1,$2,$3", MMixInstruction::CSNP(1, 2, 3)),
            ("CSNP $1,$2,5", MMixInstruction::CSNPI(1, 2, 5)),
            ("CSEV $1,$2,$3", MMixInstruction::CSEV(1, 2, 3)),
            ("CSEV $1,$2,5", MMixInstruction::CSEVI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_zero_or_set_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("ZSN $1,$2,$3", MMixInstruction::ZSN(1, 2, 3)),
            ("ZSN $1,$2,5", MMixInstruction::ZSNI(1, 2, 5)),
            ("ZSZ $1,$2,$3", MMixInstruction::ZSZ(1, 2, 3)),
            ("ZSZ $1,$2,5", MMixInstruction::ZSZI(1, 2, 5)),
            ("ZSP $1,$2,$3", MMixInstruction::ZSP(1, 2, 3)),
            ("ZSP $1,$2,5", MMixInstruction::ZSPI(1, 2, 5)),
            ("ZSOD $1,$2,$3", MMixInstruction::ZSOD(1, 2, 3)),
            ("ZSOD $1,$2,5", MMixInstruction::ZSODI(1, 2, 5)),
            ("ZSNN $1,$2,$3", MMixInstruction::ZSNN(1, 2, 3)),
            ("ZSNN $1,$2,5", MMixInstruction::ZSNNI(1, 2, 5)),
            ("ZSNZ $1,$2,$3", MMixInstruction::ZSNZ(1, 2, 3)),
            ("ZSNZ $1,$2,5", MMixInstruction::ZSNZI(1, 2, 5)),
            ("ZSNP $1,$2,$3", MMixInstruction::ZSNP(1, 2, 3)),
            ("ZSNP $1,$2,5", MMixInstruction::ZSNPI(1, 2, 5)),
            ("ZSEV $1,$2,$3", MMixInstruction::ZSEV(1, 2, 3)),
            ("ZSEV $1,$2,5", MMixInstruction::ZSEVI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    // ---- Canonical spellings for the remaining families -------------
    // MMIXAL picks the immediate opcode from the operand, so a base
    // mnemonic with an immediate Z emits what its *I spelling emits. The
    // *I spellings stay accepted as a legacy surface.

    fn first_instruction(src: &str) -> MMixInstruction {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
        asm.instructions
            .first()
            .unwrap_or_else(|| panic!("no instructions produced for {src:?}"))
            .1
            .clone()
    }

    #[test]
    fn test_auto_extended_load_store_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("LDUNC $1,$2,$3", MMixInstruction::LDUNC(1, 2, 3)),
            ("LDUNC $1,$2,5", MMixInstruction::LDUNCI(1, 2, 5)),
            ("STUNC $1,$2,$3", MMixInstruction::STUNC(1, 2, 3)),
            ("STUNC $1,$2,5", MMixInstruction::STUNCI(1, 2, 5)),
            ("LDHT $1,$2,$3", MMixInstruction::LDHT(1, 2, 3)),
            ("LDHT $1,$2,5", MMixInstruction::LDHTI(1, 2, 5)),
            ("STHT $1,$2,$3", MMixInstruction::STHT(1, 2, 3)),
            ("STHT $1,$2,5", MMixInstruction::STHTI(1, 2, 5)),
            ("LDSF $1,$2,$3", MMixInstruction::LDSF(1, 2, 3)),
            ("LDSF $1,$2,5", MMixInstruction::LDSFI(1, 2, 5)),
            ("STSF $1,$2,$3", MMixInstruction::STSF(1, 2, 3)),
            ("STSF $1,$2,5", MMixInstruction::STSFI(1, 2, 5)),
            ("LDVTS $1,$2,$3", MMixInstruction::LDVTS(1, 2, 3)),
            ("LDVTS $1,$2,5", MMixInstruction::LDVTSI(1, 2, 5)),
            ("CSWAP $1,$2,$3", MMixInstruction::CSWAP(1, 2, 3)),
            ("CSWAP $1,$2,5", MMixInstruction::CSWAPI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_cache_and_go_full_coverage() {
        let cases: &[(&str, MMixInstruction)] = &[
            ("PRELD $1,$2,$3", MMixInstruction::PRELD(1, 2, 3)),
            ("PRELD $1,$2,5", MMixInstruction::PRELDI(1, 2, 5)),
            ("PREGO $1,$2,$3", MMixInstruction::PREGO(1, 2, 3)),
            ("PREGO $1,$2,5", MMixInstruction::PREGOI(1, 2, 5)),
            ("PREST $1,$2,$3", MMixInstruction::PREST(1, 2, 3)),
            ("PREST $1,$2,5", MMixInstruction::PRESTI(1, 2, 5)),
            ("SYNCD $1,$2,$3", MMixInstruction::SYNCD(1, 2, 3)),
            ("SYNCD $1,$2,5", MMixInstruction::SYNCDI(1, 2, 5)),
            ("SYNCID $1,$2,$3", MMixInstruction::SYNCID(1, 2, 3)),
            ("SYNCID $1,$2,5", MMixInstruction::SYNCIDI(1, 2, 5)),
            ("GO $1,$2,$3", MMixInstruction::GO(1, 2, 3)),
            ("GO $1,$2,5", MMixInstruction::GOI(1, 2, 5)),
            ("PUSHGO $1,$2,$3", MMixInstruction::PUSHGO(1, 2, 3)),
            ("PUSHGO $1,$2,5", MMixInstruction::PUSHGOI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_float_conversion_full_coverage() {
        // Y is a rounding-mode value, not a register; 0 here exercises
        // Z's register/immediate auto-select, this test's point.
        let cases: &[(&str, MMixInstruction)] = &[
            ("FLOT $1,0,$3", MMixInstruction::FLOT(1, 0, 3)),
            ("FLOT $1,0,5", MMixInstruction::FLOTI(1, 0, 5)),
            ("FLOTU $1,0,$3", MMixInstruction::FLOTU(1, 0, 3)),
            ("FLOTU $1,0,5", MMixInstruction::FLOTUI(1, 0, 5)),
            ("SFLOT $1,0,$3", MMixInstruction::SFLOT(1, 0, 3)),
            ("SFLOT $1,0,5", MMixInstruction::SFLOTI(1, 0, 5)),
            ("SFLOTU $1,0,$3", MMixInstruction::SFLOTU(1, 0, 3)),
            ("SFLOTU $1,0,5", MMixInstruction::SFLOTUI(1, 0, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_auto_irregular_operand_shapes_full_coverage() {
        // STCO's X and NEG's Y stay immediate bytes; only Z auto-selects.
        let cases: &[(&str, MMixInstruction)] = &[
            ("STCO 5,$2,$3", MMixInstruction::STCO(5, 2, 3)),
            ("STCO 5,$2,7", MMixInstruction::STCOI(5, 2, 7)),
            ("NEG $1,0,$3", MMixInstruction::NEG(1, 0, 3)),
            ("NEG $1,0,7", MMixInstruction::NEGI(1, 0, 7)),
            ("NEGU $1,0,$3", MMixInstruction::NEGU(1, 0, 3)),
            ("NEGU $1,0,7", MMixInstruction::NEGUI(1, 0, 7)),
            ("PUT rA,$1", MMixInstruction::PUT(21, 1)),
            ("PUT rA,7", MMixInstruction::PUTI(21, 7)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_base_spelling_agrees_with_legacy_immediate_spelling() {
        let pairs: &[(&str, &str)] = &[
            ("LDHT $1,$2,5", "LDHTI $1,$2,5"),
            ("LDSF $1,$2,5", "LDSFI $1,$2,5"),
            ("LDUNC $1,$2,5", "LDUNCI $1,$2,5"),
            ("LDVTS $1,$2,5", "LDVTSI $1,$2,5"),
            ("STHT $1,$2,5", "STHTI $1,$2,5"),
            ("STSF $1,$2,5", "STSFI $1,$2,5"),
            ("STUNC $1,$2,5", "STUNCI $1,$2,5"),
            ("CSWAP $1,$2,5", "CSWAPI $1,$2,5"),
            ("PREGO $1,$2,5", "PREGOI $1,$2,5"),
            ("PRELD $1,$2,5", "PRELDI $1,$2,5"),
            ("PREST $1,$2,5", "PRESTI $1,$2,5"),
            ("SYNCD $1,$2,5", "SYNCDI $1,$2,5"),
            ("SYNCID $1,$2,5", "SYNCIDI $1,$2,5"),
            ("GO $1,$2,5", "GOI $1,$2,5"),
            ("PUSHGO $1,$2,5", "PUSHGOI $1,$2,5"),
            ("FLOT $1,0,5", "FLOTI $1,0,5"),
            ("FLOTU $1,0,5", "FLOTUI $1,0,5"),
            ("SFLOT $1,0,5", "SFLOTI $1,0,5"),
            ("SFLOTU $1,0,5", "SFLOTUI $1,0,5"),
            ("STCO 5,$2,7", "STCOI 5,$2,7"),
            ("NEG $1,0,7", "NEGI $1,0,7"),
            ("NEGU $1,0,7", "NEGUI $1,0,7"),
            ("PUT rA,7", "PUTI rA,7"),
        ];
        for (base, legacy) in pairs {
            assert_eq!(
                first_instruction(base),
                first_instruction(legacy),
                "{base:?} must emit what {legacy:?} emits"
            );
        }
    }

    #[test]
    fn test_no_previously_accepted_operand_form_narrowed() {
        // Every spelling these families accepted before their base
        // mnemonics auto-selected. Widening must displace none of them.
        let forms: &[&str] = &[
            "LDHT $1,$2,$3",
            "LDHTI $1,$2,5",
            "LDSF $1,$2,$3",
            "LDSFI $1,$2,5",
            "LDUNC $1,$2,$3",
            "LDUNCI $1,$2,5",
            "LDVTS $1,$2,$3",
            "LDVTSI $1,$2,5",
            "STHT $1,$2,$3",
            "STHTI $1,$2,5",
            "STSF $1,$2,$3",
            "STSFI $1,$2,5",
            "STUNC $1,$2,$3",
            "STUNCI $1,$2,5",
            "CSWAP $1,$2,$3",
            "CSWAPI $1,$2,5",
            "PREGO $1,$2,$3",
            "PREGOI $1,$2,5",
            "PRELD $1,$2,$3",
            "PRELDI $1,$2,5",
            "PREST $1,$2,$3",
            "PRESTI $1,$2,5",
            "SYNCD $1,$2,$3",
            "SYNCDI $1,$2,5",
            "SYNCID $1,$2,$3",
            "SYNCIDI $1,$2,5",
            "GO $1,$2,$3",
            "GOI $1,$2,5",
            "PUSHGO $1,$2,$3",
            "PUSHGOI $1,$2,5",
            "FLOT $1,0,$3",
            "FLOTI $1,0,5",
            "FLOTU $1,0,$3",
            "FLOTUI $1,0,5",
            "SFLOT $1,0,$3",
            "SFLOTI $1,0,5",
            "SFLOTU $1,0,$3",
            "SFLOTUI $1,0,5",
            "STCO 5,$2,$3",
            "STCOI 5,$2,7",
            "NEG $1,0,$3",
            "NEGI $1,0,7",
            "NEGU $1,0,$3",
            "NEGUI $1,0,7",
            "PUT rA,$1",
            "PUTI rA,7",
            "LDA $1,$2,3",
            "LDAI $1,$2,3",
        ];
        for src in forms {
            let mut asm = MMixAssembler::new(src, "<test>");
            asm.parse()
                .unwrap_or_else(|e| panic!("{src:?} no longer assembles: {e}"));
        }
    }

    #[test]
    fn test_flot_rejects_register_in_y_slot() {
        // Y is a rounding-mode value, never a register: every operand is an
        // `expr` now, so `$2` parses fine there, and the evaluator is what
        // rejects it -- a register where FLOT's Y demands a pure value.
        let mut asm = MMixAssembler::new("FLOT $1,$2,$3", "<test>");
        let err = asm
            .parse()
            .expect_err("a register in FLOT's Y slot must still be rejected");
        assert_eq!(
            err,
            "<test>:1:9: register $2 cannot be used where a pure value is required"
        );
    }

    #[test]
    fn test_round_mode_symbols_resolve_to_documented_values() {
        // The predefined-symbol table (MMIXAL reference), independent of
        // rA's own persistent-mode numbering.
        let asm = MMixAssembler::new("", "<test>");
        for (name, value) in [
            ("ROUND_CURRENT", 0u64),
            ("ROUND_OFF", 1),
            ("ROUND_UP", 2),
            ("ROUND_DOWN", 3),
        ] {
            assert_eq!(
                asm.symbols.get(name).copied(),
                Some(SymbolType::Constant(value)),
                "{name} must resolve to {value}"
            );
        }
    }

    /// The reference's eleven TRAP codes, checksmix's three extensions, the
    /// five `Fopen` modes, and the three standard handles, all at the
    /// values the ABI table names. Reverting `TrapCode`'s numbering turns
    /// one of these red.
    #[test]
    fn test_predefined_trap_symbols_match_the_reference_abi() {
        let asm = MMixAssembler::new("", "<test>");
        for (name, value) in [
            ("Halt", 0u64),
            ("Fopen", 1),
            ("Fclose", 2),
            ("Fread", 3),
            ("Fgets", 4),
            ("Fgetws", 5),
            ("Fwrite", 6),
            ("Fputs", 7),
            ("Fputws", 8),
            ("Fseek", 9),
            ("Ftell", 10),
            ("Fputc", 0x80),
            ("Time", 0x81),
            ("Debug", 0x82),
            ("TextRead", 0),
            ("TextWrite", 1),
            ("BinaryRead", 2),
            ("BinaryWrite", 3),
            ("BinaryReadWrite", 4),
            ("StdIn", 0),
            ("StdOut", 1),
            ("StdErr", 2),
        ] {
            assert_eq!(
                asm.symbols.get(name).copied(),
                Some(SymbolType::Constant(value)),
                "{name} must resolve to {value}"
            );
        }
        assert!(
            !asm.symbols.contains_key("Trip"),
            "Trip is not a symbol: the TRIP instruction does user trips now"
        );
    }

    #[test]
    fn test_lda_selects_addus_two_opcodes() {
        assert_first_instruction("LDA $1,$2,$3", MMixInstruction::LDA(1, 2, 3));
        assert_first_instruction("LDA $1,$2,3", MMixInstruction::LDAI(1, 2, 3));
        assert_first_instruction("LDAI $1,$2,3", MMixInstruction::LDAI(1, 2, 3));
    }

    #[test]
    fn test_lda_emits_the_same_bytes_as_addu() {
        // LDA carries no opcode of its own: it is ADDU under another name,
        // in both the register and the immediate operand form.
        let asm = MMixAssembler::new("", "<test>");
        for (lda, addu) in [
            ("LDA $1,$2,$3", "ADDU $1,$2,$3"),
            ("LDA $1,$2,3", "ADDU $1,$2,3"),
        ] {
            assert_eq!(
                asm.encode_instruction_bytes(&first_instruction(lda)),
                asm.encode_instruction_bytes(&first_instruction(addu)),
                "{lda:?} must emit the same bytes as {addu:?}"
            );
        }
    }

    #[test]
    fn test_stco_accepts_a_register_x_operand() {
        // The reference warns on a register X but assembles its number;
        // refusing it here would narrow an accepted form.
        assert_first_instruction("STCO $1,$2,$3", MMixInstruction::STCO(1, 2, 3));
    }

    #[test]
    fn test_jmpb_backward_target_encodes_knuth_field() {
        // JMPB sits at 0x104 (BACK's HALT is 4 bytes), target 0x100:
        // magnitude = 1 tetra, field = 2^24 - 1.
        let source = "LOC #100\nBACK: HALT\nJMPB BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::JMPB(0xFFFFFF));
    }

    #[test]
    fn test_jmpb_forward_target_errors() {
        let source = "JMPB LABEL\nLOC #100\nLABEL: HALT";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().expect_err("JMPB encodes only backward targets");
        assert!(
            err.contains("use JMP instead"),
            "error should name JMP as the forward form, got: {err}"
        );
    }

    // ---- Mnemonic prefix-collision tests ----------------------------
    // These pin down PEG backtracking for prefix-overlapping mnemonics
    // (ADD vs ADDU vs ADDUI; AND vs ANDN vs ANDNI; CSN vs CSNN; etc.).

    #[test]
    fn test_prefix_robust_arith_signed_vs_unsigned() {
        // ADD must not steal "ADDU"; ADDU must not steal "ADDUI" (the
        // *I path must still match through inst_arith_rri).
        assert_first_instruction("ADD $1,$2,$3", MMixInstruction::ADD(1, 2, 3));
        assert_first_instruction("ADDU $1,$2,$3", MMixInstruction::ADDU(1, 2, 3));
        assert_first_instruction("ADDI $1,$2,7", MMixInstruction::ADDI(1, 2, 7));
        assert_first_instruction("ADDUI $1,$2,7", MMixInstruction::ADDUI(1, 2, 7));
        assert_first_instruction("ADD $1,$2,7", MMixInstruction::ADDI(1, 2, 7));
        assert_first_instruction("ADDU $1,$2,7", MMixInstruction::ADDUI(1, 2, 7));
    }

    #[test]
    fn test_prefix_robust_numeric_arith() {
        // 2ADDU through 16ADDU each must claim their own mnemonic and
        // their *I variant must still route via the rri path.
        assert_first_instruction("2ADDU $1,$2,$3", MMixInstruction::ADDU2(1, 2, 3));
        assert_first_instruction("4ADDU $1,$2,$3", MMixInstruction::ADDU4(1, 2, 3));
        assert_first_instruction("8ADDU $1,$2,$3", MMixInstruction::ADDU8(1, 2, 3));
        assert_first_instruction("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3));
        assert_first_instruction("2ADDU $1,$2,5", MMixInstruction::ADDU2I(1, 2, 5));
        assert_first_instruction("4ADDU $1,$2,5", MMixInstruction::ADDU4I(1, 2, 5));
        assert_first_instruction("8ADDU $1,$2,5", MMixInstruction::ADDU8I(1, 2, 5));
        assert_first_instruction("16ADDU $1,$2,5", MMixInstruction::ADDU16I(1, 2, 5));
        assert_first_instruction("2ADDUI $1,$2,5", MMixInstruction::ADDU2I(1, 2, 5));
        assert_first_instruction("4ADDUI $1,$2,5", MMixInstruction::ADDU4I(1, 2, 5));
        assert_first_instruction("8ADDUI $1,$2,5", MMixInstruction::ADDU8I(1, 2, 5));
        assert_first_instruction("16ADDUI $1,$2,5", MMixInstruction::ADDU16I(1, 2, 5));
    }

    #[test]
    fn test_prefix_robust_bitwise_and_andn() {
        // AND vs ANDN vs ANDI vs ANDNI: each must route to its own
        // variant. Particularly important because mnemonic_and matches
        // the "AND" prefix of all four; backtracking has to recover.
        assert_first_instruction("AND $1,$2,$3", MMixInstruction::AND(1, 2, 3));
        assert_first_instruction("ANDN $1,$2,$3", MMixInstruction::ANDN(1, 2, 3));
        assert_first_instruction("AND $1,$2,7", MMixInstruction::ANDI(1, 2, 7));
        assert_first_instruction("ANDN $1,$2,7", MMixInstruction::ANDNI(1, 2, 7));
        assert_first_instruction("ANDI $1,$2,7", MMixInstruction::ANDI(1, 2, 7));
        assert_first_instruction("ANDNI $1,$2,7", MMixInstruction::ANDNI(1, 2, 7));
        // ANDN also collides with the wyde-field ANDNH/ANDNMH/ANDNML/ANDNL
        // family (2-operand reg,imm shape, distinct from the 3-operand
        // AND/ANDN/ANDNI forms above).
        assert_first_instruction("ANDNH $1,5", MMixInstruction::ANDNH(1, 5));
        assert_first_instruction("ANDNMH $1,5", MMixInstruction::ANDNMH(1, 5));
        assert_first_instruction("ANDNML $1,5", MMixInstruction::ANDNML(1, 5));
        assert_first_instruction("ANDNL $1,5", MMixInstruction::ANDNL(1, 5));
    }

    #[test]
    fn test_prefix_robust_bitwise_or_orn() {
        assert_first_instruction("OR $1,$2,$3", MMixInstruction::OR(1, 2, 3));
        assert_first_instruction("ORN $1,$2,$3", MMixInstruction::ORN(1, 2, 3));
        assert_first_instruction("OR $1,$2,7", MMixInstruction::ORI(1, 2, 7));
        assert_first_instruction("ORN $1,$2,7", MMixInstruction::ORNI(1, 2, 7));
        assert_first_instruction("ORI $1,$2,7", MMixInstruction::ORI(1, 2, 7));
        assert_first_instruction("ORNI $1,$2,7", MMixInstruction::ORNI(1, 2, 7));
        // OR also collides with the wyde-field ORH/ORMH/ORML/ORL family
        // (2-operand reg,imm shape, distinct from the 3-operand OR/ORN/ORNI
        // forms above).
        assert_first_instruction("ORH $1,5", MMixInstruction::ORH(1, 5));
        assert_first_instruction("ORMH $1,5", MMixInstruction::ORMH(1, 5));
        assert_first_instruction("ORML $1,5", MMixInstruction::ORML(1, 5));
        assert_first_instruction("ORL $1,5", MMixInstruction::ORL(1, 5));
    }

    #[test]
    fn test_prefix_robust_set_family() {
        // SET must not steal SETI's longer literal, and SETI/SETL/SETH/
        // SETMH/SETML — five mnemonics sharing the "SET" prefix — must each
        // route to their own variant.
        assert_first_instruction("SET $1,$2", MMixInstruction::SETRR(1, 2));
        assert_first_instruction("SETI $1,5", MMixInstruction::SET(1, 5));
        assert_first_instruction("SETL $1,5", MMixInstruction::SETL(1, 5));
        assert_first_instruction("SETH $1,5", MMixInstruction::SETH(1, 5));
        assert_first_instruction("SETMH $1,5", MMixInstruction::SETMH(1, 5));
        assert_first_instruction("SETML $1,5", MMixInstruction::SETML(1, 5));
    }

    #[test]
    fn test_prefix_robust_neg() {
        // NEG/NEGU/NEGI/NEGUI: NEG must not steal NEGU's, NEGI's, or
        // NEGUI's longer literal.
        assert_first_instruction("NEG $1,5,$3", MMixInstruction::NEG(1, 5, 3));
        assert_first_instruction("NEGU $1,5,$3", MMixInstruction::NEGU(1, 5, 3));
        assert_first_instruction("NEGI $1,5,7", MMixInstruction::NEGI(1, 5, 7));
        assert_first_instruction("NEGUI $1,5,7", MMixInstruction::NEGUI(1, 5, 7));
    }

    #[test]
    fn test_prefix_robust_float_fix_flot() {
        // FIX/FIXU and FLOT/FLOTI/FLOTU/FLOTUI and SFLOT/SFLOTI/SFLOTU/
        // SFLOTUI: FIX must not steal FIXU's literal, FLOT must not steal
        // FLOTU's/FLOTI's/FLOTUI's, and likewise for SFLOT. Y is a
        // rounding-mode value, not a register; 0 here is orthogonal to
        // what this test exercises.
        assert_first_instruction("FIX $1,0,$3", MMixInstruction::FIX(1, 0, 3));
        assert_first_instruction("FIXU $1,0,$3", MMixInstruction::FIXU(1, 0, 3));
        assert_first_instruction("FLOT $1,0,$3", MMixInstruction::FLOT(1, 0, 3));
        assert_first_instruction("FLOTU $1,0,$3", MMixInstruction::FLOTU(1, 0, 3));
        assert_first_instruction("FLOTI $1,0,5", MMixInstruction::FLOTI(1, 0, 5));
        assert_first_instruction("FLOTUI $1,0,5", MMixInstruction::FLOTUI(1, 0, 5));
        assert_first_instruction("SFLOT $1,0,$3", MMixInstruction::SFLOT(1, 0, 3));
        assert_first_instruction("SFLOTU $1,0,$3", MMixInstruction::SFLOTU(1, 0, 3));
        assert_first_instruction("SFLOTI $1,0,5", MMixInstruction::SFLOTI(1, 0, 5));
        assert_first_instruction("SFLOTUI $1,0,5", MMixInstruction::SFLOTUI(1, 0, 5));
    }

    #[test]
    fn test_prefix_robust_load_store_families() {
        // LDB/LDW/LDT/LDO and STB/STW/STT/STO each have a U sibling (LDBU,
        // ...) and an I sibling (LDBI, ...) and a UI sibling (LDBUI, ...);
        // the base mnemonic must not steal any of them.
        let cases: &[(&str, MMixInstruction)] = &[
            ("LDB $1,$2,$3", MMixInstruction::LDB(1, 2, 3)),
            ("LDBU $1,$2,$3", MMixInstruction::LDBU(1, 2, 3)),
            ("LDBI $1,$2,5", MMixInstruction::LDBI(1, 2, 5)),
            ("LDBUI $1,$2,5", MMixInstruction::LDBUI(1, 2, 5)),
            ("LDW $1,$2,$3", MMixInstruction::LDW(1, 2, 3)),
            ("LDWU $1,$2,$3", MMixInstruction::LDWU(1, 2, 3)),
            ("LDWI $1,$2,5", MMixInstruction::LDWI(1, 2, 5)),
            ("LDWUI $1,$2,5", MMixInstruction::LDWUI(1, 2, 5)),
            ("LDT $1,$2,$3", MMixInstruction::LDT(1, 2, 3)),
            ("LDTU $1,$2,$3", MMixInstruction::LDTU(1, 2, 3)),
            ("LDTI $1,$2,5", MMixInstruction::LDTI(1, 2, 5)),
            ("LDTUI $1,$2,5", MMixInstruction::LDTUI(1, 2, 5)),
            ("LDO $1,$2,$3", MMixInstruction::LDO(1, 2, 3)),
            ("LDOU $1,$2,$3", MMixInstruction::LDOU(1, 2, 3)),
            ("LDOI $1,$2,5", MMixInstruction::LDOI(1, 2, 5)),
            ("LDOUI $1,$2,5", MMixInstruction::LDOUI(1, 2, 5)),
            ("STB $1,$2,$3", MMixInstruction::STB(1, 2, 3)),
            ("STBU $1,$2,$3", MMixInstruction::STBU(1, 2, 3)),
            ("STBI $1,$2,5", MMixInstruction::STBI(1, 2, 5)),
            ("STBUI $1,$2,5", MMixInstruction::STBUI(1, 2, 5)),
            ("STW $1,$2,$3", MMixInstruction::STW(1, 2, 3)),
            ("STWU $1,$2,$3", MMixInstruction::STWU(1, 2, 3)),
            ("STWI $1,$2,5", MMixInstruction::STWI(1, 2, 5)),
            ("STWUI $1,$2,5", MMixInstruction::STWUI(1, 2, 5)),
            ("STT $1,$2,$3", MMixInstruction::STT(1, 2, 3)),
            ("STTU $1,$2,$3", MMixInstruction::STTU(1, 2, 3)),
            ("STTI $1,$2,5", MMixInstruction::STTI(1, 2, 5)),
            ("STTUI $1,$2,5", MMixInstruction::STTUI(1, 2, 5)),
            ("STO $1,$2,$3", MMixInstruction::STO(1, 2, 3)),
            ("STOU $1,$2,$3", MMixInstruction::STOU(1, 2, 3)),
            ("STOI $1,$2,5", MMixInstruction::STOI(1, 2, 5)),
            ("STOUI $1,$2,5", MMixInstruction::STOUI(1, 2, 5)),
        ];
        for (src, expected) in cases {
            assert_first_instruction(src, expected.clone());
        }
    }

    #[test]
    fn test_prefix_robust_lda() {
        // LDA must not steal LDAI's longer literal. Both spellings select
        // the immediate opcode here; only LDA's register form selects 0x22.
        assert_first_instruction("LDA $1,$2,$3", MMixInstruction::LDA(1, 2, 3));
        assert_first_instruction("LDA $1,$2,5", MMixInstruction::LDAI(1, 2, 5));
        assert_first_instruction("LDAI $1,$2,5", MMixInstruction::LDAI(1, 2, 5));
    }

    #[test]
    fn test_prefix_robust_get_geta_getab() {
        // GET must not swallow the "GET" prefix of GETA/GETAB, and GETA
        // must not swallow GETAB's. GETA/GETAB carry a computed PC-relative
        // offset, so only the variant discriminant (and the un-computed X
        // register) is checked here, per the offset-bearing-family note.
        assert_first_instruction("GET $1,0", MMixInstruction::GET(1, 0));
        assert_first_instruction_matches("GETA $0,4", |i| {
            matches!(i, MMixInstruction::GETA(0, _, _))
        });
        let source = "LOC #100\nBACK: HALT\nGETAB $0,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        assert!(matches!(
            asm.instructions[1].1,
            MMixInstruction::GETAB(0, _, _)
        ));
    }

    #[test]
    fn test_prefix_robust_go_pushgo() {
        // GO/GOI and PUSHGO/PUSHGOI: the base mnemonic must not steal its
        // *I sibling's literal.
        assert_first_instruction("GO $1,$2,$3", MMixInstruction::GO(1, 2, 3));
        assert_first_instruction("GOI $1,$2,5", MMixInstruction::GOI(1, 2, 5));
        assert_first_instruction("PUSHGO $1,$2,$3", MMixInstruction::PUSHGO(1, 2, 3));
        assert_first_instruction("PUSHGOI $1,$2,5", MMixInstruction::PUSHGOI(1, 2, 5));
    }

    #[test]
    fn test_prefix_robust_pushj() {
        // PUSHJ must not steal PUSHJB's longer literal. Both carry a
        // computed offset, so only the discriminant and X register are
        // checked (offset-bearing-family note).
        assert_first_instruction_matches("PUSHJ $1,4", |i| {
            matches!(i, MMixInstruction::PUSHJ(1, _, _))
        });
        let source = "LOC #100\nBACK: HALT\nPUSHJB $1,BACK";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        assert!(matches!(
            asm.instructions[1].1,
            MMixInstruction::PUSHJB(1, _, _)
        ));
    }

    #[test]
    fn test_prefix_robust_put() {
        // PUT must not steal PUTI's longer literal.
        assert_first_instruction("PUT 5,$1", MMixInstruction::PUT(5, 1));
        assert_first_instruction("PUTI 5,7", MMixInstruction::PUTI(5, 7));
    }

    #[test]
    fn test_prefix_robust_prefetch_and_sync() {
        // PREGO/PRELD/PREST/SYNCD/SYNCID each have an *I sibling, and SYNC
        // itself is a literal prefix of SYNCD/SYNCDI/SYNCID/SYNCIDI.
        assert_first_instruction("PREGO $1,$2,$3", MMixInstruction::PREGO(1, 2, 3));
        assert_first_instruction("PREGOI $1,$2,5", MMixInstruction::PREGOI(1, 2, 5));
        assert_first_instruction("PRELD $1,$2,$3", MMixInstruction::PRELD(1, 2, 3));
        assert_first_instruction("PRELDI $1,$2,5", MMixInstruction::PRELDI(1, 2, 5));
        assert_first_instruction("PREST $1,$2,$3", MMixInstruction::PREST(1, 2, 3));
        assert_first_instruction("PRESTI $1,$2,5", MMixInstruction::PRESTI(1, 2, 5));
        assert_first_instruction("SYNCD $1,$2,$3", MMixInstruction::SYNCD(1, 2, 3));
        assert_first_instruction("SYNCDI $1,$2,5", MMixInstruction::SYNCDI(1, 2, 5));
        assert_first_instruction("SYNCID $1,$2,$3", MMixInstruction::SYNCID(1, 2, 3));
        assert_first_instruction("SYNCIDI $1,$2,5", MMixInstruction::SYNCIDI(1, 2, 5));
        assert_first_instruction("SYNC 5", MMixInstruction::SYNC(5));
    }

    #[test]
    fn test_prefix_robust_extra_load_store() {
        // LDUNC/STUNC/LDHT/STHT/LDSF/STSF/LDVTS/CSWAP/STCO each have an *I
        // sibling; the base mnemonic must not steal it.
        assert_first_instruction("LDUNC $1,$2,$3", MMixInstruction::LDUNC(1, 2, 3));
        assert_first_instruction("LDUNCI $1,$2,5", MMixInstruction::LDUNCI(1, 2, 5));
        assert_first_instruction("STUNC $1,$2,$3", MMixInstruction::STUNC(1, 2, 3));
        assert_first_instruction("STUNCI $1,$2,5", MMixInstruction::STUNCI(1, 2, 5));
        assert_first_instruction("LDHT $1,$2,$3", MMixInstruction::LDHT(1, 2, 3));
        assert_first_instruction("LDHTI $1,$2,5", MMixInstruction::LDHTI(1, 2, 5));
        assert_first_instruction("STHT $1,$2,$3", MMixInstruction::STHT(1, 2, 3));
        assert_first_instruction("STHTI $1,$2,5", MMixInstruction::STHTI(1, 2, 5));
        assert_first_instruction("LDSF $1,$2,$3", MMixInstruction::LDSF(1, 2, 3));
        assert_first_instruction("LDSFI $1,$2,5", MMixInstruction::LDSFI(1, 2, 5));
        assert_first_instruction("STSF $1,$2,$3", MMixInstruction::STSF(1, 2, 3));
        assert_first_instruction("STSFI $1,$2,5", MMixInstruction::STSFI(1, 2, 5));
        assert_first_instruction("LDVTS $1,$2,$3", MMixInstruction::LDVTS(1, 2, 3));
        assert_first_instruction("LDVTSI $1,$2,5", MMixInstruction::LDVTSI(1, 2, 5));
        assert_first_instruction("CSWAP $1,$2,$3", MMixInstruction::CSWAP(1, 2, 3));
        assert_first_instruction("CSWAPI $1,$2,5", MMixInstruction::CSWAPI(1, 2, 5));

        assert_first_instruction("STCO 5,$1,$2", MMixInstruction::STCO(5, 1, 2));
        assert_first_instruction("STCOI 5,$1,7", MMixInstruction::STCOI(5, 1, 7));
    }

    /// Predicate over a parsed instruction's variant, paired with source in
    /// the `branch`/`pbranch` family tests below (offset-bearing, so the
    /// exact value isn't what those tests are proving).
    type InstructionPredicate = fn(&MMixInstruction) -> bool;

    #[test]
    fn test_prefix_robust_branch_family() {
        // BN/BNB/BNN/BNNB/BNP/BNPB/BNZ/BNZB/BEV/BEVB/BOD/BODB/BP/BPB/BZ/BZB:
        // every short mnemonic in this family is a literal prefix of at
        // least one longer sibling. Offsets are computed, so only the
        // discriminant and X register are checked. A *B mnemonic needs a
        // target behind it, so those cases branch to a preceding label.
        let cases: &[(&str, InstructionPredicate)] = &[
            ("BN $1,4", |i| matches!(i, MMixInstruction::BN(1, _))),
            ("BACK: HALT\nBNB $1,BACK", |i| {
                matches!(i, MMixInstruction::BNB(1, _))
            }),
            ("BNN $1,4", |i| matches!(i, MMixInstruction::BNN(1, _))),
            ("BACK: HALT\nBNNB $1,BACK", |i| {
                matches!(i, MMixInstruction::BNNB(1, _))
            }),
            ("BNP $1,4", |i| matches!(i, MMixInstruction::BNP(1, _))),
            ("BACK: HALT\nBNPB $1,BACK", |i| {
                matches!(i, MMixInstruction::BNPB(1, _))
            }),
            ("BNZ $1,4", |i| matches!(i, MMixInstruction::BNZ(1, _))),
            ("BACK: HALT\nBNZB $1,BACK", |i| {
                matches!(i, MMixInstruction::BNZB(1, _))
            }),
            ("BEV $1,4", |i| matches!(i, MMixInstruction::BEV(1, _))),
            ("BACK: HALT\nBEVB $1,BACK", |i| {
                matches!(i, MMixInstruction::BEVB(1, _))
            }),
            ("BOD $1,4", |i| matches!(i, MMixInstruction::BOD(1, _))),
            ("BACK: HALT\nBODB $1,BACK", |i| {
                matches!(i, MMixInstruction::BODB(1, _))
            }),
            ("BP $1,4", |i| matches!(i, MMixInstruction::BP(1, _))),
            ("BACK: HALT\nBPB $1,BACK", |i| {
                matches!(i, MMixInstruction::BPB(1, _))
            }),
            ("BZ $1,4", |i| matches!(i, MMixInstruction::BZ(1, _))),
            ("BACK: HALT\nBZB $1,BACK", |i| {
                matches!(i, MMixInstruction::BZB(1, _))
            }),
        ];
        for (src, pred) in cases {
            assert_last_instruction_matches(src, pred);
        }
    }

    #[test]
    fn test_prefix_robust_pbranch_family() {
        // Same collision shape as the branch family above, one level up
        // (PBN/PBNB/PBNN/...), with the same backward-target requirement.
        let cases: &[(&str, InstructionPredicate)] = &[
            ("PBN $1,4", |i| matches!(i, MMixInstruction::PBN(1, _, _))),
            ("BACK: HALT\nPBNB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBNB(1, _, _))
            }),
            ("PBNN $1,4", |i| matches!(i, MMixInstruction::PBNN(1, _, _))),
            ("BACK: HALT\nPBNNB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBNNB(1, _, _))
            }),
            ("PBNP $1,4", |i| matches!(i, MMixInstruction::PBNP(1, _, _))),
            ("BACK: HALT\nPBNPB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBNPB(1, _, _))
            }),
            ("PBNZ $1,4", |i| matches!(i, MMixInstruction::PBNZ(1, _, _))),
            ("BACK: HALT\nPBNZB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBNZB(1, _, _))
            }),
            ("PBEV $1,4", |i| matches!(i, MMixInstruction::PBEV(1, _, _))),
            ("BACK: HALT\nPBEVB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBEVB(1, _, _))
            }),
            ("PBOD $1,4", |i| matches!(i, MMixInstruction::PBOD(1, _, _))),
            ("BACK: HALT\nPBODB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBODB(1, _, _))
            }),
            ("PBP $1,4", |i| matches!(i, MMixInstruction::PBP(1, _, _))),
            ("BACK: HALT\nPBPB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBPB(1, _, _))
            }),
            ("PBZ $1,4", |i| matches!(i, MMixInstruction::PBZ(1, _, _))),
            ("BACK: HALT\nPBZB $1,BACK", |i| {
                matches!(i, MMixInstruction::PBZB(1, _, _))
            }),
        ];
        for (src, pred) in cases {
            assert_last_instruction_matches(src, pred);
        }
    }

    #[test]
    fn test_prefix_robust_shift_signed_vs_unsigned() {
        assert_first_instruction("SL $1,$2,$3", MMixInstruction::SL(1, 2, 3));
        assert_first_instruction("SLU $1,$2,$3", MMixInstruction::SLU(1, 2, 3));
        assert_first_instruction("SR $1,$2,$3", MMixInstruction::SR(1, 2, 3));
        assert_first_instruction("SRU $1,$2,$3", MMixInstruction::SRU(1, 2, 3));
        assert_first_instruction("SL $1,$2,7", MMixInstruction::SLI(1, 2, 7));
        assert_first_instruction("SLU $1,$2,7", MMixInstruction::SLUI(1, 2, 7));
        assert_first_instruction("SR $1,$2,7", MMixInstruction::SRI(1, 2, 7));
        assert_first_instruction("SRU $1,$2,7", MMixInstruction::SRUI(1, 2, 7));
        assert_first_instruction("SLI $1,$2,7", MMixInstruction::SLI(1, 2, 7));
        assert_first_instruction("SRUI $1,$2,7", MMixInstruction::SRUI(1, 2, 7));
    }

    #[test]
    fn test_prefix_robust_conditional_csn_csnn() {
        // CSN must not steal CSNN/CSNZ/CSNP. And the *I siblings must
        // route via the rri path (CSNI vs CSNNI etc.).
        assert_first_instruction("CSN $1,$2,$3", MMixInstruction::CSN(1, 2, 3));
        assert_first_instruction("CSNN $1,$2,$3", MMixInstruction::CSNN(1, 2, 3));
        assert_first_instruction("CSNZ $1,$2,$3", MMixInstruction::CSNZ(1, 2, 3));
        assert_first_instruction("CSNP $1,$2,$3", MMixInstruction::CSNP(1, 2, 3));
        assert_first_instruction("CSN $1,$2,7", MMixInstruction::CSNI(1, 2, 7));
        assert_first_instruction("CSNN $1,$2,7", MMixInstruction::CSNNI(1, 2, 7));
        assert_first_instruction("CSNZ $1,$2,7", MMixInstruction::CSNZI(1, 2, 7));
        assert_first_instruction("CSNP $1,$2,7", MMixInstruction::CSNPI(1, 2, 7));
        assert_first_instruction("CSNI $1,$2,7", MMixInstruction::CSNI(1, 2, 7));
        assert_first_instruction("CSNNI $1,$2,7", MMixInstruction::CSNNI(1, 2, 7));
    }

    #[test]
    fn test_prefix_robust_zero_or_set_zsn_zsnn() {
        assert_first_instruction("ZSN $1,$2,$3", MMixInstruction::ZSN(1, 2, 3));
        assert_first_instruction("ZSNN $1,$2,$3", MMixInstruction::ZSNN(1, 2, 3));
        assert_first_instruction("ZSNZ $1,$2,$3", MMixInstruction::ZSNZ(1, 2, 3));
        assert_first_instruction("ZSNP $1,$2,$3", MMixInstruction::ZSNP(1, 2, 3));
        assert_first_instruction("ZSN $1,$2,7", MMixInstruction::ZSNI(1, 2, 7));
        assert_first_instruction("ZSNN $1,$2,7", MMixInstruction::ZSNNI(1, 2, 7));
        assert_first_instruction("ZSNZ $1,$2,7", MMixInstruction::ZSNZI(1, 2, 7));
        assert_first_instruction("ZSNP $1,$2,7", MMixInstruction::ZSNPI(1, 2, 7));
    }

    // ---- Boundary value tests ---------------------------------------

    #[test]
    fn test_immediate_boundary_zero() {
        assert_first_instruction("ADD $1,$2,0", MMixInstruction::ADDI(1, 2, 0));
        assert_first_instruction("AND $1,$2,0", MMixInstruction::ANDI(1, 2, 0));
        assert_first_instruction("SR $1,$2,0", MMixInstruction::SRI(1, 2, 0));
    }

    #[test]
    fn test_immediate_boundary_max_decimal_255() {
        assert_first_instruction("ADD $1,$2,255", MMixInstruction::ADDI(1, 2, 255));
        assert_first_instruction("OR $1,$2,255", MMixInstruction::ORI(1, 2, 255));
        assert_first_instruction("ZSP $1,$2,255", MMixInstruction::ZSPI(1, 2, 255));
    }

    #[test]
    fn test_immediate_boundary_max_hex_ff() {
        assert_first_instruction("ADD $1,$2,#FF", MMixInstruction::ADDI(1, 2, 0xFF));
        assert_first_instruction("XOR $1,$2,#FF", MMixInstruction::XORI(1, 2, 0xFF));
        assert_first_instruction("CSN $1,$2,#FF", MMixInstruction::CSNI(1, 2, 0xFF));
    }

    #[test]
    fn test_immediate_boundary_overflow_decimal_256() {
        assert!(assemble_err("ADD $1,$2,256").contains("out of range 0..255"));
        assert!(assemble_err("AND $1,$2,256").contains("out of range 0..255"));
        assert!(assemble_err("SRU $1,$2,256").contains("out of range 0..255"));
    }

    #[test]
    fn test_immediate_boundary_overflow_hex_100() {
        assert!(assemble_err("ADD $1,$2,#100").contains("out of range 0..255"));
    }

    #[test]
    fn test_immediate_boundary_overflow_large_value() {
        // A genuinely large value must not silently truncate; it must
        // be rejected by the auto-immediate range check.
        assert!(assemble_err("ADD $1,$2,#10000").contains("out of range 0..255"));
        assert!(assemble_err("ADD $1,$2,1000000").contains("out of range 0..255"));
    }

    #[test]
    fn test_immediate_boundary_negative_rejected_in_auto() {
        // The auto path is strict 0..=255. Negative literals (which
        // wrap to large u64s) must be rejected. The explicit *I path
        // keeps its silent-wrap behavior — see
        // `test_parse_negative_literal_8bit_wrap`.
        assert!(assemble_err("ADD $1,$2,-1").contains("out of range 0..255"));
        assert!(assemble_err("AND $1,$2,-128").contains("out of range 0..255"));
    }

    #[test]
    fn test_immediate_register_max_255() {
        // $255 as Z must remain a register reference, not get
        // confused with the immediate 255.
        assert_first_instruction("ADD $1,$2,$255", MMixInstruction::ADD(1, 2, 255));
        assert_first_instruction("AND $1,$2,$255", MMixInstruction::AND(1, 2, 255));
    }

    #[test]
    fn test_immediate_char_literal() {
        assert_first_instruction("ADD $1,$2,'A'", MMixInstruction::ADDI(1, 2, 65));
    }

    // ---- Symbol/label resolution at the Z slot ----------------------

    #[test]
    fn test_symbol_z_constant_in_range() {
        assert_first_instruction("K IS 0\nADD $1,$2,K", MMixInstruction::ADDI(1, 2, 0));
        assert_first_instruction("K IS 255\nAND $1,$2,K", MMixInstruction::ANDI(1, 2, 255));
        assert_first_instruction("K IS 42\nCSZ $1,$2,K", MMixInstruction::CSZI(1, 2, 42));
    }

    #[test]
    fn test_symbol_z_constant_out_of_range() {
        assert!(assemble_err("K IS 256\nADD $1,$2,K").contains("out of range 0..255"));
        assert!(assemble_err("K IS 1000\nAND $1,$2,K").contains("out of range 0..255"));
    }

    #[test]
    // NEG's immediate spellings carry Z as an 8-bit field: an operand above
    // 255 is an error rather than a silent truncation, whether it is written
    // as a literal or resolved from a symbol.
    fn test_neg_immediate_spelling_range_checks_its_z() {
        assert!(assemble_err("NEGI $1,0,#300").contains("out of range 0..255"));
        assert!(assemble_err("NEGUI $1,0,#300").contains("out of range 0..255"));
        assert!(assemble_err("BigC IS #300\nNEGI $1,0,BigC").contains("out of range 0..255"));
        assert!(assemble_err("BigC IS #300\nNEGUI $1,0,BigC").contains("out of range 0..255"));
        assert!(assemble_err("NEGI $1,0,-1").contains("out of range 0..255"));
        assert!(assemble_err("NEGUI $1,0,-1").contains("out of range 0..255"));
        assert_first_instruction("NEGI $1,0,5", MMixInstruction::NEGI(1, 0, 5));
        assert_first_instruction("NEGUI $1,0,5", MMixInstruction::NEGUI(1, 0, 5));
        assert_first_instruction(
            "SmallC IS 5\nNEGI $1,0,SmallC",
            MMixInstruction::NEGI(1, 0, 5),
        );
    }

    #[test]
    fn test_symbol_z_register_alias_zero() {
        assert_first_instruction("Z IS $0\nADD $1,$2,Z", MMixInstruction::ADD(1, 2, 0));
    }

    #[test]
    fn test_symbol_z_register_alias_max() {
        assert_first_instruction("M IS $255\nAND $1,$2,M", MMixInstruction::AND(1, 2, 255));
    }

    #[test]
    fn test_symbol_z_label_address_out_of_range() {
        // A label whose address is above 255 must error rather than
        // silently truncate, even though it grammatically parses as a
        // bare identifier in the Z slot.
        let src = "\
LOC #200
Foo  OCTA 0
Main ADD $1,$2,Foo
";
        assert!(assemble_err(src).contains("out of range 0..255"));
    }

    #[test]
    fn test_symbol_z_undefined_errors() {
        assert!(assemble_err("ADD $1,$2,Nope").contains("Undefined symbol"));
        assert!(assemble_err("AND $1,$2,Nope").contains("Undefined symbol"));
    }

    // ---- Cross-family non-interference ------------------------------
    // Multiple base mnemonics from different families in one source —
    // each must route to its own family's auto rule.

    #[test]
    fn test_cross_family_routing_in_one_program() {
        let src = "\
ADD  $1,$2,5
AND  $3,$4,7
SR   $5,$6,3
BDIF $7,$8,9
CSZ  $1,$2,1
ZSP  $3,$4,2
";
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions
                .iter()
                .map(|(_, i)| i.clone())
                .collect::<Vec<_>>(),
            vec![
                MMixInstruction::ADDI(1, 2, 5),
                MMixInstruction::ANDI(3, 4, 7),
                MMixInstruction::SRI(5, 6, 3),
                MMixInstruction::BDIFI(7, 8, 9),
                MMixInstruction::CSZI(1, 2, 1),
                MMixInstruction::ZSPI(3, 4, 2),
            ]
        );
    }

    // ---- Existing-test perturbation ---------------------------------
    // Pick a handful of pre-existing register-form assertions and add
    // matching auto-immediate assertions so that any future grammar
    // change that breaks the auto path will be caught alongside the
    // original tests.

    #[test]
    fn test_perturbation_and_xor_or() {
        // Mirrors test_parse_and / test_parse_xor / test_parse_or but
        // uses the auto-immediate path.
        assert_first_instruction("AND $1,$2,#FF", MMixInstruction::ANDI(1, 2, 0xFF));
        assert_first_instruction("XOR $5,$6,#0F", MMixInstruction::XORI(5, 6, 0x0F));
        assert_first_instruction("OR $10,$20,#80", MMixInstruction::ORI(10, 20, 0x80));
    }

    #[test]
    fn test_perturbation_bitfiddle_family() {
        // Mirrors test_parse_bdif/wdif/tdif/odif/sadd/mor/mxor.
        assert_first_instruction("BDIF $1,$2,#10", MMixInstruction::BDIFI(1, 2, 0x10));
        assert_first_instruction("WDIF $1,$2,100", MMixInstruction::WDIFI(1, 2, 100));
        assert_first_instruction("TDIF $1,$2,50", MMixInstruction::TDIFI(1, 2, 50));
        assert_first_instruction("ODIF $1,$2,255", MMixInstruction::ODIFI(1, 2, 255));
        assert_first_instruction("SADD $1,$2,0", MMixInstruction::SADDI(1, 2, 0));
        assert_first_instruction("MOR $1,$2,128", MMixInstruction::MORI(1, 2, 128));
        assert_first_instruction("MXOR $1,$2,64", MMixInstruction::MXORI(1, 2, 64));
    }

    #[test]
    fn test_perturbation_shift_family() {
        // Mirrors test_parse_sl / sli / slu / slui / sr / sri / sru / srui.
        assert_first_instruction("SL $3,$1,8", MMixInstruction::SLI(3, 1, 8));
        assert_first_instruction("SLU $1,$2,16", MMixInstruction::SLUI(1, 2, 16));
        assert_first_instruction("SR $1,$2,4", MMixInstruction::SRI(1, 2, 4));
        assert_first_instruction("SRU $1,$2,32", MMixInstruction::SRUI(1, 2, 32));
    }

    // ---- Comprehensive byte-identical regression --------------------
    // Every base mnemonic in the six in-scope families paired with its
    // explicit *I sibling. This is the strongest cross-validation that
    // the auto path produces exactly the same encoded bytes as the
    // legacy path.

    #[test]
    fn test_byte_identical_regression_all_families() {
        let pairs: &[(&str, &str)] = &[
            // Arithmetic
            ("ADD $1,$2,5", "ADDI $1,$2,5"),
            ("ADDU $1,$2,5", "ADDUI $1,$2,5"),
            ("2ADDU $1,$2,5", "2ADDUI $1,$2,5"),
            ("4ADDU $1,$2,5", "4ADDUI $1,$2,5"),
            ("8ADDU $1,$2,5", "8ADDUI $1,$2,5"),
            ("16ADDU $1,$2,5", "16ADDUI $1,$2,5"),
            ("SUB $1,$2,5", "SUBI $1,$2,5"),
            ("SUBU $1,$2,5", "SUBUI $1,$2,5"),
            ("MUL $1,$2,5", "MULI $1,$2,5"),
            ("MULU $1,$2,5", "MULUI $1,$2,5"),
            ("DIV $1,$2,5", "DIVI $1,$2,5"),
            ("DIVU $1,$2,5", "DIVUI $1,$2,5"),
            ("CMP $1,$2,5", "CMPI $1,$2,5"),
            ("CMPU $1,$2,5", "CMPUI $1,$2,5"),
            // Bitwise
            ("AND $1,$2,5", "ANDI $1,$2,5"),
            ("OR $1,$2,5", "ORI $1,$2,5"),
            ("XOR $1,$2,5", "XORI $1,$2,5"),
            ("ANDN $1,$2,5", "ANDNI $1,$2,5"),
            ("ORN $1,$2,5", "ORNI $1,$2,5"),
            ("NAND $1,$2,5", "NANDI $1,$2,5"),
            ("NOR $1,$2,5", "NORI $1,$2,5"),
            ("NXOR $1,$2,5", "NXORI $1,$2,5"),
            ("MUX $1,$2,5", "MUXI $1,$2,5"),
            // Bit-fiddle
            ("BDIF $1,$2,5", "BDIFI $1,$2,5"),
            ("WDIF $1,$2,5", "WDIFI $1,$2,5"),
            ("TDIF $1,$2,5", "TDIFI $1,$2,5"),
            ("ODIF $1,$2,5", "ODIFI $1,$2,5"),
            ("SADD $1,$2,5", "SADDI $1,$2,5"),
            ("MOR $1,$2,5", "MORI $1,$2,5"),
            ("MXOR $1,$2,5", "MXORI $1,$2,5"),
            // Shift
            ("SL $1,$2,5", "SLI $1,$2,5"),
            ("SLU $1,$2,5", "SLUI $1,$2,5"),
            ("SR $1,$2,5", "SRI $1,$2,5"),
            ("SRU $1,$2,5", "SRUI $1,$2,5"),
            // Conditional set
            ("CSN $1,$2,5", "CSNI $1,$2,5"),
            ("CSZ $1,$2,5", "CSZI $1,$2,5"),
            ("CSP $1,$2,5", "CSPI $1,$2,5"),
            ("CSOD $1,$2,5", "CSODI $1,$2,5"),
            ("CSNN $1,$2,5", "CSNNI $1,$2,5"),
            ("CSNZ $1,$2,5", "CSNZI $1,$2,5"),
            ("CSNP $1,$2,5", "CSNPI $1,$2,5"),
            ("CSEV $1,$2,5", "CSEVI $1,$2,5"),
            // Zero or set
            ("ZSN $1,$2,5", "ZSNI $1,$2,5"),
            ("ZSZ $1,$2,5", "ZSZI $1,$2,5"),
            ("ZSP $1,$2,5", "ZSPI $1,$2,5"),
            ("ZSOD $1,$2,5", "ZSODI $1,$2,5"),
            ("ZSNN $1,$2,5", "ZSNNI $1,$2,5"),
            ("ZSNZ $1,$2,5", "ZSNZI $1,$2,5"),
            ("ZSNP $1,$2,5", "ZSNPI $1,$2,5"),
            ("ZSEV $1,$2,5", "ZSEVI $1,$2,5"),
        ];

        for (auto_src, explicit_src) in pairs {
            let mut auto_asm = MMixAssembler::new(auto_src, "<auto>");
            auto_asm
                .parse()
                .unwrap_or_else(|e| panic!("auto src {auto_src:?} failed: {e}"));
            let mut explicit_asm = MMixAssembler::new(explicit_src, "<explicit>");
            explicit_asm
                .parse()
                .unwrap_or_else(|e| panic!("explicit src {explicit_src:?} failed: {e}"));
            let auto_bytes = auto_asm.encode_instruction_bytes(&auto_asm.instructions[0].1);
            let explicit_bytes =
                explicit_asm.encode_instruction_bytes(&explicit_asm.instructions[0].1);
            assert_eq!(
                auto_bytes,
                explicit_bytes,
                "byte mismatch: auto {auto_src:?} -> {:?} {:?} vs explicit {explicit_src:?} -> {:?} {:?}",
                auto_asm.instructions[0].1,
                auto_bytes,
                explicit_asm.instructions[0].1,
                explicit_bytes
            );
        }
    }

    /// The explicit `*I` spellings reject a negative `Z` exactly as the
    /// auto (`ADD`) path does.
    #[test]
    fn test_explicit_i_negative_is_an_error() {
        assert_eq!(
            assemble_err("ADDI $1,$2,-1"),
            "<test>:1:12: immediate operand -1 out of range 0..255 for ADDI"
        );
        assert_eq!(
            assemble_err("ANDI $1,$2,-1"),
            "<test>:1:12: immediate operand -1 out of range 0..255 for ANDI"
        );
        assert_eq!(
            assemble_err("SLUI $1,$2,-1"),
            "<test>:1:12: immediate operand -1 out of range 0..255 for SLUI"
        );
        assert_eq!(
            assemble_err("ADD $1,$2,-1"),
            "<test>:1:11: immediate operand -1 out of range 0..255 for ADD"
        );
    }

    // ---- Source-level debug info (SourceLoc / source_loc / addr_for_line /
    // source_text): pc.1 of the mmixdb effort. ----------------------------

    /// Line fidelity across a labeled `debug` directive: the pinning test.
    /// Mirrors `examples/hello_world.mms`'s shape (a labeled `debug "..."`
    /// line, then a labeled instruction a couple of lines down). Reverting
    /// the `preprocess_debug` line-count-preserving fix (Step 1) -- i.e.
    /// restoring the old two-line label/PUSHJ expansion -- shifts every
    /// following line by one and makes this assertion fail; that was
    /// verified by hand before landing (see the prompt's report).
    #[test]
    fn test_debug_directive_preserves_line_fidelity() {
        let lines = [
            "\tLOC\tData_Segment",
            "\tGREG\t@",
            "Text\tBYTE\t\"Hello world!\",10,0",
            "",
            "\tLOC\t#100",
            "",
            "Main\tdebug \"Version 0.1: Hello World Example\"",
            "Start\tLDA\t$255,Text",
            "\tTRAP\t0,Fputs,StdOut",
            "\tTRAP\t0,Halt,0",
        ];
        let lda_line = lines.iter().position(|l| l.contains("LDA")).unwrap() + 1;
        let source = lines.join("\n");

        let mut asm = MMixAssembler::new(&source, "hello_world.mms");
        asm.parse().unwrap();

        let lda_addr = *asm.labels.get("Start").expect("Start label defined");
        let loc = asm
            .source_loc(lda_addr)
            .expect("LDA's address should have a source location");
        assert_eq!(loc.file, "hello_world.mms");
        assert_eq!(loc.line, lda_line, "LDA must report its ORIGINAL line");
    }

    /// Inverse round-trip: `addr_for_line` returns the same address
    /// `source_loc` mapped back from.
    #[test]
    fn test_addr_for_line_round_trips_with_source_loc() {
        let source = "Main\tdebug \"hi\"\nStart\tLDA\t$255,Start\n\tTRAP\t0,Halt,0\n";

        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();

        let lda_addr = *asm.labels.get("Start").unwrap();
        let loc = asm.source_loc(lda_addr).unwrap();
        assert_eq!(
            asm.addr_for_line(&loc.file, loc.line),
            Some(lda_addr),
            "addr_for_line must invert source_loc"
        );
    }

    /// `source_text` returns the ORIGINAL line (the `debug` directive as the
    /// user wrote it), not the preprocessed `PUSHJ` text.
    #[test]
    fn test_source_text_returns_original_not_preprocessed() {
        let source = "Main\tdebug \"hi\"\nStart\tLDA\t$255,Start\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();

        let text = asm.source_text("<test>", 1).expect("line 1 exists");
        assert!(
            text.contains("debug"),
            "expected original text, got {text:?}"
        );
        assert!(
            !text.contains("PUSHJ"),
            "source_text must not leak preprocessed text, got {text:?}"
        );

        let lda_text = asm.source_text("<test>", 2).expect("line 2 exists");
        assert!(lda_text.contains("LDA"));
    }

    /// A syntax error after a `debug` line must name the ORIGINAL line, not
    /// the preprocessed one the landing pad shifts it to.
    #[test]
    fn test_syntax_error_after_a_debug_line_reports_the_original_line() {
        let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
\tSET\t$1,7
\tFLOT\t$1,$2,$3
";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm
            .parse()
            .expect_err("FLOT $1,$2,$3 must still be rejected");
        assert_eq!(
            err, "<test>:4:10: register $2 cannot be used where a pure value is required",
            "must report original line 4, not the preprocessed line the \
             debug expansion's landing pad shifts it to"
        );
    }

    /// A symbol redefined after two `debug` lines reports both the current
    /// and the first-definition site at their ORIGINAL lines.
    #[test]
    fn test_redefinition_after_two_debug_lines_reports_original_lines() {
        let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
\tdebug\t\"ho\"
Foo\tIS\t1
\tSET\t$1,7
Foo\tIS\t2
";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm
            .parse()
            .expect_err("redefining Foo must still be rejected");
        assert_eq!(
            err, "<test>:6: symbol 'Foo' redefined (first defined at <test>:4)",
            "both sites must report their ORIGINAL lines, not the \
             preprocessed lines two debug expansions shift them to"
        );
    }

    /// A data directive that emits multiple words maps every emitted address
    /// to the same source line, and `addr_for_line` returns the lowest one.
    /// Checked for `BYTE` (1-byte units) and `OCTA` (8-byte units), so the
    /// invariant is shown to hold independent of unit width.
    #[test]
    fn test_multi_word_data_directive_maps_to_one_line() {
        let source = "Data\tBYTE\t1,2,3\nWide\tOCTA\t1,2,3\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();

        let base_addr = *asm.labels.get("Data").unwrap();
        for offset in 0..3 {
            let loc = asm
                .source_loc(base_addr + offset)
                .unwrap_or_else(|| panic!("no source_loc at offset {offset}"));
            assert_eq!(loc.line, 1);
        }
        assert_eq!(asm.addr_for_line("<test>", 1), Some(base_addr));

        let wide_addr = *asm.labels.get("Wide").unwrap();
        for offset in 0..3 {
            let addr = wide_addr + offset * 8;
            let loc = asm
                .source_loc(addr)
                .unwrap_or_else(|| panic!("no source_loc at unit {offset}"));
            assert_eq!(loc.line, 2);
        }
        assert_eq!(asm.addr_for_line("<test>", 2), Some(wide_addr));
    }

    /// A stack program with both a data and a text region, so `source_loc`
    /// is asked about an address in the gap between them.
    const TWO_REGION_PROGRAM: &str = "\
        LOC     Data_Segment
Cells   OCTA    0
        OCTA    0
        OCTA    0
Sp      GREG    Cells

        LOC     #100
Main    SETI    $1,7
        STOI    $1,Sp,0
        ADDUI   Sp,Sp,8
        SETI    $1,35
        STOI    $1,Sp,0
        LDOI    $2,Sp,0
        SUBUI   Sp,Sp,8
        LDOI    $3,Sp,0
        ADDU    $255,$2,$3
        TRAP    0,Halt,0
";

    /// `SETI $X,imm` expands to four tetras. Every address in the expansion
    /// belongs to the statement that emitted it, and the address just past
    /// the last text entry belongs to nothing -- the data region lies far
    /// above it, and a lookup that ran to the end of the image would hand
    /// that whole gap to the last text line.
    #[test]
    fn test_source_loc_covers_an_expansion_and_stops_at_its_end() {
        let mut asm = MMixAssembler::new(TWO_REGION_PROGRAM, "stack.mms");
        asm.parse().unwrap();

        let main = *asm.labels.get("Main").unwrap();
        assert_eq!(main, 0x100);
        for addr in [0x100, 0x104, 0x108, 0x10c] {
            let loc = asm
                .source_loc(addr)
                .unwrap_or_else(|| panic!("no source_loc at 0x{addr:x}"));
            assert_eq!(loc.line, 8, "0x{addr:x} is inside line 8's SETI");
        }
        assert_eq!(asm.source_loc(0x110).map(|loc| loc.line), Some(9));

        // One tetra past the TRAP that ends the text region, and far below
        // the data region.
        assert_eq!(asm.source_loc(0x140), None);
    }

    /// The data region keeps its own lines; the text region above it does
    /// not bleed into the gap, nor the data region below.
    #[test]
    fn test_source_loc_maps_the_data_region_independently() {
        let mut asm = MMixAssembler::new(TWO_REGION_PROGRAM, "stack.mms");
        asm.parse().unwrap();

        let cells = *asm.labels.get("Cells").unwrap();
        assert!(cells >= 0x2000_0000_0000_0000);
        assert_eq!(asm.source_loc(cells).map(|loc| loc.line), Some(2));
        assert_eq!(asm.source_loc(cells + 7).map(|loc| loc.line), Some(2));
        assert_eq!(asm.source_loc(cells + 8).map(|loc| loc.line), Some(3));
        assert_eq!(asm.source_loc(cells + 24), None);
    }

    /// Duplicate-name translation units: reverse lookups (`addr_for_line`,
    /// `source_text`) resolve to the FIRST unit with that filename in
    /// command-line order, while `source_loc` (keyed by address) stays
    /// unambiguous regardless.
    #[test]
    fn test_source_text_resolves_duplicate_filename_to_first_unit() {
        let mut asm = MMixAssembler::new("First\tHALT\n", "dup.mms");
        asm.add_source("Second\tHALT\n", "dup.mms");
        asm.parse().unwrap();

        let text = asm.source_text("dup.mms", 1).unwrap();
        assert!(text.contains("First"));
    }

    // --- resolve_includes (INCLUDE directive) ---

    /// Build an in-memory `read` closure keyed by exact `PathBuf`s, so tests
    /// stay hermetic (no real filesystem access).
    fn fixture_reader(
        files: Vec<(&str, &str)>,
    ) -> impl Fn(&std::path::Path) -> std::io::Result<String> {
        let map: HashMap<std::path::PathBuf, String> = files
            .into_iter()
            .map(|(p, s)| (std::path::PathBuf::from(p), s.to_string()))
            .collect();
        move |p: &std::path::Path| {
            map.get(p).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "no such fixture file")
            })
        }
    }

    #[test]
    fn resolve_includes_single_include_inserts_a_unit() {
        let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);
        let units = MMixAssembler::resolve_includes(
            "INCLUDE lib.mms\nOCTA 2\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap();

        assert_eq!(units.len(), 2);
        assert_eq!(units[0].0, "lib.mms");
        assert!(units[0].1.contains("OCTA 1"));
        assert_eq!(units[1].0, "root.mms");
        assert!(units[1].1.contains("OCTA 2"));
    }

    #[test]
    fn resolve_includes_filename_fidelity() {
        // The included unit's filename must be the included file's own path,
        // NOT the including (root) file's name -- the property text-splicing
        // could never give, since a spliced file has no filename of its own.
        let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);
        let units = MMixAssembler::resolve_includes(
            "INCLUDE lib.mms\nOCTA 2\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap();

        assert_eq!(units[0].0, "lib.mms");
        assert_ne!(units[0].0, "root.mms");
    }

    #[test]
    fn resolve_includes_pads_line_numbers_after_an_include() {
        let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);
        let units = MMixAssembler::resolve_includes(
            "% a\nINCLUDE lib.mms\nBYTE 0\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap();

        // Trailing host segment: `BYTE 0` was on absolute line 3, so it must
        // be preceded by exactly 2 padding newlines.
        let host_segment = &units.last().unwrap().1;
        assert_eq!(host_segment.matches('\n').count() - 1, 2);
        assert!(host_segment.starts_with("\n\nBYTE 0"));

        // Parse it through the real assembler and confirm the reported line
        // number is the ABSOLUTE line 3, not the padded segment's line 1.
        let mut asm = MMixAssembler::new(&units[0].1, &units[0].0);
        for (name, src) in units.iter().skip(1) {
            asm.add_source(src, name);
        }
        asm.parse().unwrap();
        // `lib.mms`'s `OCTA 1` occupies address 0..8, so `BYTE 0` -- at
        // absolute line 3 thanks to padding -- lands at address 8.
        assert_eq!(asm.addr_for_line("root.mms", 3), Some(8));
    }

    #[test]
    fn resolve_includes_nested_relative_resolution() {
        let reader = fixture_reader(vec![
            ("a/sub/b.mms", "INCLUDE c.mms\nOCTA 2\n"),
            ("a/sub/c.mms", "OCTA 3\n"),
        ]);
        let units = MMixAssembler::resolve_includes(
            "INCLUDE sub/b.mms\nOCTA 1\n",
            "a/root.mms",
            std::path::Path::new("a"),
            &reader,
        )
        .unwrap();

        let c_unit = units
            .iter()
            .find(|(name, _)| name.contains("c.mms"))
            .expect("c.mms unit present");
        assert_eq!(c_unit.0, "a/sub/c.mms");
        assert!(c_unit.1.contains("OCTA 3"));
    }

    #[test]
    fn resolve_includes_cycle_is_an_error() {
        let reader = fixture_reader(vec![
            ("a.mms", "INCLUDE b.mms\n"),
            ("b.mms", "INCLUDE a.mms\n"),
        ]);
        let err = MMixAssembler::resolve_includes(
            "INCLUDE a.mms\n",
            "driver.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap_err();

        assert!(err.contains("cycle"));
        assert!(err.contains("a.mms"));
        assert!(err.contains("b.mms"));
    }

    #[test]
    fn resolve_includes_missing_file_is_an_error_not_a_panic() {
        let reader = fixture_reader(vec![]);
        let err = MMixAssembler::resolve_includes(
            "INCLUDE missing.mms\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap_err();

        assert!(err.contains("missing.mms"));
    }

    #[test]
    fn resolve_includes_passthrough_preserves_content_with_no_include() {
        let reader = fixture_reader(vec![]);
        let source = "OCTA 1\nOCTA 2";
        let units =
            MMixAssembler::resolve_includes(source, "root.mms", std::path::Path::new(""), &reader)
                .unwrap();

        assert_eq!(units.len(), 1);
        assert_eq!(units[0].0, "root.mms");
        assert_eq!(units[0].1, source);
    }

    #[test]
    fn resolve_includes_recognizes_comment_case() {
        // INCLUDE matches in upper case only; a lower-case
        // `include` is ordinary source text, never expanded.
        let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);

        let lower_with_comment = MMixAssembler::resolve_includes(
            "include lib.mms  % pull it in\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap();
        let quoted = MMixAssembler::resolve_includes(
            "INCLUDE \"lib.mms\"\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap();

        assert_eq!(lower_with_comment.len(), 1);
        assert_eq!(lower_with_comment[0].1, "include lib.mms  % pull it in\n");
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].1, "OCTA 1\n");
        assert!(quoted[0].1.contains("OCTA 1"));
    }

    // ---- Expressions (C9.1) -------------------------------------------

    fn assemble_err(source: &str) -> String {
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .expect_err(&format!("{source:?} must be rejected"))
    }

    #[test]
    fn test_expr_left_associative_weak_chain() {
        // a-b-c is (a-b)-c, not a-(b-c).
        assert_first_instruction("OCTA 10-3-2", MMixInstruction::OCTA(5));
    }

    #[test]
    fn test_expr_strong_binds_tighter_than_weak() {
        assert_first_instruction("OCTA 2+3*4", MMixInstruction::OCTA(14));
    }

    #[test]
    fn test_expr_reference_shift_and_add_chain() {
        // The MMIXAL reference's o<<24+x<<16+y<<8+z, left-associated:
        // (o<<24)+(x<<16)+(y<<8)+z.
        let mut asm = MMixAssembler::new(
            "o IS 1\nx IS 2\ny IS 3\nz IS 4\nOCTA o<<24+x<<16+y<<8+z",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::OCTA((1 << 24) + (2 << 16) + (3 << 8) + 4)
        );
    }

    #[test]
    fn test_expr_wraps_subtraction_below_zero() {
        assert_first_instruction("OCTA 0-1", MMixInstruction::OCTA(u64::MAX));
    }

    #[test]
    fn test_expr_wraps_addition_above_max() {
        assert_first_instruction("OCTA #FFFFFFFFFFFFFFFF+1", MMixInstruction::OCTA(0));
    }

    #[test]
    fn test_expr_floor_fraction_operator() {
        // 1//2 is floor(2^64 * 1/2) = 2^63.
        assert_first_instruction("OCTA 1//2", MMixInstruction::OCTA(1u64 << 63));
    }

    #[test]
    fn test_expr_shift_by_64_or_more_is_zero() {
        assert_first_instruction("OCTA 1<<64", MMixInstruction::OCTA(0));
        assert_first_instruction("OCTA 1>>64", MMixInstruction::OCTA(0));
    }

    #[test]
    fn test_expr_register_plus_pure_selects_register_form() {
        // x IS $1, y IS $2: ADD x,y,y+1 is ADD $1,$2,$3 (register form).
        let mut asm = MMixAssembler::new("x IS $1\ny IS $2\nADD x,y,y+1", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 3));
    }

    #[test]
    fn test_expr_register_minus_register_selects_immediate_form() {
        // ADD $1,$2,y-x is the immediate form, since register-register
        // subtraction is a pure value.
        let mut asm = MMixAssembler::new("x IS $1\ny IS $2\nADD $1,$2,y-x", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 1));
    }

    #[test]
    fn test_expr_is_records_a_register_from_register_arithmetic() {
        // x IS $1+1 records a register, not a pure constant.
        let mut asm = MMixAssembler::new("x IS $1+1", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.symbols.get("x"), Some(&SymbolType::Register(2)));
    }

    #[test]
    fn test_expr_set_register_arithmetic_copies() {
        // SET $1,$2+1 copies register $3 (the register NAMED $2+1),
        // never an arithmetic add on $2's runtime value.
        assert_first_instruction("SET $1,$2+1", MMixInstruction::SETRR(1, 3));
    }

    #[test]
    fn test_expr_register_plus_register_is_error() {
        assert!(
            assemble_err("x IS $1\ny IS $2\nADD $1,$2,x+y")
                .contains("+ cannot apply to a register operand")
        );
    }

    #[test]
    fn test_expr_pure_minus_register_is_error() {
        assert!(assemble_err("x IS $1\nOCTA 3-x").contains("- cannot apply to a register operand"));
    }

    #[test]
    fn test_expr_strong_operator_on_register_is_error() {
        assert!(assemble_err("x IS $1\nOCTA x*2").contains("* cannot apply to a register operand"));
    }

    #[test]
    fn test_expr_unary_minus_on_register_is_error() {
        assert!(assemble_err("x IS $1\nSET $2,-x").contains("unary - cannot apply to a register"));
    }

    #[test]
    fn test_expr_unary_tilde_on_register_is_error() {
        assert!(assemble_err("x IS $1\nSET $2,~x").contains("unary ~ cannot apply to a register"));
    }

    #[test]
    fn test_expr_unary_dollar_on_register_is_error() {
        assert!(assemble_err("x IS $1\nSET $2,$x").contains("unary $ cannot apply to a register"));
    }

    #[test]
    fn test_expr_register_in_pure_site_is_error() {
        assert!(
            assemble_err("x IS $1\nLOC x")
                .contains("cannot be used where a pure value is required")
        );
    }

    #[test]
    fn test_expr_pure_value_in_register_site_is_error() {
        assert!(
            assemble_err("ADD 3,$1,$2").contains("cannot be used where a register is required")
        );
    }

    #[test]
    fn test_expr_final_register_above_255_is_error() {
        assert!(assemble_err("SET $1,$260").contains("out of range 0..255"));
    }

    #[test]
    fn test_expr_division_by_zero_is_error() {
        assert!(assemble_err("OCTA 5/0").contains("division by zero"));
    }

    #[test]
    fn test_expr_percent_by_zero_is_error() {
        // `%` shares `/`'s zero-divisor check: it computes the remainder of
        // the same division, which is illegal at y=0.
        assert!(assemble_err("OCTA 5%0").contains("division by zero"));
    }

    #[test]
    fn test_expr_illegal_fraction_is_error() {
        assert!(assemble_err("OCTA 2//1").contains("illegal fraction"));
    }

    #[test]
    fn test_expr_unary_ampersand_is_unsupported() {
        assert!(
            assemble_err("Foo IS 1\nOCTA &Foo")
                .contains("unary & (a symbol's serial number) is unsupported")
        );
    }

    #[test]
    fn test_expr_dangling_operator_is_syntax_error() {
        assert!(
            assemble_err("SETL $1,5+")
                .contains("a remark must be separated from the statement by a blank")
        );
    }

    #[test]
    fn test_percent_inside_bare_expression_is_remainder() {
        assert_first_instruction("SET $1,5%3", MMixInstruction::SETL(1, 2));
    }

    #[test]
    fn test_percent_after_space_opens_a_comment() {
        assert_first_instruction("SET $1,5 % 3", MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_percent_before_space_still_opens_a_comment() {
        assert_first_instruction("SET $1,5% 3", MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_percent_inside_a_group_is_remainder() {
        assert_first_instruction("SET $1,(5 % 3)", MMixInstruction::SETL(1, 2));
    }

    #[test]
    fn test_percent_after_a_closed_group_opens_a_comment() {
        // `sum` is undefined; if this parsed as an operator the undefined
        // symbol would fail, so success proves the comment.
        assert_first_instruction("SET $1,(2 + 3) % sum", MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_semicolon_after_an_expression_starts_a_new_statement() {
        // `;` no longer opens a comment: `text` is a second statement, a
        // bare label needing no leading blank, defined at SET's address.
        let mut asm = MMixAssembler::new("SET $1,5;text", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
        assert_eq!(asm.labels.get("text"), Some(&4));
    }

    #[test]
    fn test_whitespace_after_a_weak_operator_is_a_syntax_error() {
        assert!(assemble_err("SETL $1,2 + 3").contains("a remark cannot begin with"));
    }

    #[test]
    fn test_whitespace_after_unary_minus_is_a_syntax_error() {
        assert!(assemble_err("SET $1,- 5").contains("unknown operation"));
    }

    #[test]
    fn test_bare_expression_closed_up_assembles() {
        assert_first_instruction("SETL $1,2+3", MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_parenthesized_group_may_hold_whitespace() {
        assert_first_instruction("SETL $1,(2 + 3)", MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_nested_groups_evaluate_innermost_first() {
        assert_first_instruction("SETL $1,((2 + 3) * 4)", MMixInstruction::SETL(1, 20));
    }

    #[test]
    fn test_group_and_bare_operators_left_associate() {
        // Strong binds tighter than weak, and both are left-associative:
        // 2+(3*4)+5 is (2+(3*4))+5 = 2+12+5 = 19.
        assert_first_instruction("SETL $1,2+(3 * 4)+5", MMixInstruction::SETL(1, 19));
    }

    #[test]
    fn test_unclosed_group_reports_unterminated_group() {
        let err = assemble_err("SETL $1,(2 + 3");
        assert!(
            err.contains("unterminated group"),
            "expected an unterminated-group diagnostic, got: {err}"
        );
    }

    #[test]
    fn test_comma_inside_an_open_group_is_an_error() {
        assert!(assemble_err("SETL $1,(1 , 2)").contains("unknown operation"));
    }

    #[test]
    fn test_newline_inside_an_open_group_is_an_error() {
        assert!(assemble_err("SETL $1,(1\n2)").contains("unterminated group"));
    }

    /// A negative `SET` source is an error whether the literal is decimal
    /// or hex.
    #[test]
    fn test_set_negative_literal_is_an_error_for_decimal_and_hex() {
        assert_eq!(
            assemble_err("SET $1,-1"),
            "<test>:1:8: immediate operand -1 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
        );
        assert_eq!(
            assemble_err("SET $1,-5"),
            "<test>:1:8: immediate operand -5 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
        );
        assert_eq!(
            assemble_err("SET $1,-#10"),
            "<test>:1:8: immediate operand -16 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
        );
    }

    #[test]
    fn test_at_in_an_instruction_is_the_aligned_address_after_byte() {
        let mut asm = MMixAssembler::new("BYTE 1\nSET $1,@", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(1, 4));
    }

    #[test]
    fn test_at_at_in_a_data_directive_both_hold_the_aligned_address() {
        let mut asm = MMixAssembler::new("BYTE 1,1,1\nOCTA @,@", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[3].1, MMixInstruction::OCTA(8));
        assert_eq!(asm.instructions[4].1, MMixInstruction::OCTA(8));
    }

    #[test]
    fn test_forward_reference_with_operator_resolves() {
        let mut asm = MMixAssembler::new(
            "JMP Later+4\nOCTA Later-8\nLater IS 100\nJMP Later",
            "<test>",
        );
        asm.parse()
            .unwrap_or_else(|e| panic!("forward reference with operator must resolve: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::OCTA(92));
    }

    #[test]
    fn test_loc_forward_reference_fails_like_today() {
        assert!(assemble_err("LOC Later+4\nLater IS 100").contains("Undefined symbol: Later"));
    }

    #[test]
    fn test_is_forward_reference_fails_like_today() {
        assert!(assemble_err("Foo IS Later+1\nLater IS 100").contains("Undefined symbol: Later"));
    }

    #[test]
    fn test_greg_forward_reference_fails_like_today() {
        assert!(assemble_err("GREG Later+1\nLater IS 100").contains("Undefined symbol: Later"));
    }

    #[test]
    fn test_loc_label_takes_the_location_before_loc() {
        // After LOC #100 and one instruction, the counter is #104; `Gap`
        // must name #104, not the #300 the LOC on its own line jumps to.
        let mut asm = MMixAssembler::new("LOC #100\nMain TRAP 0,Halt,0\nGap LOC #300", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Gap"), Some(&0x104));
    }

    #[test]
    fn test_loc_label_order_holds_within_pass_one() {
        // GREG's init value is computed once, in pass 1, and never
        // recomputed in pass 2 (which only checks the symbol is present),
        // so this is the one place a pass-1-only ordering bug survives to
        // the final state: `GREG Gap` must read `Gap`'s pre-move address.
        let mut asm = MMixAssembler::new("LOC #100\nGap LOC #300\nGREG Gap", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.greg_inits.last().map(|(_, v)| *v), Some(0x100));
    }

    #[test]
    fn test_loc_at_plus_offset_names_the_prior_location() {
        // X LOC @+500 gives X the location before LOC, and leaves the
        // counter at X+500.
        let mut asm = MMixAssembler::new("LOC #100\nX LOC @+500\nBYTE 1", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("X"), Some(&0x100));
        assert_eq!(asm.instructions[0].0, 0x100 + 500);
    }

    #[test]
    fn test_wyde_data_list_mixes_expression_items_and_a_string() {
        // The pass-agreement pattern of test_byte_string_pass1_pass2_agree:
        // a forward OCTA reads pass 1's size for the list, so pass 2 must
        // compute the same expression values or Next's address disagrees.
        let mut asm = MMixAssembler::new(
            "Base IS 2\nOCTA Next\nList WYDE Base+8,\"ab\",Base*10\nNext BYTE 99",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("List"), Some(&8));
        let list: Vec<_> = asm.instructions[1..5]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            list,
            vec![
                (8, MMixInstruction::WYDE(10)),
                (10, MMixInstruction::WYDE(b'a' as u16)),
                (12, MMixInstruction::WYDE(b'b' as u16)),
                (14, MMixInstruction::WYDE(20)),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&16));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(16));
    }

    #[test]
    fn test_tetra_data_list_mixes_expression_items_and_a_string() {
        let mut asm = MMixAssembler::new(
            "Base IS 2\nOCTA Next\nList TETRA Base+8,\"ab\",Base*10\nNext BYTE 99",
            "<test>",
        );
        asm.parse().unwrap();
        let list: Vec<_> = asm.instructions[1..5]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            list,
            vec![
                (8, MMixInstruction::TETRA(10)),
                (12, MMixInstruction::TETRA(b'a' as u32)),
                (16, MMixInstruction::TETRA(b'b' as u32)),
                (20, MMixInstruction::TETRA(20)),
            ]
        );
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(24));
    }

    #[test]
    fn test_octa_data_list_mixes_expression_items_and_a_string() {
        let mut asm = MMixAssembler::new(
            "Base IS 2\nOCTA Next\nList OCTA Base+8,\"ab\",Base*10\nNext BYTE 99",
            "<test>",
        );
        asm.parse().unwrap();
        let list: Vec<_> = asm.instructions[1..5]
            .iter()
            .map(|(addr, inst)| (*addr, inst.clone()))
            .collect();
        assert_eq!(
            list,
            vec![
                (8, MMixInstruction::OCTA(10)),
                (16, MMixInstruction::OCTA(b'a' as u64)),
                (24, MMixInstruction::OCTA(b'b' as u64)),
                (32, MMixInstruction::OCTA(20)),
            ]
        );
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(40));
    }

    #[test]
    fn test_expr_weak_bitwise_or_and_xor() {
        assert_first_instruction("OCTA 0xF0|0x0F", MMixInstruction::OCTA(0xFF));
        assert_first_instruction("OCTA 0xFF^0x0F", MMixInstruction::OCTA(0xF0));
    }

    #[test]
    fn test_expr_strong_bitwise_and() {
        assert_first_instruction("OCTA 0xFF&0x0F", MMixInstruction::OCTA(0x0F));
    }

    // ---- Lexical conformance (C9.2) ------------------------------------

    /// One of the three diagnostics a dangling operator's abutting text may
    /// raise -- an unterminated group, a missing blank, or an operator that
    /// opens the remark -- rather than a specific one of them.
    fn assert_abutting_text_is_rejected(source: &str) {
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm
            .parse()
            .err()
            .unwrap_or_else(|| panic!("expected an error for {source:?}, parse succeeded"));
        assert!(
            err.contains("separated from the statement by a blank")
                || err.contains("a remark cannot begin with")
                || err.contains("unterminated group"),
            "error for {source:?} does not name the remark rule: {err}"
        );
    }

    #[test]
    fn test_semicolon_separates_two_statements_on_one_line() {
        let mut asm = MMixAssembler::new("SETL $1,1; ADD $1,$1,1", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
        assert_eq!(asm.instructions[1].1, MMixInstruction::ADDI(1, 1, 1));
    }

    #[test]
    fn test_semicolon_needs_no_surrounding_blank() {
        let mut asm = MMixAssembler::new("SETL $1,1;ADD $1,$1,1", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
        assert_eq!(asm.instructions[1].1, MMixInstruction::ADDI(1, 1, 1));
    }

    #[test]
    fn test_label_after_semicolon_is_defined_at_that_statements_address() {
        let mut asm = MMixAssembler::new("SETL $1,1;Here ADD $1,$1,1", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.labels.get("Here"), Some(&4));
    }

    #[test]
    fn test_three_statements_on_one_line() {
        let mut asm = MMixAssembler::new("SETL $1,1;SETL $2,2;SETL $3,3", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 3);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 2));
        assert_eq!(asm.instructions[2].1, MMixInstruction::SETL(3, 3));
    }

    #[test]
    fn test_empty_statements_between_and_around_semicolons_parse() {
        for source in [";;", "SETL $1,1;"] {
            let mut asm = MMixAssembler::new(source, "<test>");
            asm.parse()
                .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        }
    }

    #[test]
    fn test_indented_leading_semicolon_starts_an_empty_statement_not_a_comment() {
        // Indentation means the whole-line comment rule doesn't cover this
        // `;`: it opens an empty first statement, and `SETL` is the second.
        // Under `;`-as-comment, this line assembled nothing at all.
        let mut asm = MMixAssembler::new("   ;SETL $1,1", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
    }

    #[test]
    fn test_percent_comment_wins_over_a_later_semicolon() {
        let mut asm = MMixAssembler::new("SETL $1,1 % note; ADD $1,$1,1", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
    }

    #[test]
    fn test_semicolon_inside_a_string_literal_is_ordinary_text() {
        let mut asm = MMixAssembler::new("BYTE \";\"", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0x3B));
    }

    #[test]
    fn test_semicolon_inside_a_char_literal_is_ordinary_text() {
        assert_first_instruction("SET $1,';'", MMixInstruction::SETL(1, 0x3B));
    }

    #[test]
    fn test_whole_line_comment_openers_contribute_nothing() {
        for opener in [";", "*", "#", "/", "-"] {
            let source = format!("{opener} not a statement\nSETL $1,1");
            let mut asm = MMixAssembler::new(&source, "<test>");
            asm.parse()
                .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
            assert_eq!(asm.instructions.len(), 1, "for opener {opener:?}");
            assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
        }
    }

    #[test]
    fn test_column_one_colon_and_underscore_still_open_a_label() {
        let mut asm = MMixAssembler::new(":Foo HALT\n_Bar HALT", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        // `:Foo` opts out of (the empty) active prefix and is keyed at the
        // root without its colon.
        assert_eq!(asm.labels.get("Foo"), Some(&0));
        assert_eq!(asm.labels.get("_Bar"), Some(&4));
    }

    #[test]
    fn test_line_starting_with_a_digit_still_parses() {
        assert_first_instruction("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3));
    }

    #[test]
    fn test_indented_percent_comment_still_a_comment() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    % note\nSETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_leading_zero_literal_is_decimal_not_octal() {
        assert_first_instruction("SETL $1,0100", MMixInstruction::SETL(1, 100));
    }

    /// The error keeps the decimal reading of a leading-zero literal
    /// (`-010` reads `-10`); a leading zero is never octal.
    #[test]
    fn test_negative_leading_zero_literal_is_an_error_reading_decimal() {
        assert_eq!(
            assemble_err("SET $1,-010"),
            "<test>:1:8: immediate operand -10 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
        );
    }

    #[test]
    fn test_hex_literal_forms_unaffected_by_octal_removal() {
        assert_first_instruction("SET $1,0x10", MMixInstruction::SETL(1, 16));
        assert_first_instruction("SET $1,#10", MMixInstruction::SETL(1, 16));
    }

    #[test]
    fn test_leading_zero_literal_with_a_single_trailing_digit() {
        assert_first_instruction("SETL $1,08", MMixInstruction::SETL(1, 8));
    }

    #[test]
    fn test_remark_after_an_operand_is_ignored() {
        assert_first_instruction(
            "ADD $1,$2,$3 sum of the parts",
            MMixInstruction::ADD(1, 2, 3),
        );
        assert_first_instruction("SET $1,5 the answer", MMixInstruction::SETL(1, 5));
        assert_first_instruction("SET $1,(2 + 3) ) stray", MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_remark_after_an_empty_operand_list_is_ignored() {
        assert_first_instruction("HALT exit here", MMixInstruction::HALT);
    }

    #[test]
    fn test_trailing_operator_after_a_blank_is_rejected_naming_the_rule() {
        for source in [
            "SETL $1,2 + 3",
            "SET $1,2 , 3",
            "SET $1,2 * 3",
            "SET $1,2 - 3",
            "SET $1,$2 $3",
            "SET $1,2 / 3",
        ] {
            assert!(assemble_err(source).contains("a remark cannot begin with"));
        }
    }

    #[test]
    fn test_abutting_text_is_rejected() {
        for source in ["SETL $1,2abc", "SET $1,5)", "SET $1,3/", "SET $1,4//"] {
            assert_abutting_text_is_rejected(source);
        }
    }

    #[test]
    fn test_division_inside_an_expression_or_group_is_untouched() {
        assert_first_instruction("SET $1,3/4", MMixInstruction::SETL(1, 0));
        assert_first_instruction("SET $1,(3 / 4)", MMixInstruction::SETL(1, 0));
        assert_first_instruction("SET $1,8/4", MMixInstruction::SETL(1, 2));
    }

    #[test]
    fn test_whitespace_around_commas_in_operand_lists_still_parses() {
        assert_first_instruction_matches("TRAP 0, Time, 2", |inst| {
            matches!(inst, MMixInstruction::TRAP(0, _, 2))
        });
        assert_first_instruction("SETI $2, 10", MMixInstruction::SET(2, 10));
    }

    #[test]
    fn test_unrecognized_opcode_after_a_label_is_rejected() {
        // `ADDx` fails `word_end`, so it never matches ADD; nothing follows
        // it but `a,b,1` (register aliases via IS), so `ADDx` reads as a
        // bare label. A label statement holds nothing but blanks and a
        // comment, so this is a syntax error -- but `ADDx` is a perfectly
        // valid label, and `a` alone is just as plausible a culprit as
        // `ADDx`, so the diagnostic prints the whole statement and leaves
        // the reader to place the fault, rather than guess a single word.
        let source = "a IS $1\nb IS $2\nADDx a,b,1";
        assert!(assemble_err(source).contains("unknown operation: ADDx a,b,1"));
    }

    #[test]
    fn test_known_directive_missing_its_operand_is_not_unknown_operation() {
        // `IS`, `LOC` and `SET` are all real keywords; each is just
        // missing what must follow it. The branch that rejects a truly
        // unrecognized opcode must not fire here -- these are malformed,
        // not unknown -- so pest's own "expected ..." diagnostic surfaces
        // instead, the same shape base reports. `GREG`'s operand is
        // optional (an empty field holds 0), so `Foo GREG` assembles and
        // does not belong in this list.
        for source in ["Foo IS", "Foo LOC", "Foo SET"] {
            let err = assemble_err(source);
            assert!(
                !err.contains("unknown operation"),
                "{source:?} must not be misreported as an unknown operation: {err}"
            );
        }
    }

    #[test]
    fn test_debug_directive_followed_by_a_semicolon_is_rejected() {
        // The closing quote must be followed by nothing but blanks, a `%`
        // comment, or end of line; `; HALT` fails that, so the preprocessor
        // leaves the line untouched and the statement -- `Main` plus the
        // unrecognized `debug "hi"` -- is printed whole rather than
        // silently dropped.
        let source = "\tLOC\t#100\nMain\tdebug \"hi\" ; HALT\n";
        assert!(assemble_err(source).contains("unknown operation: Main\tdebug \"hi\""));
    }

    #[test]
    fn test_unclosed_group_check_ignores_parens_in_other_statements() {
        // "ADDx a,b" is unrecognized on its own -- independent of the
        // second statement's unclosed group in "(4+5" -- so the first
        // statement's own diagnostic must print its own text alone, not
        // borrow "unterminated group" from a sibling's parens that
        // whole-line scanning would have seen and this statement's own
        // (paren-free) text does not have.
        let err = assemble_err("ADDx a,b;SET $2,(4+5");
        assert!(
            err.contains("unknown operation: ADDx a,b"),
            "expected the first statement's own diagnostic, got: {err}"
        );
        assert!(
            !err.contains("unterminated group"),
            "must not borrow the second statement's unclosed group, got: {err}"
        );
    }

    #[test]
    fn test_unknown_operation_statement_keeps_a_percent_inside_a_literal() {
        // A `%` inside a string or character literal is ordinary text,
        // not this release's comment opener -- the same rule `BYTE ";"`
        // relies on for `;`. Cutting the printed statement at the first
        // `%` anywhere, ignoring literal contents, truncates it mid-quote
        // and shows the reader an unterminated string they never wrote.
        let err = assemble_err(r#"ADDx "50%",b"#);
        assert!(
            err.contains(r#"unknown operation: ADDx "50%",b"#),
            "expected the literal's `%` to survive intact, got: {err}"
        );
        let err = assemble_err("ADDx '%',b");
        assert!(
            err.contains("unknown operation: ADDx '%',b"),
            "expected the literal's `%` to survive intact, got: {err}"
        );
        // A character literal's scan must consume exactly one character
        // plus its closing quote: it must not over-consume into whatever
        // follows just because that character happens to be a backslash.
        let err = assemble_err("ADDx '\\'\"50%\"");
        assert!(
            err.contains("unknown operation: ADDx '\\'\"50%\""),
            "expected both literals to survive intact, got: {err}"
        );
        let err = assemble_err("ADDx '\\''%'");
        assert!(
            err.contains("unknown operation: ADDx '\\''%'"),
            "expected both literals to survive intact, got: {err}"
        );
    }

    #[test]
    fn test_unclosed_group_check_reaches_a_statement_after_a_valid_one() {
        // The first statement is fully valid ("Main HALT"), its own
        // trailing "note)" a balanced-looking but net-closing paren that
        // ignored commentary never checks; the second, past the `;`,
        // opens a group it never closes. A whole-line scan sums both
        // statements' parens together (0 opens, 1 close, then 1 open) to
        // a NET-BALANCED total and misses the real problem entirely;
        // scoped to its own statement, the second statement's own text
        // alone is unclosed and is reported as such, proving both that
        // `segment_start` correctly advances past a successful first
        // statement and that the check is genuinely per-statement, not
        // whole-line.
        let err = assemble_err("Main HALT note);SET $2,(4+5");
        assert!(
            err.contains("<test>:1:21:") && err.contains("unterminated group"),
            "expected an unterminated-group diagnostic at column 21, got: {err}"
        );
    }

    #[test]
    fn test_unclosed_paren_in_ignored_commentary_is_not_an_unterminated_group() {
        // The guard that catches a real unclosed group only applies to a
        // label that never resolved to an instruction or directive.
        // "HALT" fully matches on its own, so "note (see below" is
        // ordinary ignored commentary -- a stray `(` there is not a
        // group, and must not be reported as one.
        let mut asm = MMixAssembler::new("Main HALT note (see below", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[0].1, MMixInstruction::HALT);
    }

    #[test]
    fn test_unclosed_group_check_ignores_parens_in_a_string_literal() {
        // A `(` quoted inside a string is data, not a group opener; the
        // unclosed-group check must not count it.
        let mut asm = MMixAssembler::new("Main SET $1,5 stray;BYTE \"(\"", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
        assert_eq!(asm.instructions[1].1, MMixInstruction::BYTE(b'('));
    }

    #[test]
    fn test_unclosed_group_check_ignores_parens_in_a_string_literal_reduced() {
        let mut asm = MMixAssembler::new("Main BYTE \"(\" stray", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(b'('));
    }

    // ---- Comment and ignored-remark map (C9.2) --------------------------
    //
    // Three independent mechanisms produce this map:
    // `blank_whole_line_comments` decides, from column 1 alone, whether a
    // line is a label candidate at all; the grammar's `;` separator decides
    // where one statement ends and the next begins; and the remark check
    // decides whether a statement's candidate remark is permitted
    // commentary or a fault. A cell below is named for the mechanism that
    // decides it.

    // -- Column 1: every marker discards the line -------------------------
    //
    // Each payload abuts its operand (`2abc`), which is a syntax error if
    // parsed at all, so a passing assertion proves the line was discarded,
    // never merely tolerated.

    #[test]
    fn test_percent_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new("%SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_hash_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new("#SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_bang_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new("!SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_dot_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new(".SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_at_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new("@SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_semicolon_in_column_one_discards_the_line_as_a_label_rule_not_a_comment_rule() {
        // `;` opens no comment syntax of its own; it is discarded here only
        // because it is not a letter, digit, `:` or `_` -- the same reason
        // `#` and `*` are discarded on this row.
        let mut asm = MMixAssembler::new(";SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_asterisk_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new("*SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_slash_in_column_one_discards_the_line() {
        let mut asm = MMixAssembler::new("/SETL $1,2abc", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.instructions.is_empty());
    }

    #[test]
    fn test_a_letter_colon_or_underscore_in_column_one_still_opens_a_label() {
        // The pairing that makes column 1 a label rule rather than a
        // comment rule: the set that opens a statement here is exactly the
        // set the blanking predicate keeps.
        let mut asm = MMixAssembler::new("Main HALT\n:Foo HALT\n_Bar HALT", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.labels.get("Main"), Some(&0));
        // `:Foo` names the same root symbol as `Foo`, keyed without the
        // colon.
        assert_eq!(asm.labels.get("Foo"), Some(&4));
        assert_eq!(asm.labels.get("_Bar"), Some(&8));
    }

    // -- Indented alone: a marker line between two real instructions -----
    //
    // Indentation puts the line past `blank_whole_line_comments`'s reach --
    // that mechanism decides from column 1 alone. What happens to the line
    // instead is decided by pest's `COMMENT` for `%`, the grammar's `;`
    // separator, and the remark check for `*`, `/` and no marker. `#`, `!`,
    // `.` and `@` carry a `; SETL $2,2` tail and assert the second
    // statement still assembles -- without the tail these assertions would
    // stay green even if the marker became a true comment character.

    #[test]
    fn test_percent_indented_alone_is_a_comment_the_tailed_statement_is_lost() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    % note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 1);
    }

    #[test]
    fn test_hash_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    # note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_bang_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    ! note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_dot_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    . note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_at_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    @ note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_semicolon_indented_alone_opens_an_empty_statement_then_an_unknown_operation() {
        // The indented `;` opens an empty first statement, per the
        // grammar's `;` separator, not a comment; the prose after it is
        // then read as its own statement, a bare word read as a label with
        // text trailing it.
        let err = assemble_err("SETL $1,1\n    ; note text\nSETL $3,3");
        assert!(
            err.contains("unknown operation: note text"),
            "expected the prose after the indented `;` to be an unknown \
             operation, got: {err}"
        );
    }

    #[test]
    fn test_semicolon_indented_alone_lone_word_defines_a_label() {
        // a lone word after ';' is defined as a label
        let mut asm = MMixAssembler::new("SETL $1,1\n    ; counter\nSET $2,counter", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.labels.get("counter"), Some(&4));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 4));
    }

    #[test]
    fn test_semicolon_indented_alone_defines_an_is_constant() {
        let mut asm = MMixAssembler::new("SETL $1,1\n    ; offset IS 8\nSET $2,offset", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 8));
    }

    #[test]
    fn test_asterisk_indented_alone_is_rejected_as_a_dropped_operator() {
        // No statement precedes this line's candidate remark, so a failed
        // remark reports an unknown operation, not a remark diagnostic.
        assert!(
            assemble_err("SETL $1,1\n    * note text\nSETL $3,3")
                .contains("unknown operation: * note text",)
        );
    }

    #[test]
    fn test_slash_indented_alone_is_rejected_as_a_dropped_operator() {
        assert!(
            assemble_err("SETL $1,1\n    / note text\nSETL $3,3")
                .contains("unknown operation: / note text",)
        );
    }

    #[test]
    fn test_no_marker_indented_alone_is_an_unknown_operation() {
        // No marker at all: the indented prose's first word reads as a
        // label, and the second word is text a label statement cannot
        // carry, so together they are an unknown operation.
        assert!(
            assemble_err("SETL $1,1\n    note text\nSETL $3,3")
                .contains("unknown operation: note text",)
        );
    }

    // -- Trailing: the same run after a complete statement ---------------
    //
    // The trailing `;` cell (four outcomes) is pinned separately below; it
    // is not one of these.

    #[test]
    fn test_percent_wins_over_a_later_semicolon_dropping_the_second_statement() {
        // `%` beats a later `;` because it is a pest implicit comment,
        // consumed before the `;`-loop ever runs.
        let mut asm = MMixAssembler::new("SETL $1,1 % note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 1);
    }

    #[test]
    fn test_hash_trailing_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1 # note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_bang_trailing_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1 ! note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_dot_trailing_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1 . note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_at_trailing_is_ignored_the_tailed_statement_still_assembles() {
        let mut asm = MMixAssembler::new("SETL $1,1 @ note; SETL $2,2", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_asterisk_trailing_is_rejected_as_a_dropped_operator() {
        assert!(assemble_err("SETL $1,1 * note").contains("a remark cannot begin with"));
    }

    #[test]
    fn test_slash_trailing_is_rejected_as_a_dropped_operator() {
        assert!(assemble_err("SETL $1,1 / note").contains("a remark cannot begin with"));
    }

    // -- The three ambiguities that disqualify a remark --------------------

    #[test]
    fn test_abutting_remark_must_be_separated_by_a_blank() {
        assert!(assemble_err("SETL $1,2abc").contains("separated from the statement by a blank"));
    }

    #[test]
    fn test_operator_led_remark_errors_for_every_operator_char() {
        for c in [',', '+', '-', '*', '/', '~', '&', '|', '^', '<', '>', '$'] {
            assert!(
                assemble_err(&format!("SETL $1,2 {c} 3")).contains("a remark cannot begin with")
            );
        }
    }

    #[test]
    fn test_digit_led_remark_errors_as_a_dropped_separator() {
        assert!(assemble_err("HALT 2 apples").contains("a remark cannot begin with"));
    }

    #[test]
    fn test_remark_opening_with_a_letter_is_ignored() {
        assert_first_instruction(
            "ADD $1,$2,$3 sum of the parts",
            MMixInstruction::ADD(1, 2, 3),
        );
    }

    #[test]
    fn test_remark_opening_with_underscore_or_colon_is_ignored_too() {
        assert_first_instruction("ADD $1,$2,$3 _underscore", MMixInstruction::ADD(1, 2, 3));
        assert_first_instruction("ADD $1,$2,$3 :colon", MMixInstruction::ADD(1, 2, 3));
    }

    // -- The trailing `;`: four outcomes -----------------------------------

    #[test]
    fn test_trailing_semicolon_lone_word_defines_a_label() {
        // a lone word after ';' is defined as a label
        let mut asm = MMixAssembler::new("SET $1,0 ; counter\nSET $2,counter", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.labels.get("counter"), Some(&4));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 4));

        // Control: behind a real comment, `counter` is never defined.
        assert!(
            assemble_err("SET $1,0 % counter\nSET $2,counter")
                .contains("Undefined symbol: counter",)
        );
    }

    #[test]
    fn test_trailing_semicolon_lone_word_defines_an_is_constant() {
        // IS matches in upper case only; lower-case `is` is not the
        // directive.
        let mut asm = MMixAssembler::new("SET $1,0 ; offset IS 8\nSET $2,offset", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 8));

        // Control: behind a real comment, `offset` is never defined.
        assert!(
            assemble_err("SET $1,0 % offset IS 8\nSET $2,offset")
                .contains("Undefined symbol: offset",)
        );
    }

    #[test]
    fn test_trailing_semicolon_prose_that_reads_as_a_bad_expression_is_an_error() {
        // IS matches in upper case only.
        assert!(assemble_err("SET $1,0 ; this IS invalid").contains("Undefined symbol: invalid"));
    }

    #[test]
    fn test_trailing_semicolon_prose_that_reads_as_an_unknown_operation_is_an_error() {
        assert!(
            assemble_err("SET $1,0 ; set the counter")
                .contains("unknown operation: set the counter",)
        );
    }

    // -- The shield: `#` carries no comment meaning of its own -----------

    #[test]
    fn test_hash_shield_ignores_arbitrary_trailing_prose() {
        assert_first_instruction(
            "SET $1,0 # anything at all here",
            MMixInstruction::SETL(1, 0),
        );
    }

    #[test]
    fn test_bare_trailing_hash_assembles_like_no_remark_at_all() {
        assert_first_instruction("SET $1,0 #", MMixInstruction::SETL(1, 0));
    }

    #[test]
    fn test_hash_shield_ignores_digit_led_prose_that_would_otherwise_error() {
        assert_first_instruction("SET $1,0 # 2 apples", MMixInstruction::SETL(1, 0));
        // Without the shield, a digit-led run is the deliberate exception:
        // an error, not a warning.
        assert!(assemble_err("SET $1,0 2 apples").contains("a remark cannot begin with"));
    }

    // -- The three remaining pins -----------------------------------------

    #[test]
    fn test_percent_inside_a_literal_is_ordinary_text_not_a_comment() {
        let mut asm = MMixAssembler::new(r#"BYTE "50%""#, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(
            asm.instructions
                .iter()
                .map(|(_, i)| i.clone())
                .collect::<Vec<_>>(),
            vec![
                MMixInstruction::BYTE(b'5'),
                MMixInstruction::BYTE(b'0'),
                MMixInstruction::BYTE(b'%'),
            ]
        );

        // The same literal survives intact in the unknown-operation
        // diagnostic rather than truncating at the `%`.
        assert!(assemble_err(r#"ADDx "50%",b"#).contains(r#"unknown operation: ADDx "50%",b"#));
    }

    #[test]
    fn test_at_is_a_valid_operand_but_bang_and_dot_have_no_grammar_token() {
        assert_first_instruction("SET $1,@", MMixInstruction::SETL(1, 0));
        assert!(assemble_err("SET $1,!").contains("unknown operation: SET $1,!"));
        assert!(assemble_err("SET $1,.").contains("unknown operation: SET $1,."));
    }

    // ---- Remark diagnostics: full message, every position row ----

    #[test]
    fn test_remark_ambiguity_messages_pin_position_and_text() {
        assert_eq!(
            assemble_err("SETL $1,2abc"),
            "<test>:1:10: syntax error: a remark must be separated from the statement by a blank"
        );
        assert_eq!(
            assemble_err("HALT+3"),
            "<test>:1:5: syntax error: a remark must be separated from the statement by a blank"
        );
        assert_eq!(
            assemble_err("SETL $1,2 + 3"),
            "<test>:1:11: syntax error: a remark cannot begin with `+` — it reads as part of \
             the statement; start a comment with `%`"
        );
        assert_eq!(
            assemble_err("HALT + 3"),
            "<test>:1:6: syntax error: a remark cannot begin with `+` — it reads as part of \
             the statement; start a comment with `%`"
        );
        assert_eq!(
            assemble_err("SET $1,2 , 3"),
            "<test>:1:10: syntax error: a remark cannot begin with `,` — it reads as part of \
             the statement; start a comment with `%`"
        );
        assert_eq!(
            assemble_err("HALT 2 apples"),
            "<test>:1:6: syntax error: a remark cannot begin with `2` — it reads as part of \
             the statement; start a comment with `%`"
        );
    }

    #[test]
    fn test_unknown_operation_with_no_statement_ahead_pins_position_and_text() {
        assert_eq!(
            assemble_err("9Bar\tSETL\t$1,1"),
            "<test>:1:1: syntax error: unknown operation: 9Bar\tSETL\t$1,1"
        );
        assert_eq!(
            assemble_err("SETL $1,1;9foo"),
            "<test>:1:11: syntax error: unknown operation: 9foo"
        );
        assert_eq!(
            assemble_err("SETL $1,1 ;+3"),
            "<test>:1:12: syntax error: unknown operation: +3"
        );
        assert_eq!(
            assemble_err("\tSETL $1,1\n    * note"),
            "<test>:2:5: syntax error: unknown operation: * note"
        );
        assert_eq!(
            assemble_err("  2 apples"),
            "<test>:1:3: syntax error: unknown operation: 2 apples"
        );
    }

    // ---- Local symbols ------------------------------------------------

    #[test]
    fn test_local_labels_forward_and_backward_meet_in_the_middle() {
        // The reference's own idiom: the first jumps to the second and the
        // second jumps back to the first -- a same-line local reference
        // never resolves to that same line's own (not yet recorded)
        // occurrence.
        let mut asm = MMixAssembler::new("2H      JMP 2F\n2H      JMP 2B\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].0, 0x0);
        assert_eq!(asm.instructions[0].1, MMixInstruction::JMP(1));
        assert_eq!(asm.instructions[1].0, 0x4);
        assert_eq!(asm.instructions[1].1, MMixInstruction::JMPB(0xFFFFFF));
    }

    #[test]
    fn test_local_label_on_a_greg_line_keeps_both_passes_in_step() {
        let parse = |src: &str| {
            let mut asm = MMixAssembler::new(src, "<test>");
            asm.parse().unwrap();
            asm.instructions
        };
        let local = parse("2H      GREG 0\nMain    SET $1,2B\n        SET $2,2F\n2H      HALT\n");
        let named = parse("R       GREG 0\nMain    SET $1,R\n        SET $2,L\nL       HALT\n");
        assert_eq!(local, named);
    }

    #[test]
    fn test_local_back_reference_before_any_definition_is_zero() {
        // `2B` ahead of any `2H` is `0`, never an error.
        assert_first_instruction("OCTA 2B", MMixInstruction::OCTA(0));
    }

    #[test]
    fn test_local_forward_reference_with_no_later_definition_is_undefined() {
        assert!(assemble_err("OCTA 2F").contains("Undefined symbol: 2F"));
    }

    #[test]
    fn test_local_labels_are_independent_per_digit() {
        let mut asm = MMixAssembler::new(
            "1H      HALT\n\
             2H      HALT\n\
             Main    SET $1,1B\n\
                     SET $2,2B\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[2].1, MMixInstruction::SETL(1, 0x0));
        assert_eq!(asm.instructions[3].1, MMixInstruction::SETL(2, 0x4));
    }

    #[test]
    fn test_second_local_label_redefines_rather_than_erroring() {
        let mut asm = MMixAssembler::new(
            "2H      HALT\n\
             2H      HALT\n\
             Main    SET $1,2B\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[2].1, MMixInstruction::SETL(1, 0x4));
    }

    #[test]
    fn test_local_label_is_directive_counts_like_a_running_counter() {
        // The reference's own idiom: `9H IS 9B+1` twice leaves the counter
        // at 2 (0 -> 1 -> 2).
        assert_first_instruction(
            "9H IS 0\n\
                 9H IS 9B+1\n\
                 9H IS 9B+1\n\
                 Main SET $1,9B\n",
            MMixInstruction::SETL(1, 2),
        );
    }

    #[test]
    fn test_local_label_alone_on_a_line_takes_the_unrounded_counter() {
        let mut asm = MMixAssembler::new(
            "Main    BYTE 1\n\
             2H\n\
                     OCTA 2B\n",
            "<test>",
        );
        asm.parse().unwrap();
        // 2H sits right after the one BYTE, unrounded (address 1); the
        // following OCTA still aligns to 8.
        let octa = asm
            .instructions
            .iter()
            .find(|(_, i)| matches!(i, MMixInstruction::OCTA(_)))
            .unwrap();
        assert_eq!(octa.1, MMixInstruction::OCTA(1));
        assert_eq!(octa.0, 8);
    }

    #[test]
    fn test_lowercase_local_symbols_are_rejected() {
        assert!(assemble_err("2h SET $1,0").contains("syntax error"));
        assert!(assemble_err("Main SET $1,2b").contains("syntax error"));
    }

    #[test]
    fn test_local_back_reference_after_loc_moves_backward_sees_source_order() {
        // Resolution is by source order, not by address: a LOC that moves
        // the counter backward still leaves a later `2B` seeing the
        // textually preceding `2H`.
        let mut asm = MMixAssembler::new(
            "2H      HALT\n\
             LOC #100\n\
             LOC #10\n\
             Main    SET $1,2B\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(1, 0));
    }

    #[test]
    fn test_local_label_h_as_an_operand_is_an_error() {
        assert!(assemble_err("Main SET $1,2H").contains("syntax error"));
    }

    #[test]
    fn test_local_ref_b_in_the_label_field_is_an_error() {
        assert!(assemble_err("2B JMP Main\nMain HALT\n").contains("unknown operation"));
    }

    #[test]
    fn test_digit_literal_operand_forms_are_unaffected() {
        assert_first_instruction("SET $1,2", MMixInstruction::SETL(1, 2));
        assert_first_instruction("SET $1,#2B", MMixInstruction::SETL(1, 0x2B));
        assert_first_instruction("SET $1,0x2B", MMixInstruction::SETL(1, 0x2B));
        assert_first_instruction("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3));
        assert!(assemble_err("SETL $1,2abc").contains("syntax error"));
    }

    #[test]
    fn test_set_1_2f_now_resolves_where_it_once_errored() {
        let mut asm = MMixAssembler::new("Main SET $1,2F\n2H HALT\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 0x4));
    }

    #[test]
    fn test_local_forward_reference_resolves_where_named_ones_do() {
        let mut asm = MMixAssembler::new(
            "Main    JMP 2F-4\n\
                     OCTA 2F\n\
             2H      HALT\n",
            "<test>",
        );
        asm.parse().unwrap();
        // 2H sits at 0x10 (JMP at 0x0..0x3, OCTA aligned at 0x8..0xF).
        assert!(
            asm.instructions
                .iter()
                .any(|(_, i)| matches!(i, MMixInstruction::OCTA(v) if *v == 0x10))
        );
    }

    #[test]
    fn test_local_forward_reference_as_is_loc_greg_operand_is_undefined() {
        assert!(assemble_err("Foo IS 2F\nMain HALT\n").contains("Undefined symbol: 2F"));
        assert!(assemble_err("2H IS 2F+1\nMain HALT\n").contains("Undefined symbol: 2F"));
        assert!(assemble_err("LOC 2F\nMain HALT\n").contains("Undefined symbol: 2F"));
        assert!(assemble_err("G1 GREG 2F\nMain HALT\n").contains("Undefined symbol: 2F"));
    }

    // ---- Qualified references -----------------------------------------

    #[test]
    fn test_qualified_reference_reads_a_prefix_definition_from_outside() {
        let mut asm = MMixAssembler::new(
            "PREFIX Foo:\n\
             Bar IS 5\n\
             PREFIX :\n\
             Main SET $1,Foo:Bar\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_qualified_reference_three_part() {
        let mut asm = MMixAssembler::new(
            "PREFIX Foo:Bar:\n\
             Baz IS 7\n\
             PREFIX :\n\
             Main SET $1,Foo:Bar:Baz\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 7));
    }

    #[test]
    fn test_leading_colon_qualified_reference_opts_out_of_prefix() {
        let mut asm = MMixAssembler::new(
            "Foo IS 3\n\
             PREFIX Sub_\n\
             Main SET $1,:Foo\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 3));
    }

    #[test]
    fn test_label_with_trailing_colon_and_blank_still_defines_the_plain_name() {
        let mut asm = MMixAssembler::new("Main: SET $1,0\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Main"), Some(&0));
        assert!(!asm.labels.contains_key("Main:"));
    }

    #[test]
    fn test_label_with_trailing_colon_and_no_blank_is_now_an_error() {
        // The accepted break: interior colons make `Main:SET` one symbol,
        // so `Main:SET $1,0` no longer defines `Main`.
        assert!(assemble_err("Main:SET $1,0").contains("syntax error"));
    }

    #[test]
    fn test_qualified_definition_in_the_label_field() {
        let mut asm = MMixAssembler::new(
            "PREFIX Foo:\n\
             Bar HALT\n\
             PREFIX :\n\
             Main SET $1,Foo:Bar\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Foo:Bar"), Some(&0));
    }

    #[test]
    fn test_prefix_colon_foo_colon_composes_like_a_relative_reference() {
        let mut asm = MMixAssembler::new(
            "PREFIX :Foo:\n\
             bar IS 5\n\
             PREFIX :\n\
             Main SET $1,Foo:bar\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    }

    // ---- LOCAL ---------------------------------------------------------

    #[test]
    fn test_local_directive_with_a_global_register_assembles() {
        let mut asm = MMixAssembler::new("LOCAL $10\nMain HALT\n", "<test>");
        asm.parse().unwrap();
    }

    #[test]
    fn test_local_directive_bare_value_draws_the_register_required_diagnostic() {
        assert!(
            assemble_err("LOCAL 10\nMain HALT\n")
                .contains("pure value 10 cannot be used where a register is required",)
        );
    }

    #[test]
    fn test_local_directive_at_or_above_the_threshold_fails_naming_both() {
        let err = {
            let mut asm = MMixAssembler::new("G1 GREG 0\nLOCAL $254\nMain HALT\n", "<test>");
            asm.parse().expect_err("expected threshold error")
        };
        assert!(err.contains("$254"), "err: {err}");
        assert!(err.contains("threshold"), "err: {err}");
    }

    #[test]
    fn test_local_directive_below_32_never_fails_regardless_of_gregs() {
        let mut asm = MMixAssembler::new("LOCAL $5\nMain HALT\n", "<test>");
        asm.parse().unwrap();
    }

    #[test]
    fn test_local_directive_with_a_label_is_an_error() {
        assert!(assemble_err("Foo LOCAL $10\nMain HALT\n").contains("takes no label"));
    }

    // ---- BSPEC / ESPEC -------------------------------------------------

    #[test]
    fn test_espec_label_address_matches_the_block_deleted() {
        let mut with_block = MMixAssembler::new(
            "Main    SET $1,0\n\
             BSPEC 1\n\
             BYTE 1,2,3,4,5\n\
             ESPEC\n\
             After   SET $2,0\n",
            "<test>",
        );
        with_block.parse().unwrap();
        let mut without_block = MMixAssembler::new("Main SET $1,0\nAfter SET $2,0\n", "<test>");
        without_block.parse().unwrap();
        assert_eq!(
            with_block.labels.get("After"),
            without_block.labels.get("After")
        );
    }

    #[test]
    fn test_bspec_block_emits_no_instructions() {
        let mut asm = MMixAssembler::new(
            "Main SET $1,0\nBSPEC 1\nBYTE 1,2,3\nESPEC\nHALT\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_bspec_allows_greg_and_is_with_full_effect() {
        let mut asm = MMixAssembler::new(
            "BSPEC 1\n\
             G1 GREG 0\n\
             Foo IS 5\n\
             ESPEC\n\
             Main SET $1,Foo\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
        assert!(asm.symbols.contains_key("G1"));
    }

    #[test]
    fn test_bspec_rejects_an_instruction() {
        assert!(
            assemble_err("BSPEC 1\nSET $1,0\nESPEC\nMain HALT\n")
                .contains("not allowed inside BSPEC/ESPEC",)
        );
    }

    #[test]
    fn test_bspec_rejects_loc() {
        assert!(
            assemble_err("BSPEC 1\nLOC #200\nESPEC\nMain HALT\n")
                .contains("not allowed inside BSPEC/ESPEC",)
        );
    }

    #[test]
    fn test_bspec_with_no_espec_is_an_error() {
        assert!(assemble_err("BSPEC 1\nFoo IS 5\n").contains("BSPEC"));
    }

    #[test]
    fn test_espec_with_no_bspec_is_an_error() {
        assert!(assemble_err("ESPEC\nMain HALT\n").contains("ESPEC"));
    }

    #[test]
    fn test_bspec_does_not_nest() {
        assert!(
            assemble_err("BSPEC 1\nBSPEC 2\nESPEC\nESPEC\nMain HALT\n").contains("does not nest",)
        );
    }

    #[test]
    fn test_bspec_operand_wider_than_two_bytes_is_an_error() {
        assert!(
            assemble_err("BSPEC #10000\nESPEC\nMain HALT\n").contains("does not fit in two bytes",)
        );
    }

    // ---- The predefined symbols -----------------------------------------

    #[test]
    fn test_seventeen_predefined_symbols_resolve_to_the_reference_table() {
        let cases: &[(&str, u64)] = &[
            ("Inf", 0x7FF0000000000000),
            ("D_BIT", 0x80),
            ("D_Handler", 0x10),
            ("V_BIT", 0x40),
            ("V_Handler", 0x20),
            ("W_BIT", 0x20),
            ("W_Handler", 0x30),
            ("I_BIT", 0x10),
            ("I_Handler", 0x40),
            ("O_BIT", 0x08),
            ("O_Handler", 0x50),
            ("U_BIT", 0x04),
            ("U_Handler", 0x60),
            ("Z_BIT", 0x02),
            ("Z_Handler", 0x70),
            ("X_BIT", 0x01),
            ("X_Handler", 0x80),
        ];
        for (name, value) in cases {
            assert_first_instruction(&format!("OCTA {name}"), MMixInstruction::OCTA(*value));
            // The root-colon spelling reaches the same value.
            assert_first_instruction(&format!("OCTA :{name}"), MMixInstruction::OCTA(*value));
        }
    }

    #[test]
    fn test_text_segment_is_still_undefined() {
        // The reference's predefined-symbol table has no `Text_Segment`.
        assert!(assemble_err("OCTA Text_Segment").contains("Undefined symbol"));
    }

    // ---- The root prefix ------------------------------------------------

    #[test]
    fn test_root_prefix_row_prefix_pk_then_reset() {
        let mut asm = MMixAssembler::new("v IS 7\nPREFIX Pk:\nPREFIX :\nMain SET $0,v\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(0, 7));
    }

    #[test]
    fn test_root_prefix_row_colon_x_reference() {
        let mut asm = MMixAssembler::new("x IS 5\nMain SET $1,:x\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_root_prefix_row_prefix_foo_then_reset() {
        let mut asm = MMixAssembler::new(
            "PREFIX Foo\nbar IS 5\nPREFIX :\nMain SET $1,Foobar\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    }

    #[test]
    fn test_root_prefix_main_key_carries_no_colon() {
        let mut asm = MMixAssembler::new("PREFIX :\nMain HALT\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Main"), Some(&0));
        assert!(!asm.labels.contains_key(":Main"));
    }

    #[test]
    fn test_root_prefix_x_then_colon_x_is_the_redefinition_error() {
        assert!(assemble_err("x IS 1\n:x IS 2\nMain HALT\n").contains("redefined"));
    }

    #[test]
    fn test_root_prefix_labels_keys_colon_lib_without_colon() {
        let mut asm = MMixAssembler::new(":Lib HALT\nMain HALT\n", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Lib"), Some(&0));
        assert!(!asm.labels.contains_key(":Lib"));
    }

    // ---- Predefined names: a program's own definition wins --------------

    #[test]
    fn test_label_named_predefined_wins_for_a_later_reference() {
        let mut asm = MMixAssembler::new(
            "LOC #108\nFputs HALT\nLOC #100\nMain SET $1,Fputs\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(1, 0x108));
    }

    #[test]
    fn test_label_named_predefined_reaches_a_pushj_target() {
        let mut asm = MMixAssembler::new(
            "LOC #108\nTime POP 0,0\nLOC #100\nMain PUSHJ $0,Time\n",
            "<test>",
        );
        asm.parse().unwrap();
        assert!(matches!(asm.instructions[1].1, MMixInstruction::PUSHJ(..)));
    }

    #[test]
    fn test_use_then_redefine_via_label_is_an_error() {
        let err = {
            let mut asm = MMixAssembler::new("Main SET $1,Fputs\nFputs HALT\n", "<test>");
            asm.parse().expect_err("expected used-then-redefined error")
        };
        assert!(
            err.contains("predefined symbol 'Fputs' redefined"),
            "err: {err}"
        );
        assert!(err.contains("its value was used at"), "err: {err}");
    }

    #[test]
    fn test_use_then_redefine_via_is_is_an_error() {
        let err = {
            let mut asm = MMixAssembler::new("Main SET $1,Fputs\nFputs IS 3\n", "<test>");
            asm.parse().expect_err("expected used-then-redefined error")
        };
        assert!(
            err.contains("predefined symbol 'Fputs' redefined"),
            "err: {err}"
        );
        assert!(err.contains("its value was used at"), "err: {err}");
    }

    #[test]
    fn test_second_different_definition_after_a_label_is_the_ordinary_redefinition_error() {
        assert!(
            assemble_err("Fputs HALT\nFputs IS 3\nMain HALT\n")
                .contains("symbol 'Fputs' redefined",)
        );
    }

    // ---- Equal redefinition ----------------------------------------------

    #[test]
    fn test_equal_redefinition_is_then_label_same_value_assembles() {
        let mut asm = MMixAssembler::new("Here IS #104\nLOC #104\nHere HALT\n", "<test>");
        asm.parse().unwrap();
    }

    #[test]
    fn test_equal_redefinition_is_then_label_different_value_is_an_error() {
        assert!(
            assemble_err("Here IS #104\nLOC #108\nHere HALT\n")
                .contains("symbol 'Here' redefined",)
        );
    }

    #[test]
    fn test_equal_redefinition_register_vs_pure_value_is_an_error() {
        assert!(assemble_err("x IS $1\nx IS 1\nMain HALT\n").contains("symbol 'x' redefined"));
    }

    // ---- The two-operand memory form (base-address search) -------------

    #[test]
    fn test_base_address_form_resolves_against_preceding_greg() {
        assert_first_instruction(
            "Base GREG #1000\nData IS #1000\nLDO $1,Data",
            MMixInstruction::LDOI(1, 254, 0),
        );
    }

    #[test]
    fn test_base_address_form_offset_255_is_the_widest_accepted() {
        assert_first_instruction(
            "Base GREG #1000\nData IS #10FF\nLDO $1,Data",
            MMixInstruction::LDOI(1, 254, 255),
        );
    }

    #[test]
    fn test_base_address_form_offset_256_is_an_error() {
        assert!(
            assemble_err("Base GREG #1000\nData IS #1100\nLDO $1,Data")
                .contains("no GREG before this instruction holds a base address")
        );
    }

    #[test]
    fn test_base_address_form_closer_greg_wins() {
        // Far allocates $254, Near allocates $253; Near's base (#1080) is
        // closer to Data (#1090) than Far's (#1000).
        assert_first_instruction(
            "Far GREG #1000\nNear GREG #1080\nData IS #1090\nLDO $1,Data",
            MMixInstruction::LDOI(1, 253, 16),
        );
    }

    #[test]
    fn test_base_address_form_tie_takes_the_earliest_allocated() {
        assert_first_instruction(
            "A GREG #1000\nB GREG #1000\nData IS #1000\nLDO $1,Data",
            MMixInstruction::LDOI(1, 254, 0),
        );
    }

    #[test]
    fn test_base_address_form_ignores_a_greg_after_the_instruction() {
        assert!(
            assemble_err("Data IS #1000\nLDO $1,Data\nLate GREG #1000")
                .contains("no GREG before this instruction holds a base address")
        );
    }

    #[test]
    fn test_base_address_form_greg_zero_never_matches() {
        assert!(
            assemble_err("Zero GREG 0\nData IS #10\nLDO $1,Data")
                .contains("no GREG before this instruction holds a base address")
        );
    }

    #[test]
    fn test_base_address_form_stb_takes_it() {
        assert_first_instruction(
            "Base GREG #1000\nData IS #1000\nSTB $1,Data",
            MMixInstruction::STBI(1, 254, 0),
        );
    }

    #[test]
    fn test_base_address_form_go_takes_it() {
        assert_first_instruction(
            "Base GREG #1000\nData IS #1000\nGO $1,Data",
            MMixInstruction::GOI(1, 254, 0),
        );
    }

    #[test]
    fn test_base_address_form_preld_takes_it() {
        assert_first_instruction(
            "Base GREG #1000\nData IS #1000\nPRELD 3,Data",
            MMixInstruction::PRELDI(3, 254, 0),
        );
    }

    #[test]
    fn test_base_address_form_stco_takes_it() {
        assert_first_instruction(
            "Base GREG #1000\nData IS #1000\nSTCO 5,Data",
            MMixInstruction::STCOI(5, 254, 0),
        );
    }

    #[test]
    fn test_base_address_form_forward_reference_resolves_and_is_one_tetra() {
        let mut asm = MMixAssembler::new("Base GREG #1000\nLDO $1,Data\nData IS #1000", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDOI(1, 254, 0));
        assert_eq!(
            MMixAssembler::instruction_size(&asm.instructions[0].1),
            4,
            "the base-address form is always one tetra"
        );
    }

    #[test]
    fn test_memory_two_operand_register_is_offset_zero() {
        assert_first_instruction("LDO $1,$2", MMixInstruction::LDOI(1, 2, 0));
    }

    #[test]
    fn test_memory_two_operand_register_follows_value_not_spelling() {
        assert_first_instruction("x IS $2\nLDO $1,x", MMixInstruction::LDOI(1, 2, 0));
    }

    #[test]
    fn test_lda_two_operand_form_never_takes_the_base_address_path() {
        // A preceding GREG close to Data does not change LDA's own sizing:
        // LDA expands to SET when the address exceeds one byte.
        assert_first_instruction(
            "Base GREG #1000\nLDA $1,Data\nData IS #1000",
            MMixInstruction::SET(1, 0x1000),
        );
    }

    // ---- Operand counts and kinds ----------------------------------------

    #[test]
    fn test_trap_two_operand_form_splits_yz() {
        assert_first_instruction("TRAP 1,#0203", MMixInstruction::TRAP(1, 2, 3));
    }

    #[test]
    fn test_trap_one_operand_form_splits_xyz() {
        assert_first_instruction("TRAP #010203", MMixInstruction::TRAP(1, 2, 3));
    }

    #[test]
    fn test_trip_two_and_one_operand_forms_split_the_same_way() {
        assert_first_instruction("TRIP 1,#0203", MMixInstruction::TRIP(1, 2, 3));
        assert_first_instruction("TRIP #010203", MMixInstruction::TRIP(1, 2, 3));
    }

    #[test]
    fn test_swym_two_and_one_operand_forms_split_the_same_way() {
        assert_first_instruction("SWYM 1,#0203", MMixInstruction::SWYM(1, 2, 3));
        assert_first_instruction("SWYM #010203", MMixInstruction::SWYM(1, 2, 3));
    }

    #[test]
    fn test_swym_one_operand_is_xyz() {
        assert_first_instruction("SWYM 1", MMixInstruction::SWYM(0, 0, 1));
    }

    #[test]
    fn test_swym_two_operands_splits_yz() {
        assert_first_instruction("SWYM 1,2", MMixInstruction::SWYM(1, 0, 2));
    }

    #[test]
    fn test_pop_one_operand_is_xyz() {
        assert_first_instruction("POP 1", MMixInstruction::POP(0, 0, 1));
    }

    #[test]
    fn test_unsave_one_operand_matches_the_two_operand_spelling() {
        assert_first_instruction("UNSAVE $2", MMixInstruction::UNSAVE(0, 2));
    }

    #[test]
    fn test_neg_two_operand_form_omits_y() {
        assert_first_instruction("NEG $1,5", MMixInstruction::NEGI(1, 0, 5));
        assert_first_instruction("NEGU $1,$0", MMixInstruction::NEGU(1, 0, 0));
    }

    #[test]
    fn test_bare_greg_allocates_a_register_holding_zero() {
        let mut asm = MMixAssembler::new("g GREG\nMain HALT", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.greg_inits.last(), Some(&(254, 0)));
        assert_eq!(asm.symbols.get("g"), Some(&SymbolType::Register(254)));
    }

    #[test]
    fn test_swym_three_registers_assembles() {
        assert_first_instruction("SWYM $5,$6,$7", MMixInstruction::SWYM(5, 6, 7));
    }

    #[test]
    fn test_trap_three_registers_assembles() {
        assert_first_instruction("TRAP $1,$2,$3", MMixInstruction::TRAP(1, 2, 3));
    }

    #[test]
    fn test_preld_and_stco_accept_a_pure_x() {
        assert_first_instruction("PRELD 7,$2,0", MMixInstruction::PRELDI(7, 2, 0));
        assert_first_instruction("STCO $1,$2,0", MMixInstruction::STCOI(1, 2, 0));
    }

    #[test]
    fn test_pushj_pure_x_and_register_x_assemble_the_same_bytes() {
        assert_first_instruction("PUSHJ 0,Sub\nSub HALT", MMixInstruction::PUSHJ(0, 0, 1));
        let by_number = {
            let mut asm = MMixAssembler::new("PUSHJ 2,Sub\nSub HALT", "<test>");
            asm.parse().unwrap();
            asm.instructions[0].1.clone()
        };
        let by_register = {
            let mut asm = MMixAssembler::new("PUSHJ $2,Sub\nSub HALT", "<test>");
            asm.parse().unwrap();
            asm.instructions[0].1.clone()
        };
        assert_eq!(by_number, by_register);
    }

    #[test]
    fn test_pushgo_pure_x_matches_register_x() {
        // Z=0 is a pure value, which auto-selects the immediate opcode
        // regardless of X's spelling.
        assert_first_instruction("PUSHGO 2,$3,0", MMixInstruction::PUSHGOI(2, 3, 0));
        assert_first_instruction("PUSHGO $2,$3,0", MMixInstruction::PUSHGOI(2, 3, 0));
    }

    #[test]
    fn test_go_pure_x_is_still_an_error() {
        assert!(
            assemble_err("GO 2,$3,0")
                .contains("pure value 2 cannot be used where a register is required")
        );
    }

    // ---- No bare mnemonic is a silent label --------------------------------

    #[test]
    fn test_bare_pop_between_instructions_is_the_zero_form() {
        let mut asm = MMixAssembler::new("SET $1,0\n\tPOP\nSET $2,0", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::POP(0, 0, 0));
        assert!(!asm.labels.contains_key("POP"));
    }

    #[test]
    fn test_bare_resume_between_instructions_is_the_zero_form() {
        let mut asm = MMixAssembler::new("SET $1,0\n\tRESUME\nSET $2,0", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::RESUME(0));
        assert!(!asm.labels.contains_key("RESUME"));
    }

    #[test]
    fn test_bare_sync_between_instructions_is_the_zero_form() {
        let mut asm = MMixAssembler::new("SET $1,0\n\tSYNC\nSET $2,0", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::SYNC(0));
        assert!(!asm.labels.contains_key("SYNC"));
    }

    #[test]
    fn test_bare_trap_between_instructions_is_the_zero_form() {
        let mut asm = MMixAssembler::new("SET $1,0\n\tTRAP\nSET $2,0", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::TRAP(0, 0, 0));
        assert!(!asm.labels.contains_key("TRAP"));
    }

    #[test]
    fn test_bare_trip_between_instructions_is_the_zero_form() {
        let mut asm = MMixAssembler::new("SET $1,0\n\tTRIP\nSET $2,0", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[1].1, MMixInstruction::TRIP(0, 0, 0));
        assert!(!asm.labels.contains_key("TRIP"));
    }

    #[test]
    fn test_bare_save_between_instructions_is_unknown_operation() {
        assert_eq!(
            assemble_err("SET $1,0\n\tSAVE\nSET $2,0"),
            "<test>:2:2: syntax error: unknown operation: SAVE"
        );
    }

    #[test]
    fn test_bare_unsave_between_instructions_requires_a_register() {
        assert_eq!(
            assemble_err("SET $1,0\n\tUNSAVE\nSET $2,0"),
            "<test>:2:2: pure value 0 cannot be used where a register is required"
        );
    }

    #[test]
    fn test_save_after_semicolon_is_unknown_operation_not_a_label() {
        assert!(assemble_err("SET $2,2 ; SAVE").contains("unknown operation: SAVE"));
    }

    #[test]
    fn test_column_one_lone_pop_is_a_pop_not_a_label() {
        let mut asm = MMixAssembler::new("POP\nSET $1,0", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[0].1, MMixInstruction::POP(0, 0, 0));
        assert!(!asm.labels.contains_key("POP"));
    }

    // ---- Upper-case opcodes and the indented line --------------------------

    #[test]
    fn test_lowercase_loc_prefix_defines_a_register_label() {
        assert_first_instruction("loc GREG 0\nSET loc,5", MMixInstruction::SETL(254, 5));
    }

    #[test]
    fn test_mixed_case_sync_defines_a_label_not_the_instruction() {
        let mut asm = MMixAssembler::new("Sync BNZ $1,Main\nMain HALT", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.labels.contains_key("Sync"));
    }

    #[test]
    fn test_indented_lowercase_mnemonic_is_unknown_operation() {
        assert!(assemble_err("SET $1,0\n\tset $1,2").contains("unknown operation: set $1,2"));
    }

    #[test]
    fn test_indented_label_with_instruction_is_unknown_operation() {
        assert!(
            assemble_err("SET $1,0\n\tFoo SET $2,9").contains("unknown operation: Foo SET $2,9")
        );
    }

    #[test]
    fn test_indented_label_with_is_directive_is_unknown_operation() {
        assert!(assemble_err("SET $1,0\n\tFoo IS 5").contains("unknown operation: Foo IS 5"));
    }

    #[test]
    fn test_tab_then_carriage_return_still_opens_indented() {
        assert!(
            assemble_err("SET $1,0\n\t\rFoo SET $1,0").contains("unknown operation: Foo SET $1,0")
        );
    }

    #[test]
    fn test_indented_lone_word_is_unknown_operation() {
        assert_eq!(
            assemble_err("SET $1,0\n\tFoo\nSET $2,0"),
            "<test>:2:2: syntax error: unknown operation: Foo"
        );
    }

    #[test]
    fn test_indented_lone_local_label_is_unknown_operation() {
        assert_eq!(
            assemble_err("\tSET $1,0\n\t2H\nSET $2,0"),
            "<test>:2:2: syntax error: unknown operation: 2H"
        );
    }

    #[test]
    fn test_column_one_local_label_with_text_is_unknown_operation() {
        assert_eq!(
            assemble_err("2H note text\n\tTRAP 0,Halt,0"),
            "<test>:1:4: syntax error: unknown operation: 2H note text"
        );
    }

    #[test]
    fn test_column_one_local_label_with_a_digit_is_unknown_operation() {
        assert_eq!(
            assemble_err("2H 5\n\tTRAP 0,Halt,0"),
            "<test>:1:4: syntax error: unknown operation: 2H 5"
        );
    }

    #[test]
    fn test_column_one_local_label_with_a_remark_marker_is_unknown_operation() {
        assert_eq!(
            assemble_err("2H * note\n\tTRAP 0,Halt,0"),
            "<test>:1:4: syntax error: unknown operation: 2H * note"
        );
    }

    #[test]
    fn test_indented_lone_word_with_trailing_blanks_drops_them() {
        assert_eq!(
            assemble_err("SET $1,0\n\tFoo  \nSET $2,0"),
            "<test>:2:2: syntax error: unknown operation: Foo"
        );
    }

    #[test]
    fn test_bare_save_with_a_colon_keeps_it() {
        assert_eq!(
            assemble_err("SET $1,0\n\tSAVE:\nSET $2,0"),
            "<test>:2:2: syntax error: unknown operation: SAVE:"
        );
    }

    #[test]
    fn test_semicolon_lone_word_still_defines_a_label() {
        let mut asm = MMixAssembler::new("SET $2,2 ; loop\nSET $3,loop", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.labels.contains_key("loop"));
    }

    #[test]
    fn test_column_one_lone_word_is_still_a_label() {
        let mut asm = MMixAssembler::new("Loop\nSET $1,Loop", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert!(asm.labels.contains_key("Loop"));
    }

    // ---- Longest form wins; a partial operand list is an error -------------

    #[test]
    fn test_trap_three_operand_form_is_not_swallowed_by_shorter_forms() {
        assert_first_instruction("TRAP 0,1,2", MMixInstruction::TRAP(0, 1, 2));
    }

    #[test]
    fn test_pop_two_operand_form_unchanged_from_base() {
        assert_first_instruction("POP 1,2", MMixInstruction::POP(1, 0, 2));
    }

    #[test]
    fn test_trap_partial_operand_list_is_an_error() {
        assert!(
            assemble_err("TRAP 0,")
                .contains("a remark must be separated from the statement by a blank")
        );
    }

    #[test]
    fn test_pop_partial_operand_list_is_an_error() {
        assert!(
            assemble_err("POP 1,")
                .contains("a remark must be separated from the statement by a blank")
        );
    }

    // ---- The remark boundary against multi-operand counts -------------------

    #[test]
    fn test_swym_one_operand_then_digit_is_a_dropped_operand_error() {
        assert!(assemble_err("SWYM 1 2").contains("a remark cannot begin with `2`"));
    }

    #[test]
    fn test_trap_two_operand_then_digit_is_a_dropped_operand_error() {
        assert!(assemble_err("TRAP 0,1 2").contains("a remark cannot begin with `2`"));
    }

    #[test]
    fn test_trap_blanks_around_comma_still_parse() {
        assert_first_instruction("TRAP 0,1 ,2", MMixInstruction::TRAP(0, 1, 2));
    }

    #[test]
    fn test_swym_one_operand_then_note_is_ignored() {
        assert_first_instruction("SWYM 1 note", MMixInstruction::SWYM(0, 0, 1));
    }

    #[test]
    fn test_bare_swym_then_undefined_word_is_undefined_symbol() {
        assert!(assemble_err("SWYM do nothing").contains("Undefined symbol: do"));
    }

    #[test]
    fn test_bare_swym_then_a_defined_symbol_is_its_address() {
        let mut asm = MMixAssembler::new("JMP Skip\nMain HALT\nSkip SWYM Main", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        assert_eq!(asm.instructions[2].1, MMixInstruction::SWYM(0, 0, 4));
    }

    // ---- Arity unchanged for instructions this unit does not widen ---------

    #[test]
    fn test_save_one_operand_is_still_an_error() {
        assert!(assemble_err("SAVE $2").contains("unknown operation: SAVE $2"));
    }

    #[test]
    fn test_get_with_a_predefined_special_register_name_still_assembles() {
        assert_first_instruction("GET $1,rA", MMixInstruction::GET(1, 21));
    }

    #[test]
    fn test_lowercase_greg_is_not_the_directive() {
        // If lower-case `greg` matched the directive, this would allocate a
        // register holding 0 instead of erroring on the digit-led text
        // after the label `greg`.
        assert!(assemble_err("greg 0").contains("unknown operation: greg 0"));
    }

    // ---- Character and string literals, and wide numeric constants -----

    #[test]
    fn test_char_literal_takes_the_reference_form() {
        // One quote, one character, one quote -- the character may itself
        // be a quote, so `'''` is the apostrophe and `'\'` the backslash.
        // An ordinary letter, digit or operator character between the
        // quotes is its own ASCII value.
        assert_first_instruction("SET $1,'''", MMixInstruction::SETL(1, 39));
        assert_first_instruction("SET $1,'\\'", MMixInstruction::SETL(1, 92));
        assert_first_instruction("SET $1,'A'", MMixInstruction::SETL(1, 65));
        assert_first_instruction("SET $1,'0'", MMixInstruction::SETL(1, 48));
        assert_first_instruction("SET $1,'%'", MMixInstruction::SETL(1, 37));
        assert_first_instruction("SET $1,';'", MMixInstruction::SETL(1, 59));
    }

    #[test]
    fn test_char_literal_takes_any_characters_unicode_scalar_value() {
        // p. 37 rule 2(c): a character constant is the Unicode value of
        // the quoted character.
        assert_first_instruction("SET $1,'é'", MMixInstruction::SETL(1, 0xE9));
        assert_first_instruction("SET $1,'π'", MMixInstruction::SETL(1, 0x3C0));
        assert_first_instruction("SET $1,'Ω'", MMixInstruction::SETL(1, 0x3A9));
    }

    #[test]
    fn test_wyde_char_and_string_literals_take_their_code_point() {
        // p. 37, the paragraph after rule 2: a string stands for the
        // character constants of its characters.
        assert_first_instruction("WYDE '算'", MMixInstruction::WYDE(0x7B97));
        assert_first_instruction("WYDE \"π\"", MMixInstruction::WYDE(0x03C0));
    }

    #[test]
    fn test_octa_string_takes_the_characters_full_code_point() {
        assert_first_instruction("OCTA \"€\"", MMixInstruction::OCTA(0x20AC));
    }

    #[test]
    fn test_wyde_string_combines_its_code_point_with_an_operator() {
        assert_first_instruction("WYDE \"π\"+1", MMixInstruction::WYDE(0x03C1));
    }

    #[test]
    fn test_byte_string_char_below_0x100_takes_its_code_point() {
        // A BYTE string's character takes its code point value; below
        // #100 that value fits the byte directly.
        assert_first_instruction("BYTE \"é\"", MMixInstruction::BYTE(0xE9));
    }

    #[test]
    fn test_wyde_string_label_offset_counts_characters_not_utf8_bytes() {
        // A string contributes one item per character, not per UTF-8
        // byte: "πé" is two WYDE items, six bytes, regardless of either
        // character's own value. Checks the label against the address
        // `L`'s own instruction actually lands at, not only the label
        // map.
        let mut asm = MMixAssembler::new("W WYDE \"πé\",0\nL BYTE 1", "<test>");
        asm.parse().unwrap();
        let w = *asm.labels.get("W").unwrap();
        let l = *asm.labels.get("L").unwrap();
        assert_eq!(l, w + 6);
        assert_eq!(
            asm.instructions.iter().find(|(addr, _)| *addr == l),
            Some(&(l, MMixInstruction::BYTE(1)))
        );
    }

    #[test]
    fn test_byte_string_backslash_is_an_ordinary_byte() {
        let mut asm = MMixAssembler::new("BYTE \"a\\nb\"", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![
                MMixInstruction::BYTE(b'a'),
                MMixInstruction::BYTE(b'\\'),
                MMixInstruction::BYTE(b'n'),
                MMixInstruction::BYTE(b'b'),
            ]
        );
    }

    #[test]
    fn test_debug_string_holds_a_backslash_as_four_ordinary_bytes() {
        let source = "        LOC     #100\nMain    debug \"a\\tb\"\n        TRAP    0,Halt,0\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.debug_strings(), &[b"a\\tb".to_vec()]);
    }

    #[test]
    fn test_abutting_string_after_a_backslash_constant_is_a_remark_error() {
        // '\'' is a whole constant (the backslash); the string that abuts
        // it with no blank between reads as continuing the statement, not
        // as a remark -- the same rule any other abutting text follows.
        assert_eq!(
            assemble_err("SET $1,'\\'\"(\""),
            "<test>:1:11: syntax error: a remark must be separated from the \
             statement by a blank"
        );
    }

    #[test]
    fn test_hex_and_decimal_constants_reduce_mod_2_64() {
        assert_first_instruction(
            "OCTA #112233445566778899",
            MMixInstruction::OCTA(0x2233445566778899),
        );
        assert_first_instruction("OCTA 18446744073709551621", MMixInstruction::OCTA(5));
        assert_first_instruction("OCTA 0x10000000000000005", MMixInstruction::OCTA(5));
        assert_first_instruction(
            "OCTA 340282366920938463463374607431768211461",
            MMixInstruction::OCTA(5),
        );
        let src = format!("OCTA #1{}5", "0".repeat(32));
        assert_first_instruction(&src, MMixInstruction::OCTA(5));
    }

    #[test]
    fn test_byte_list_string_in_expression_splits_at_its_boundary_characters() {
        // MMIX.md's example: an operator before the string binds to its
        // first character, one after it to its last, and the characters
        // between stand alone. The forward OCTA is resolved in pass 1, so
        // it must agree with the label pass 2 actually places.
        let mut asm = MMixAssembler::new("OCTA Next\nBYTE 1+\"ace\"+2,0\nNext BYTE 99", "<test>");
        asm.parse().unwrap();
        let items: Vec<_> = asm.instructions[1..5]
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            items,
            vec![
                MMixInstruction::BYTE(b'b'),
                MMixInstruction::BYTE(b'c'),
                MMixInstruction::BYTE(b'g'),
                MMixInstruction::BYTE(0),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&12));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(12));
    }

    #[test]
    fn test_byte_list_two_strings_joined_by_an_operator_merge_at_the_seam() {
        let mut asm = MMixAssembler::new("OCTA Next\nBYTE \"ab\"+\"cd\"\nNext BYTE 99", "<test>");
        asm.parse().unwrap();
        let items: Vec<_> = asm.instructions[1..4]
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            items,
            vec![
                MMixInstruction::BYTE(b'a'),
                MMixInstruction::BYTE(197), // 'b' (98) + 'c' (99)
                MMixInstruction::BYTE(b'd'),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&11));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(11));
    }

    #[test]
    fn test_wyde_single_char_string_combines_with_its_operator() {
        assert_first_instruction("WYDE \"a\"+1", MMixInstruction::WYDE(0x0062));
    }

    #[test]
    fn test_byte_list_parenthesized_single_char_string_is_its_value() {
        assert_first_instruction("BYTE (\"a\")", MMixInstruction::BYTE(97));
    }

    #[test]
    fn test_byte_list_parenthesized_multi_char_string_is_an_error() {
        assert!(assemble_err("BYTE (\"ab\")").contains("not a single value"));
    }

    #[test]
    fn test_byte_list_empty_string_inside_an_expression_is_an_error() {
        assert!(
            assemble_err("BYTE 1+\"\"+2")
                .contains("an empty string is not a value inside an expression")
        );
    }

    #[test]
    fn test_set_operand_string_is_an_error() {
        assemble_err("SET $1,\"a\"");
    }

    #[test]
    fn test_byte_list_strong_operator_binds_to_the_strings_near_character() {
        // A strong operator before a string combines with its first
        // character only; one after combines with its last. The characters
        // between stand alone, as MMIX.md's rule requires whatever operator
        // surrounds a string.
        let mut asm = MMixAssembler::new("BYTE 2*\"ab\"", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(194), MMixInstruction::BYTE(98)]
        );

        let mut asm = MMixAssembler::new("BYTE \"ab\"*2", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(97), MMixInstruction::BYTE(196)]
        );
    }

    #[test]
    fn test_byte_list_weak_and_strong_operators_both_reach_the_string() {
        let mut asm = MMixAssembler::new("BYTE 1+\"ab\"*2", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(98), MMixInstruction::BYTE(196)]
        );

        let mut asm = MMixAssembler::new("BYTE 2*\"ab\"+1", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(194), MMixInstruction::BYTE(99)]
        );
    }

    #[test]
    fn test_byte_list_unary_operator_binds_to_the_strings_first_character() {
        // A unary operator applies to the string's first character only,
        // exactly like a strong or weak operator before it.
        let mut asm = MMixAssembler::new("BYTE -\"ab\"", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(159), MMixInstruction::BYTE(98)]
        );

        let mut asm = MMixAssembler::new("BYTE ~\"ab\"", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(158), MMixInstruction::BYTE(98)]
        );

        let mut asm = MMixAssembler::new("BYTE 1+-\"ab\"", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![MMixInstruction::BYTE(160), MMixInstruction::BYTE(98)]
        );
    }

    #[test]
    fn test_byte_list_single_char_string_combines_with_a_strong_operator() {
        // A single-character string is a primary like any other: a strong
        // operator on either side reaches it whole, and division truncates.
        assert_first_instruction("BYTE \"a\"<<1", MMixInstruction::BYTE(194));

        let mut asm = MMixAssembler::new("BYTE \"abc\"/2", "<test>");
        asm.parse().unwrap();
        let bytes: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            bytes,
            vec![
                MMixInstruction::BYTE(97),
                MMixInstruction::BYTE(98),
                MMixInstruction::BYTE(49),
            ]
        );
    }

    #[test]
    fn test_wyde_list_two_strings_each_meet_the_operator_between_them() {
        // Only the seam characters ('b' and 'c') combine with the shared
        // `*`; the outer characters combine with their own neighbor.
        let mut asm = MMixAssembler::new("WYDE 2*\"ab\"*\"cd\"*3", "<test>");
        asm.parse().unwrap();
        let words: Vec<_> = asm
            .instructions
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            words,
            vec![
                MMixInstruction::WYDE(194),
                MMixInstruction::WYDE(9702),
                MMixInstruction::WYDE(300),
            ]
        );
    }

    #[test]
    fn test_byte_list_parenthesized_string_combines_with_a_single_operator() {
        // Inside parentheses a string must reduce to one value; a single
        // character does, whatever operator reaches it.
        assert_first_instruction("BYTE (2*\"a\")", MMixInstruction::BYTE(194));
        assert_first_instruction("BYTE (-\"a\")", MMixInstruction::BYTE(159));
    }

    #[test]
    fn test_byte_list_parenthesized_multi_char_string_is_an_error_with_any_operator() {
        // A multi-character string inside parentheses can never reduce to
        // one value, whatever operator surrounds it.
        assert_eq!(
            assemble_err("BYTE (2*\"ab\")"),
            "<test>:1:9: a 2-character string is not a single value here"
        );
        assert_eq!(
            assemble_err("BYTE (-\"ab\")"),
            "<test>:1:8: a 2-character string is not a single value here"
        );
    }

    #[test]
    fn test_byte_list_empty_string_beside_any_operator_is_an_error() {
        assert_eq!(
            assemble_err("BYTE 2*\"\""),
            "<test>:1:8: an empty string is not a value inside an expression"
        );
        assert_eq!(
            assemble_err("BYTE -\"\""),
            "<test>:1:7: an empty string is not a value inside an expression"
        );
    }

    #[test]
    fn test_byte_list_parenthesized_empty_string_is_an_error() {
        // An empty string inside parentheses gives the same message as one
        // beside any operator, whatever surrounds it.
        assert_eq!(
            assemble_err("BYTE (\"\")"),
            "<test>:1:7: an empty string is not a value inside an expression"
        );
        assert_eq!(
            assemble_err("BYTE (2*\"\")"),
            "<test>:1:9: an empty string is not a value inside an expression"
        );
    }

    #[test]
    fn test_byte_list_strong_op_string_label_offset_agrees_across_passes() {
        // `2*"abc"` combines the leading `2*` with only `"abc"`'s first
        // character; the other two characters stand as their own items, so
        // the whole item is three units. The forward OCTA resolves in
        // pass 1, so it must agree with the label pass 2 actually places.
        let mut asm = MMixAssembler::new("OCTA Next\nBYTE 2*\"abc\",0\nNext BYTE 7", "<test>");
        asm.parse().unwrap();
        let items: Vec<_> = asm.instructions[1..6]
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            items,
            vec![
                MMixInstruction::BYTE(194),
                MMixInstruction::BYTE(b'b'),
                MMixInstruction::BYTE(b'c'),
                MMixInstruction::BYTE(0),
                MMixInstruction::BYTE(7),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&12));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(12));
    }

    #[test]
    fn test_byte_list_two_strings_meeting_at_a_strong_op_label_offset_agrees_across_passes() {
        // `-"ab"*"cd"` combines only `"ab"`'s last character with `"cd"`'s
        // first through the `*` between them; `"ab"`'s first character
        // (under the unary `-`) and `"cd"`'s last stand on their own, so the
        // whole item is three units, not four.
        let mut asm = MMixAssembler::new("OCTA Next\nBYTE -\"ab\"*\"cd\",0\nNext BYTE 7", "<test>");
        asm.parse().unwrap();
        let items: Vec<_> = asm.instructions[1..6]
            .iter()
            .map(|(_, inst)| inst.clone())
            .collect();
        assert_eq!(
            items,
            vec![
                MMixInstruction::BYTE(159),
                MMixInstruction::BYTE(230),
                MMixInstruction::BYTE(100),
                MMixInstruction::BYTE(0),
                MMixInstruction::BYTE(7),
            ]
        );
        assert_eq!(asm.labels.get("Next"), Some(&12));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(12));
    }

    // ---- A string-free data item diagnoses exactly as `expr` does -------

    #[test]
    fn test_byte_list_unary_with_no_operand_reports_expected_primary() {
        assert_eq!(
            assemble_err("Main\tBYTE\t-\n\tTRAP\t0,Halt,0"),
            "<test>:1:12: syntax error: expected primary"
        );
    }

    #[test]
    fn test_byte_list_group_missing_its_second_term_reports_expected_group_primary() {
        assert_eq!(
            assemble_err("Main\tBYTE\t(1+)\n\tTRAP\t0,Halt,0"),
            "<test>:1:14: syntax error: expected group_primary"
        );
    }

    #[test]
    fn test_byte_list_leading_comma_reports_expected_data_value() {
        assert_eq!(
            assemble_err("Main\tBYTE\t,1\n\tTRAP\t0,Halt,0"),
            "<test>:1:11: syntax error: expected data_value"
        );
    }

    // ---- Every instruction field's edge, one field past it -------------

    #[test]
    fn test_instruction_field_edges_by_range_table() {
        // Each of the sixteen wyde immediates is its own function.
        assert_first_instruction("SETL $1,#FFFF", MMixInstruction::SETL(1, 0xFFFF));
        assert_eq!(
            assemble_err("SETL $1,#10000"),
            "<test>:1:9: immediate operand 65536 out of range 0..65535 for SETL"
        );
        assert_first_instruction("SETH $1,#FFFF", MMixInstruction::SETH(1, 0xFFFF));
        assert_eq!(
            assemble_err("SETH $1,#10000"),
            "<test>:1:9: immediate operand 65536 out of range 0..65535 for SETH"
        );
        assert_first_instruction("SETMH $1,#FFFF", MMixInstruction::SETMH(1, 0xFFFF));
        assert_eq!(
            assemble_err("SETMH $1,#10000"),
            "<test>:1:10: immediate operand 65536 out of range 0..65535 for SETMH"
        );
        assert_first_instruction("SETML $1,#FFFF", MMixInstruction::SETML(1, 0xFFFF));
        assert_eq!(
            assemble_err("SETML $1,#10000"),
            "<test>:1:10: immediate operand 65536 out of range 0..65535 for SETML"
        );
        assert_first_instruction("INCL $1,#FFFF", MMixInstruction::INCL(1, 0xFFFF));
        assert_eq!(
            assemble_err("INCL $1,#10000"),
            "<test>:1:9: immediate operand 65536 out of range 0..65535 for INCL"
        );
        assert_first_instruction("INCH $1,#FFFF", MMixInstruction::INCH(1, 0xFFFF));
        assert_eq!(
            assemble_err("INCH $1,#1FFFF"),
            "<test>:1:9: immediate operand 131071 out of range 0..65535 for INCH"
        );
        assert_first_instruction("INCMH $1,#FFFF", MMixInstruction::INCMH(1, 0xFFFF));
        assert_eq!(
            assemble_err("INCMH $1,#10000"),
            "<test>:1:10: immediate operand 65536 out of range 0..65535 for INCMH"
        );
        assert_first_instruction("INCML $1,#FFFF", MMixInstruction::INCML(1, 0xFFFF));
        assert_eq!(
            assemble_err("INCML $1,#10000"),
            "<test>:1:10: immediate operand 65536 out of range 0..65535 for INCML"
        );
        assert_first_instruction("ORH $1,#FFFF", MMixInstruction::ORH(1, 0xFFFF));
        assert_eq!(
            assemble_err("ORH $1,#10000"),
            "<test>:1:8: immediate operand 65536 out of range 0..65535 for ORH"
        );
        assert_first_instruction("ORMH $1,#FFFF", MMixInstruction::ORMH(1, 0xFFFF));
        assert_eq!(
            assemble_err("ORMH $1,#10000"),
            "<test>:1:9: immediate operand 65536 out of range 0..65535 for ORMH"
        );
        assert_first_instruction("ORML $1,#FFFF", MMixInstruction::ORML(1, 0xFFFF));
        assert_eq!(
            assemble_err("ORML $1,#10000"),
            "<test>:1:9: immediate operand 65536 out of range 0..65535 for ORML"
        );
        assert_first_instruction("ORL $1,#FFFF", MMixInstruction::ORL(1, 0xFFFF));
        assert_eq!(
            assemble_err("ORL $1,#10000"),
            "<test>:1:8: immediate operand 65536 out of range 0..65535 for ORL"
        );
        assert_first_instruction("ANDNH $1,#FFFF", MMixInstruction::ANDNH(1, 0xFFFF));
        assert_eq!(
            assemble_err("ANDNH $1,#10000"),
            "<test>:1:10: immediate operand 65536 out of range 0..65535 for ANDNH"
        );
        assert_first_instruction("ANDNMH $1,#FFFF", MMixInstruction::ANDNMH(1, 0xFFFF));
        assert_eq!(
            assemble_err("ANDNMH $1,#10000"),
            "<test>:1:11: immediate operand 65536 out of range 0..65535 for ANDNMH"
        );
        assert_first_instruction("ANDNML $1,#FFFF", MMixInstruction::ANDNML(1, 0xFFFF));
        assert_eq!(
            assemble_err("ANDNML $1,#10000"),
            "<test>:1:11: immediate operand 65536 out of range 0..65535 for ANDNML"
        );
        assert_first_instruction("ANDNL $1,#FFFF", MMixInstruction::ANDNL(1, 0xFFFF));
        assert_eq!(
            assemble_err("ANDNL $1,#10000"),
            "<test>:1:10: immediate operand 65536 out of range 0..65535 for ANDNL"
        );

        // `parse_inst_arith_rri` (ADDI and its auto-immediate kin) and
        // `parse_inst_load_store_rri` (LDBI and its kin): each a distinct
        // function, its own byte-field check.
        assert_first_instruction("ADDI $1,$2,255", MMixInstruction::ADDI(1, 2, 255));
        assert_eq!(
            assemble_err("ADDI $1,$2,256"),
            "<test>:1:12: immediate operand 256 out of range 0..255 for ADDI"
        );
        assert_first_instruction("LDBI $1,$2,255", MMixInstruction::LDBI(1, 2, 255));
        assert_eq!(
            assemble_err("LDBI $1,$2,999"),
            "<test>:1:12: immediate operand 999 out of range 0..255 for LDBI"
        );

        // `parse_rri`, the helper `LDUNCI` and fifteen other explicit `*I`
        // three-operand spellings share.
        assert_first_instruction("LDUNCI $1,$2,255", MMixInstruction::LDUNCI(1, 2, 255));
        assert_eq!(
            assemble_err("LDUNCI $1,$2,256"),
            "<test>:1:14: immediate operand 256 out of range 0..255 for LDUNCI"
        );

        // `parse_inst_bitfiddle_rri`, `parse_inst_conditional_set_rri` and
        // `parse_inst_zero_or_set_rri`: each its own Z check.
        assert_first_instruction("BDIFI $1,$2,255", MMixInstruction::BDIFI(1, 2, 255));
        assert_eq!(
            assemble_err("BDIFI $1,$2,256"),
            "<test>:1:13: immediate operand 256 out of range 0..255 for BDIFI"
        );
        assert_first_instruction("CSNI $1,$2,255", MMixInstruction::CSNI(1, 2, 255));
        assert_eq!(
            assemble_err("CSNI $1,$2,256"),
            "<test>:1:12: immediate operand 256 out of range 0..255 for CSNI"
        );
        assert_first_instruction("ZSNI $1,$2,255", MMixInstruction::ZSNI(1, 2, 255));
        assert_eq!(
            assemble_err("ZSNI $1,$2,256"),
            "<test>:1:12: immediate operand 256 out of range 0..255 for ZSNI"
        );

        // NEG/NEGU's Y (the auto-immediate path) and NEGI/NEGUI's own Y
        // (the explicit-immediate path): two distinct functions.
        assert_first_instruction("NEG $1,255,$2", MMixInstruction::NEG(1, 255, 2));
        assert_eq!(
            assemble_err("NEG $1,256,$2"),
            "<test>:1:8: immediate operand 256 out of range 0..255 for NEG"
        );
        assert_first_instruction("NEGI $1,255,5", MMixInstruction::NEGI(1, 255, 5));
        assert_eq!(
            assemble_err("NEGI $1,256,$2"),
            "<test>:1:9: immediate operand 256 out of range 0..255 for NEGI"
        );

        // The float rounding-mode forms: `FIX`'s explicit `Y`, `FLOT`'s
        // forced `Y`, `FLOTI`'s three- and two-operand `Y`/`Z`.
        assert_first_instruction("FIX $1,255,$2", MMixInstruction::FIX(1, 255, 2));
        assert_eq!(
            assemble_err("FIX $1,256,$2"),
            "<test>:1:8: immediate operand 256 out of range 0..255 for FIX"
        );
        assert_first_instruction("FLOT $1,255,$2", MMixInstruction::FLOT(1, 255, 2));
        assert_eq!(
            assemble_err("FLOT $1,256,$2"),
            "<test>:1:9: immediate operand 256 out of range 0..255 for FLOT"
        );
        assert_first_instruction("FLOTI $1,255,5", MMixInstruction::FLOTI(1, 255, 5));
        assert_eq!(
            assemble_err("FLOTI $1,256,5"),
            "<test>:1:10: immediate operand 256 out of range 0..255 for FLOTI"
        );
        assert_first_instruction("FLOTI $1,1,255", MMixInstruction::FLOTI(1, 1, 255));
        assert_eq!(
            assemble_err("FLOTI $1,1,256"),
            "<test>:1:12: immediate operand 256 out of range 0..255 for FLOTI"
        );
        assert_first_instruction("FLOTI $1,255", MMixInstruction::FLOTI(1, 0, 255));
        assert_eq!(
            assemble_err("FLOTI $1,256"),
            "<test>:1:10: immediate operand 256 out of range 0..255 for FLOTI"
        );

        // GET's Z and PUT's X: a special register, 0..=31.
        assert_first_instruction("GET $1,31", MMixInstruction::GET(1, 31));
        assert_eq!(
            assemble_err("GET $1,32"),
            "<test>:1:8: immediate operand 32 out of range 0..31 for GET"
        );
        assert_first_instruction("PUT 31,$1", MMixInstruction::PUT(31, 1));
        assert_eq!(
            assemble_err("PUT 32,$1"),
            "<test>:1:5: immediate operand 32 out of range 0..31 for PUT"
        );

        // PUTI's X (special register) and Z (byte) check independently.
        assert_first_instruction("PUTI 31,255", MMixInstruction::PUTI(31, 255));
        assert_eq!(
            assemble_err("PUTI 32,1"),
            "<test>:1:6: immediate operand 32 out of range 0..31 for PUTI"
        );
        assert_eq!(
            assemble_err("PUTI 1,256"),
            "<test>:1:8: immediate operand 256 out of range 0..255 for PUTI"
        );

        // SAVE's Z and UNSAVE's X.
        assert_first_instruction("SAVE $1,255", MMixInstruction::SAVE(1, 255));
        assert_eq!(
            assemble_err("SAVE $255,256"),
            "<test>:1:11: immediate operand 256 out of range 0..255 for SAVE"
        );
        assert_first_instruction("UNSAVE 255,$1", MMixInstruction::UNSAVE(255, 1));
        assert_eq!(
            assemble_err("UNSAVE 256,$1"),
            "<test>:1:8: immediate operand 256 out of range 0..255 for UNSAVE"
        );

        // `STCOI`'s X and Z check independently.
        assert_first_instruction("STCOI 255,$2,5", MMixInstruction::STCOI(255, 2, 5));
        assert_eq!(
            assemble_err("STCOI 256,$2,5"),
            "<test>:1:7: immediate operand 256 out of range 0..255 for STCOI"
        );
        assert_first_instruction("STCOI 5,$2,255", MMixInstruction::STCOI(5, 2, 255));
        assert_eq!(
            assemble_err("STCOI 5,$2,256"),
            "<test>:1:12: immediate operand 256 out of range 0..255 for STCOI"
        );

        // RESUME and SYNC take the full 24-bit XYZ.
        assert_first_instruction("RESUME #FFFFFF", MMixInstruction::RESUME(0xFFFFFF));
        assert_eq!(
            assemble_err("RESUME #1000000"),
            "<test>:1:8: immediate operand 16777216 out of range 0..16777215 for RESUME"
        );
        assert_first_instruction("SYNC #FFFFFF", MMixInstruction::SYNC(0xFFFFFF));
        assert_eq!(
            assemble_err("SYNC #1000000"),
            "<test>:1:6: immediate operand 16777216 out of range 0..16777215 for SYNC"
        );

        // POP's X (byte), yz (wyde) and xyz (three bytes), both operand
        // forms.
        assert_first_instruction("POP 255,#FFFF", MMixInstruction::POP(255, 255, 255));
        assert_eq!(
            assemble_err("POP 256,0"),
            "<test>:1:5: immediate operand 256 out of range 0..255 for POP"
        );
        assert_eq!(
            assemble_err("POP 0,#10000"),
            "<test>:1:7: immediate operand 65536 out of range 0..65535 for POP"
        );
        assert_first_instruction("POP #FFFFFF", MMixInstruction::POP(255, 255, 255));
        assert_eq!(
            assemble_err("POP #1000000"),
            "<test>:1:5: immediate operand 16777216 out of range 0..16777215 for POP"
        );

        // TRAP's yz (wyde) and xyz (three bytes); both fit at their edge.
        assert_eq!(
            assemble_err("TRAP 0,#10000"),
            "<test>:1:8: immediate operand 65536 out of range 0..65535 for TRAP"
        );
        assert_eq!(
            assemble_err("TRAP #1000000"),
            "<test>:1:6: immediate operand 16777216 out of range 0..16777215 for TRAP"
        );
        assert_first_instruction("TRAP 0,#FFFF", MMixInstruction::TRAP(0, 0xFF, 0xFF));
        assert_first_instruction("TRAP #FFFFFF", MMixInstruction::TRAP(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn test_resume_and_sync_encode_all_of_xyz() {
        let asm = MMixAssembler::new("", "<test>");
        assert_eq!(
            asm.encode_instruction_bytes(&MMixInstruction::SYNC(300)),
            vec![0xFC, 0x00, 0x01, 0x2C]
        );
        assert_eq!(
            asm.encode_instruction_bytes(&MMixInstruction::RESUME(0x10203)),
            vec![0xF9, 0x01, 0x02, 0x03]
        );
    }

    // `SET $1,-1`/`-5`/`-#10` are covered by
    // `test_set_negative_literal_is_an_error_for_decimal_and_hex`.
    #[test]
    fn test_set_wide_immediate_diagnostic_and_positive_forms() {
        assert_eq!(
            assemble_err("SET $1,#10000"),
            "<test>:1:8: immediate operand 65536 out of range 0..65535 for SET; \
             use SETI for a wider constant"
        );
        assert_first_instruction("SET $1,#FFFF", MMixInstruction::SETL(1, 0xFFFF));
        assert_first_instruction("SETI $1,-1", MMixInstruction::SET(1, u64::MAX));
    }
}
