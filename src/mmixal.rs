use pest_derive::Parser;
use std::collections::HashMap;
use tracing::{debug, instrument};

#[derive(Parser)]
#[grammar = "mmixal.pest"]
struct MMixalParser;

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

    // Arithmetic - Add and Subtract (§9)
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

    INCL(u8, u8, u8), // INCL $X, $Y, $Z

    // Bitwise operations (§10)
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

    // Bit fiddling operations (§11-12)
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

    // Shift instructions (§14)
    SL(u8, u8, u8),   // SL $X, $Y, $Z - shift left
    SLI(u8, u8, u8),  // SL $X, $Y, Z - shift left immediate
    SLU(u8, u8, u8),  // SLU $X, $Y, $Z - shift left unsigned
    SLUI(u8, u8, u8), // SLU $X, $Y, Z - shift left unsigned immediate
    SR(u8, u8, u8),   // SR $X, $Y, $Z - shift right
    SRI(u8, u8, u8),  // SR $X, $Y, Z - shift right immediate
    SRU(u8, u8, u8),  // SRU $X, $Y, $Z - shift right unsigned
    SRUI(u8, u8, u8), // SRU $X, $Y, Z - shift right unsigned immediate

    // Branch instructions
    JMP(u32),          // JMP offset (24-bit)
    JE(u8, u16),       // JE $X, offset
    JNE(u8, u16),      // JNE $X, offset
    JL(u8, u16),       // JL $X, offset
    JG(u8, u16),       // JG $X, offset
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
    POP(u8, u8),         // POP X, YZ - pop registers and return
    GO(u8, u8, u8),      // GO $X, $Y, $Z - go to location
    GOI(u8, u8, u8),     // GOI $X, $Y, Z - go to location (immediate)
    GET(u8, u8),         // GET $X, Z - get from special register
    PUT(u8, u8),         // PUT X, $Z - put into special register
    PUTI(u8, u8),        // PUTI X, Z - put immediate into special register
    SAVE(u8, u8),        // SAVE $X, 0 - save context
    UNSAVE(u8, u8),      // UNSAVE 0, $Z - unsave/restore context
    RESUME(u8),          // RESUME XYZ - resume after interrupt
    SYNC(u8),            // SYNC XYZ - synchronize
    SWYM,                // SWYM - sympathize with your machinery (nop)
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

pub struct MMixAssembler {
    source: String,
    filename: String,
    pub labels: HashMap<String, u64>,
    pub symbols: HashMap<String, u64>, // For IS directive - symbolic names
    pub instructions: Vec<(u64, MMixInstruction)>,
    current_addr: u64,
    next_greg: u8, // Next global register to allocate (starts at 254, counts down)
    pub greg_inits: Vec<(u8, u64)>, // Global register initialization values: (register, value)
}

impl MMixAssembler {
    /// Preprocess the source code to expand debug directives
    /// Transforms: debug "text"
    /// Into: GETA t,DbgStr_NNNN
    ///       TRAP 0,Fputs,StdOut
    ///       DbgStr_NNNN BYTE "text",#a,0
    fn preprocess_debug(source: &str) -> String {
        use regex::Regex;

        // Match debug directive anywhere on a line (after label or standalone)
        // Captures: optional label, the debug keyword, and the quoted string
        let debug_re = Regex::new(r#"(?m)^([^\s]*\s+)?debug\s+"([^"]*)"\s*$"#).unwrap();
        let mut counter = 0;
        let mut result = String::new();
        let mut debug_strings = Vec::new();

        for line in source.lines() {
            if let Some(caps) = debug_re.captures(line) {
                let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text = &caps[2];
                counter += 1;
                let label = format!("DbgStr_{:04}", counter);

                debug!(
                    "Preprocessing debug directive: \"{}\" -> label {}",
                    text, label
                );

                // If there was a label/prefix before debug, preserve it on its own line
                if !prefix.trim().is_empty() {
                    result.push_str(prefix.trim());
                    result.push('\n');
                }

                // Generate call to debug subroutine using SAVE/UNSAVE for full context preservation
                // PUSHJ manages return address via rJ special register
                result.push_str(&format!("\tPUSHJ\t$0,{}\n", label));

                // Store the subroutine definition for later
                debug_strings.push((label, text.to_string()));
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Append all debug subroutines and strings at the end
        if !debug_strings.is_empty() {
            result.push('\n');
            result.push_str("; Debug subroutines generated by preprocessor\n");
            for (label, text) in debug_strings {
                // Each debug subroutine:
                // 1. SAVE context to memory (address returned in $254)
                // 2. Load string address into $0
                // 3. Call Fputs TRAP
                // 4. UNSAVE restores context (Note: rJ is NOT in context, so return address is safe)
                // 5. POP returns via rJ
                result.push_str(&format!("{}  \tSAVE\t$254,0\n", label));
                result.push_str(&format!("\tGETA\t$0,{}Str\n", label));
                result.push_str("\tTRAP\t0,Fputs,StdOut\n");
                result.push_str("\tUNSAVE\t0,$254\n");
                result.push_str("\tPOP\t0,0\n"); // Return via rJ
                // String data right after the subroutine
                result.push_str(&format!("{}Str\tBYTE\t\"{}\",#a,0\n", label, text));
            }
        }

        debug!("Preprocessed source:\n{}", result);
        result
    }
    pub fn new(source: &str, filename: &str) -> Self {
        let mut symbols = HashMap::new();

        // Standard MMIXAL predefined symbols
        // Segment constants
        symbols.insert("Data_Segment".to_string(), 0x2000000000000000);
        symbols.insert("Pool_Segment".to_string(), 0x4000000000000000);
        symbols.insert("Stack_Segment".to_string(), 0x6000000000000000);

        // Standard I/O handles
        symbols.insert("StdIn".to_string(), 0);
        symbols.insert("StdOut".to_string(), 1);
        symbols.insert("StdErr".to_string(), 2);

        // Common TRAP function codes (C library emulation)
        symbols.insert("Halt".to_string(), 0);
        symbols.insert("Fopen".to_string(), 1);
        symbols.insert("Fclose".to_string(), 2);
        symbols.insert("Fread".to_string(), 3);
        symbols.insert("Fgets".to_string(), 4);
        symbols.insert("Fgetws".to_string(), 5);
        symbols.insert("Fwrite".to_string(), 6);
        symbols.insert("Fputs".to_string(), 7);
        symbols.insert("Fputws".to_string(), 8);
        symbols.insert("Fseek".to_string(), 9);
        symbols.insert("Ftell".to_string(), 10);

        // Preprocess the source to expand debug directives
        let preprocessed_source = Self::preprocess_debug(source);

        Self {
            source: preprocessed_source,
            filename: filename.to_string(),
            labels: HashMap::new(),
            symbols,
            instructions: Vec::new(),
            current_addr: 0,
            next_greg: 254, // Start allocating from $254, count down
            greg_inits: Vec::new(),
        }
    }

    /// Format Pest parse errors in a user-friendly way
    fn format_parse_error(error: &pest::error::Error<Rule>, filename: &str) -> String {
        use pest::error::LineColLocation;

        let (line, col) = match error.line_col {
            LineColLocation::Pos((l, c)) => (l, c),
            LineColLocation::Span((l, c), _) => (l, c),
        };

        // Create a user-friendly error message based on what was expected
        let expected_msg = match &error.variant {
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
                            Rule::register => "register (e.g., $0, $1, $255)".to_string(),
                            Rule::expr_value => "number or expression".to_string(),
                            Rule::identifier => "label or symbol name".to_string(),
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
        };

        format!(
            "{}:{}:{}: syntax error: expected {}",
            filename, line, col, expected_msg
        )
    }

    #[instrument(skip(self), fields(source_len = self.source.len()))]
    pub fn parse(&mut self) -> Result<(), String> {
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
    #[instrument(skip(self))]
    fn parse_two_pass(&mut self) -> Result<(), String> {
        use pest::Parser;

        let source = self.source.clone();
        debug!("Pass 1: Collecting labels and symbols");

        // Pass 1: Scan for labels and symbols
        let pairs = MMixalParser::parse(Rule::program, &source).map_err(|e| {
            // Format Pest error in a user-friendly way
            Self::format_parse_error(&e, &self.filename)
        })?;
        for pair in pairs {
            if pair.as_rule() == Rule::program {
                for line_pair in pair.into_inner() {
                    if line_pair.as_rule() == Rule::line {
                        // A line may contain a statement or be empty
                        for stmt_pair in line_pair.into_inner() {
                            if stmt_pair.as_rule() == Rule::statement {
                                self.first_pass_statement(stmt_pair)?;
                            }
                        }
                    }
                }
            }
        }

        debug!(
            "Pass 1 complete: {} labels, {} symbols",
            self.labels.len(),
            self.symbols.len()
        );

        // Reset current address for second pass
        let saved_addr = self.current_addr;
        self.current_addr = 0;

        debug!("Pass 2: Generating instructions");

        // Pass 2: Generate instructions with resolved references
        let pairs = MMixalParser::parse(Rule::program, &source).map_err(|e| format!("{}", e))?;
        for pair in pairs {
            if pair.as_rule() == Rule::program {
                for line_pair in pair.into_inner() {
                    if line_pair.as_rule() == Rule::line {
                        // A line may contain a statement or be empty
                        for stmt_pair in line_pair.into_inner() {
                            if stmt_pair.as_rule() == Rule::statement {
                                self.second_pass_statement(stmt_pair)?;
                            }
                        }
                    }
                }
            }
        }

        self.current_addr = saved_addr;
        Ok(())
    }

    /// First pass: collect labels and process IS directives
    #[instrument(skip(self, pair), fields(current_addr = format!("0x{:X}", self.current_addr)))]
    fn first_pass_statement(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut label_name: Option<String> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let ident = inner_pair.into_inner().next().unwrap();
                    label_name = Some(ident.as_str().to_string());
                }
                Rule::instruction => {
                    // Just advance current_addr by instruction size
                    let inst = self.peek_instruction_type(inner_pair)?;
                    let size = Self::instruction_size(&inst);
                    if let Some(label) = label_name.take() {
                        debug!(label = %label, addr = format!("0x{:X}", self.current_addr), "Collected label");
                        self.labels.insert(label, self.current_addr);
                    }
                    self.current_addr += size;
                }
                Rule::directive => {
                    // Handle the grouped directive rule by extracting the actual directive
                    let directive_pair = inner_pair.into_inner().next().unwrap();
                    match directive_pair.as_rule() {
                        Rule::data_directive => {
                            let size = self.data_directive_size(directive_pair.clone())?;
                            if let Some(label) = label_name.take() {
                                debug!(label = %label, addr = format!("0x{:X}", self.current_addr), "Collected label");
                                self.labels.insert(label, self.current_addr);
                            }
                            self.current_addr += size;
                        }
                        Rule::loc_directive => {
                            self.parse_loc_directive(directive_pair)?;
                            if let Some(label) = label_name.take() {
                                self.labels.insert(label, self.current_addr);
                            }
                        }
                        Rule::greg_directive => {
                            // GREG allocates a global register
                            // If there's a label, it should map to the register number, not an address
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

                            if let Some(label) = label_name.take() {
                                // Map the label to the register NUMBER (not address)
                                self.symbols.insert(label, allocated_reg as u64);
                            }

                            // Parse to get the init value (will be processed again in second pass)
                            let mut greg_parts = directive_pair.clone().into_inner();
                            let _directive = greg_parts.next();
                            let value = self.parse_number(greg_parts.next().unwrap())?;
                            self.greg_inits.push((allocated_reg, value));
                        }
                        Rule::is_directive => {
                            // Process IS directive immediately
                            self.parse_is_directive(directive_pair)?;
                            // Note: IS directive doesn't advance current_addr
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Handle standalone labels (labels not followed by instruction or directive)
        if let Some(label) = label_name {
            debug!(label = %label, addr = format!("0x{:X}", self.current_addr), "Collected standalone label");
            self.labels.insert(label, self.current_addr);
        }

        Ok(())
    }

    /// Second pass: generate actual instructions with resolved labels
    #[instrument(skip(self, pair), fields(current_addr = format!("0x{:X}", self.current_addr)))]
    fn second_pass_statement(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut label_name: Option<String> = None;
        let mut inst: Option<MMixInstruction> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let ident = inner_pair.into_inner().next().unwrap();
                    label_name = Some(ident.as_str().to_string());
                }
                Rule::instruction => {
                    // Define label before processing instruction
                    if let Some(label) = label_name.take() {
                        debug!(label = %label, addr = format!("0x{:X}", self.current_addr), "Defined label");
                        self.labels.insert(label, self.current_addr);
                    }
                    inst = Some(self.parse_instruction(inner_pair)?);
                }
                Rule::directive => {
                    // Handle the grouped directive rule by extracting the actual directive
                    let directive_pair = inner_pair.into_inner().next().unwrap();
                    match directive_pair.as_rule() {
                        Rule::data_directive => {
                            // Define label before processing data directive
                            if let Some(label) = label_name.take() {
                                debug!(label = %label, addr = format!("0x{:X}", self.current_addr), "Defined label");
                                self.labels.insert(label, self.current_addr);
                            }
                            // Data directives might expand to multiple bytes (e.g., BYTE "string")
                            let instructions = self.parse_data_directive(directive_pair)?;
                            for instruction in instructions {
                                let size = Self::instruction_size(&instruction);
                                debug!(inst = ?instruction, addr = format!("0x{:X}", self.current_addr), size, "Added instruction");
                                self.instructions.push((self.current_addr, instruction));
                                self.current_addr += size;
                            }
                        }
                        Rule::loc_directive => {
                            self.parse_loc_directive(directive_pair)?;
                        }
                        Rule::greg_directive => {
                            // GREG was already processed in first pass
                            // Verify that any label was properly added to symbols
                            if let Some(label) = label_name.take() {
                                if !self.symbols.contains_key(&label) {
                                    return Err(format!(
                                        "Internal error: GREG label '{}' not found in symbols from first pass",
                                        label
                                    ));
                                }
                            }
                        }
                        Rule::is_directive => {
                            self.parse_is_directive(directive_pair)?;
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
            self.instructions.push((self.current_addr, instruction));
            self.current_addr += size;
        }

        // Handle standalone labels (labels not followed by instruction or directive)
        if let Some(label) = label_name {
            debug!(label = %label, addr = format!("0x{:X}", self.current_addr), "Defined standalone label");
            self.labels.insert(label, self.current_addr);
        }

        Ok(())
    }

    /// Peek at instruction type to determine size without modifying state
    fn peek_instruction_type(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let inner = pair.into_inner().next().ok_or("Empty instruction")?;

        match inner.as_rule() {
            Rule::inst_set_rr => Ok(MMixInstruction::SETRR(0, 0)), // Placeholder for sizing
            Rule::inst_set_ri => Ok(MMixInstruction::SET(0, 0)),   // Placeholder for sizing
            Rule::inst_setl_ri => Ok(MMixInstruction::SETL(0, 0)),
            Rule::inst_seth_ri => Ok(MMixInstruction::SETH(0, 0)),
            Rule::inst_setmh_ri => Ok(MMixInstruction::SETMH(0, 0)),
            Rule::inst_setml_ri => Ok(MMixInstruction::SETML(0, 0)),
            Rule::inst_incl_rrr => Ok(MMixInstruction::INCL(0, 0, 0)),
            Rule::inst_lda_ri | Rule::inst_lda_rri => {
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

    /// Calculate the actual size of a data directive (accounting for string expansion)
    fn data_directive_size(&self, pair: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let mut parts = pair.clone().into_inner();
        let directive = parts.next().ok_or("Empty data directive")?;

        match directive.as_rule() {
            Rule::directive_byte => {
                // Count total bytes from all values (strings expand)
                let mut total_size = 0u64;
                let byte_values = parts.next().ok_or("Missing byte values")?;

                for byte_value in byte_values.into_inner() {
                    let mut value_parts = byte_value.into_inner();
                    let first = value_parts.next().unwrap();

                    if first.as_rule() == Rule::string_literal {
                        // String: count characters + process escape sequences
                        let text = first.as_str();
                        // Remove surrounding quotes
                        let content = &text[1..text.len() - 1];
                        total_size += content.len() as u64;
                        debug!("BYTE string: \"{}\" = {} bytes", content, content.len());
                    } else {
                        // Single value (number or expr)
                        total_size += 1;
                        debug!("BYTE value: 1 byte");
                    }
                }
                debug!("Total BYTE size: {} bytes", total_size);
                Ok(total_size)
            }
            Rule::directive_wyde => Ok(2),
            Rule::directive_tetra => Ok(4),
            Rule::directive_octa => Ok(8),
            _ => Err(format!("Unknown data directive: {:?}", directive.as_rule())),
        }
    }

    fn parse_instruction(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let inner = pair.into_inner().next().ok_or("Empty instruction")?;

        match inner.as_rule() {
            Rule::inst_set_rr => self.parse_inst_set_rr(inner),
            Rule::inst_set_ri => self.parse_inst_set(inner),
            Rule::inst_setl_ri => self.parse_inst_setl(inner),
            Rule::inst_seth_ri => self.parse_inst_seth(inner),
            Rule::inst_setmh_ri => self.parse_inst_setmh(inner),
            Rule::inst_setml_ri => self.parse_inst_setml(inner),
            Rule::inst_incl_rrr => self.parse_inst_incl(inner),
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
            Rule::inst_load_store_rrr => self.parse_inst_load_store_rrr(inner),
            Rule::inst_load_store_rri => self.parse_inst_load_store_rri(inner),
            Rule::inst_lda_rri => self.parse_inst_lda_rri(inner),
            Rule::inst_lda_ri => self.parse_inst_lda_ri(inner),
            Rule::inst_arith_rrr => self.parse_inst_arith_rrr(inner),
            Rule::inst_arith_rri => self.parse_inst_arith_rri(inner),
            Rule::inst_float_rrr => self.parse_inst_float_rrr(inner),
            Rule::inst_float_rri => self.parse_inst_float_rri(inner),
            Rule::inst_neg_rrr => self.parse_inst_neg_rrr(inner),
            Rule::inst_neg_rri => self.parse_inst_neg_rri(inner),
            Rule::inst_bitwise_rrr => self.parse_inst_bitwise_rrr(inner),
            Rule::inst_bitwise_rri => self.parse_inst_bitwise_rri(inner),
            Rule::inst_bitfiddle_rrr => self.parse_inst_bitfiddle_rrr(inner),
            Rule::inst_bitfiddle_rri => self.parse_inst_bitfiddle_rri(inner),
            Rule::inst_shift_rrr => self.parse_inst_shift_rrr(inner),
            Rule::inst_shift_rri => self.parse_inst_shift_rri(inner),
            Rule::inst_conditional_set_rrr => self.parse_inst_conditional_set_rrr(inner),
            Rule::inst_conditional_set_rri => self.parse_inst_conditional_set_rri(inner),
            Rule::inst_zero_or_set_rrr => self.parse_inst_zero_or_set_rrr(inner),
            Rule::inst_zero_or_set_rri => self.parse_inst_zero_or_set_rri(inner),
            Rule::inst_pbranch => self.parse_inst_pbranch(inner),
            Rule::inst_branch => self.parse_inst_branch(inner),
            Rule::inst_jmp => self.parse_inst_jmp(inner),
            Rule::inst_geta => self.parse_inst_geta(inner),
            Rule::inst_getab => self.parse_inst_getab(inner),
            Rule::inst_pushj => self.parse_inst_pushj(inner),
            Rule::inst_pushjb => self.parse_inst_pushjb(inner),
            Rule::inst_pushgo_rrr => self.parse_inst_pushgo_rrr(inner),
            Rule::inst_pushgo_rri => self.parse_inst_pushgo_rri(inner),
            Rule::inst_pop => self.parse_inst_pop(inner),
            Rule::inst_go_rrr => self.parse_inst_go_rrr(inner),
            Rule::inst_go_rri => self.parse_inst_go_rri(inner),
            Rule::inst_get => self.parse_inst_get(inner),
            Rule::inst_put => self.parse_inst_put(inner),
            Rule::inst_puti => self.parse_inst_puti(inner),
            Rule::inst_save => self.parse_inst_save(inner),
            Rule::inst_unsave => self.parse_inst_unsave(inner),
            Rule::inst_ldunc_rrr => self.parse_inst_ldunc_rrr(inner),
            Rule::inst_ldunc_rri => self.parse_inst_ldunc_rri(inner),
            Rule::inst_stunc_rrr => self.parse_inst_stunc_rrr(inner),
            Rule::inst_stunc_rri => self.parse_inst_stunc_rri(inner),
            Rule::inst_ldht_rrr => self.parse_inst_ldht_rrr(inner),
            Rule::inst_ldht_rri => self.parse_inst_ldht_rri(inner),
            Rule::inst_stht_rrr => self.parse_inst_stht_rrr(inner),
            Rule::inst_stht_rri => self.parse_inst_stht_rri(inner),
            Rule::inst_ldsf_rrr => self.parse_inst_ldsf_rrr(inner),
            Rule::inst_ldsf_rri => self.parse_inst_ldsf_rri(inner),
            Rule::inst_stsf_rrr => self.parse_inst_stsf_rrr(inner),
            Rule::inst_stsf_rri => self.parse_inst_stsf_rri(inner),
            Rule::inst_ldvts_rrr => self.parse_inst_ldvts_rrr(inner),
            Rule::inst_ldvts_rri => self.parse_inst_ldvts_rri(inner),
            Rule::inst_cswap_rrr => self.parse_inst_cswap_rrr(inner),
            Rule::inst_cswap_rri => self.parse_inst_cswap_rri(inner),
            Rule::inst_stco_rrr => self.parse_inst_stco_rrr(inner),
            Rule::inst_stco_rri => self.parse_inst_stco_rri(inner),
            Rule::inst_preld_rrr => self.parse_inst_preld_rrr(inner),
            Rule::inst_preld_rri => self.parse_inst_preld_rri(inner),
            Rule::inst_prego_rrr => self.parse_inst_prego_rrr(inner),
            Rule::inst_prego_rri => self.parse_inst_prego_rri(inner),
            Rule::inst_prest_rrr => self.parse_inst_prest_rrr(inner),
            Rule::inst_prest_rri => self.parse_inst_prest_rri(inner),
            Rule::inst_syncd_rrr => self.parse_inst_syncd_rrr(inner),
            Rule::inst_syncd_rri => self.parse_inst_syncd_rri(inner),
            Rule::inst_syncid_rrr => self.parse_inst_syncid_rrr(inner),
            Rule::inst_syncid_rri => self.parse_inst_syncid_rri(inner),
            Rule::inst_resume => self.parse_inst_resume(inner),
            Rule::inst_trip => self.parse_inst_trip(inner),
            Rule::inst_swym => Ok(MMixInstruction::SWYM),
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
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.parse_number(ops.next().unwrap())?;
        Ok(MMixInstruction::SET(reg, val))
    }

    fn parse_inst_set_rr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg_x = self.parse_register(ops.next().unwrap())?;
        let reg_y = self.parse_register(ops.next().unwrap())?;
        Ok(MMixInstruction::SETRR(reg_x, reg_y))
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.parse_register(ops.next().unwrap())?;
        Ok(MMixInstruction::INCL(x, y, z))
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
        Ok(MMixInstruction::INCML(reg, val))
    }

    fn parse_inst_orh(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
        Ok(MMixInstruction::ORML(reg, val))
    }

    fn parse_inst_orl(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
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
        let val = self.parse_number(ops.next().unwrap())? as u16;
        Ok(MMixInstruction::ANDNL(reg, val))
    }

    fn parse_inst_load_store_rrr(
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
            "LDB" => Ok(MMixInstruction::LDB(x, y, z)),
            "LDBU" => Ok(MMixInstruction::LDBU(x, y, z)),
            "LDW" => Ok(MMixInstruction::LDW(x, y, z)),
            "LDWU" => Ok(MMixInstruction::LDWU(x, y, z)),
            "LDT" => Ok(MMixInstruction::LDT(x, y, z)),
            "LDTU" => Ok(MMixInstruction::LDTU(x, y, z)),
            "LDO" => Ok(MMixInstruction::LDO(x, y, z)),
            "LDOU" => Ok(MMixInstruction::LDOU(x, y, z)),
            "LDA" => Ok(MMixInstruction::LDA(x, y, z)),
            "STB" => Ok(MMixInstruction::STB(x, y, z)),
            "STBU" => Ok(MMixInstruction::STBU(x, y, z)),
            "STW" => Ok(MMixInstruction::STW(x, y, z)),
            "STWU" => Ok(MMixInstruction::STWU(x, y, z)),
            "STT" => Ok(MMixInstruction::STT(x, y, z)),
            "STTU" => Ok(MMixInstruction::STTU(x, y, z)),
            "STO" => Ok(MMixInstruction::STO(x, y, z)),
            "STOU" => Ok(MMixInstruction::STOU(x, y, z)),
            _ => Err(format!("Unknown load/store instruction: {}", mnem.as_str())),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_lda_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.parse_number(ops.next().unwrap())? as u8;

        match mnem.as_str().to_uppercase().as_str() {
            "LDA" => Ok(MMixInstruction::LDA(x, y, z)),
            "LDAI" => Ok(MMixInstruction::LDAI(x, y, z)),
            _ => Err(format!("Unknown LDA instruction: {}", mnem.as_str())),
        }
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

    fn parse_inst_arith_rrr(
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
            "ADD" => Ok(MMixInstruction::ADD(x, y, z)),
            "ADDU" => Ok(MMixInstruction::ADDU(x, y, z)),
            "2ADDU" => Ok(MMixInstruction::ADDU2(x, y, z)),
            "4ADDU" => Ok(MMixInstruction::ADDU4(x, y, z)),
            "8ADDU" => Ok(MMixInstruction::ADDU8(x, y, z)),
            "16ADDU" => Ok(MMixInstruction::ADDU16(x, y, z)),
            "SUB" => Ok(MMixInstruction::SUB(x, y, z)),
            "SUBU" => Ok(MMixInstruction::SUBU(x, y, z)),
            "MUL" => Ok(MMixInstruction::MUL(x, y, z)),
            "MULU" => Ok(MMixInstruction::MULU(x, y, z)),
            "DIV" => Ok(MMixInstruction::DIV(x, y, z)),
            "DIVU" => Ok(MMixInstruction::DIVU(x, y, z)),
            "CMP" => Ok(MMixInstruction::CMP(x, y, z)),
            "CMPU" => Ok(MMixInstruction::CMPU(x, y, z)),
            _ => Err(format!("Unknown arithmetic instruction: {}", mnem.as_str())),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_neg_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        // No operand wrapper for inst_neg_rrr - operands are directly in the rule
        let x = self.parse_register(parts.next().unwrap())?;
        let y = self.parse_number(parts.next().unwrap())? as u8;
        let z = self.parse_register(parts.next().unwrap())?;

        match mnem.as_str().to_uppercase().as_str() {
            "NEG" => Ok(MMixInstruction::NEG(x, y, z)),
            "NEGU" => Ok(MMixInstruction::NEGU(x, y, z)),
            _ => Err(format!("Unknown NEG instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_neg_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        // No operand wrapper for inst_neg_rri - operands are directly in the rule
        let x = self.parse_register(parts.next().unwrap())?;
        let y = self.parse_number(parts.next().unwrap())? as u8;
        let z = self.parse_number(parts.next().unwrap())? as u8;

        match mnem.as_str().to_uppercase().as_str() {
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
            "FADD" => Ok(MMixInstruction::FADD(x, y, z)),
            "FSUB" => Ok(MMixInstruction::FSUB(x, y, z)),
            "FMUL" => Ok(MMixInstruction::FMUL(x, y, z)),
            "FDIV" => Ok(MMixInstruction::FDIV(x, y, z)),
            "FREM" => Ok(MMixInstruction::FREM(x, y, z)),
            "FSQRT" => Ok(MMixInstruction::FSQRT(x, y, z)),
            "FINT" => Ok(MMixInstruction::FINT(x, y, z)),
            "FIX" => Ok(MMixInstruction::FIX(x, y, z)),
            "FIXU" => Ok(MMixInstruction::FIXU(x, y, z)),
            "FLOT" => Ok(MMixInstruction::FLOT(x, y, z)),
            "FLOTU" => Ok(MMixInstruction::FLOTU(x, y, z)),
            "SFLOT" => Ok(MMixInstruction::SFLOT(x, y, z)),
            "SFLOTU" => Ok(MMixInstruction::SFLOTU(x, y, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    fn parse_inst_float_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let y = self.parse_register(ops.next().unwrap())?;
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_bitwise_rrr(
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
            "AND" => Ok(MMixInstruction::AND(x, y, z)),
            "OR" => Ok(MMixInstruction::OR(x, y, z)),
            "XOR" => Ok(MMixInstruction::XOR(x, y, z)),
            "ANDN" => Ok(MMixInstruction::ANDN(x, y, z)),
            "ORN" => Ok(MMixInstruction::ORN(x, y, z)),
            "NAND" => Ok(MMixInstruction::NAND(x, y, z)),
            "NOR" => Ok(MMixInstruction::NOR(x, y, z)),
            "NXOR" => Ok(MMixInstruction::NXOR(x, y, z)),
            "MUX" => Ok(MMixInstruction::MUX(x, y, z)),
            _ => Err(format!("Unknown bitwise instruction: {}", mnem.as_str())),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_bitfiddle_rrr(
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
            "BDIF" => Ok(MMixInstruction::BDIF(x, y, z)),
            "WDIF" => Ok(MMixInstruction::WDIF(x, y, z)),
            "TDIF" => Ok(MMixInstruction::TDIF(x, y, z)),
            "ODIF" => Ok(MMixInstruction::ODIF(x, y, z)),
            "SADD" => Ok(MMixInstruction::SADD(x, y, z)),
            "MOR" => Ok(MMixInstruction::MOR(x, y, z)),
            "MXOR" => Ok(MMixInstruction::MXOR(x, y, z)),
            _ => Err(format!(
                "Unknown bit fiddling instruction: {}",
                mnem.as_str()
            )),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_shift_rrr(
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
            "SL" => Ok(MMixInstruction::SL(x, y, z)),
            "SLU" => Ok(MMixInstruction::SLU(x, y, z)),
            "SR" => Ok(MMixInstruction::SR(x, y, z)),
            "SRU" => Ok(MMixInstruction::SRU(x, y, z)),
            _ => Err(format!("Unknown shift instruction: {}", mnem.as_str())),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

        match mnem.as_str().to_uppercase().as_str() {
            "SLI" => Ok(MMixInstruction::SLI(x, y, z)),
            "SLUI" => Ok(MMixInstruction::SLUI(x, y, z)),
            "SRI" => Ok(MMixInstruction::SRI(x, y, z)),
            "SRUI" => Ok(MMixInstruction::SRUI(x, y, z)),
            _ => Err(format!("Unknown shift instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_conditional_set_rrr(
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
            "CSN" => Ok(MMixInstruction::CSN(x, y, z)),
            "CSZ" => Ok(MMixInstruction::CSZ(x, y, z)),
            "CSP" => Ok(MMixInstruction::CSP(x, y, z)),
            "CSOD" => Ok(MMixInstruction::CSOD(x, y, z)),
            "CSNN" => Ok(MMixInstruction::CSNN(x, y, z)),
            "CSNZ" => Ok(MMixInstruction::CSNZ(x, y, z)),
            "CSNP" => Ok(MMixInstruction::CSNP(x, y, z)),
            "CSEV" => Ok(MMixInstruction::CSEV(x, y, z)),
            _ => Err(format!(
                "Unknown conditional set instruction: {}",
                mnem.as_str()
            )),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_zero_or_set_rrr(
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
            "ZSN" => Ok(MMixInstruction::ZSN(x, y, z)),
            "ZSZ" => Ok(MMixInstruction::ZSZ(x, y, z)),
            "ZSP" => Ok(MMixInstruction::ZSP(x, y, z)),
            "ZSOD" => Ok(MMixInstruction::ZSOD(x, y, z)),
            "ZSNN" => Ok(MMixInstruction::ZSNN(x, y, z)),
            "ZSNZ" => Ok(MMixInstruction::ZSNZ(x, y, z)),
            "ZSNP" => Ok(MMixInstruction::ZSNP(x, y, z)),
            "ZSEV" => Ok(MMixInstruction::ZSEV(x, y, z)),
            _ => Err(format!(
                "Unknown zero or set instruction: {}",
                mnem.as_str()
            )),
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
        let z = self.parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_branch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let target = self.parse_number(ops.next().unwrap())?;

        // Calculate relative offset from current instruction
        // Offset is (target - PC) / 4 as a signed 16-bit value
        let pc = self.current_addr;
        let offset_bytes = (target as i64 - pc as i64) as i16;
        let offset = (offset_bytes / 4) as u16;

        match mnem.as_str().to_uppercase().as_str() {
            "JE" => Ok(MMixInstruction::JE(x, offset)),
            "JNE" => Ok(MMixInstruction::JNE(x, offset)),
            "JL" => Ok(MMixInstruction::JL(x, offset)),
            "JG" => Ok(MMixInstruction::JG(x, offset)),
            "BN" => Ok(MMixInstruction::BN(x, offset)),
            "BNB" => Ok(MMixInstruction::BNB(x, offset)),
            "BZ" => Ok(MMixInstruction::BZ(x, offset)),
            "BZB" => Ok(MMixInstruction::BZB(x, offset)),
            "BP" => Ok(MMixInstruction::BP(x, offset)),
            "BPB" => Ok(MMixInstruction::BPB(x, offset)),
            "BOD" => Ok(MMixInstruction::BOD(x, offset)),
            "BODB" => Ok(MMixInstruction::BODB(x, offset)),
            "BNN" => Ok(MMixInstruction::BNN(x, offset)),
            "BNNB" => Ok(MMixInstruction::BNNB(x, offset)),
            "BNZ" => Ok(MMixInstruction::BNZ(x, offset)),
            "BNZB" => Ok(MMixInstruction::BNZB(x, offset)),
            "BNP" => Ok(MMixInstruction::BNP(x, offset)),
            "BNPB" => Ok(MMixInstruction::BNPB(x, offset)),
            "BEV" => Ok(MMixInstruction::BEV(x, offset)),
            "BEVB" => Ok(MMixInstruction::BEVB(x, offset)),
            _ => Err(format!("Unknown branch instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_jmp(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let target = self.parse_number(ops.next().unwrap())?;
        // Calculate relative offset from current instruction
        // Offset is (target - PC) / 4 as a signed 24-bit value
        let pc = self.current_addr;
        let offset = ((target as i64 - pc as i64) / 4) as i32;
        // Mask to 24 bits
        let offset_24 = (offset & 0xFFFFFF) as u32;
        Ok(MMixInstruction::JMP(offset_24))
    }

    fn parse_inst_pbranch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let target = self.parse_number(ops.next().unwrap())?;

        // Calculate relative offset from current instruction
        // PBZ uses YZ as a 16-bit offset: offset = (target - PC) / 4
        let pc = self.current_addr;
        let offset = ((target as i64 - pc as i64) / 4) as i16;
        // Split into Y (high byte) and Z (low byte)
        let offset_u16 = offset as u16;
        let y = ((offset_u16 >> 8) & 0xFF) as u8;
        let z = (offset_u16 & 0xFF) as u8;

        match mnem.as_str().to_uppercase().as_str() {
            "PBN" => Ok(MMixInstruction::PBN(x, y, z)),
            "PBZ" => Ok(MMixInstruction::PBZ(x, y, z)),
            "PBP" => Ok(MMixInstruction::PBP(x, y, z)),
            "PBOD" => Ok(MMixInstruction::PBOD(x, y, z)),
            "PBNN" => Ok(MMixInstruction::PBNN(x, y, z)),
            "PBNZ" => Ok(MMixInstruction::PBNZ(x, y, z)),
            "PBNP" => Ok(MMixInstruction::PBNP(x, y, z)),
            "PBEV" => Ok(MMixInstruction::PBEV(x, y, z)),
            "PBNB" => Ok(MMixInstruction::PBNB(x, y, z)),
            "PBZB" => Ok(MMixInstruction::PBZB(x, y, z)),
            "PBPB" => Ok(MMixInstruction::PBPB(x, y, z)),
            "PBODB" => Ok(MMixInstruction::PBODB(x, y, z)),
            "PBNNB" => Ok(MMixInstruction::PBNNB(x, y, z)),
            "PBNZB" => Ok(MMixInstruction::PBNZB(x, y, z)),
            "PBNPB" => Ok(MMixInstruction::PBNPB(x, y, z)),
            "PBEVB" => Ok(MMixInstruction::PBEVB(x, y, z)),
            _ => Err(format!(
                "Unknown probable branch instruction: {}",
                mnem.as_str()
            )),
        }
    }

    fn parse_inst_geta(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next(); // Skip mnemonic
        let operand = parts.next().unwrap(); // Get operand_reg_imm

        let mut operand_parts = operand.into_inner();
        let reg_pair = operand_parts.next().unwrap();
        let addr_pair = operand_parts.next().unwrap();

        let x = self.parse_register(reg_pair)?;
        let addr = self.parse_number(addr_pair)?;

        debug!(
            "GETA: current_addr=0x{:X}, target_addr=0x{:X}",
            self.current_addr, addr
        );

        // GETA uses relative addressing: calculate offset from current address
        // The offset is split into YZ (16-bit signed)
        let offset = addr.wrapping_sub(self.current_addr) as i64;
        let offset_16 = ((offset >> 2) & 0xFFFF) as u16; // Divide by 4 and take lower 16 bits
        let y = ((offset_16 >> 8) & 0xFF) as u8;
        let z = (offset_16 & 0xFF) as u8;

        debug!(
            "GETA: offset={}, offset_16=0x{:X}, y=0x{:X}, z=0x{:X}",
            offset, offset_16, y, z
        );

        Ok(MMixInstruction::GETA(x, y, z))
    }

    fn parse_inst_getab(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next(); // Skip mnemonic
        let operand = parts.next().unwrap(); // Get operand_reg_imm

        let mut operand_parts = operand.into_inner();
        let reg_pair = operand_parts.next().unwrap();
        let addr_pair = operand_parts.next().unwrap();

        let x = self.parse_register(reg_pair)?;
        let addr = self.parse_number(addr_pair)?;

        // GETAB uses backward relative addressing
        let offset = addr.wrapping_sub(self.current_addr) as i64;
        let offset_16 = ((offset >> 2) & 0xFFFF) as u16;
        let y = ((offset_16 >> 8) & 0xFF) as u8;
        let z = (offset_16 & 0xFF) as u8;

        Ok(MMixInstruction::GETAB(x, y, z))
    }

    fn parse_inst_trap(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        let y = self.parse_number(parts.next().unwrap())? as u8;
        let z = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::TRAP(x, y, z))
    }

    // Helper: parse instruction with format (reg, reg, reg)
    fn parse_rrr<F>(
        &self,
        pair: pest::iterators::Pair<Rule>,
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
        let z = self.parse_register(ops.next().unwrap())?;
        Ok(f(x, y, z))
    }

    // Helper: parse instruction with format (reg, reg, imm)
    fn parse_rri<F>(
        &self,
        pair: pest::iterators::Pair<Rule>,
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
        let z = self.parse_number(ops.next().unwrap())? as u8;
        Ok(f(x, y, z))
    }

    // PUSHJ/PUSHJB: format (reg, imm) where imm is 16-bit offset
    fn parse_inst_pushj(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operand = parts.next().unwrap();
        let mut ops = operand.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let addr = self.parse_number(ops.next().unwrap())?;
        let offset = addr.wrapping_sub(self.current_addr) as i64;
        let offset_16 = ((offset >> 2) & 0xFFFF) as u16;
        let y = ((offset_16 >> 8) & 0xFF) as u8;
        let z = (offset_16 & 0xFF) as u8;
        Ok(MMixInstruction::PUSHJ(x, y, z))
    }

    fn parse_inst_pushjb(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operand = parts.next().unwrap();
        let mut ops = operand.into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let addr = self.parse_number(ops.next().unwrap())?;
        let offset = addr.wrapping_sub(self.current_addr) as i64;
        let offset_16 = ((offset >> 2) & 0xFFFF) as u16;
        let y = ((offset_16 >> 8) & 0xFF) as u8;
        let z = (offset_16 & 0xFF) as u8;
        Ok(MMixInstruction::PUSHJB(x, y, z))
    }

    fn parse_inst_pushgo_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::PUSHGO)
    }

    fn parse_inst_pushgo_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::PUSHGOI)
    }

    fn parse_inst_pop(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        let yz = self.parse_number(parts.next().unwrap())? as u16;
        let y = ((yz >> 8) & 0xFF) as u8;
        let z = (yz & 0xFF) as u8;
        Ok(MMixInstruction::POP(x, y | z))
    }

    fn parse_inst_go_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::GO)
    }

    fn parse_inst_go_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::GOI)
    }

    fn parse_inst_get(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_register(parts.next().unwrap())?;
        // comma is silent in grammar, not in parts
        let z = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::GET(x, z))
    }

    fn parse_inst_put(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        // comma is silent in grammar, not in parts
        let z = self.parse_register(parts.next().unwrap())?;
        Ok(MMixInstruction::PUT(x, z))
    }

    fn parse_inst_puti(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        // comma is silent in grammar, not in parts
        let z = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::PUTI(x, z))
    }

    fn parse_inst_save(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x_pair = parts.next().ok_or("Missing X register in SAVE")?;
        let x = self.parse_register(x_pair)?;
        let z_pair = parts.next().ok_or("Missing Z value in SAVE")?;
        let z = self.parse_number(z_pair)? as u8;
        Ok(MMixInstruction::SAVE(x, z))
    }

    fn parse_inst_unsave(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x_pair = parts.next().ok_or("Missing X value in UNSAVE")?;
        let x = self.parse_number(x_pair)? as u8;
        let z_pair = parts.next().ok_or("Missing Z register in UNSAVE")?;
        let z = self.parse_register(z_pair)?;
        Ok(MMixInstruction::UNSAVE(x, z))
    }

    fn parse_inst_ldunc_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::LDUNC)
    }

    fn parse_inst_ldunc_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::LDUNCI)
    }

    fn parse_inst_stunc_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::STUNC)
    }

    fn parse_inst_stunc_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::STUNCI)
    }

    fn parse_inst_ldht_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::LDHT)
    }

    fn parse_inst_ldht_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::LDHTI)
    }

    fn parse_inst_stht_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::STHT)
    }

    fn parse_inst_stht_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::STHTI)
    }

    fn parse_inst_ldsf_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::LDSF)
    }

    fn parse_inst_ldsf_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::LDSFI)
    }

    fn parse_inst_stsf_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::STSF)
    }

    fn parse_inst_stsf_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::STSFI)
    }

    fn parse_inst_ldvts_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::LDVTS)
    }

    fn parse_inst_ldvts_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::LDVTSI)
    }

    fn parse_inst_cswap_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::CSWAP)
    }

    fn parse_inst_cswap_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::CSWAPI)
    }

    fn parse_inst_stco_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        let _comma1 = parts.next();
        let y = self.parse_register(parts.next().unwrap())?;
        let _comma2 = parts.next();
        let z = self.parse_register(parts.next().unwrap())?;
        Ok(MMixInstruction::STCO(x, y, z))
    }

    fn parse_inst_stco_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        let _comma1 = parts.next();
        let y = self.parse_register(parts.next().unwrap())?;
        let _comma2 = parts.next();
        let z = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::STCOI(x, y, z))
    }

    fn parse_inst_preld_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::PRELD)
    }

    fn parse_inst_preld_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::PRELDI)
    }

    fn parse_inst_prego_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::PREGO)
    }

    fn parse_inst_prego_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::PREGOI)
    }

    fn parse_inst_prest_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::PREST)
    }

    fn parse_inst_prest_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::PRESTI)
    }

    fn parse_inst_syncd_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::SYNCD)
    }

    fn parse_inst_syncd_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::SYNCDI)
    }

    fn parse_inst_syncid_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rrr(pair, MMixInstruction::SYNCID)
    }

    fn parse_inst_syncid_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, MMixInstruction::SYNCIDI)
    }

    fn parse_inst_resume(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let xyz = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::RESUME(xyz))
    }

    fn parse_inst_trip(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let x = self.parse_number(parts.next().unwrap())? as u8;
        let y = self.parse_number(parts.next().unwrap())? as u8;
        let z = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::TRIP(x, y, z))
    }

    fn parse_inst_sync(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let xyz = self.parse_number(parts.next().unwrap())? as u8;
        Ok(MMixInstruction::SYNC(xyz))
    }

    /// Parse a data directive and expand it to potentially multiple instructions
    /// (e.g., BYTE "Hello" becomes multiple BYTE instructions)
    fn parse_data_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<MMixInstruction>, String> {
        let mut parts = pair.into_inner();
        let directive = parts.next().unwrap();

        match directive.as_rule() {
            Rule::directive_byte => {
                let mut result = Vec::new();
                let values_pair = parts.next().unwrap(); // This is byte_values

                let values: Vec<_> = values_pair.into_inner().collect();
                let num_values = values.len();

                for (idx, byte_value) in values.into_iter().enumerate() {
                    let actual_value = byte_value.into_inner().next().unwrap(); // string_literal or number

                    if actual_value.as_rule() == Rule::string_literal {
                        // String literal - expand to one BYTE per character
                        let s = actual_value.as_str();
                        let s = &s[1..s.len() - 1]; // Remove quotes
                        for ch in s.chars() {
                            result.push(MMixInstruction::BYTE(ch as u8));
                        }
                        // Only add null terminator if this is the last value
                        if idx == num_values - 1 {
                            result.push(MMixInstruction::BYTE(0));
                        }
                    } else {
                        let val = self.parse_number(actual_value)? as u8;
                        result.push(MMixInstruction::BYTE(val));
                    }
                }
                Ok(result)
            }
            Rule::directive_wyde => {
                let val = self.parse_number(parts.next().unwrap())? as u16;
                Ok(vec![MMixInstruction::WYDE(val)])
            }
            Rule::directive_tetra => {
                let val = self.parse_number(parts.next().unwrap())? as u32;
                Ok(vec![MMixInstruction::TETRA(val)])
            }
            Rule::directive_octa => {
                let value_pair = parts.next().unwrap();
                let val = self.parse_number(value_pair)?;
                Ok(vec![MMixInstruction::OCTA(val)])
            }
            _ => Err(format!("Unknown data directive: {:?}", directive.as_rule())),
        }
    }

    fn parse_loc_directive(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut parts = pair.into_inner();
        let _directive = parts.next(); // Skip "LOC" keyword
        let addr = self.parse_number(parts.next().unwrap())?;
        self.current_addr = addr;
        Ok(())
    }

    fn parse_is_directive(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut parts = pair.into_inner();
        let symbol_name = parts.next().unwrap().as_str().to_string();
        let _is_keyword = parts.next(); // Skip "IS" keyword
        let value_pair = parts.next().unwrap();

        let value = match value_pair.as_rule() {
            Rule::register => self.parse_register(value_pair)? as u64,
            Rule::expr_value => self.parse_number(value_pair)?,
            _ => return Err("IS directive requires register or expr_value".to_string()),
        };

        self.symbols.insert(symbol_name, value);
        Ok(())
    }

    fn parse_register(&self, pair: pest::iterators::Pair<Rule>) -> Result<u8, String> {
        let text = pair.as_str();
        let (line, col) = pair.line_col();

        // Check if it's a symbolic name from IS directive
        if !text.starts_with('$') {
            // Try to resolve as symbol
            if let Some(&value) = self.symbols.get(text) {
                if value <= 255 {
                    return Ok(value as u8);
                } else {
                    return Err(format!(
                        "Line {}:{}: Symbol '{}' value {} exceeds register range (max 255)",
                        line, col, text, value
                    ));
                }
            }
            return Err(format!(
                "Line {}:{}: Undefined symbol '{}' (expected register like $0 or defined symbol)",
                line, col, text
            ));
        }

        text[1..]
            .parse::<u8>()
            .map_err(|e| format!("Line {}:{}: Invalid register number: {}", line, col, e))
    }

    fn parse_number(&self, pair: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let rule = pair.as_rule();
        let (line, col) = pair.line_col();

        // Handle container rules that have children
        if rule == Rule::expr_value || rule == Rule::number_literal || rule == Rule::operand_imm {
            let inner = pair.into_inner().next().unwrap();
            return self.parse_number(inner);
        }

        // For atomic rules, use the text directly
        let text = pair.as_str();

        match rule {
            Rule::at_symbol => {
                // @ represents the current location
                Ok(self.current_addr)
            }
            Rule::hex_literal => {
                // Support both # and 0x/0X prefixes
                let hex_str = if let Some(stripped) = text.strip_prefix('#') {
                    stripped
                } else if let Some(stripped) =
                    text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
                {
                    stripped
                } else {
                    text
                };
                u64::from_str_radix(hex_str, 16)
                    .map_err(|e| format!("Line {}:{}: Invalid hex number: {}", line, col, e))
            }
            Rule::oct_literal => u64::from_str_radix(&text[1..], 8)
                .map_err(|e| format!("Line {}:{}: Invalid octal number: {}", line, col, e)),
            Rule::dec_literal => text
                .parse::<u64>()
                .map_err(|e| format!("Line {}:{}: Invalid decimal number: {}", line, col, e)),
            Rule::symbol => {
                // Try to resolve as symbol from IS directive or label
                self.symbols
                    .get(text)
                    .or_else(|| self.labels.get(text))
                    .copied()
                    .ok_or_else(|| format!("Line {}:{}: Undefined symbol: {}", line, col, text))
            }
            Rule::identifier => {
                // Backward compatibility: identifier same as symbol
                self.symbols
                    .get(text)
                    .or_else(|| self.labels.get(text))
                    .copied()
                    .ok_or_else(|| format!("Line {}:{}: Undefined symbol: {}", line, col, text))
            }
            _ => Err(format!(
                "Line {}:{}: Expected number, got: {:?}",
                line, col, rule
            )),
        }
    }

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
        crate::mmo::MmoGenerator::new(self.instructions.clone(), self.labels.clone()).generate()
    }
}

// Keep all the existing tests - they should work unchanged
#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_set() {
        let mut asm = MMixAssembler::new("SET $2, 10", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SET(2, 10));
    }

    #[test]
    fn test_parse_set_register() {
        let mut asm = MMixAssembler::new("SET $1, $7", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETRR(1, 7));
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

    // Bit fiddling operations tests (§11-12)
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

    // Shift instruction parsing tests (§14)
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
}
