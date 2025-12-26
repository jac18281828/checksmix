// MMIX Instruction Encoding Module
//
// This module provides instruction encoding functionality for MMIX instructions.
// It converts MMixInstruction enum variants into their byte representations
// according to the MMIX specification.

use crate::mmixal::MMixInstruction;

/// Encode a MMIX instruction into its byte representation
pub fn encode_instruction_bytes(instruction: &MMixInstruction) -> Vec<u8> {
    let mut bytes = Vec::new();

    match instruction {
        MMixInstruction::SET(x, value) => {
            let b0 = (value >> 48) as u16;
            let b1 = (value >> 32) as u16;
            let b2 = (value >> 16) as u16;
            let b3 = *value as u16;
            bytes.extend_from_slice(&encode_instruction(0xE0, *x, b0)); // SETH
            bytes.extend_from_slice(&encode_instruction(0xE1, *x, b1)); // SETMH
            bytes.extend_from_slice(&encode_instruction(0xE2, *x, b2)); // SETML
            bytes.extend_from_slice(&encode_instruction(0xE3, *x, b3)); // SETL
        }
        MMixInstruction::SETRR(x, y) => {
            // SET $X, $Y -> ORI $X, $Y, 0 (machine copy)
            bytes.extend_from_slice(&[0xC1, *x, *y, 0]);
        }
        MMixInstruction::SETL(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE3, *x, *yz));
        }
        MMixInstruction::SETH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE0, *x, *yz));
        }
        MMixInstruction::SETMH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE1, *x, *yz));
        }
        MMixInstruction::SETML(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE2, *x, *yz));
        }
        MMixInstruction::BYTE(value) => {
            bytes.push(*value);
        }
        MMixInstruction::WYDE(value) => {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        MMixInstruction::TETRA(value) => {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        MMixInstruction::OCTA(value) => {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        MMixInstruction::LDA(x, y, z) => {
            bytes.extend_from_slice(&[0x22, *x, *y, *z]);
        }
        MMixInstruction::LDAI(x, y, z) => {
            bytes.extend_from_slice(&[0x23, *x, *y, *z]);
        }
        // Arithmetic instructions
        MMixInstruction::ADD(x, y, z) => {
            bytes.extend_from_slice(&[0x20, *x, *y, *z]);
        }
        MMixInstruction::ADDI(x, y, z) => {
            bytes.extend_from_slice(&[0x21, *x, *y, *z]);
        }
        MMixInstruction::ADDU(x, y, z) => {
            bytes.extend_from_slice(&[0x22, *x, *y, *z]); // Same as LDA
        }
        MMixInstruction::ADDUI(x, y, z) => {
            bytes.extend_from_slice(&[0x23, *x, *y, *z]); // Same as LDAI
        }
        MMixInstruction::ADDU2(x, y, z) => {
            bytes.extend_from_slice(&[0x28, *x, *y, *z]);
        }
        MMixInstruction::ADDU2I(x, y, z) => {
            bytes.extend_from_slice(&[0x29, *x, *y, *z]);
        }
        MMixInstruction::ADDU4(x, y, z) => {
            bytes.extend_from_slice(&[0x2A, *x, *y, *z]);
        }
        MMixInstruction::ADDU4I(x, y, z) => {
            bytes.extend_from_slice(&[0x2B, *x, *y, *z]);
        }
        MMixInstruction::ADDU8(x, y, z) => {
            bytes.extend_from_slice(&[0x2C, *x, *y, *z]);
        }
        MMixInstruction::ADDU8I(x, y, z) => {
            bytes.extend_from_slice(&[0x2D, *x, *y, *z]);
        }
        MMixInstruction::ADDU16(x, y, z) => {
            bytes.extend_from_slice(&[0x2E, *x, *y, *z]);
        }
        MMixInstruction::ADDU16I(x, y, z) => {
            bytes.extend_from_slice(&[0x2F, *x, *y, *z]);
        }
        MMixInstruction::SUB(x, y, z) => {
            bytes.extend_from_slice(&[0x24, *x, *y, *z]);
        }
        MMixInstruction::SUBI(x, y, z) => {
            bytes.extend_from_slice(&[0x25, *x, *y, *z]);
        }
        MMixInstruction::SUBU(x, y, z) => {
            bytes.extend_from_slice(&[0x26, *x, *y, *z]);
        }
        MMixInstruction::SUBUI(x, y, z) => {
            bytes.extend_from_slice(&[0x27, *x, *y, *z]);
        }
        MMixInstruction::NEG(x, y, z) => {
            bytes.extend_from_slice(&[0x34, *x, *y, *z]);
        }
        MMixInstruction::NEGI(x, y, z) => {
            bytes.extend_from_slice(&[0x35, *x, *y, *z]);
        }
        MMixInstruction::NEGU(x, y, z) => {
            bytes.extend_from_slice(&[0x36, *x, *y, *z]);
        }
        MMixInstruction::NEGUI(x, y, z) => {
            bytes.extend_from_slice(&[0x37, *x, *y, *z]);
        }
        MMixInstruction::MUL(x, y, z) => {
            bytes.extend_from_slice(&[0x18, *x, *y, *z]);
        }
        MMixInstruction::MULI(x, y, z) => {
            bytes.extend_from_slice(&[0x19, *x, *y, *z]);
        }
        MMixInstruction::MULU(x, y, z) => {
            bytes.extend_from_slice(&[0x1A, *x, *y, *z]);
        }
        MMixInstruction::MULUI(x, y, z) => {
            bytes.extend_from_slice(&[0x1B, *x, *y, *z]);
        }
        MMixInstruction::DIV(x, y, z) => {
            bytes.extend_from_slice(&[0x1C, *x, *y, *z]);
        }
        MMixInstruction::DIVI(x, y, z) => {
            bytes.extend_from_slice(&[0x1D, *x, *y, *z]);
        }
        MMixInstruction::DIVU(x, y, z) => {
            bytes.extend_from_slice(&[0x1E, *x, *y, *z]);
        }
        MMixInstruction::DIVUI(x, y, z) => {
            bytes.extend_from_slice(&[0x1F, *x, *y, *z]);
        }
        // Comparison instructions
        MMixInstruction::CMP(x, y, z) => {
            bytes.extend_from_slice(&[0x30, *x, *y, *z]);
        }
        MMixInstruction::CMPI(x, y, z) => {
            bytes.extend_from_slice(&[0x31, *x, *y, *z]);
        }
        MMixInstruction::CMPU(x, y, z) => {
            bytes.extend_from_slice(&[0x32, *x, *y, *z]);
        }
        MMixInstruction::CMPUI(x, y, z) => {
            bytes.extend_from_slice(&[0x33, *x, *y, *z]);
        }
        // Bitwise instructions
        MMixInstruction::AND(x, y, z) => {
            bytes.extend_from_slice(&[0xC8, *x, *y, *z]);
        }
        MMixInstruction::ANDI(x, y, z) => {
            bytes.extend_from_slice(&[0xC9, *x, *y, *z]);
        }
        MMixInstruction::OR(x, y, z) => {
            bytes.extend_from_slice(&[0xC0, *x, *y, *z]);
        }
        MMixInstruction::ORI(x, y, z) => {
            bytes.extend_from_slice(&[0xC1, *x, *y, *z]);
        }
        MMixInstruction::XOR(x, y, z) => {
            bytes.extend_from_slice(&[0xC6, *x, *y, *z]);
        }
        MMixInstruction::XORI(x, y, z) => {
            bytes.extend_from_slice(&[0xC7, *x, *y, *z]);
        }
        MMixInstruction::ANDN(x, y, z) => {
            bytes.extend_from_slice(&[0xCA, *x, *y, *z]);
        }
        MMixInstruction::ANDNI(x, y, z) => {
            bytes.extend_from_slice(&[0xCB, *x, *y, *z]);
        }
        MMixInstruction::ORN(x, y, z) => {
            bytes.extend_from_slice(&[0xC2, *x, *y, *z]);
        }
        MMixInstruction::ORNI(x, y, z) => {
            bytes.extend_from_slice(&[0xC3, *x, *y, *z]);
        }
        MMixInstruction::NAND(x, y, z) => {
            bytes.extend_from_slice(&[0xCC, *x, *y, *z]);
        }
        MMixInstruction::NANDI(x, y, z) => {
            bytes.extend_from_slice(&[0xCD, *x, *y, *z]);
        }
        MMixInstruction::NOR(x, y, z) => {
            bytes.extend_from_slice(&[0xC4, *x, *y, *z]);
        }
        MMixInstruction::NORI(x, y, z) => {
            bytes.extend_from_slice(&[0xC5, *x, *y, *z]);
        }
        MMixInstruction::NXOR(x, y, z) => {
            bytes.extend_from_slice(&[0xCE, *x, *y, *z]);
        }
        MMixInstruction::NXORI(x, y, z) => {
            bytes.extend_from_slice(&[0xCF, *x, *y, *z]);
        }
        // Bit fiddling
        MMixInstruction::BDIF(x, y, z) => {
            bytes.extend_from_slice(&[0xD0, *x, *y, *z]);
        }
        MMixInstruction::BDIFI(x, y, z) => {
            bytes.extend_from_slice(&[0xD1, *x, *y, *z]);
        }
        MMixInstruction::WDIF(x, y, z) => {
            bytes.extend_from_slice(&[0xD2, *x, *y, *z]);
        }
        MMixInstruction::WDIFI(x, y, z) => {
            bytes.extend_from_slice(&[0xD3, *x, *y, *z]);
        }
        MMixInstruction::TDIF(x, y, z) => {
            bytes.extend_from_slice(&[0xD4, *x, *y, *z]);
        }
        MMixInstruction::TDIFI(x, y, z) => {
            bytes.extend_from_slice(&[0xD5, *x, *y, *z]);
        }
        MMixInstruction::ODIF(x, y, z) => {
            bytes.extend_from_slice(&[0xD6, *x, *y, *z]);
        }
        MMixInstruction::ODIFI(x, y, z) => {
            bytes.extend_from_slice(&[0xD7, *x, *y, *z]);
        }
        MMixInstruction::MUX(x, y, z) => {
            bytes.extend_from_slice(&[0xD8, *x, *y, *z]);
        }
        MMixInstruction::MUXI(x, y, z) => {
            bytes.extend_from_slice(&[0xD9, *x, *y, *z]);
        }
        MMixInstruction::SADD(x, y, z) => {
            bytes.extend_from_slice(&[0xDA, *x, *y, *z]);
        }
        MMixInstruction::SADDI(x, y, z) => {
            bytes.extend_from_slice(&[0xDB, *x, *y, *z]);
        }
        MMixInstruction::MOR(x, y, z) => {
            bytes.extend_from_slice(&[0xDC, *x, *y, *z]);
        }
        MMixInstruction::MORI(x, y, z) => {
            bytes.extend_from_slice(&[0xDD, *x, *y, *z]);
        }
        MMixInstruction::MXOR(x, y, z) => {
            bytes.extend_from_slice(&[0xDE, *x, *y, *z]);
        }
        MMixInstruction::MXORI(x, y, z) => {
            bytes.extend_from_slice(&[0xDF, *x, *y, *z]);
        }
        // Load special
        MMixInstruction::LDSF(x, y, z) => {
            bytes.extend_from_slice(&[0x90, *x, *y, *z]);
        }
        MMixInstruction::LDSFI(x, y, z) => {
            bytes.extend_from_slice(&[0x91, *x, *y, *z]);
        }
        MMixInstruction::LDHT(x, y, z) => {
            bytes.extend_from_slice(&[0x92, *x, *y, *z]);
        }
        MMixInstruction::LDHTI(x, y, z) => {
            bytes.extend_from_slice(&[0x93, *x, *y, *z]);
        }
        MMixInstruction::LDVTS(x, y, z) => {
            bytes.extend_from_slice(&[0x98, *x, *y, *z]);
        }
        MMixInstruction::LDVTSI(x, y, z) => {
            bytes.extend_from_slice(&[0x99, *x, *y, *z]);
        }
        // Shifts
        MMixInstruction::SL(x, y, z) => {
            bytes.extend_from_slice(&[0x38, *x, *y, *z]);
        }
        MMixInstruction::SLI(x, y, z) => {
            bytes.extend_from_slice(&[0x39, *x, *y, *z]);
        }
        MMixInstruction::SLU(x, y, z) => {
            bytes.extend_from_slice(&[0x3A, *x, *y, *z]);
        }
        MMixInstruction::SLUI(x, y, z) => {
            bytes.extend_from_slice(&[0x3B, *x, *y, *z]);
        }
        MMixInstruction::SR(x, y, z) => {
            bytes.extend_from_slice(&[0x3C, *x, *y, *z]);
        }
        MMixInstruction::SRI(x, y, z) => {
            bytes.extend_from_slice(&[0x3D, *x, *y, *z]);
        }
        MMixInstruction::SRU(x, y, z) => {
            bytes.extend_from_slice(&[0x3E, *x, *y, *z]);
        }
        MMixInstruction::SRUI(x, y, z) => {
            bytes.extend_from_slice(&[0x3F, *x, *y, *z]);
        }
        MMixInstruction::GETA(x, y, z) => {
            bytes.extend_from_slice(&[0xF4, *x, *y, *z]);
        }
        MMixInstruction::GETAB(x, y, z) => {
            bytes.extend_from_slice(&[0xF5, *x, *y, *z]);
        }
        MMixInstruction::PUSHJ(x, y, z) => {
            bytes.extend_from_slice(&[0xF2, *x, *y, *z]);
        }
        MMixInstruction::PUSHJB(x, y, z) => {
            bytes.extend_from_slice(&[0xF3, *x, *y, *z]);
        }
        MMixInstruction::PUSHGO(x, y, z) => {
            bytes.extend_from_slice(&[0xBE, *x, *y, *z]);
        }
        MMixInstruction::PUSHGOI(x, y, z) => {
            bytes.extend_from_slice(&[0xBF, *x, *y, *z]);
        }
        MMixInstruction::POP(x, yz) => {
            bytes.extend_from_slice(&[0xF8, *x, 0, *yz]);
        }
        MMixInstruction::GO(x, y, z) => {
            bytes.extend_from_slice(&[0x9E, *x, *y, *z]);
        }
        MMixInstruction::GOI(x, y, z) => {
            bytes.extend_from_slice(&[0x9F, *x, *y, *z]);
        }
        MMixInstruction::GET(x, z) => {
            bytes.extend_from_slice(&[0xFE, *x, 0, *z]);
        }
        MMixInstruction::PUT(x, z) => {
            bytes.extend_from_slice(&[0xF6, *x, 0, *z]);
        }
        MMixInstruction::PUTI(x, z) => {
            bytes.extend_from_slice(&[0xF7, *x, 0, *z]);
        }
        MMixInstruction::SAVE(x, z) => {
            bytes.extend_from_slice(&[0xFA, *x, 0, *z]);
        }
        MMixInstruction::UNSAVE(x, z) => {
            bytes.extend_from_slice(&[0xFB, *x, 0, *z]);
        }
        // Load instructions
        MMixInstruction::LDB(x, y, z) => {
            bytes.extend_from_slice(&[0x80, *x, *y, *z]);
        }
        MMixInstruction::LDBI(x, y, z) => {
            bytes.extend_from_slice(&[0x81, *x, *y, *z]);
        }
        MMixInstruction::LDBU(x, y, z) => {
            bytes.extend_from_slice(&[0x82, *x, *y, *z]);
        }
        MMixInstruction::LDBUI(x, y, z) => {
            bytes.extend_from_slice(&[0x83, *x, *y, *z]);
        }
        MMixInstruction::LDW(x, y, z) => {
            bytes.extend_from_slice(&[0x84, *x, *y, *z]);
        }
        MMixInstruction::LDWI(x, y, z) => {
            bytes.extend_from_slice(&[0x85, *x, *y, *z]);
        }
        MMixInstruction::LDWU(x, y, z) => {
            bytes.extend_from_slice(&[0x86, *x, *y, *z]);
        }
        MMixInstruction::LDWUI(x, y, z) => {
            bytes.extend_from_slice(&[0x87, *x, *y, *z]);
        }
        MMixInstruction::LDT(x, y, z) => {
            bytes.extend_from_slice(&[0x88, *x, *y, *z]);
        }
        MMixInstruction::LDTI(x, y, z) => {
            bytes.extend_from_slice(&[0x89, *x, *y, *z]);
        }
        MMixInstruction::LDTU(x, y, z) => {
            bytes.extend_from_slice(&[0x8A, *x, *y, *z]);
        }
        MMixInstruction::LDTUI(x, y, z) => {
            bytes.extend_from_slice(&[0x8B, *x, *y, *z]);
        }
        MMixInstruction::LDO(x, y, z) => {
            bytes.extend_from_slice(&[0x8C, *x, *y, *z]);
        }
        MMixInstruction::LDOI(x, y, z) => {
            bytes.extend_from_slice(&[0x8D, *x, *y, *z]);
        }
        MMixInstruction::LDOU(x, y, z) => {
            bytes.extend_from_slice(&[0x8E, *x, *y, *z]);
        }
        MMixInstruction::LDOUI(x, y, z) => {
            bytes.extend_from_slice(&[0x8F, *x, *y, *z]);
        }
        MMixInstruction::LDUNC(x, y, z) => {
            bytes.extend_from_slice(&[0x96, *x, *y, *z]);
        }
        MMixInstruction::LDUNCI(x, y, z) => {
            bytes.extend_from_slice(&[0x97, *x, *y, *z]);
        }
        // Store instructions
        MMixInstruction::STB(x, y, z) => {
            bytes.extend_from_slice(&[0xA0, *x, *y, *z]);
        }
        MMixInstruction::STBI(x, y, z) => {
            bytes.extend_from_slice(&[0xA1, *x, *y, *z]);
        }
        MMixInstruction::STBU(x, y, z) => {
            bytes.extend_from_slice(&[0xA2, *x, *y, *z]);
        }
        MMixInstruction::STBUI(x, y, z) => {
            bytes.extend_from_slice(&[0xA3, *x, *y, *z]);
        }
        MMixInstruction::STW(x, y, z) => {
            bytes.extend_from_slice(&[0xA4, *x, *y, *z]);
        }
        MMixInstruction::STWI(x, y, z) => {
            bytes.extend_from_slice(&[0xA5, *x, *y, *z]);
        }
        MMixInstruction::STWU(x, y, z) => {
            bytes.extend_from_slice(&[0xA6, *x, *y, *z]);
        }
        MMixInstruction::STWUI(x, y, z) => {
            bytes.extend_from_slice(&[0xA7, *x, *y, *z]);
        }
        MMixInstruction::STT(x, y, z) => {
            bytes.extend_from_slice(&[0xA8, *x, *y, *z]);
        }
        MMixInstruction::STTI(x, y, z) => {
            bytes.extend_from_slice(&[0xA9, *x, *y, *z]);
        }
        MMixInstruction::STTU(x, y, z) => {
            bytes.extend_from_slice(&[0xAA, *x, *y, *z]);
        }
        MMixInstruction::STTUI(x, y, z) => {
            bytes.extend_from_slice(&[0xAB, *x, *y, *z]);
        }
        MMixInstruction::STO(x, y, z) => {
            bytes.extend_from_slice(&[0xAC, *x, *y, *z]);
        }
        MMixInstruction::STOI(x, y, z) => {
            bytes.extend_from_slice(&[0xAD, *x, *y, *z]);
        }
        MMixInstruction::STOU(x, y, z) => {
            bytes.extend_from_slice(&[0xAE, *x, *y, *z]);
        }
        MMixInstruction::STOUI(x, y, z) => {
            bytes.extend_from_slice(&[0xAF, *x, *y, *z]);
        }
        MMixInstruction::STSF(x, y, z) => {
            bytes.extend_from_slice(&[0xB0, *x, *y, *z]);
        }
        MMixInstruction::STSFI(x, y, z) => {
            bytes.extend_from_slice(&[0xB1, *x, *y, *z]);
        }
        MMixInstruction::STHT(x, y, z) => {
            bytes.extend_from_slice(&[0xB2, *x, *y, *z]);
        }
        MMixInstruction::STHTI(x, y, z) => {
            bytes.extend_from_slice(&[0xB3, *x, *y, *z]);
        }
        MMixInstruction::STUNC(x, y, z) => {
            bytes.extend_from_slice(&[0xB6, *x, *y, *z]);
        }
        MMixInstruction::STUNCI(x, y, z) => {
            bytes.extend_from_slice(&[0xB7, *x, *y, *z]);
        }
        // Special load/store and system instructions
        MMixInstruction::CSWAP(x, y, z) => {
            bytes.extend_from_slice(&[0x94, *x, *y, *z]);
        }
        MMixInstruction::CSWAPI(x, y, z) => {
            bytes.extend_from_slice(&[0x95, *x, *y, *z]);
        }
        MMixInstruction::STCO(x, y, z) => {
            bytes.extend_from_slice(&[0xB4, *x, *y, *z]);
        }
        MMixInstruction::STCOI(x, y, z) => {
            bytes.extend_from_slice(&[0xB5, *x, *y, *z]);
        }
        MMixInstruction::PRELD(x, y, z) => {
            bytes.extend_from_slice(&[0x9A, *x, *y, *z]);
        }
        MMixInstruction::PRELDI(x, y, z) => {
            bytes.extend_from_slice(&[0x9B, *x, *y, *z]);
        }
        MMixInstruction::PREGO(x, y, z) => {
            bytes.extend_from_slice(&[0x9C, *x, *y, *z]);
        }
        MMixInstruction::PREGOI(x, y, z) => {
            bytes.extend_from_slice(&[0x9D, *x, *y, *z]);
        }
        MMixInstruction::PREST(x, y, z) => {
            bytes.extend_from_slice(&[0xBA, *x, *y, *z]);
        }
        MMixInstruction::PRESTI(x, y, z) => {
            bytes.extend_from_slice(&[0xBB, *x, *y, *z]);
        }
        MMixInstruction::SYNCD(x, y, z) => {
            bytes.extend_from_slice(&[0xB8, *x, *y, *z]);
        }
        MMixInstruction::SYNCDI(x, y, z) => {
            bytes.extend_from_slice(&[0xB9, *x, *y, *z]);
        }
        MMixInstruction::SYNCID(x, y, z) => {
            bytes.extend_from_slice(&[0xBC, *x, *y, *z]);
        }
        MMixInstruction::SYNCIDI(x, y, z) => {
            bytes.extend_from_slice(&[0xBD, *x, *y, *z]);
        }
        // Control flow
        MMixInstruction::TRAP(x, y, z) => {
            bytes.extend_from_slice(&[0x00, *x, *y, *z]);
        }
        MMixInstruction::JMP(offset) => {
            let x = ((offset >> 16) & 0xFF) as u8;
            let y = ((offset >> 8) & 0xFF) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0xF0, x, y, z]);
        }
        // Probable branch instructions
        MMixInstruction::PBN(x, y, z) => {
            bytes.extend_from_slice(&[0x50, *x, *y, *z]);
        }
        MMixInstruction::PBNB(x, y, z) => {
            bytes.extend_from_slice(&[0x51, *x, *y, *z]);
        }
        MMixInstruction::PBZ(x, y, z) => {
            bytes.extend_from_slice(&[0x52, *x, *y, *z]);
        }
        MMixInstruction::PBZB(x, y, z) => {
            bytes.extend_from_slice(&[0x53, *x, *y, *z]);
        }
        MMixInstruction::PBP(x, y, z) => {
            bytes.extend_from_slice(&[0x54, *x, *y, *z]);
        }
        MMixInstruction::PBPB(x, y, z) => {
            bytes.extend_from_slice(&[0x55, *x, *y, *z]);
        }
        MMixInstruction::PBOD(x, y, z) => {
            bytes.extend_from_slice(&[0x56, *x, *y, *z]);
        }
        MMixInstruction::PBODB(x, y, z) => {
            bytes.extend_from_slice(&[0x57, *x, *y, *z]);
        }
        MMixInstruction::PBNN(x, y, z) => {
            bytes.extend_from_slice(&[0x58, *x, *y, *z]);
        }
        MMixInstruction::PBNNB(x, y, z) => {
            bytes.extend_from_slice(&[0x59, *x, *y, *z]);
        }
        MMixInstruction::PBNZ(x, y, z) => {
            bytes.extend_from_slice(&[0x5A, *x, *y, *z]);
        }
        MMixInstruction::PBNZB(x, y, z) => {
            bytes.extend_from_slice(&[0x5B, *x, *y, *z]);
        }
        MMixInstruction::PBNP(x, y, z) => {
            bytes.extend_from_slice(&[0x5C, *x, *y, *z]);
        }
        MMixInstruction::PBNPB(x, y, z) => {
            bytes.extend_from_slice(&[0x5D, *x, *y, *z]);
        }
        MMixInstruction::PBEV(x, y, z) => {
            bytes.extend_from_slice(&[0x5E, *x, *y, *z]);
        }
        MMixInstruction::PBEVB(x, y, z) => {
            bytes.extend_from_slice(&[0x5F, *x, *y, *z]);
        }
        // Branch instructions
        MMixInstruction::BN(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x40, *x, y, z]);
        }
        MMixInstruction::BNB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x41, *x, y, z]);
        }
        MMixInstruction::BZ(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x42, *x, y, z]);
        }
        MMixInstruction::BZB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x43, *x, y, z]);
        }
        MMixInstruction::BP(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x44, *x, y, z]);
        }
        MMixInstruction::BPB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x45, *x, y, z]);
        }
        MMixInstruction::BOD(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x46, *x, y, z]);
        }
        MMixInstruction::BODB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x47, *x, y, z]);
        }
        MMixInstruction::BNN(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x48, *x, y, z]);
        }
        MMixInstruction::BNNB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x49, *x, y, z]);
        }
        MMixInstruction::BNZ(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4A, *x, y, z]);
        }
        MMixInstruction::BNZB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4B, *x, y, z]);
        }
        MMixInstruction::BNP(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4C, *x, y, z]);
        }
        MMixInstruction::BNPB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4D, *x, y, z]);
        }
        MMixInstruction::BEV(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4E, *x, y, z]);
        }
        MMixInstruction::BEVB(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4F, *x, y, z]);
        }
        // Pseudo-branch instructions (map to conditional branches with inverted conditions)
        MMixInstruction::JE(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x42, *x, y, z]); // BZ
        }
        MMixInstruction::JNE(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x4A, *x, y, z]); // BNZ
        }
        MMixInstruction::JL(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x40, *x, y, z]); // BN
        }
        MMixInstruction::JG(x, offset) => {
            let y = (offset >> 8) as u8;
            let z = (offset & 0xFF) as u8;
            bytes.extend_from_slice(&[0x44, *x, y, z]); // BP
        }
        // System control instructions
        MMixInstruction::TRIP(x, y, z) => {
            bytes.extend_from_slice(&[0xFF, *x, *y, *z]);
        }
        MMixInstruction::RESUME(xyz) => {
            bytes.extend_from_slice(&[0xF9, 0, 0, *xyz]);
        }
        MMixInstruction::SYNC(xyz) => {
            bytes.extend_from_slice(&[0xFC, 0, 0, *xyz]);
        }
        MMixInstruction::SWYM => {
            bytes.extend_from_slice(&[0xFD, 0, 0, 0]);
        }
        MMixInstruction::HALT => {
            bytes.extend_from_slice(&[0x00, 0, 0, 0]); // TRAP 0,Halt,0
        }
        MMixInstruction::INCL(x, y, z) => {
            bytes.extend_from_slice(&[0xE7, *x, *y, *z]);
        }
        // Floating point instructions
        MMixInstruction::FCMP(x, y, z) => {
            bytes.extend_from_slice(&[0x01, *x, *y, *z]);
        }
        MMixInstruction::FUN(x, y, z) => {
            bytes.extend_from_slice(&[0x02, *x, *y, *z]);
        }
        MMixInstruction::FEQL(x, y, z) => {
            bytes.extend_from_slice(&[0x03, *x, *y, *z]);
        }
        MMixInstruction::FADD(x, y, z) => {
            bytes.extend_from_slice(&[0x04, *x, *y, *z]);
        }
        MMixInstruction::FIX(x, y, z) => {
            bytes.extend_from_slice(&[0x05, *x, *y, *z]);
        }
        MMixInstruction::FSUB(x, y, z) => {
            bytes.extend_from_slice(&[0x06, *x, *y, *z]);
        }
        MMixInstruction::FIXU(x, y, z) => {
            bytes.extend_from_slice(&[0x07, *x, *y, *z]);
        }
        MMixInstruction::FLOT(x, y, z) => {
            bytes.extend_from_slice(&[0x08, *x, *y, *z]);
        }
        MMixInstruction::FLOTI(x, y, z) => {
            bytes.extend_from_slice(&[0x09, *x, *y, *z]);
        }
        MMixInstruction::FLOTU(x, y, z) => {
            bytes.extend_from_slice(&[0x0A, *x, *y, *z]);
        }
        MMixInstruction::FLOTUI(x, y, z) => {
            bytes.extend_from_slice(&[0x0B, *x, *y, *z]);
        }
        MMixInstruction::SFLOT(x, y, z) => {
            bytes.extend_from_slice(&[0x0C, *x, *y, *z]);
        }
        MMixInstruction::SFLOTI(x, y, z) => {
            bytes.extend_from_slice(&[0x0D, *x, *y, *z]);
        }
        MMixInstruction::SFLOTU(x, y, z) => {
            bytes.extend_from_slice(&[0x0E, *x, *y, *z]);
        }
        MMixInstruction::SFLOTUI(x, y, z) => {
            bytes.extend_from_slice(&[0x0F, *x, *y, *z]);
        }
        MMixInstruction::FMUL(x, y, z) => {
            bytes.extend_from_slice(&[0x10, *x, *y, *z]);
        }
        MMixInstruction::FDIV(x, y, z) => {
            bytes.extend_from_slice(&[0x14, *x, *y, *z]);
        }
        MMixInstruction::FSQRT(x, y, z) => {
            bytes.extend_from_slice(&[0x15, *x, *y, *z]);
        }
        MMixInstruction::FREM(x, y, z) => {
            bytes.extend_from_slice(&[0x16, *x, *y, *z]);
        }
        MMixInstruction::FINT(x, y, z) => {
            bytes.extend_from_slice(&[0x17, *x, *y, *z]);
        }
        // Conditional set instructions
        MMixInstruction::CSN(x, y, z) => {
            bytes.extend_from_slice(&[0x60, *x, *y, *z]);
        }
        MMixInstruction::CSNI(x, y, z) => {
            bytes.extend_from_slice(&[0x61, *x, *y, *z]);
        }
        MMixInstruction::CSZ(x, y, z) => {
            bytes.extend_from_slice(&[0x62, *x, *y, *z]);
        }
        MMixInstruction::CSZI(x, y, z) => {
            bytes.extend_from_slice(&[0x63, *x, *y, *z]);
        }
        MMixInstruction::CSP(x, y, z) => {
            bytes.extend_from_slice(&[0x64, *x, *y, *z]);
        }
        MMixInstruction::CSPI(x, y, z) => {
            bytes.extend_from_slice(&[0x65, *x, *y, *z]);
        }
        MMixInstruction::CSOD(x, y, z) => {
            bytes.extend_from_slice(&[0x66, *x, *y, *z]);
        }
        MMixInstruction::CSODI(x, y, z) => {
            bytes.extend_from_slice(&[0x67, *x, *y, *z]);
        }
        MMixInstruction::CSNN(x, y, z) => {
            bytes.extend_from_slice(&[0x68, *x, *y, *z]);
        }
        MMixInstruction::CSNNI(x, y, z) => {
            bytes.extend_from_slice(&[0x69, *x, *y, *z]);
        }
        MMixInstruction::CSNZ(x, y, z) => {
            bytes.extend_from_slice(&[0x6A, *x, *y, *z]);
        }
        MMixInstruction::CSNZI(x, y, z) => {
            bytes.extend_from_slice(&[0x6B, *x, *y, *z]);
        }
        MMixInstruction::CSNP(x, y, z) => {
            bytes.extend_from_slice(&[0x6C, *x, *y, *z]);
        }
        MMixInstruction::CSNPI(x, y, z) => {
            bytes.extend_from_slice(&[0x6D, *x, *y, *z]);
        }
        MMixInstruction::CSEV(x, y, z) => {
            bytes.extend_from_slice(&[0x6E, *x, *y, *z]);
        }
        MMixInstruction::CSEVI(x, y, z) => {
            bytes.extend_from_slice(&[0x6F, *x, *y, *z]);
        }
        // Zero or set instructions
        MMixInstruction::ZSN(x, y, z) => {
            bytes.extend_from_slice(&[0x70, *x, *y, *z]);
        }
        MMixInstruction::ZSNI(x, y, z) => {
            bytes.extend_from_slice(&[0x71, *x, *y, *z]);
        }
        MMixInstruction::ZSZ(x, y, z) => {
            bytes.extend_from_slice(&[0x72, *x, *y, *z]);
        }
        MMixInstruction::ZSZI(x, y, z) => {
            bytes.extend_from_slice(&[0x73, *x, *y, *z]);
        }
        MMixInstruction::ZSP(x, y, z) => {
            bytes.extend_from_slice(&[0x74, *x, *y, *z]);
        }
        MMixInstruction::ZSPI(x, y, z) => {
            bytes.extend_from_slice(&[0x75, *x, *y, *z]);
        }
        MMixInstruction::ZSOD(x, y, z) => {
            bytes.extend_from_slice(&[0x76, *x, *y, *z]);
        }
        MMixInstruction::ZSODI(x, y, z) => {
            bytes.extend_from_slice(&[0x77, *x, *y, *z]);
        }
        MMixInstruction::ZSNN(x, y, z) => {
            bytes.extend_from_slice(&[0x78, *x, *y, *z]);
        }
        MMixInstruction::ZSNNI(x, y, z) => {
            bytes.extend_from_slice(&[0x79, *x, *y, *z]);
        }
        MMixInstruction::ZSNZ(x, y, z) => {
            bytes.extend_from_slice(&[0x7A, *x, *y, *z]);
        }
        MMixInstruction::ZSNZI(x, y, z) => {
            bytes.extend_from_slice(&[0x7B, *x, *y, *z]);
        }
        MMixInstruction::ZSNP(x, y, z) => {
            bytes.extend_from_slice(&[0x7C, *x, *y, *z]);
        }
        MMixInstruction::ZSNPI(x, y, z) => {
            bytes.extend_from_slice(&[0x7D, *x, *y, *z]);
        }
        MMixInstruction::ZSEV(x, y, z) => {
            bytes.extend_from_slice(&[0x7E, *x, *y, *z]);
        }
        MMixInstruction::ZSEVI(x, y, z) => {
            bytes.extend_from_slice(&[0x7F, *x, *y, *z]);
        }
        // SET-family wyde instructions
        MMixInstruction::INCH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE4, *x, *yz));
        }
        MMixInstruction::INCMH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE5, *x, *yz));
        }
        MMixInstruction::INCML(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE6, *x, *yz));
        }
        MMixInstruction::ORH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE8, *x, *yz));
        }
        MMixInstruction::ORMH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xE9, *x, *yz));
        }
        MMixInstruction::ORML(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xEA, *x, *yz));
        }
        MMixInstruction::ORL(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xEB, *x, *yz));
        }
        MMixInstruction::ANDNH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xEC, *x, *yz));
        }
        MMixInstruction::ANDNMH(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xED, *x, *yz));
        }
        MMixInstruction::ANDNML(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xEE, *x, *yz));
        }
        MMixInstruction::ANDNL(x, yz) => {
            bytes.extend_from_slice(&encode_instruction(0xEF, *x, *yz));
        }
    }

    bytes
}

/// Helper to encode a standard instruction with YZ field
fn encode_instruction(opcode: u8, x: u8, yz: u16) -> [u8; 4] {
    [opcode, x, (yz >> 8) as u8, (yz & 0xFF) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmixal::MMixInstruction;

    /// Test all MMIX instruction opcodes match the official specification
    /// Reference: https://www-cs-faculty.stanford.edu/~knuth/mmop.html

    #[test]
    fn test_trap_encoding() {
        // TRAP - opcode 0x00
        let bytes = encode_instruction_bytes(&MMixInstruction::TRAP(1, 2, 3));
        assert_eq!(bytes, vec![0x00, 1, 2, 3]);
    }

    #[test]
    fn test_setrr_encoding() {
        // SETRR - should encode as ORI $X, $Y, 0 (opcode 0xC1)
        let bytes = encode_instruction_bytes(&MMixInstruction::SETRR(2, 1));
        assert_eq!(bytes, vec![0xC1, 2, 1, 0]);
    }

    #[test]
    fn test_floating_point_encodings() {
        // FCMP - 0x01
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FCMP(1, 2, 3)),
            vec![0x01, 1, 2, 3]
        );
        // FUN - 0x02
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FUN(1, 2, 3)),
            vec![0x02, 1, 2, 3]
        );
        // FEQL - 0x03
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FEQL(1, 2, 3)),
            vec![0x03, 1, 2, 3]
        );
        // FADD - 0x04
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FADD(1, 2, 3)),
            vec![0x04, 1, 2, 3]
        );
        // FIX - 0x05
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FIX(1, 2, 3)),
            vec![0x05, 1, 2, 3]
        );
        // FSUB - 0x06
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FSUB(1, 2, 3)),
            vec![0x06, 1, 2, 3]
        );
        // FIXU - 0x07
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FIXU(1, 2, 3)),
            vec![0x07, 1, 2, 3]
        );
        // FLOT - 0x08
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FLOT(1, 2, 3)),
            vec![0x08, 1, 2, 3]
        );
        // FLOTI - 0x09
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FLOTI(1, 2, 3)),
            vec![0x09, 1, 2, 3]
        );
        // FLOTU - 0x0A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FLOTU(1, 2, 3)),
            vec![0x0A, 1, 2, 3]
        );
        // FLOTUI - 0x0B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::FLOTUI(1, 2, 3)),
            vec![0x0B, 1, 2, 3]
        );
        // SFLOT - 0x0C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SFLOT(1, 2, 3)),
            vec![0x0C, 1, 2, 3]
        );
        // SFLOTI - 0x0D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SFLOTI(1, 2, 3)),
            vec![0x0D, 1, 2, 3]
        );
        // SFLOTU - 0x0E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SFLOTU(1, 2, 3)),
            vec![0x0E, 1, 2, 3]
        );
        // SFLOTUI - 0x0F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SFLOTUI(1, 2, 3)),
            vec![0x0F, 1, 2, 3]
        );
    }

    #[test]
    fn test_integer_arithmetic_encodings() {
        // ADD - 0x20
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADD(1, 2, 3)),
            vec![0x20, 1, 2, 3]
        );
        // ADDI - 0x21
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDI(1, 2, 3)),
            vec![0x21, 1, 2, 3]
        );
        // ADDU - 0x22
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU(1, 2, 3)),
            vec![0x22, 1, 2, 3]
        );
        // ADDUI - 0x23
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDUI(1, 2, 3)),
            vec![0x23, 1, 2, 3]
        );
        // SUB - 0x24
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SUB(1, 2, 3)),
            vec![0x24, 1, 2, 3]
        );
        // SUBI - 0x25
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SUBI(1, 2, 3)),
            vec![0x25, 1, 2, 3]
        );
        // SUBU - 0x26
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SUBU(1, 2, 3)),
            vec![0x26, 1, 2, 3]
        );
        // SUBUI - 0x27
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SUBUI(1, 2, 3)),
            vec![0x27, 1, 2, 3]
        );
        // 2ADDU - 0x28
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU2(1, 2, 3)),
            vec![0x28, 1, 2, 3]
        );
        // 2ADDUI - 0x29
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU2I(1, 2, 3)),
            vec![0x29, 1, 2, 3]
        );
        // 4ADDU - 0x2A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU4(1, 2, 3)),
            vec![0x2A, 1, 2, 3]
        );
        // 4ADDUI - 0x2B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU4I(1, 2, 3)),
            vec![0x2B, 1, 2, 3]
        );
        // 8ADDU - 0x2C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU8(1, 2, 3)),
            vec![0x2C, 1, 2, 3]
        );
        // 8ADDUI - 0x2D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU8I(1, 2, 3)),
            vec![0x2D, 1, 2, 3]
        );
        // 16ADDU - 0x2E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU16(1, 2, 3)),
            vec![0x2E, 1, 2, 3]
        );
        // 16ADDUI - 0x2F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ADDU16I(1, 2, 3)),
            vec![0x2F, 1, 2, 3]
        );
    }

    #[test]
    fn test_comparison_encodings() {
        // CMP - 0x30
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CMP(1, 2, 3)),
            vec![0x30, 1, 2, 3]
        );
        // CMPI - 0x31
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CMPI(1, 2, 3)),
            vec![0x31, 1, 2, 3]
        );
        // CMPU - 0x32
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CMPU(1, 2, 3)),
            vec![0x32, 1, 2, 3]
        );
        // CMPUI - 0x33
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CMPUI(1, 2, 3)),
            vec![0x33, 1, 2, 3]
        );
        // NEG - 0x34
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NEG(1, 2, 3)),
            vec![0x34, 1, 2, 3]
        );
        // NEGI - 0x35
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NEGI(1, 2, 3)),
            vec![0x35, 1, 2, 3]
        );
        // NEGU - 0x36
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NEGU(1, 2, 3)),
            vec![0x36, 1, 2, 3]
        );
        // NEGUI - 0x37
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NEGUI(1, 2, 3)),
            vec![0x37, 1, 2, 3]
        );
    }

    #[test]
    fn test_shift_encodings() {
        // SL - 0x38
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SL(1, 2, 3)),
            vec![0x38, 1, 2, 3]
        );
        // SLI - 0x39
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SLI(1, 2, 3)),
            vec![0x39, 1, 2, 3]
        );
        // SLU - 0x3A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SLU(1, 2, 3)),
            vec![0x3A, 1, 2, 3]
        );
        // SLUI - 0x3B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SLUI(1, 2, 3)),
            vec![0x3B, 1, 2, 3]
        );
        // SR - 0x3C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SR(1, 2, 3)),
            vec![0x3C, 1, 2, 3]
        );
        // SRI - 0x3D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SRI(1, 2, 3)),
            vec![0x3D, 1, 2, 3]
        );
        // SRU - 0x3E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SRU(1, 2, 3)),
            vec![0x3E, 1, 2, 3]
        );
        // SRUI - 0x3F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SRUI(1, 2, 3)),
            vec![0x3F, 1, 2, 3]
        );
    }

    #[test]
    fn test_branch_encodings() {
        // BN - 0x40
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BN(1, 2)),
            vec![0x40, 1, 0, 2]
        );
        // BNB - 0x41
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNB(1, 2)),
            vec![0x41, 1, 0, 2]
        );
        // BZ - 0x42
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BZ(1, 2)),
            vec![0x42, 1, 0, 2]
        );
        // BZB - 0x43
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BZB(1, 2)),
            vec![0x43, 1, 0, 2]
        );
        // BP - 0x44
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BP(1, 2)),
            vec![0x44, 1, 0, 2]
        );
        // BPB - 0x45
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BPB(1, 2)),
            vec![0x45, 1, 0, 2]
        );
        // BOD - 0x46
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BOD(1, 2)),
            vec![0x46, 1, 0, 2]
        );
        // BODB - 0x47
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BODB(1, 2)),
            vec![0x47, 1, 0, 2]
        );
        // BNN - 0x48
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNN(1, 2)),
            vec![0x48, 1, 0, 2]
        );
        // BNNB - 0x49
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNNB(1, 2)),
            vec![0x49, 1, 0, 2]
        );
        // BNZ - 0x4A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNZ(1, 2)),
            vec![0x4A, 1, 0, 2]
        );
        // BNZB - 0x4B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNZB(1, 2)),
            vec![0x4B, 1, 0, 2]
        );
        // BNP - 0x4C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNP(1, 2)),
            vec![0x4C, 1, 0, 2]
        );
        // BNPB - 0x4D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BNPB(1, 2)),
            vec![0x4D, 1, 0, 2]
        );
        // BEV - 0x4E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BEV(1, 2)),
            vec![0x4E, 1, 0, 2]
        );
        // BEVB - 0x4F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BEVB(1, 2)),
            vec![0x4F, 1, 0, 2]
        );
    }

    #[test]
    fn test_probable_branch_encodings() {
        // PBN - 0x50
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBN(1, 2, 3)),
            vec![0x50, 1, 2, 3]
        );
        // PBNB - 0x51
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNB(1, 2, 3)),
            vec![0x51, 1, 2, 3]
        );
        // PBZ - 0x52
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBZ(1, 2, 3)),
            vec![0x52, 1, 2, 3]
        );
        // PBZB - 0x53
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBZB(1, 2, 3)),
            vec![0x53, 1, 2, 3]
        );
        // PBP - 0x54
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBP(1, 2, 3)),
            vec![0x54, 1, 2, 3]
        );
        // PBPB - 0x55
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBPB(1, 2, 3)),
            vec![0x55, 1, 2, 3]
        );
        // PBOD - 0x56
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBOD(1, 2, 3)),
            vec![0x56, 1, 2, 3]
        );
        // PBODB - 0x57
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBODB(1, 2, 3)),
            vec![0x57, 1, 2, 3]
        );
        // PBNN - 0x58
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNN(1, 2, 3)),
            vec![0x58, 1, 2, 3]
        );
        // PBNNB - 0x59
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNNB(1, 2, 3)),
            vec![0x59, 1, 2, 3]
        );
        // PBNZ - 0x5A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNZ(1, 2, 3)),
            vec![0x5A, 1, 2, 3]
        );
        // PBNZB - 0x5B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNZB(1, 2, 3)),
            vec![0x5B, 1, 2, 3]
        );
        // PBNP - 0x5C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNP(1, 2, 3)),
            vec![0x5C, 1, 2, 3]
        );
        // PBNPB - 0x5D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBNPB(1, 2, 3)),
            vec![0x5D, 1, 2, 3]
        );
        // PBEV - 0x5E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBEV(1, 2, 3)),
            vec![0x5E, 1, 2, 3]
        );
        // PBEVB - 0x5F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PBEVB(1, 2, 3)),
            vec![0x5F, 1, 2, 3]
        );
    }

    #[test]
    fn test_conditional_set_encodings() {
        // CSN - 0x60
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSN(1, 2, 3)),
            vec![0x60, 1, 2, 3]
        );
        // CSNI - 0x61
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNI(1, 2, 3)),
            vec![0x61, 1, 2, 3]
        );
        // CSZ - 0x62
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSZ(1, 2, 3)),
            vec![0x62, 1, 2, 3]
        );
        // CSZI - 0x63
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSZI(1, 2, 3)),
            vec![0x63, 1, 2, 3]
        );
        // CSP - 0x64
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSP(1, 2, 3)),
            vec![0x64, 1, 2, 3]
        );
        // CSPI - 0x65
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSPI(1, 2, 3)),
            vec![0x65, 1, 2, 3]
        );
        // CSOD - 0x66
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSOD(1, 2, 3)),
            vec![0x66, 1, 2, 3]
        );
        // CSODI - 0x67
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSODI(1, 2, 3)),
            vec![0x67, 1, 2, 3]
        );
        // CSNN - 0x68
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNN(1, 2, 3)),
            vec![0x68, 1, 2, 3]
        );
        // CSNNI - 0x69
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNNI(1, 2, 3)),
            vec![0x69, 1, 2, 3]
        );
        // CSNZ - 0x6A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNZ(1, 2, 3)),
            vec![0x6A, 1, 2, 3]
        );
        // CSNZI - 0x6B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNZI(1, 2, 3)),
            vec![0x6B, 1, 2, 3]
        );
        // CSNP - 0x6C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNP(1, 2, 3)),
            vec![0x6C, 1, 2, 3]
        );
        // CSNPI - 0x6D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSNPI(1, 2, 3)),
            vec![0x6D, 1, 2, 3]
        );
        // CSEV - 0x6E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSEV(1, 2, 3)),
            vec![0x6E, 1, 2, 3]
        );
        // CSEVI - 0x6F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSEVI(1, 2, 3)),
            vec![0x6F, 1, 2, 3]
        );
    }

    #[test]
    fn test_conditional_swap_encodings() {
        // ZSN - 0x70
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSN(1, 2, 3)),
            vec![0x70, 1, 2, 3]
        );
        // ZSNI - 0x71
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNI(1, 2, 3)),
            vec![0x71, 1, 2, 3]
        );
        // ZSZ - 0x72
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSZ(1, 2, 3)),
            vec![0x72, 1, 2, 3]
        );
        // ZSZI - 0x73
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSZI(1, 2, 3)),
            vec![0x73, 1, 2, 3]
        );
        // ZSP - 0x74
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSP(1, 2, 3)),
            vec![0x74, 1, 2, 3]
        );
        // ZSPI - 0x75
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSPI(1, 2, 3)),
            vec![0x75, 1, 2, 3]
        );
        // ZSOD - 0x76
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSOD(1, 2, 3)),
            vec![0x76, 1, 2, 3]
        );
        // ZSODI - 0x77
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSODI(1, 2, 3)),
            vec![0x77, 1, 2, 3]
        );
        // ZSNN - 0x78
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNN(1, 2, 3)),
            vec![0x78, 1, 2, 3]
        );
        // ZSNNI - 0x79
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNNI(1, 2, 3)),
            vec![0x79, 1, 2, 3]
        );
        // ZSNZ - 0x7A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNZ(1, 2, 3)),
            vec![0x7A, 1, 2, 3]
        );
        // ZSNZI - 0x7B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNZI(1, 2, 3)),
            vec![0x7B, 1, 2, 3]
        );
        // ZSNP - 0x7C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNP(1, 2, 3)),
            vec![0x7C, 1, 2, 3]
        );
        // ZSNPI - 0x7D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSNPI(1, 2, 3)),
            vec![0x7D, 1, 2, 3]
        );
        // ZSEV - 0x7E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSEV(1, 2, 3)),
            vec![0x7E, 1, 2, 3]
        );
        // ZSEVI - 0x7F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ZSEVI(1, 2, 3)),
            vec![0x7F, 1, 2, 3]
        );
    }

    #[test]
    fn test_load_byte_encodings() {
        // LDB - 0x80
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDB(1, 2, 3)),
            vec![0x80, 1, 2, 3]
        );
        // LDBI - 0x81
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDBI(1, 2, 3)),
            vec![0x81, 1, 2, 3]
        );
        // LDBU - 0x82
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDBU(1, 2, 3)),
            vec![0x82, 1, 2, 3]
        );
        // LDBUI - 0x83
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDBUI(1, 2, 3)),
            vec![0x83, 1, 2, 3]
        );
    }

    #[test]
    fn test_load_wyde_encodings() {
        // LDW - 0x84
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDW(1, 2, 3)),
            vec![0x84, 1, 2, 3]
        );
        // LDWI - 0x85
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDWI(1, 2, 3)),
            vec![0x85, 1, 2, 3]
        );
        // LDWU - 0x86
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDWU(1, 2, 3)),
            vec![0x86, 1, 2, 3]
        );
        // LDWUI - 0x87
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDWUI(1, 2, 3)),
            vec![0x87, 1, 2, 3]
        );
    }

    #[test]
    fn test_load_tetra_encodings() {
        // LDT - 0x88
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDT(1, 2, 3)),
            vec![0x88, 1, 2, 3]
        );
        // LDTI - 0x89
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDTI(1, 2, 3)),
            vec![0x89, 1, 2, 3]
        );
        // LDTU - 0x8A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDTU(1, 2, 3)),
            vec![0x8A, 1, 2, 3]
        );
        // LDTUI - 0x8B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDTUI(1, 2, 3)),
            vec![0x8B, 1, 2, 3]
        );
    }

    #[test]
    fn test_load_octa_encodings() {
        // LDO - 0x8C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDO(1, 2, 3)),
            vec![0x8C, 1, 2, 3]
        );
        // LDOI - 0x8D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDOI(1, 2, 3)),
            vec![0x8D, 1, 2, 3]
        );
        // LDOU - 0x8E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDOU(1, 2, 3)),
            vec![0x8E, 1, 2, 3]
        );
        // LDOUI - 0x8F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDOUI(1, 2, 3)),
            vec![0x8F, 1, 2, 3]
        );
    }

    #[test]
    fn test_load_special_encodings() {
        // LDSF - 0x90
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDSF(1, 2, 3)),
            vec![0x90, 1, 2, 3]
        );
        // LDSFI - 0x91
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDSFI(1, 2, 3)),
            vec![0x91, 1, 2, 3]
        );
        // LDHT - 0x92
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDHT(1, 2, 3)),
            vec![0x92, 1, 2, 3]
        );
        // LDHTI - 0x93
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDHTI(1, 2, 3)),
            vec![0x93, 1, 2, 3]
        );
        // CSWAP - 0x94
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSWAP(1, 2, 3)),
            vec![0x94, 1, 2, 3]
        );
        // CSWAPI - 0x95
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::CSWAPI(1, 2, 3)),
            vec![0x95, 1, 2, 3]
        );
        // LDUNC - 0x96
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDUNC(1, 2, 3)),
            vec![0x96, 1, 2, 3]
        );
        // LDUNCI - 0x97
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDUNCI(1, 2, 3)),
            vec![0x97, 1, 2, 3]
        );
        // LDVTS - 0x98
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDVTS(1, 2, 3)),
            vec![0x98, 1, 2, 3]
        );
        // LDVTSI - 0x99
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDVTSI(1, 2, 3)),
            vec![0x99, 1, 2, 3]
        );
    }

    #[test]
    fn test_prefetch_encodings() {
        // PRELD - 0x9A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PRELD(1, 2, 3)),
            vec![0x9A, 1, 2, 3]
        );
        // PRELDI - 0x9B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PRELDI(1, 2, 3)),
            vec![0x9B, 1, 2, 3]
        );
        // PREGO - 0x9C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PREGO(1, 2, 3)),
            vec![0x9C, 1, 2, 3]
        );
        // PREGOI - 0x9D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PREGOI(1, 2, 3)),
            vec![0x9D, 1, 2, 3]
        );
        // GO - 0x9E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::GO(1, 2, 3)),
            vec![0x9E, 1, 2, 3]
        );
        // GOI - 0x9F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::GOI(1, 2, 3)),
            vec![0x9F, 1, 2, 3]
        );
    }

    #[test]
    fn test_store_byte_encodings() {
        // STB - 0xA0
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STB(1, 2, 3)),
            vec![0xA0, 1, 2, 3]
        );
        // STBI - 0xA1
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STBI(1, 2, 3)),
            vec![0xA1, 1, 2, 3]
        );
        // STBU - 0xA2
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STBU(1, 2, 3)),
            vec![0xA2, 1, 2, 3]
        );
        // STBUI - 0xA3
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STBUI(1, 2, 3)),
            vec![0xA3, 1, 2, 3]
        );
    }

    #[test]
    fn test_store_wyde_encodings() {
        // STW - 0xA4
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STW(1, 2, 3)),
            vec![0xA4, 1, 2, 3]
        );
        // STWI - 0xA5
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STWI(1, 2, 3)),
            vec![0xA5, 1, 2, 3]
        );
        // STWU - 0xA6
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STWU(1, 2, 3)),
            vec![0xA6, 1, 2, 3]
        );
        // STWUI - 0xA7
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STWUI(1, 2, 3)),
            vec![0xA7, 1, 2, 3]
        );
    }

    #[test]
    fn test_store_tetra_encodings() {
        // STT - 0xA8
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STT(1, 2, 3)),
            vec![0xA8, 1, 2, 3]
        );
        // STTI - 0xA9
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STTI(1, 2, 3)),
            vec![0xA9, 1, 2, 3]
        );
        // STTU - 0xAA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STTU(1, 2, 3)),
            vec![0xAA, 1, 2, 3]
        );
        // STTUI - 0xAB
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STTUI(1, 2, 3)),
            vec![0xAB, 1, 2, 3]
        );
    }

    #[test]
    fn test_store_octa_encodings() {
        // STO - 0xAC
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STO(1, 2, 3)),
            vec![0xAC, 1, 2, 3]
        );
        // STOI - 0xAD
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STOI(1, 2, 3)),
            vec![0xAD, 1, 2, 3]
        );
        // STOU - 0xAE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STOU(1, 2, 3)),
            vec![0xAE, 1, 2, 3]
        );
        // STOUI - 0xAF
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STOUI(1, 2, 3)),
            vec![0xAF, 1, 2, 3]
        );
    }

    #[test]
    fn test_store_special_encodings() {
        // STSF - 0xB0
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STSF(1, 2, 3)),
            vec![0xB0, 1, 2, 3]
        );
        // STSFI - 0xB1
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STSFI(1, 2, 3)),
            vec![0xB1, 1, 2, 3]
        );
        // STHT - 0xB2
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STHT(1, 2, 3)),
            vec![0xB2, 1, 2, 3]
        );
        // STHTI - 0xB3
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STHTI(1, 2, 3)),
            vec![0xB3, 1, 2, 3]
        );
        // STCO - 0xB4
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STCO(1, 2, 3)),
            vec![0xB4, 1, 2, 3]
        );
        // STCOI - 0xB5
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STCOI(1, 2, 3)),
            vec![0xB5, 1, 2, 3]
        );
        // STUNC - 0xB6
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STUNC(1, 2, 3)),
            vec![0xB6, 1, 2, 3]
        );
        // STUNCI - 0xB7
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::STUNCI(1, 2, 3)),
            vec![0xB7, 1, 2, 3]
        );
    }

    #[test]
    fn test_sync_encodings() {
        // SYNCD - 0xB8
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SYNCD(1, 2, 3)),
            vec![0xB8, 1, 2, 3]
        );
        // SYNCDI - 0xB9
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SYNCDI(1, 2, 3)),
            vec![0xB9, 1, 2, 3]
        );
        // PREST - 0xBA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PREST(1, 2, 3)),
            vec![0xBA, 1, 2, 3]
        );
        // PRESTI - 0xBB
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PRESTI(1, 2, 3)),
            vec![0xBB, 1, 2, 3]
        );
        // SYNCID - 0xBC
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SYNCID(1, 2, 3)),
            vec![0xBC, 1, 2, 3]
        );
        // SYNCIDI - 0xBD
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SYNCIDI(1, 2, 3)),
            vec![0xBD, 1, 2, 3]
        );
        // PUSHGO - 0xBE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PUSHGO(1, 2, 3)),
            vec![0xBE, 1, 2, 3]
        );
        // PUSHGOI - 0xBF
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PUSHGOI(1, 2, 3)),
            vec![0xBF, 1, 2, 3]
        );
    }

    #[test]
    fn test_bitwise_encodings() {
        // OR - 0xC0
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::OR(1, 2, 3)),
            vec![0xC0, 1, 2, 3]
        );
        // ORI - 0xC1
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORI(1, 2, 3)),
            vec![0xC1, 1, 2, 3]
        );
        // ORN - 0xC2
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORN(1, 2, 3)),
            vec![0xC2, 1, 2, 3]
        );
        // ORNI - 0xC3
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORNI(1, 2, 3)),
            vec![0xC3, 1, 2, 3]
        );
        // NOR - 0xC4
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NOR(1, 2, 3)),
            vec![0xC4, 1, 2, 3]
        );
        // NORI - 0xC5
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NORI(1, 2, 3)),
            vec![0xC5, 1, 2, 3]
        );
        // XOR - 0xC6
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::XOR(1, 2, 3)),
            vec![0xC6, 1, 2, 3]
        );
        // XORI - 0xC7
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::XORI(1, 2, 3)),
            vec![0xC7, 1, 2, 3]
        );
        // AND - 0xC8
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::AND(1, 2, 3)),
            vec![0xC8, 1, 2, 3]
        );
        // ANDI - 0xC9
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDI(1, 2, 3)),
            vec![0xC9, 1, 2, 3]
        );
        // ANDN - 0xCA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDN(1, 2, 3)),
            vec![0xCA, 1, 2, 3]
        );
        // ANDNI - 0xCB
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDNI(1, 2, 3)),
            vec![0xCB, 1, 2, 3]
        );
        // NAND - 0xCC
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NAND(1, 2, 3)),
            vec![0xCC, 1, 2, 3]
        );
        // NANDI - 0xCD
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NANDI(1, 2, 3)),
            vec![0xCD, 1, 2, 3]
        );
        // NXOR - 0xCE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NXOR(1, 2, 3)),
            vec![0xCE, 1, 2, 3]
        );
        // NXORI - 0xCF
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::NXORI(1, 2, 3)),
            vec![0xCF, 1, 2, 3]
        );
    }

    #[test]
    fn test_bit_fiddling_encodings() {
        // BDIF - 0xD0
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BDIF(1, 2, 3)),
            vec![0xD0, 1, 2, 3]
        );
        // BDIFI - 0xD1
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BDIFI(1, 2, 3)),
            vec![0xD1, 1, 2, 3]
        );
        // WDIF - 0xD2
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::WDIF(1, 2, 3)),
            vec![0xD2, 1, 2, 3]
        );
        // WDIFI - 0xD3
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::WDIFI(1, 2, 3)),
            vec![0xD3, 1, 2, 3]
        );
        // TDIF - 0xD4
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::TDIF(1, 2, 3)),
            vec![0xD4, 1, 2, 3]
        );
        // TDIFI - 0xD5
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::TDIFI(1, 2, 3)),
            vec![0xD5, 1, 2, 3]
        );
        // ODIF - 0xD6
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ODIF(1, 2, 3)),
            vec![0xD6, 1, 2, 3]
        );
        // ODIFI - 0xD7
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ODIFI(1, 2, 3)),
            vec![0xD7, 1, 2, 3]
        );
        // MUX - 0xD8
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MUX(1, 2, 3)),
            vec![0xD8, 1, 2, 3]
        );
        // MUXI - 0xD9
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MUXI(1, 2, 3)),
            vec![0xD9, 1, 2, 3]
        );
        // SADD - 0xDA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SADD(1, 2, 3)),
            vec![0xDA, 1, 2, 3]
        );
        // SADDI - 0xDB
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SADDI(1, 2, 3)),
            vec![0xDB, 1, 2, 3]
        );
        // MOR - 0xDC
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MOR(1, 2, 3)),
            vec![0xDC, 1, 2, 3]
        );
        // MORI - 0xDD
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MORI(1, 2, 3)),
            vec![0xDD, 1, 2, 3]
        );
        // MXOR - 0xDE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MXOR(1, 2, 3)),
            vec![0xDE, 1, 2, 3]
        );
        // MXORI - 0xDF
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MXORI(1, 2, 3)),
            vec![0xDF, 1, 2, 3]
        );
    }

    #[test]
    fn test_set_encodings() {
        // SETH - 0xE0
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SETH(1, 0x1234)),
            vec![0xE0, 1, 0x12, 0x34]
        );
        // SETMH - 0xE1
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SETMH(1, 0x5678)),
            vec![0xE1, 1, 0x56, 0x78]
        );
        // SETML - 0xE2
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SETML(1, 0x9ABC)),
            vec![0xE2, 1, 0x9A, 0xBC]
        );
        // SETL - 0xE3
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SETL(1, 0xDEF0)),
            vec![0xE3, 1, 0xDE, 0xF0]
        );
        // INCH - 0xE4
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::INCH(1, 0x0001)),
            vec![0xE4, 1, 0x00, 0x01]
        );
        // INCMH - 0xE5
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::INCMH(1, 0x0002)),
            vec![0xE5, 1, 0x00, 0x02]
        );
        // INCML - 0xE6
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::INCML(1, 0x0003)),
            vec![0xE6, 1, 0x00, 0x03]
        );
        // INCL - 0xE7
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::INCL(1, 2, 3)),
            vec![0xE7, 1, 2, 3]
        );
        // ORH - 0xE8
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORH(1, 0xFFFF)),
            vec![0xE8, 1, 0xFF, 0xFF]
        );
        // ORMH - 0xE9
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORMH(1, 0xFFFF)),
            vec![0xE9, 1, 0xFF, 0xFF]
        );
        // ORML - 0xEA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORML(1, 0xFFFF)),
            vec![0xEA, 1, 0xFF, 0xFF]
        );
        // ORL - 0xEB
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ORL(1, 0xFFFF)),
            vec![0xEB, 1, 0xFF, 0xFF]
        );
        // ANDNH - 0xEC
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDNH(1, 0xFFFF)),
            vec![0xEC, 1, 0xFF, 0xFF]
        );
        // ANDNMH - 0xED
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDNMH(1, 0xFFFF)),
            vec![0xED, 1, 0xFF, 0xFF]
        );
        // ANDNML - 0xEE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDNML(1, 0xFFFF)),
            vec![0xEE, 1, 0xFF, 0xFF]
        );
        // ANDNL - 0xEF
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::ANDNL(1, 0xFFFF)),
            vec![0xEF, 1, 0xFF, 0xFF]
        );
    }

    #[test]
    fn test_jump_and_special_encodings() {
        // JMP - 0xF0
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::JMP(0x123456)),
            vec![0xF0, 0x12, 0x34, 0x56]
        );
        // PUSHJ - 0xF2
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PUSHJ(1, 2, 3)),
            vec![0xF2, 1, 2, 3]
        );
        // PUSHJB - 0xF3
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PUSHJB(1, 2, 3)),
            vec![0xF3, 1, 2, 3]
        );
        // GETA - 0xF4
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::GETA(1, 2, 3)),
            vec![0xF4, 1, 2, 3]
        );
        // GETAB - 0xF5
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::GETAB(1, 2, 3)),
            vec![0xF5, 1, 2, 3]
        );
        // PUT - 0xF6
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PUT(1, 2)),
            vec![0xF6, 1, 0, 2]
        );
        // PUTI - 0xF7
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::PUTI(1, 2)),
            vec![0xF7, 1, 0, 2]
        );
        // POP - 0xF8
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::POP(1, 2)),
            vec![0xF8, 1, 0, 2]
        );
        // RESUME - 0xF9
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::RESUME(0)),
            vec![0xF9, 0, 0, 0]
        );
        // SAVE - 0xFA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SAVE(1, 0)),
            vec![0xFA, 1, 0, 0]
        );
        // UNSAVE - 0xFB
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::UNSAVE(0, 1)),
            vec![0xFB, 0, 0, 1]
        );
        // SYNC - 0xFC
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SYNC(0)),
            vec![0xFC, 0, 0, 0]
        );
        // SWYM - 0xFD
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::SWYM),
            vec![0xFD, 0, 0, 0]
        );
        // GET - 0xFE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::GET(1, 2)),
            vec![0xFE, 1, 0, 2]
        );
        // TRIP - 0xFF
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::TRIP(1, 2, 3)),
            vec![0xFF, 1, 2, 3]
        );
    }

    #[test]
    fn test_multiply_divide_encodings() {
        // MUL - 0x18
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MUL(1, 2, 3)),
            vec![0x18, 1, 2, 3]
        );
        // MULI - 0x19
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MULI(1, 2, 3)),
            vec![0x19, 1, 2, 3]
        );
        // MULU - 0x1A
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MULU(1, 2, 3)),
            vec![0x1A, 1, 2, 3]
        );
        // MULUI - 0x1B
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::MULUI(1, 2, 3)),
            vec![0x1B, 1, 2, 3]
        );
        // DIV - 0x1C
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::DIV(1, 2, 3)),
            vec![0x1C, 1, 2, 3]
        );
        // DIVI - 0x1D
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::DIVI(1, 2, 3)),
            vec![0x1D, 1, 2, 3]
        );
        // DIVU - 0x1E
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::DIVU(1, 2, 3)),
            vec![0x1E, 1, 2, 3]
        );
        // DIVUI - 0x1F
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::DIVUI(1, 2, 3)),
            vec![0x1F, 1, 2, 3]
        );
    }

    #[test]
    fn test_data_directives() {
        // BYTE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::BYTE(0x42)),
            vec![0x42]
        );
        // WYDE
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::WYDE(0x1234)),
            vec![0x12, 0x34]
        );
        // TETRA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::TETRA(0x12345678)),
            vec![0x12, 0x34, 0x56, 0x78]
        );
        // OCTA
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::OCTA(0x123456789ABCDEF0)),
            vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]
        );
    }

    #[test]
    fn test_set_pseudo_instruction() {
        // SET should expand to SETH, SETMH, SETML, SETL
        let bytes = encode_instruction_bytes(&MMixInstruction::SET(1, 0x123456789ABCDEF0));
        assert_eq!(
            bytes,
            vec![
                0xE0, 1, 0x12, 0x34, // SETH $1, 0x1234
                0xE1, 1, 0x56, 0x78, // SETMH $1, 0x5678
                0xE2, 1, 0x9A, 0xBC, // SETML $1, 0x9ABC
                0xE3, 1, 0xDE, 0xF0, // SETL $1, 0xDEF0
            ]
        );
    }

    #[test]
    fn test_lda_encoding() {
        // LDA is ADDU with specific encoding
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDA(1, 2, 3)),
            vec![0x22, 1, 2, 3]
        );
        assert_eq!(
            encode_instruction_bytes(&MMixInstruction::LDAI(1, 2, 3)),
            vec![0x23, 1, 2, 3]
        );
    }
}
