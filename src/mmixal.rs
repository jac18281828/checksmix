use lyn::Scanner;
use std::collections::HashMap;

/// MMIX Assembly Language Parser
/// Parses MMIX assembly language into binary object code (.mmo)

#[derive(Debug, Clone, PartialEq)]
pub enum MMixInstruction {
    // Immediate load instructions
    SET(u8, u64),   // SET $X, value - pseudo-instruction
    SETL(u8, u16),  // SETL $X, YZ - set low wyde
    SETH(u8, u16),  // SETH $X, YZ - set high wyde
    SETMH(u8, u16), // SETMH $X, YZ - set medium high wyde
    SETML(u8, u16), // SETML $X, YZ - set medium low wyde

    // Load instructions
    LDB(u8, u8, u8),   // LDB $X, $Y, $Z - load byte signed
    LDBI(u8, u8, u8),  // LDB $X, $Y, Z - load byte signed (immediate)
    LDBU(u8, u8, u8),  // LDBU $X, $Y, $Z - load byte unsigned
    LDBUI(u8, u8, u8), // LDBU $X, $Y, Z - load byte unsigned (immediate)
    LDW(u8, u8, u8),   // LDW $X, $Y, $Z - load wyde signed
    LDWI(u8, u8, u8),  // LDW $X, $Y, Z - load wyde signed (immediate)
    LDWU(u8, u8, u8),  // LDWU $X, $Y, $Z - load wyde unsigned
    LDWUI(u8, u8, u8), // LDWU $X, $Y, Z - load wyde unsigned (immediate)
    LDT(u8, u8, u8),   // LDT $X, $Y, $Z - load tetra signed
    LDTI(u8, u8, u8),  // LDT $X, $Y, Z - load tetra signed (immediate)
    LDTU(u8, u8, u8),  // LDTU $X, $Y, $Z - load tetra unsigned
    LDTUI(u8, u8, u8), // LDTU $X, $Y, Z - load tetra unsigned (immediate)
    LDO(u8, u8, u8),   // LDO $X, $Y, $Z - load octa
    LDOI(u8, u8, u8),  // LDO $X, $Y, Z - load octa (immediate)
    LDOU(u8, u8, u8),  // LDOU $X, $Y, $Z - load octa unsigned
    LDOUI(u8, u8, u8), // LDOU $X, $Y, Z - load octa unsigned (immediate)
    LDHT(u8, u8, u8),  // LDHT $X, $Y, $Z - load high tetra
    LDHTI(u8, u8, u8), // LDHT $X, $Y, Z - load high tetra (immediate)
    LDA(u8, u8, u8),   // LDA $X, $Y, $Z - load address (ADDU)
    LDAI(u8, u8, u8),  // LDA $X, $Y, Z - load address (immediate)

    // Store instructions
    STB(u8, u8, u8),   // STB $X, $Y, $Z - store byte signed
    STBI(u8, u8, u8),  // STB $X, $Y, Z - store byte signed (immediate)
    STBU(u8, u8, u8),  // STBU $X, $Y, $Z - store byte unsigned
    STBUI(u8, u8, u8), // STBU $X, $Y, Z - store byte unsigned (immediate)
    STW(u8, u8, u8),   // STW $X, $Y, $Z - store wyde signed
    STWI(u8, u8, u8),  // STW $X, $Y, Z - store wyde signed (immediate)
    STWU(u8, u8, u8),  // STWU $X, $Y, $Z - store wyde unsigned
    STWUI(u8, u8, u8), // STWU $X, $Y, Z - store wyde unsigned (immediate)
    STT(u8, u8, u8),   // STT $X, $Y, $Z - store tetra signed
    STTI(u8, u8, u8),  // STT $X, $Y, Z - store tetra signed (immediate)
    STTU(u8, u8, u8),  // STTU $X, $Y, $Z - store tetra unsigned
    STTUI(u8, u8, u8), // STTU $X, $Y, Z - store tetra unsigned (immediate)
    STO(u8, u8, u8),   // STO $X, $Y, $Z - store octa
    STOI(u8, u8, u8),  // STO $X, $Y, Z - store octa (immediate)
    STOU(u8, u8, u8),  // STOU $X, $Y, $Z - store octa unsigned
    STOUI(u8, u8, u8), // STOU $X, $Y, Z - store octa unsigned (immediate)
    STCO(u8, u8, u8),  // STCO X, $Y, $Z - store constant octabyte
    STCOI(u8, u8, u8), // STCO X, $Y, Z - store constant octabyte (immediate)
    STHT(u8, u8, u8),  // STHT $X, $Y, $Z - store high tetra
    STHTI(u8, u8, u8), // STHT $X, $Y, Z - store high tetra (immediate)

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
    INCL(u8, u8, u8),    // INCL $X, $Y, $Z

    // Data directives
    BYTE(u8),   // .byte - 1 byte of data
    WYDE(u16),  // .wyde - 2 bytes of data
    TETRA(u32), // .tetra - 4 bytes of data
    OCTA(u64),  // .octa - 8 bytes of data

    // Control
    HALT, // HALT - stop execution (encoded as invalid opcode)
}

pub struct MMixAssembler {
    scanner: Scanner,
    line: usize,
    labels: HashMap<String, u64>,
    instructions: Vec<(u64, MMixInstruction)>,
    current_addr: u64,
}

impl MMixAssembler {
    pub fn new(input: &str) -> Self {
        Self {
            scanner: Scanner::new(input),
            line: 1,
            labels: HashMap::new(),
            instructions: Vec::new(),
            current_addr: 0,
        }
    }

    pub fn parse(&mut self) {
        while !self.scanner.is_done() {
            self.skip_whitespace_and_comments();
            if self.scanner.is_done() {
                break;
            }

            // Check for label (identifier followed by colon)
            if let Some(label) = self.try_parse_label() {
                self.labels.insert(label, self.current_addr);
                self.skip_whitespace_and_comments();
                // Label may be alone on a line
                if self.scanner.is_done() || matches!(self.scanner.peek(), Some(&'\n') | Some(&';'))
                {
                    continue;
                }
            }

            if let Some(instruction) = self.parse_instruction() {
                self.instructions
                    .push((self.current_addr, instruction.clone()));

                // Calculate instruction size
                let size = match instruction {
                    MMixInstruction::SET(_, _) => 16, // Expands to 4 instructions
                    MMixInstruction::BYTE(_) => 1,
                    MMixInstruction::WYDE(_) => 2,
                    MMixInstruction::TETRA(_) => 4,
                    MMixInstruction::OCTA(_) => 8,
                    _ => 4, // All other instructions are 4 bytes
                };
                self.current_addr += size;
            }
        }
    }

    /// Generate binary object code
    pub fn generate_object_code(&self) -> Vec<u8> {
        let mut code = Vec::new();

        for (_, instruction) in &self.instructions {
            match instruction {
                MMixInstruction::SET(x, value) => {
                    // SET expands to SETH, SETMH, SETML, SETL
                    let b0 = (value >> 48) as u16;
                    let b1 = (value >> 32) as u16;
                    let b2 = (value >> 16) as u16;
                    let b3 = *value as u16;

                    // SETH $X, high wyde (opcode 0xE0)
                    code.extend_from_slice(&Self::encode_instruction(0xE1, *x, b0));
                    // SETMH $X, medium high wyde (opcode 0xE2)
                    code.extend_from_slice(&Self::encode_instruction(0xE2, *x, b1));
                    // SETML $X, medium low wyde (opcode 0xE3)
                    code.extend_from_slice(&Self::encode_instruction(0xE3, *x, b2));
                    // SETL $X, low wyde (opcode 0xE4)
                    code.extend_from_slice(&Self::encode_instruction(0xE4, *x, b3));
                }
                MMixInstruction::SETL(x, yz) => {
                    code.extend_from_slice(&Self::encode_instruction(0xE4, *x, *yz));
                }
                MMixInstruction::SETH(x, yz) => {
                    code.extend_from_slice(&Self::encode_instruction(0xE1, *x, *yz));
                }
                MMixInstruction::SETMH(x, yz) => {
                    code.extend_from_slice(&Self::encode_instruction(0xE2, *x, *yz));
                }
                MMixInstruction::SETML(x, yz) => {
                    code.extend_from_slice(&Self::encode_instruction(0xE3, *x, *yz));
                }
                MMixInstruction::INCL(x, y, z) => {
                    code.extend_from_slice(&[0xE0, *x, *y, *z]);
                }
                // Load instructions
                MMixInstruction::LDB(x, y, z) => {
                    code.extend_from_slice(&[0x80, *x, *y, *z]);
                }
                MMixInstruction::LDBI(x, y, z) => {
                    code.extend_from_slice(&[0x81, *x, *y, *z]);
                }
                MMixInstruction::LDBU(x, y, z) => {
                    code.extend_from_slice(&[0x82, *x, *y, *z]);
                }
                MMixInstruction::LDBUI(x, y, z) => {
                    code.extend_from_slice(&[0x83, *x, *y, *z]);
                }
                MMixInstruction::LDW(x, y, z) => {
                    code.extend_from_slice(&[0x84, *x, *y, *z]);
                }
                MMixInstruction::LDWI(x, y, z) => {
                    code.extend_from_slice(&[0x85, *x, *y, *z]);
                }
                MMixInstruction::LDWU(x, y, z) => {
                    code.extend_from_slice(&[0x86, *x, *y, *z]);
                }
                MMixInstruction::LDWUI(x, y, z) => {
                    code.extend_from_slice(&[0x87, *x, *y, *z]);
                }
                MMixInstruction::LDT(x, y, z) => {
                    code.extend_from_slice(&[0x88, *x, *y, *z]);
                }
                MMixInstruction::LDTI(x, y, z) => {
                    code.extend_from_slice(&[0x89, *x, *y, *z]);
                }
                MMixInstruction::LDTU(x, y, z) => {
                    code.extend_from_slice(&[0x8A, *x, *y, *z]);
                }
                MMixInstruction::LDTUI(x, y, z) => {
                    code.extend_from_slice(&[0x8B, *x, *y, *z]);
                }
                MMixInstruction::LDO(x, y, z) => {
                    code.extend_from_slice(&[0x8C, *x, *y, *z]);
                }
                MMixInstruction::LDOI(x, y, z) => {
                    code.extend_from_slice(&[0x8D, *x, *y, *z]);
                }
                MMixInstruction::LDOU(x, y, z) => {
                    code.extend_from_slice(&[0x8E, *x, *y, *z]);
                }
                MMixInstruction::LDOUI(x, y, z) => {
                    code.extend_from_slice(&[0x8F, *x, *y, *z]);
                }
                MMixInstruction::LDHT(x, y, z) => {
                    code.extend_from_slice(&[0x90, *x, *y, *z]);
                }
                MMixInstruction::LDHTI(x, y, z) => {
                    code.extend_from_slice(&[0x91, *x, *y, *z]);
                }
                MMixInstruction::LDA(x, y, z) => {
                    code.extend_from_slice(&[0x22, *x, *y, *z]);
                }
                MMixInstruction::LDAI(x, y, z) => {
                    code.extend_from_slice(&[0x23, *x, *y, *z]);
                }
                // Store instructions
                MMixInstruction::STB(x, y, z) => {
                    code.extend_from_slice(&[0xA0, *x, *y, *z]);
                }
                MMixInstruction::STBI(x, y, z) => {
                    code.extend_from_slice(&[0xA1, *x, *y, *z]);
                }
                MMixInstruction::STBU(x, y, z) => {
                    code.extend_from_slice(&[0xA2, *x, *y, *z]);
                }
                MMixInstruction::STBUI(x, y, z) => {
                    code.extend_from_slice(&[0xA3, *x, *y, *z]);
                }
                MMixInstruction::STW(x, y, z) => {
                    code.extend_from_slice(&[0xA4, *x, *y, *z]);
                }
                MMixInstruction::STWI(x, y, z) => {
                    code.extend_from_slice(&[0xA5, *x, *y, *z]);
                }
                MMixInstruction::STWU(x, y, z) => {
                    code.extend_from_slice(&[0xA6, *x, *y, *z]);
                }
                MMixInstruction::STWUI(x, y, z) => {
                    code.extend_from_slice(&[0xA7, *x, *y, *z]);
                }
                MMixInstruction::STT(x, y, z) => {
                    code.extend_from_slice(&[0xA8, *x, *y, *z]);
                }
                MMixInstruction::STTI(x, y, z) => {
                    code.extend_from_slice(&[0xA9, *x, *y, *z]);
                }
                MMixInstruction::STTU(x, y, z) => {
                    code.extend_from_slice(&[0xAA, *x, *y, *z]);
                }
                MMixInstruction::STTUI(x, y, z) => {
                    code.extend_from_slice(&[0xAB, *x, *y, *z]);
                }
                MMixInstruction::STO(x, y, z) => {
                    code.extend_from_slice(&[0xAC, *x, *y, *z]);
                }
                MMixInstruction::STOI(x, y, z) => {
                    code.extend_from_slice(&[0xAD, *x, *y, *z]);
                }
                MMixInstruction::STOU(x, y, z) => {
                    code.extend_from_slice(&[0xAE, *x, *y, *z]);
                }
                MMixInstruction::STOUI(x, y, z) => {
                    code.extend_from_slice(&[0xAF, *x, *y, *z]);
                }
                MMixInstruction::STCO(x, y, z) => {
                    code.extend_from_slice(&[0xB0, *x, *y, *z]);
                }
                MMixInstruction::STCOI(x, y, z) => {
                    code.extend_from_slice(&[0xB1, *x, *y, *z]);
                }
                MMixInstruction::STHT(x, y, z) => {
                    code.extend_from_slice(&[0xB2, *x, *y, *z]);
                }
                MMixInstruction::STHTI(x, y, z) => {
                    code.extend_from_slice(&[0xB3, *x, *y, *z]);
                }
                // Arithmetic instructions
                MMixInstruction::ADD(x, y, z) => {
                    code.extend_from_slice(&[0x20, *x, *y, *z]);
                }
                MMixInstruction::ADDI(x, y, z) => {
                    code.extend_from_slice(&[0x21, *x, *y, *z]);
                }
                MMixInstruction::ADDU(x, y, z) => {
                    code.extend_from_slice(&[0x22, *x, *y, *z]);
                }
                MMixInstruction::ADDUI(x, y, z) => {
                    code.extend_from_slice(&[0x23, *x, *y, *z]);
                }
                MMixInstruction::ADDU2(x, y, z) => {
                    code.extend_from_slice(&[0x24, *x, *y, *z]);
                }
                MMixInstruction::ADDU2I(x, y, z) => {
                    code.extend_from_slice(&[0x25, *x, *y, *z]);
                }
                MMixInstruction::ADDU4(x, y, z) => {
                    code.extend_from_slice(&[0x26, *x, *y, *z]);
                }
                MMixInstruction::ADDU4I(x, y, z) => {
                    code.extend_from_slice(&[0x27, *x, *y, *z]);
                }
                MMixInstruction::ADDU8(x, y, z) => {
                    code.extend_from_slice(&[0x28, *x, *y, *z]);
                }
                MMixInstruction::ADDU8I(x, y, z) => {
                    code.extend_from_slice(&[0x29, *x, *y, *z]);
                }
                MMixInstruction::ADDU16(x, y, z) => {
                    code.extend_from_slice(&[0x2A, *x, *y, *z]);
                }
                MMixInstruction::ADDU16I(x, y, z) => {
                    code.extend_from_slice(&[0x2B, *x, *y, *z]);
                }
                MMixInstruction::SUB(x, y, z) => {
                    code.extend_from_slice(&[0x30, *x, *y, *z]);
                }
                MMixInstruction::SUBI(x, y, z) => {
                    code.extend_from_slice(&[0x31, *x, *y, *z]);
                }
                MMixInstruction::SUBU(x, y, z) => {
                    code.extend_from_slice(&[0x32, *x, *y, *z]);
                }
                MMixInstruction::SUBUI(x, y, z) => {
                    code.extend_from_slice(&[0x33, *x, *y, *z]);
                }
                MMixInstruction::NEG(x, y, z) => {
                    code.extend_from_slice(&[0x34, *x, *y, *z]);
                }
                MMixInstruction::NEGI(x, y, z) => {
                    code.extend_from_slice(&[0x35, *x, *y, *z]);
                }
                MMixInstruction::NEGU(x, y, z) => {
                    code.extend_from_slice(&[0x36, *x, *y, *z]);
                }
                MMixInstruction::NEGUI(x, y, z) => {
                    code.extend_from_slice(&[0x37, *x, *y, *z]);
                }
                MMixInstruction::BYTE(value) => {
                    code.push(*value);
                }
                MMixInstruction::WYDE(value) => {
                    code.extend_from_slice(&value.to_be_bytes());
                }
                MMixInstruction::TETRA(value) => {
                    code.extend_from_slice(&value.to_be_bytes());
                }
                MMixInstruction::OCTA(value) => {
                    code.extend_from_slice(&value.to_be_bytes());
                }
                MMixInstruction::HALT => {
                    code.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
                }
            }
        }

        code
    }

    fn encode_instruction(opcode: u8, x: u8, yz: u16) -> [u8; 4] {
        let y = (yz >> 8) as u8;
        let z = yz as u8;
        [opcode, x, y, z]
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.scanner.is_done() {
            let ch = self.scanner.peek();
            if let Some(&c) = ch {
                match c {
                    ' ' | '\t' | '\r' => {
                        self.scanner.pop();
                    }
                    '\n' => {
                        self.scanner.pop();
                        self.line += 1;
                    }
                    ';' => {
                        // Skip until end of line
                        while !self.scanner.is_done() {
                            if let Some(&ch) = self.scanner.peek() {
                                self.scanner.pop();
                                if ch == '\n' {
                                    self.line += 1;
                                    break;
                                }
                            }
                        }
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }
    }

    fn parse_instruction(&mut self) -> Option<MMixInstruction> {
        let mnemonic = self.parse_mnemonic()?;

        match mnemonic.to_uppercase().as_str() {
            ".BYTE" => {
                let value = self.parse_number()? as u8;
                Some(MMixInstruction::BYTE(value))
            }
            ".WYDE" => {
                let value = self.parse_number()? as u16;
                Some(MMixInstruction::WYDE(value))
            }
            ".TETRA" => {
                let value = self.parse_number()? as u32;
                Some(MMixInstruction::TETRA(value))
            }
            ".OCTA" | ".QUAD" => {
                // .quad is alias for .octa (8 bytes)
                let value = self.parse_number_or_label()?;
                Some(MMixInstruction::OCTA(value))
            }
            "SET" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let value = self.parse_number()?;
                Some(MMixInstruction::SET(x, value))
            }
            "SETL" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let yz = self.parse_number()? as u16;
                Some(MMixInstruction::SETL(x, yz))
            }
            "SETH" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let yz = self.parse_number()? as u16;
                Some(MMixInstruction::SETH(x, yz))
            }
            "SETMH" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let yz = self.parse_number()? as u16;
                Some(MMixInstruction::SETMH(x, yz))
            }
            "SETML" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let yz = self.parse_number()? as u16;
                Some(MMixInstruction::SETML(x, yz))
            }
            "INCL" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                let z = self.parse_register()?;
                Some(MMixInstruction::INCL(x, y, z))
            }
            "LDB" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDB(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDBI(x, y, z_imm))
                }
            }
            "LDBU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDBU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDBUI(x, y, z_imm))
                }
            }
            "LDW" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDW(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDWI(x, y, z_imm))
                }
            }
            "LDWU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDWU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDWUI(x, y, z_imm))
                }
            }
            "LDT" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDT(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDTI(x, y, z_imm))
                }
            }
            "LDTU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDTU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDTUI(x, y, z_imm))
                }
            }
            "LDO" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDO(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDOI(x, y, z_imm))
                }
            }
            "LDOU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDOU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDOUI(x, y, z_imm))
                }
            }
            "LDHT" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDHT(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDHTI(x, y, z_imm))
                }
            }
            "LDA" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::LDA(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::LDAI(x, y, z_imm))
                }
            }
            "STB" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STB(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STBI(x, y, z_imm))
                }
            }
            "STBU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STBU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STBUI(x, y, z_imm))
                }
            }
            "STW" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STW(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STWI(x, y, z_imm))
                }
            }
            "STWU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STWU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STWUI(x, y, z_imm))
                }
            }
            "STT" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STT(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STTI(x, y, z_imm))
                }
            }
            "STTU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STTU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STTUI(x, y, z_imm))
                }
            }
            "STO" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STO(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STOI(x, y, z_imm))
                }
            }
            "STOU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STOU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STOUI(x, y, z_imm))
                }
            }
            "STCO" => {
                let x = self.parse_number()? as u8; // STCO uses immediate for X
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STCO(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STCOI(x, y, z_imm))
                }
            }
            "STHT" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::STHT(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::STHTI(x, y, z_imm))
                }
            }
            "ADD" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::ADD(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::ADDI(x, y, z_imm))
                }
            }
            "ADDU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::ADDU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::ADDUI(x, y, z_imm))
                }
            }
            "2ADDU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::ADDU2(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::ADDU2I(x, y, z_imm))
                }
            }
            "4ADDU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::ADDU4(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::ADDU4I(x, y, z_imm))
                }
            }
            "8ADDU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::ADDU8(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::ADDU8I(x, y, z_imm))
                }
            }
            "16ADDU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::ADDU16(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::ADDU16I(x, y, z_imm))
                }
            }
            "SUB" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::SUB(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::SUBI(x, y, z_imm))
                }
            }
            "SUBU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_register()?;
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::SUBU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::SUBUI(x, y, z_imm))
                }
            }
            "NEG" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_number()? as u8; // Y is immediate
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::NEG(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::NEGI(x, y, z_imm))
                }
            }
            "NEGU" => {
                let x = self.parse_register()?;
                self.expect_comma();
                let y = self.parse_number()? as u8; // Y is immediate
                self.expect_comma();
                if let Some(z_reg) = self.try_parse_register() {
                    Some(MMixInstruction::NEGU(x, y, z_reg))
                } else {
                    let z_imm = self.parse_number()? as u8;
                    Some(MMixInstruction::NEGUI(x, y, z_imm))
                }
            }
            "HALT" => Some(MMixInstruction::HALT),
            _ => {
                eprintln!("Unknown mnemonic '{}' at line {}", mnemonic, self.line);
                None
            }
        }
    }

    fn parse_mnemonic(&mut self) -> Option<String> {
        let mut mnemonic = String::new();

        // Allow leading dot for directives
        if matches!(self.scanner.peek(), Some(&'.')) {
            mnemonic.push('.');
            self.scanner.pop();
        }

        loop {
            match self.scanner.peek() {
                Some(&c) if c.is_ascii_alphanumeric() => {
                    mnemonic.push(c);
                    self.scanner.pop();
                }
                _ => break,
            }
        }
        if mnemonic.is_empty() || mnemonic == "." {
            None
        } else {
            Some(mnemonic)
        }
    }

    fn parse_register(&mut self) -> Option<u8> {
        self.skip_whitespace_and_comments();

        // Expect '$'
        if !matches!(self.scanner.peek(), Some(&'$')) {
            return None;
        }
        self.scanner.pop();

        // Parse register number
        let mut num_str = String::new();
        loop {
            match self.scanner.peek() {
                Some(&c) if c.is_ascii_digit() => {
                    num_str.push(c);
                    self.scanner.pop();
                }
                _ => break,
            }
        }
        num_str.parse::<u8>().ok()
    }

    /// Try to parse a register, peeking ahead to check for '$' without consuming on failure
    fn try_parse_register(&mut self) -> Option<u8> {
        self.skip_whitespace_and_comments();

        // Check if next character is '$' (register marker)
        if matches!(self.scanner.peek(), Some(&'$')) {
            self.parse_register()
        } else {
            None
        }
    }

    fn parse_number(&mut self) -> Option<u64> {
        self.skip_whitespace_and_comments();

        let mut num_str = String::new();
        let mut is_hex = false;

        // Check for hex prefix
        if matches!(self.scanner.peek(), Some(&'#')) {
            self.scanner.pop();
            is_hex = true;
        } else if matches!(self.scanner.peek(), Some(&'0')) {
            self.scanner.pop();
            if matches!(self.scanner.peek(), Some(&'x')) {
                self.scanner.pop();
                is_hex = true;
            } else {
                num_str.push('0');
            }
        }

        loop {
            match self.scanner.peek() {
                Some(&c)
                    if (is_hex && c.is_ascii_hexdigit()) || (!is_hex && c.is_ascii_digit()) =>
                {
                    num_str.push(c);
                    self.scanner.pop();
                }
                _ => break,
            }
        }

        if num_str.is_empty() {
            return None;
        }

        if is_hex {
            u64::from_str_radix(&num_str, 16).ok()
        } else {
            num_str.parse::<u64>().ok()
        }
    }

    fn expect_comma(&mut self) {
        self.skip_whitespace_and_comments();
        if matches!(self.scanner.peek(), Some(&',')) {
            self.scanner.pop();
        }
    }

    fn try_parse_label(&mut self) -> Option<String> {
        // Try to parse an identifier
        let mut temp_label = String::new();

        // Peek ahead to see if this looks like a label (identifier followed by colon)
        loop {
            match self.scanner.peek() {
                Some(&c) if c.is_ascii_alphanumeric() || c == '_' => {
                    temp_label.push(c);
                    self.scanner.pop();
                }
                _ => break,
            }
        }

        if temp_label.is_empty() {
            return None;
        }

        // Now check if followed by colon (with possible whitespace)
        loop {
            match self.scanner.peek() {
                Some(&' ') | Some(&'\t') | Some(&'\r') => {
                    self.scanner.pop();
                }
                _ => break,
            }
        }

        if matches!(self.scanner.peek(), Some(&':')) {
            self.scanner.pop();
            Some(temp_label)
        } else {
            // Not a label - this will be parsed as an instruction mnemonic instead
            // We've consumed the identifier, which is fine - parse_mnemonic won't be called
            None
        }
    }

    fn parse_number_or_label(&mut self) -> Option<u64> {
        self.skip_whitespace_and_comments();

        // Try to parse as a number first
        if matches!(self.scanner.peek(), Some(&c) if c.is_ascii_digit() || c == '0') {
            return self.parse_number();
        }

        // Otherwise, try to parse as a label reference
        let mut label = String::new();
        loop {
            match self.scanner.peek() {
                Some(&c) if c.is_ascii_alphanumeric() || c == '_' => {
                    label.push(c);
                    self.scanner.pop();
                }
                _ => break,
            }
        }

        if label.is_empty() {
            return None;
        }

        // Look up label address (0 if not found yet - will be resolved later)
        Some(*self.labels.get(&label).unwrap_or(&0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_set() {
        let mut asm = MMixAssembler::new("SET $2, 10");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SET(2, 10));
    }

    #[test]
    fn test_parse_incl() {
        let mut asm = MMixAssembler::new("INCL $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::INCL(1, 2, 3));
    }

    #[test]
    fn test_parse_halt() {
        let mut asm = MMixAssembler::new("HALT");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::HALT);
    }

    #[test]
    fn test_parse_program() {
        let program = r#"
            SET $2, 10
            SET $3, 20
            INCL $1, $2, $3
            HALT
        "#;
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.instructions.len(), 4);
    }

    #[test]
    fn test_generate_object_code() {
        let mut asm = MMixAssembler::new("INCL $1, $2, $3");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0xE0, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_parse_with_comments() {
        let program = r#"
            SET $2, 10   ; Set register 2 to 10
            ; This is a comment line
            INCL $1, $2, $3  ; Add them
        "#;
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_parse_setl() {
        let mut asm = MMixAssembler::new("SETL $5, 100");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(5, 100));
    }

    #[test]
    fn test_parse_seth() {
        let mut asm = MMixAssembler::new("SETH $6, 200");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETH(6, 200));
    }

    #[test]
    fn test_parse_setmh() {
        let mut asm = MMixAssembler::new("SETMH $7, 300");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETMH(7, 300));
    }

    #[test]
    fn test_parse_setml() {
        let mut asm = MMixAssembler::new("SETML $8, 400");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETML(8, 400));
    }

    #[test]
    fn test_parse_ldb_register() {
        let mut asm = MMixAssembler::new("LDB $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDB(1, 2, 3));
    }

    #[test]
    fn test_parse_ldb_immediate() {
        let mut asm = MMixAssembler::new("LDB $1, $2, 10");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDBI(1, 2, 10));
    }

    #[test]
    fn test_parse_ldbu_register() {
        let mut asm = MMixAssembler::new("LDBU $4, $5, $6");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDBU(4, 5, 6));
    }

    #[test]
    fn test_parse_ldbu_immediate() {
        let mut asm = MMixAssembler::new("LDBU $4, $5, 20");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDBUI(4, 5, 20));
    }

    #[test]
    fn test_parse_ldw_register() {
        let mut asm = MMixAssembler::new("LDW $7, $8, $9");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDW(7, 8, 9));
    }

    #[test]
    fn test_parse_ldw_immediate() {
        let mut asm = MMixAssembler::new("LDW $7, $8, 30");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDWI(7, 8, 30));
    }

    #[test]
    fn test_parse_ldwu_register() {
        let mut asm = MMixAssembler::new("LDWU $10, $11, $12");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDWU(10, 11, 12));
    }

    #[test]
    fn test_parse_ldwu_immediate() {
        let mut asm = MMixAssembler::new("LDWU $10, $11, 40");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDWUI(10, 11, 40));
    }

    #[test]
    fn test_parse_ldt_register() {
        let mut asm = MMixAssembler::new("LDT $13, $14, $15");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDT(13, 14, 15));
    }

    #[test]
    fn test_parse_ldt_immediate() {
        let mut asm = MMixAssembler::new("LDT $13, $14, 50");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDTI(13, 14, 50));
    }

    #[test]
    fn test_parse_ldtu_register() {
        let mut asm = MMixAssembler::new("LDTU $16, $17, $18");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDTU(16, 17, 18));
    }

    #[test]
    fn test_parse_ldtu_immediate() {
        let mut asm = MMixAssembler::new("LDTU $16, $17, 60");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDTUI(16, 17, 60));
    }

    #[test]
    fn test_parse_ldo_register() {
        let mut asm = MMixAssembler::new("LDO $19, $20, $21");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDO(19, 20, 21));
    }

    #[test]
    fn test_parse_ldo_immediate() {
        let mut asm = MMixAssembler::new("LDO $19, $20, 70");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDOI(19, 20, 70));
    }

    #[test]
    fn test_parse_ldou_register() {
        let mut asm = MMixAssembler::new("LDOU $22, $23, $24");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDOU(22, 23, 24));
    }

    #[test]
    fn test_parse_ldou_immediate() {
        let mut asm = MMixAssembler::new("LDOU $22, $23, 80");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDOUI(22, 23, 80));
    }

    #[test]
    fn test_parse_ldht_register() {
        let mut asm = MMixAssembler::new("LDHT $25, $26, $27");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDHT(25, 26, 27));
    }

    #[test]
    fn test_parse_ldht_immediate() {
        let mut asm = MMixAssembler::new("LDHT $25, $26, 90");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDHTI(25, 26, 90));
    }

    #[test]
    fn test_parse_lda_register() {
        let mut asm = MMixAssembler::new("LDA $28, $29, $30");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDA(28, 29, 30));
    }

    #[test]
    fn test_parse_lda_immediate() {
        let mut asm = MMixAssembler::new("LDA $28, $29, 100");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::LDAI(28, 29, 100));
    }

    #[test]
    fn test_generate_load_object_code() {
        let mut asm = MMixAssembler::new("LDB $1, $2, $3");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x80, 0x01, 0x02, 0x03]);

        let mut asm = MMixAssembler::new("LDBU $4, $5, 10");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x83, 0x04, 0x05, 0x0A]);

        let mut asm = MMixAssembler::new("LDA $7, $8, $9");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x22, 0x07, 0x08, 0x09]);
    }

    #[test]
    fn test_parse_load_program() {
        let program = r#"
            SET $10, 1000
            LDB $1, $10, 0
            LDBU $2, $10, 1
            LDW $3, $10, 2
            LDT $4, $10, 4
            LDO $5, $10, 8
            HALT
        "#;
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.instructions.len(), 7);
    }

    // Store instruction parser tests

    #[test]
    fn test_parse_stb_register() {
        let mut asm = MMixAssembler::new("STB $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STB(1, 2, 3));
    }

    #[test]
    fn test_parse_stb_immediate() {
        let mut asm = MMixAssembler::new("STB $1, $2, 10");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STBI(1, 2, 10));
    }

    #[test]
    fn test_parse_stbu_register() {
        let mut asm = MMixAssembler::new("STBU $4, $5, $6");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STBU(4, 5, 6));
    }

    #[test]
    fn test_parse_stbu_immediate() {
        let mut asm = MMixAssembler::new("STBU $4, $5, 20");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STBUI(4, 5, 20));
    }

    #[test]
    fn test_parse_stw_register() {
        let mut asm = MMixAssembler::new("STW $7, $8, $9");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STW(7, 8, 9));
    }

    #[test]
    fn test_parse_stw_immediate() {
        let mut asm = MMixAssembler::new("STW $7, $8, 30");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STWI(7, 8, 30));
    }

    #[test]
    fn test_parse_stwu_register() {
        let mut asm = MMixAssembler::new("STWU $10, $11, $12");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STWU(10, 11, 12));
    }

    #[test]
    fn test_parse_stwu_immediate() {
        let mut asm = MMixAssembler::new("STWU $10, $11, 40");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STWUI(10, 11, 40));
    }

    #[test]
    fn test_parse_stt_register() {
        let mut asm = MMixAssembler::new("STT $13, $14, $15");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STT(13, 14, 15));
    }

    #[test]
    fn test_parse_stt_immediate() {
        let mut asm = MMixAssembler::new("STT $13, $14, 50");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STTI(13, 14, 50));
    }

    #[test]
    fn test_parse_sttu_register() {
        let mut asm = MMixAssembler::new("STTU $16, $17, $18");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STTU(16, 17, 18));
    }

    #[test]
    fn test_parse_sttu_immediate() {
        let mut asm = MMixAssembler::new("STTU $16, $17, 60");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STTUI(16, 17, 60));
    }

    #[test]
    fn test_parse_sto_register() {
        let mut asm = MMixAssembler::new("STO $19, $20, $21");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STO(19, 20, 21));
    }

    #[test]
    fn test_parse_sto_immediate() {
        let mut asm = MMixAssembler::new("STO $19, $20, 70");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STOI(19, 20, 70));
    }

    #[test]
    fn test_parse_stou_register() {
        let mut asm = MMixAssembler::new("STOU $22, $23, $24");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STOU(22, 23, 24));
    }

    #[test]
    fn test_parse_stou_immediate() {
        let mut asm = MMixAssembler::new("STOU $22, $23, 80");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STOUI(22, 23, 80));
    }

    #[test]
    fn test_parse_stco_register() {
        let mut asm = MMixAssembler::new("STCO 42, $26, $27");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STCO(42, 26, 27));
    }

    #[test]
    fn test_parse_stco_immediate() {
        let mut asm = MMixAssembler::new("STCO 255, $26, 90");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STCOI(255, 26, 90));
    }

    #[test]
    fn test_parse_stht_register() {
        let mut asm = MMixAssembler::new("STHT $25, $26, $27");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STHT(25, 26, 27));
    }

    #[test]
    fn test_parse_stht_immediate() {
        let mut asm = MMixAssembler::new("STHT $25, $26, 100");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::STHTI(25, 26, 100));
    }

    #[test]
    fn test_generate_store_object_code() {
        let mut asm = MMixAssembler::new("STB $1, $2, $3");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0xA0, 0x01, 0x02, 0x03]);

        let mut asm = MMixAssembler::new("STBU $4, $5, 10");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0xA3, 0x04, 0x05, 0x0A]);

        let mut asm = MMixAssembler::new("STCO 42, $6, $7");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0xB0, 0x2A, 0x06, 0x07]);
    }

    #[test]
    fn test_parse_store_program() {
        let program = r#"
            SET $10, 2000
            STB $1, $10, 0
            STBU $2, $10, 1
            STW $3, $10, 2
            STT $4, $10, 4
            STO $5, $10, 8
            STCO 123, $10, 16
            STHT $6, $10, 24
            HALT
        "#;
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.instructions.len(), 9);
    }

    // Arithmetic instruction parser tests

    #[test]
    fn test_parse_add_register() {
        let mut asm = MMixAssembler::new("ADD $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 3));
    }

    #[test]
    fn test_parse_add_immediate() {
        let mut asm = MMixAssembler::new("ADD $1, $2, 100");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 100));
    }

    #[test]
    fn test_parse_addu_register() {
        let mut asm = MMixAssembler::new("ADDU $5, $6, $7");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU(5, 6, 7));
    }

    #[test]
    fn test_parse_addu_immediate() {
        let mut asm = MMixAssembler::new("ADDU $5, $6, 50");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDUI(5, 6, 50));
    }

    #[test]
    fn test_parse_2addu_register() {
        let mut asm = MMixAssembler::new("2ADDU $10, $11, $12");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU2(10, 11, 12));
    }

    #[test]
    fn test_parse_2addu_immediate() {
        let mut asm = MMixAssembler::new("2ADDU $10, $11, 20");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU2I(10, 11, 20));
    }

    #[test]
    fn test_parse_4addu_register() {
        let mut asm = MMixAssembler::new("4ADDU $15, $16, $17");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU4(15, 16, 17));
    }

    #[test]
    fn test_parse_4addu_immediate() {
        let mut asm = MMixAssembler::new("4ADDU $15, $16, 40");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU4I(15, 16, 40));
    }

    #[test]
    fn test_parse_8addu_register() {
        let mut asm = MMixAssembler::new("8ADDU $20, $21, $22");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU8(20, 21, 22));
    }

    #[test]
    fn test_parse_8addu_immediate() {
        let mut asm = MMixAssembler::new("8ADDU $20, $21, 80");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU8I(20, 21, 80));
    }

    #[test]
    fn test_parse_16addu_register() {
        let mut asm = MMixAssembler::new("16ADDU $25, $26, $27");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU16(25, 26, 27));
    }

    #[test]
    fn test_parse_16addu_immediate() {
        let mut asm = MMixAssembler::new("16ADDU $25, $26, 160");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::ADDU16I(25, 26, 160));
    }

    #[test]
    fn test_parse_sub_register() {
        let mut asm = MMixAssembler::new("SUB $30, $31, $32");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SUB(30, 31, 32));
    }

    #[test]
    fn test_parse_sub_immediate() {
        let mut asm = MMixAssembler::new("SUB $30, $31, 75");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SUBI(30, 31, 75));
    }

    #[test]
    fn test_parse_subu_register() {
        let mut asm = MMixAssembler::new("SUBU $35, $36, $37");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SUBU(35, 36, 37));
    }

    #[test]
    fn test_parse_subu_immediate() {
        let mut asm = MMixAssembler::new("SUBU $35, $36, 90");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::SUBUI(35, 36, 90));
    }

    #[test]
    fn test_parse_neg_register() {
        let mut asm = MMixAssembler::new("NEG $40, 0, $41");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::NEG(40, 0, 41));
    }

    #[test]
    fn test_parse_neg_immediate() {
        let mut asm = MMixAssembler::new("NEG $40, 10, 5");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::NEGI(40, 10, 5));
    }

    #[test]
    fn test_parse_negu_register() {
        let mut asm = MMixAssembler::new("NEGU $45, 100, $46");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::NEGU(45, 100, 46));
    }

    #[test]
    fn test_parse_negu_immediate() {
        let mut asm = MMixAssembler::new("NEGU $45, 200, 150");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::NEGUI(45, 200, 150));
    }

    #[test]
    fn test_generate_arithmetic_object_code() {
        let mut asm = MMixAssembler::new("ADD $1, $2, $3");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x20, 0x01, 0x02, 0x03]);

        let mut asm = MMixAssembler::new("ADDU $4, $5, 10");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x23, 0x04, 0x05, 0x0A]);

        let mut asm = MMixAssembler::new("SUB $7, $8, $9");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x30, 0x07, 0x08, 0x09]);

        let mut asm = MMixAssembler::new("8ADDU $10, $11, $12");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x28, 0x0A, 0x0B, 0x0C]);
    }

    #[test]
    fn test_parse_arithmetic_program() {
        let program = r#"
            SET $10, 100
            SET $11, 50
            ADD $1, $10, $11
            ADDU $2, $10, 25
            2ADDU $3, $10, $11
            4ADDU $4, $10, 10
            8ADDU $5, $10, 5
            16ADDU $6, $10, 1
            SUB $7, $10, $11
            SUBU $8, $10, 30
            NEG $9, 0, $10
            NEGU $12, 200, $11
            HALT
        "#;
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.instructions.len(), 13);
    }

    // Label and directive tests

    #[test]
    fn test_parse_label() {
        let mut asm = MMixAssembler::new("LOOP: HALT");
        asm.parse();
        assert_eq!(asm.labels.get("LOOP"), Some(&0));
        assert_eq!(asm.instructions.len(), 1);
    }

    #[test]
    fn test_parse_label_with_instruction() {
        let program = "SET $1, 100\nLOOP:\nINCL $1, $1, $1\nHALT";
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.labels.get("LOOP"), Some(&16)); // After SET (16 bytes)
        assert_eq!(asm.instructions.len(), 3);
    }

    #[test]
    fn test_parse_octa_directive() {
        let mut asm = MMixAssembler::new(".octa 0x123456789ABCDEF0");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::OCTA(0x123456789ABCDEF0)
        );
    }

    #[test]
    fn test_parse_quad_directive() {
        let mut asm = MMixAssembler::new(".quad 42");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(42));
    }

    #[test]
    fn test_parse_byte_directive() {
        let mut asm = MMixAssembler::new(".byte 255");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(255));
    }

    #[test]
    fn test_parse_wyde_directive() {
        let mut asm = MMixAssembler::new(".wyde 0x1234");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::WYDE(0x1234));
    }

    #[test]
    fn test_parse_tetra_directive() {
        let mut asm = MMixAssembler::new(".tetra 0xDEADBEEF");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(asm.instructions[0].1, MMixInstruction::TETRA(0xDEADBEEF));
    }

    #[test]
    fn test_parse_node_structure() {
        let program = "NODE:\n.quad 42\n.quad 0";
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.labels.get("NODE"), Some(&0));
        assert_eq!(asm.instructions.len(), 2);
        assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(42));
        assert_eq!(asm.instructions[1].1, MMixInstruction::OCTA(0));
    }

    #[test]
    fn test_generate_data_directives() {
        let mut asm = MMixAssembler::new(".octa 0x123456789ABCDEF0");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);

        let mut asm = MMixAssembler::new(".tetra 0xDEADBEEF");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0xDE, 0xAD, 0xBE, 0xEF]);

        let mut asm = MMixAssembler::new(".wyde 0x1234");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![0x12, 0x34]);

        let mut asm = MMixAssembler::new(".byte 42");
        asm.parse();
        let code = asm.generate_object_code();
        assert_eq!(code, vec![42]);
    }

    #[test]
    fn test_label_addresses() {
        let program = "START: SET $1, 10\nSET $2, 20\nLOOP: ADD $3, $1, $2\nHALT\nEND:";
        let mut asm = MMixAssembler::new(program);
        asm.parse();
        assert_eq!(asm.labels.get("START"), Some(&0));
        assert_eq!(asm.labels.get("LOOP"), Some(&32)); // After 2 SETs (16 bytes each)
        assert_eq!(asm.labels.get("END"), Some(&40)); // After LOOP + ADD (4 bytes) + HALT (4 bytes)
    }
}
