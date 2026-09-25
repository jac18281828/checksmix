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

pub use instructions::MMixInstruction;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmix::{MMix, SpecialReg};
    use crate::mmo::MmoDecoder;

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

    // ---- the address space ends at #FFFFFFFFFFFFFFFF ------------------

    #[test]
    fn test_swym_at_the_top_of_memory_assembles() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions,
            vec![(0xFFFFFFFFFFFFFFFC, MMixInstruction::SWYM(0, 0, 0))]
        );
    }

    #[test]
    fn test_octa_at_the_top_of_memory_assembles() {
        let source = " LOC #FFFFFFFFFFFFFFF8\n OCTA 1\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions,
            vec![(0xFFFFFFFFFFFFFFF8, MMixInstruction::OCTA(1))]
        );
    }

    #[test]
    fn test_two_bytes_fill_the_last_two_addresses_exactly() {
        let source = " LOC #FFFFFFFFFFFFFFFE\n BYTE 1\n BYTE 2\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions,
            vec![
                (0xFFFFFFFFFFFFFFFE, MMixInstruction::BYTE(1)),
                (0xFFFFFFFFFFFFFFFF, MMixInstruction::BYTE(2)),
            ]
        );
    }

    #[test]
    fn test_an_item_after_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:2: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_an_item_extending_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFF\n BYTE 1,2\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:2:2: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_alignment_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFD\n SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:2:2: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_at_symbol_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd IS @\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:8: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_standalone_label_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_loc_after_past_end_restores_a_valid_counter() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n LOC #100\nMain SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Main"), Some(&0x100));
    }

    #[test]
    fn test_past_end_resets_between_the_two_passes() {
        // Pass 1 ends past the end (the second SWYM fills the last byte);
        // pass 2 must start over clean rather than inherit that state, or
        // its own second SWYM would spuriously error.
        let source = " SWYM\n LOC #FFFFFFFFFFFFFFFC\n SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions,
            vec![
                (0, MMixInstruction::SWYM(0, 0, 0)),
                (0xFFFFFFFFFFFFFFFC, MMixInstruction::SWYM(0, 0, 0)),
            ]
        );
    }

    #[test]
    fn test_standalone_local_label_past_the_end_is_an_error() {
        // A bare `1H` binds to the counter the same as a named label, so
        // one past the end is an error rather than silently binding to
        // the last item's address (`$0` reading `#FFFFFFFFFFFFFFF8`
        // through the `1B` reference below, never reached once this
        // errors).
        let source = " LOC #FFFFFFFFFFFFFFF8\n OCTA 7\n1H\n LOC #100\nMain GETA $0,1B\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_loc_lines_own_label_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd LOC #100\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_loc_lines_own_local_label_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n1H LOC #100\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_loc_lines_own_local_label_outranks_at_in_its_operand() {
        // The local label's own site is leftmost on the line, so it is
        // reported even though the operand's `@` needs the same missing
        // address.
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n1H LOC @+4\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_a_label_inside_bspec_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n BSPEC 0\nEnd BYTE 1\n ESPEC\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:4:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_a_local_label_inside_bspec_past_the_end_is_an_error() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n BSPEC 0\n1H BYTE 1\n ESPEC\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:4:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// A label and its instruction both need the address past the end;
    /// the label's own column (leftmost) is reported, not the mnemonic's.
    #[test]
    fn test_a_label_and_its_item_past_the_end_reports_the_labels_column() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_a_local_label_and_its_item_past_the_end_reports_its_column() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n1H SWYM\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn test_a_label_and_its_data_item_past_the_end_reports_the_labels_column() {
        let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd BYTE 1\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        let err = asm.parse().unwrap_err();
        assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// Parses one bare statement for a direct `second_pass_statement` call,
    /// bypassing `parse_two_pass`'s two-pass walk entirely.
    fn lone_statement(source: &'static str) -> pest::iterators::Pair<'static, Rule> {
        use pest::Parser;
        MMixalParser::parse(Rule::statement, source)
            .unwrap()
            .next()
            .unwrap()
    }

    /// Two-operand `LDA` is one tetra whatever its operand, so pass 1 and
    /// pass 2 always place every item at the same address and no source
    /// program can make pass 2 alone reach a past-end statement pass 1
    /// missed. The seven tests below drive `second_pass_statement` directly
    /// instead, one per `require_addr`/`require_valid` call it makes, and
    /// each still fails if its call is replaced with `.expect(...)`. The
    /// instruction-align and data-directive-align tests below pin a
    /// located error, never a panic, at the align call: `place_item`'s own
    /// `past_end` guard would report the same error if the align call's
    /// guard were skipped, so those two alone do not isolate the align
    /// check.
    ///
    /// Pins the instruction's own alignment check.
    #[test]
    fn test_pass_2_disagreeing_with_pass_1_on_size_is_a_located_error_not_a_panic() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.past_end = true;
        let err = asm
            .second_pass_statement(lone_statement("SWYM"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// `place_item` rejects a second item in the same data directive once
    /// the first has already filled the address space's last byte, rather
    /// than reusing its address: `current_addr` sits at the last address an
    /// `OCTA` can start from, so the first of two exactly fills the top and
    /// the second must still be rejected, not silently placed at the same
    /// address.
    #[test]
    fn test_items_after_the_end_within_one_directive_never_overlap() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.current_addr = u64::MAX - 7;
        let err = asm
            .second_pass_statement(lone_statement("OCTA 1,2"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// Pins the standalone-label site at the end of `second_pass_statement`.
    #[test]
    fn test_pass_2_only_overrun_on_a_standalone_label_is_an_error() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.past_end = true;
        let err = asm
            .second_pass_statement(lone_statement("End"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// Pins `second_pass_statement`'s `loc_directive` label site.
    #[test]
    fn test_pass_2_only_overrun_on_a_locs_own_label_is_an_error() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.past_end = true;
        let err = asm
            .second_pass_statement(lone_statement("End LOC #100"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// Pins `second_pass_statement`'s special-mode label site.
    #[test]
    fn test_pass_2_only_overrun_on_a_label_inside_bspec_is_an_error() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.past_end = true;
        asm.in_special_mode = true;
        let err = asm
            .second_pass_statement(lone_statement("End BYTE 1"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// Pins the data-directive alignment's `require_addr` call, which
    /// `.expect` also passed until now.
    #[test]
    fn test_pass_2_only_overrun_on_a_data_directives_alignment_is_an_error() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.past_end = true;
        let err = asm
            .second_pass_statement(lone_statement("OCTA 1"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
    }

    /// Pins an instruction's own `place_item` call: `SETI` is 16 bytes
    /// though its alignment is 4, so the last 4-aligned address still
    /// overruns placing it, where a plain 4-byte instruction never could.
    #[test]
    fn test_pass_2_only_overrun_on_the_final_instructions_place_is_an_error() {
        let mut asm = MMixAssembler::new("", "<test>");
        asm.current_addr = u64::MAX - 3;
        let err = asm
            .second_pass_statement(lone_statement("SETI $1,5"))
            .unwrap_err();
        assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
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
        let source =
            "Base\tGREG\t1\nMain\tdebug \"hi\"\nStart\tLDA\t$255,Start\n\tTRAP\t0,Halt,0\n";

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
        let source = "Base\tGREG\t1\nMain\tdebug \"hi\"\nStart\tLDA\t$255,Start\n";
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().unwrap();

        let text = asm.source_text("<test>", 2).expect("line 2 exists");
        assert!(
            text.contains("debug"),
            "expected original text, got {text:?}"
        );
        assert!(
            !text.contains("PUSHJ"),
            "source_text must not leak preprocessed text, got {text:?}"
        );

        let lda_text = asm.source_text("<test>", 3).expect("line 3 exists");
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
            err, "<test>:6:1: symbol 'Foo' redefined (first defined at <test>:4)",
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
    fn test_lda_two_operand_form_always_takes_the_base_address_path() {
        // LDA resolves against Base exactly as LDO does, whatever the
        // address's own value.
        assert_first_instruction(
            "Base GREG #1000\nLDA $1,Data\nData IS #1000",
            MMixInstruction::LDAI(1, 254, 0),
        );
    }

    #[test]
    fn test_forward_lda_keeps_every_label_in_place() {
        // LDA costs one tetra regardless of whether its operand has
        // resolved yet, so pass 1 and pass 2 agree on every label after it
        // (the scan's `lda_fwd2`). Main's SET reads After before pass 2
        // revisits it, so it still carries pass 1's own estimate -- equal
        // to After's own SET only if that estimate already matches.
        let mut asm = MMixAssembler::new(
            "LOC #100\nBase GREG 1\nMain SET $3,After\nLDA $1,K\n\
             After SET $2,After\nTRAP 0,Halt,0\nK IS 5\n",
            "<test>",
        );
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        let after_addr = *asm.labels.get("After").expect("After label");
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::SETL(3, after_addr as u16)
        );
        assert_eq!(
            asm.instructions[2].1,
            MMixInstruction::SETL(2, after_addr as u16)
        );
        assert_eq!(asm.instructions[1].1, MMixInstruction::LDAI(1, 254, 4));
    }

    #[test]
    fn test_lda_pure_value_never_encodes_addu_register_form() {
        // With no GREG in scope, a pure second operand is the base-address
        // error, never register form #22.
        assert_eq!(
            assemble_err("LDA $1,5"),
            "<test>:1:8: no GREG before this instruction holds a base \
             address 0 to 255 bytes below 0x5"
        );
        // With a base in scope, it's LDAI -- opcode #23, never #22.
        let mut asm = MMixAssembler::new("B GREG 1\nLDA $1,5", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDAI(1, 254, 4));
        assert_eq!(
            asm.encode_instruction_bytes(&asm.instructions[0].1)[0],
            0x23
        );
    }

    /// Assemble `src` and return its LDA/LDO/etc. instruction: the last
    /// item in `instructions`, since every case here places exactly one
    /// data directive (an `OCTA` base value) ahead of the instruction under
    /// test.
    fn last_instruction(src: &str) -> MMixInstruction {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
        asm.instructions
            .last()
            .unwrap_or_else(|| panic!("no instructions produced for {src:?}"))
            .1
            .clone()
    }

    #[test]
    fn test_lda_base_search_matches_the_memory_forms() {
        let source = |addr: &str| {
            format!("LOC Data_Segment\nBase GREG @\nX OCTA 7\nLOC #100\nMain LDA $1,{addr}")
        };
        // Offset 0: Base itself covers X.
        assert_eq!(
            last_instruction(&source("X")),
            MMixInstruction::LDAI(1, 254, 0)
        );
        // Offset 255 assembles; 256 is the base-address error.
        assert_eq!(
            last_instruction(&source("X+255")),
            MMixInstruction::LDAI(1, 254, 255)
        );
        assert_eq!(
            assemble_err(&source("X+256")),
            "<test>:5:13: no GREG before this instruction holds a base \
             address 0 to 255 bytes below 0x2000000000000100"
        );
        // LDAI matches LDA.
        assert_eq!(
            last_instruction(&source("X")),
            last_instruction(&source("X").replacen("LDA", "LDAI", 1))
        );
    }

    #[test]
    fn test_lda_base_search_ignores_a_greg_appearing_after_the_instruction() {
        // Closer holds Y's own value (offset 0), but it comes after Main:
        // the search bounds itself to GREGs already seen, so Base (offset
        // 8) wins regardless.
        assert_eq!(
            last_instruction(
                "LOC Data_Segment\nBase GREG @\nX OCTA 7\nY OCTA 9\n\
                 LOC #100\nMain LDA $1,Y\nCloser GREG Y"
            ),
            MMixInstruction::LDAI(1, 254, 8)
        );
    }

    #[test]
    fn test_lda_register_operand_is_offset_zero() {
        assert_first_instruction("LDA $3,$2", MMixInstruction::LDAI(3, 2, 0));
        assert_first_instruction("x IS $2\nLDA $3,x", MMixInstruction::LDAI(3, 2, 0));
    }

    #[test]
    fn test_lda_one_tetra_even_with_an_unresolved_forward_operand() {
        // Far exceeds #FF and is a forward reference; LDA still costs one
        // tetra, so After sits exactly 4 bytes past Main. Pre's own operand
        // reads After before pass 2 revisits it, so it still carries pass
        // 1's estimate of After's address -- proving that estimate is
        // already exact, not merely that pass 2's own later walk is.
        let mut asm = MMixAssembler::new(
            "Base GREG #150\nPre SET $2,After\nMain LDA $1,Far\nAfter HALT\nFar IS #200",
            "<test>",
        );
        asm.parse().unwrap();
        let main_addr = *asm.labels.get("Main").unwrap();
        let after_addr = *asm.labels.get("After").unwrap();
        assert_eq!(after_addr, main_addr + 4);
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::SETL(2, after_addr as u16)
        );
        assert_eq!(asm.instructions[1].1, MMixInstruction::LDAI(1, 254, 176));
    }

    #[test]
    fn test_greg_limit_is_223_and_the_last_is_32() {
        let mut source = String::new();
        for i in 0..223 {
            source.push_str(&format!("G{i}\tGREG\t0\n"));
        }
        source.push_str("Main\tHALT\n");
        let mut asm = MMixAssembler::new(&source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("223 GREGs must assemble: {e}"));
        assert_eq!(asm.greg_inits.last().map(|&(reg, _)| reg), Some(32));
    }

    #[test]
    fn test_the_224th_greg_reports_the_limit_at_its_line_and_column() {
        let mut source = String::new();
        for i in 0..224 {
            source.push_str(&format!("G{i}\tGREG\t0\n"));
        }
        source.push_str("Main\tHALT\n");
        assert_eq!(
            assemble_err(&source),
            "<test>:224:6: GREG has no global register left: \
             $32 through $254 are all allocated"
        );
    }

    #[test]
    fn test_generate_object_code_carries_greg_values_through_the_loader() {
        let mut asm = MMixAssembler::new("Base GREG #1234\nMain HALT\n", "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse: {e}"));
        let object_code = asm.generate_object_code();

        let decoder = MmoDecoder::new(object_code);
        let mut mmix = MMix::new();
        decoder
            .load(&mut mmix)
            .expect("generate_object_code's output must load");

        assert_eq!(mmix.get_special(SpecialReg::RG), 254);
        assert_eq!(mmix.get_register(254), 0x1234);
    }

    // ---- Locations: file:line:col on every assembler error ----------------

    #[test]
    fn test_symbol_redefined_names_file_line_and_column() {
        assert_eq!(
            assemble_err("X IS 1\nMain HALT;X IS 2\n"),
            "<test>:2:11: symbol 'X' redefined (first defined at <test>:1)"
        );
    }

    #[test]
    fn test_predefined_symbol_redefined_after_use_names_file_line_and_column() {
        assert_eq!(
            assemble_err("Main SET $1,Fputs\nSetup HALT;Fputs IS 9\n"),
            "<test>:2:12: predefined symbol 'Fputs' redefined after its \
             value was used at <test>:1"
        );
    }

    #[test]
    fn test_too_many_debug_directives_names_file_line_and_column() {
        let mut source = String::new();
        for _ in 0..256 {
            source.push_str("debug \"x\"\n");
        }
        source.push_str("L debug \"overflow\"\n");
        assert_eq!(
            assemble_err(&source),
            "<test>:257:3: error: too many `debug` directives in this \
             program; the string table holds at most 256"
        );
    }

    #[test]
    fn test_bspec_unterminated_names_the_bspec_keywords_column() {
        assert_eq!(
            assemble_err("Main HALT\n\tBSPEC 1\nBYTE 1\n"),
            "<test>:2:2: syntax error: BSPEC has no matching ESPEC before end of input"
        );
    }

    #[test]
    fn test_local_over_threshold_names_the_operands_column() {
        assert_eq!(
            assemble_err("G1 GREG 0\nLOCAL $254\nMain HALT\n"),
            "<test>:2:7: LOCAL $254 is not below the global threshold $254"
        );
    }

    #[test]
    fn test_bspec_content_error_names_the_offending_opcodes_column() {
        assert_eq!(
            assemble_err("BSPEC 1\nMain HALT\nESPEC\n"),
            "<test>:2:6: syntax error: an instruction is not allowed inside BSPEC/ESPEC"
        );
    }

    #[test]
    fn test_takes_no_label_names_the_labels_column() {
        assert_eq!(
            assemble_err("Main HALT;Foo ESPEC\n"),
            "<test>:1:11: syntax error: ESPEC takes no label"
        );
    }

    #[test]
    fn test_bspec_does_not_nest_names_the_inner_keywords_column() {
        assert_eq!(
            assemble_err("BSPEC 1\n\tBSPEC 2\nESPEC\nESPEC\nMain HALT\n"),
            "<test>:2:2: syntax error: BSPEC does not nest"
        );
    }

    #[test]
    fn test_bspec_operand_too_wide_names_the_operands_column() {
        assert_eq!(
            assemble_err("BSPEC #10000\nMain HALT\nESPEC\n"),
            "<test>:1:7: syntax error: BSPEC operand 65536 does not fit in two bytes"
        );
    }

    #[test]
    fn test_espec_unmatched_names_the_espec_keywords_column() {
        assert_eq!(
            assemble_err("Main HALT\n\tESPEC\n"),
            "<test>:2:2: syntax error: ESPEC has no matching BSPEC"
        );
    }

    #[test]
    fn test_include_cycle_names_the_including_files_own_include_line() {
        let reader = fixture_reader(vec![
            ("a.mms", "INCLUDE b.mms\n"),
            ("b.mms", "  INCLUDE a.mms\n"),
        ]);
        let err = MMixAssembler::resolve_includes(
            "INCLUDE a.mms\n",
            "driver.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "b.mms:1:3: include cycle detected: a.mms -> b.mms -> a.mms"
        );
    }

    #[test]
    fn test_include_unreadable_file_names_the_including_files_own_include_line() {
        let reader = fixture_reader(vec![]);
        let err = MMixAssembler::resolve_includes(
            "\n\tINCLUDE missing.mms\n",
            "root.mms",
            std::path::Path::new(""),
            &reader,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "root.mms:2:2: cannot read included file 'missing.mms': no such fixture file"
        );
    }

    #[test]
    fn test_locate_fallback_prefixes_an_unlocated_error_and_passes_a_located_one_through() {
        let asm = MMixAssembler::new("", "<test>");
        assert_eq!(
            asm.locate_fallback("Empty instruction".to_string(), 3, 5),
            "<test>:3:5: Empty instruction"
        );
        let located = "<test>:1:1: symbol 'X' redefined (first defined at <test>:1)".to_string();
        assert_eq!(asm.locate_fallback(located.clone(), 3, 5), located);
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

    // ---- The data-unit warning channel ----------------------------------

    #[test]
    fn test_data_unit_overflow_warns_and_keeps_low_bytes() {
        let mut asm = MMixAssembler::new("BYTE 300", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0x2C));
        assert_eq!(
            asm.warnings(),
            ["<test>:1:6: warning: value 300 does not fit in a byte; \
              its low byte assembles"]
        );

        let mut asm = MMixAssembler::new("WYDE #12345", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::WYDE(0x2345));
        assert_eq!(
            asm.warnings(),
            ["<test>:1:6: warning: value 74565 does not fit in a wyde; \
              its low wyde assembles"]
        );

        let mut asm = MMixAssembler::new("TETRA #100000000", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::TETRA(0));
        assert_eq!(
            asm.warnings(),
            [
                "<test>:1:7: warning: value 4294967296 does not fit in a tetra; \
              its low tetra assembles"
            ]
        );

        let mut asm = MMixAssembler::new("BYTE -1", "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.warnings(),
            ["<test>:1:6: warning: value -1 does not fit in a byte; \
              its low byte assembles"]
        );

        for src in ["BYTE 255", "OCTA -1"] {
            let mut asm = MMixAssembler::new(src, "<test>");
            asm.parse().unwrap();
            assert!(asm.warnings().is_empty(), "{src:?} should not warn");
        }
    }

    #[test]
    fn test_data_list_two_overflowing_items_warn_in_order_once_each() {
        let mut asm = MMixAssembler::new("BYTE 300,300", "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.warnings(),
            [
                "<test>:1:6: warning: value 300 does not fit in a byte; \
                 its low byte assembles",
                "<test>:1:10: warning: value 300 does not fit in a byte; \
                 its low byte assembles",
            ]
        );
    }

    #[test]
    fn test_warnings_reports_only_the_last_parse() {
        let mut asm = MMixAssembler::new("BYTE 300", "<test>");
        asm.parse().unwrap();
        asm.parse().unwrap();
        assert_eq!(asm.warnings().len(), 1);
    }

    #[test]
    fn test_data_item_two_overflowing_values_warn_twice_at_its_column() {
        let mut asm = MMixAssembler::new(r#"BYTE 300+"ab"+300"#, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.warnings(),
            [
                "<test>:1:6: warning: value 397 does not fit in a byte; \
                 its low byte assembles",
                "<test>:1:6: warning: value 398 does not fit in a byte; \
                 its low byte assembles",
            ]
        );
    }

    #[test]
    fn test_string_character_overflow_warns_at_its_opening_quote() {
        let mut asm = MMixAssembler::new(r#"BYTE "a"+300"#, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(141));
        assert_eq!(
            asm.warnings(),
            ["<test>:1:6: warning: value 397 does not fit in a byte; \
              its low byte assembles"]
        );
    }

    #[test]
    fn test_string_character_above_ff_warns_as_a_byte_overflow() {
        let mut asm = MMixAssembler::new("BYTE \"€\"", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0xAC));
        assert_eq!(
            asm.warnings(),
            ["<test>:1:6: warning: value 8364 does not fit in a byte; \
              its low byte assembles"]
        );

        let mut asm = MMixAssembler::new("BYTE \"€€\"", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions.len(), 2);
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0xAC));
        assert_eq!(asm.instructions[1].1, MMixInstruction::BYTE(0xAC));
        assert_eq!(
            asm.warnings(),
            [
                "<test>:1:6: warning: value 8364 does not fit in a byte; \
                 its low byte assembles",
                "<test>:1:6: warning: value 8364 does not fit in a byte; \
                 its low byte assembles",
            ]
        );
    }

    // ---- The bare empty string --------------------------------------------

    #[test]
    fn test_bare_empty_string_assembles_one_zero_unit_and_warns() {
        let mut asm = MMixAssembler::new(r#"BYTE """#, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0));
        assert_eq!(
            asm.warnings(),
            ["<test>:1:6: warning: an empty string assembles as one zero byte"]
        );

        let mut asm = MMixAssembler::new(r#"WYDE """#, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::WYDE(0));

        let mut asm = MMixAssembler::new(r#"OCTA """#, "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(0));

        let mut asm = MMixAssembler::new(r#"BYTE "",1"#, "<test>");
        asm.parse().unwrap();
        assert_eq!(
            asm.instructions[0..2]
                .iter()
                .map(|(_, i)| i.clone())
                .collect::<Vec<_>>(),
            vec![MMixInstruction::BYTE(0), MMixInstruction::BYTE(1)]
        );
    }

    /// A bare `""` sizes to one unit in pass 1 (`data_value_unit_count`)
    /// the same as pass 2's synthesized zero, so a forward reference past
    /// it lands at the same address either pass computes -- the same
    /// property `test_byte_string_pass1_pass2_agree` proves for a string.
    #[test]
    fn test_bare_empty_string_pass1_pass2_agree() {
        let mut asm = MMixAssembler::new("OCTA Label\nBYTE \"\"\nLabel BYTE 7", "<test>");
        asm.parse().unwrap();
        assert_eq!(asm.labels.get("Label"), Some(&9));
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(9));
        assert_eq!(asm.instructions[2].1, MMixInstruction::BYTE(7));
    }

    #[test]
    fn test_bare_empty_string_beside_an_operator_or_in_parens_is_an_error() {
        assert_eq!(
            assemble_err(r#"BYTE 2*"""#),
            "<test>:1:8: an empty string is not a value inside an expression"
        );
        assert_eq!(
            assemble_err(r#"BYTE ("")"#),
            "<test>:1:7: an empty string is not a value inside an expression"
        );
    }
}
