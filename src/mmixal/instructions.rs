//! Instruction parsing: `MMixInstruction` and the dispatch shared by every family.

use super::MMixAssembler;
use super::Rule;

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

impl MMixAssembler {
    pub(super) fn parse_instruction(
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

    /// Splits a pure 16-bit value into its high and low bytes: Y and Z for
    /// the two-operand forms `TRAP`'s family, `POP`, `PUSHJ` and `PUSHJB`
    /// each resolve a combined field into.
    pub(super) fn split_hi_lo_byte(value: u16) -> (u8, u8) {
        ((value >> 8) as u8, (value & 0xFF) as u8)
    }

    /// Splits a pure 24-bit value into X, Y and Z: the one-operand form
    /// `TRAP`'s family and `POP` both resolve a combined field into.
    pub(super) fn split_xyz_bytes(value: u32) -> (u8, u8, u8) {
        (
            (value >> 16) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        )
    }

    // Helper: parse instruction with format (reg, reg, imm)
    pub(super) fn parse_rri<F>(
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
}

mod floating_point;
mod integer;
mod load_store;
mod wyde_immediate;
