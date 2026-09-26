use std::collections::{BTreeMap, HashMap, HashSet};

use crate::mmix::{STACK_SEGMENT_START, TrapCode};
use pest_derive::Parser;

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

/// The file, line and column of the first `debug` directive past the
/// string table's 256-entry limit, if the program has one.
type DebugDirectiveOverflow = Option<(String, usize, usize)>;

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
    /// Whether the location counter has passed `#FFFFFFFFFFFFFFFF`: the
    /// last item assembled reached the end of the address space, so
    /// `current_addr` no longer names a valid address. Every statement
    /// that needs one -- an instruction, a data item, a label bound to
    /// the counter, or `@` -- is an error until a `LOC` clears this.
    /// Reset alongside `current_addr` between the two passes.
    past_end: bool,
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
    /// The file, original line and column of the first `debug` directive
    /// past the table's 256-entry limit, if the program has one. `parse`
    /// turns this into an assembly error before walking either pass.
    debug_directive_overflow: DebugDirectiveOverflow,
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
    local_declarations: Vec<(u8, String, usize, usize)>,
    /// Whether the walk is currently between a `BSPEC` and its `ESPEC`.
    in_special_mode: bool,
    /// Where the currently open `BSPEC` was written, for the
    /// unterminated-at-end-of-input diagnostic.
    bspec_open_site: Option<(String, usize, usize)>,
    /// Every predefined symbol's root-namespace key, snapshotted right
    /// after `new` seeds them, before any user statement runs.
    predefined_names: HashSet<String>,
    /// First (file, line) a still-predefined name was named in an operand.
    /// A later label/IS/GREG redefining that name is an error exactly when
    /// this is populated: the reference already saw the predefined value.
    predefined_used_at: HashMap<String, (String, usize)>,
    /// Every warning the last `parse()` raised, in source order: a data
    /// value that overflowed its unit or a bare empty string. `parse()`
    /// still returns `Ok` when these are the only findings.
    warnings: Vec<String>,
}

/// The original (user-facing) source location of an assembled instruction:
/// the file as given on the command line, and the 1-based line in that
/// file's ORIGINAL (un-preprocessed) text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub file: String,
    pub line: usize,
}

impl MMixAssembler {
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
            past_end: false,
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
            warnings: Vec::new(),
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

    /// Every warning the last `parse()` raised, in source order.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Encode a single instruction into bytes using the shared encode module
    pub fn encode_instruction_bytes(&self, instruction: &MMixInstruction) -> Vec<u8> {
        crate::encode::encode_instruction_bytes(instruction)
    }

    /// The `.mmo` this program assembles to: every binary that writes an
    /// object file builds it through this method, so its `GREG` values and
    /// debug strings always reach the file.
    pub fn generate_object_code(&self) -> Vec<u8> {
        crate::mmo::MmoGenerator::new(self.instructions.clone(), self.labels.clone())
            .with_debug_strings(self.debug_strings.clone())
            .with_greg_inits(self.greg_inits.clone())
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
}

mod diagnostics;
mod directives;
mod expressions;
mod instructions;
mod location;
mod operands;
mod passes;
mod preprocess;
mod symbols;
mod tree;

pub use instructions::MMixInstruction;

#[cfg(test)]
mod tests;
