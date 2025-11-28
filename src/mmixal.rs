use pest_derive::Parser;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "mmixal.pest"]
struct MMixalParser;

/// MMIX Assembly Language Parser
/// Parses MMIX assembly language into binary object code (.mmo)
#[allow(clippy::upper_case_acronyms)]
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

    MUL(u8, u8, u8),  // MUL $X, $Y, $Z - multiply
    MULI(u8, u8, u8), // MUL $X, $Y, Z - multiply immediate
    DIV(u8, u8, u8),  // DIV $X, $Y, $Z - divide
    DIVI(u8, u8, u8), // DIV $X, $Y, Z - divide immediate

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
    JMP(u8),     // JMP offset
    JE(u8, u8),  // JE $X, offset
    JNE(u8, u8), // JNE $X, offset
    JL(u8, u8),  // JL $X, offset
    JG(u8, u8),  // JG $X, offset

    // Data directives
    BYTE(u8),   // BYTE - 1 byte of data
    WYDE(u16),  // WYDE - 2 bytes of data
    TETRA(u32), // TETRA - 4 bytes of data
    OCTA(u64),  // OCTA - 8 bytes of data

    // Control
    HALT, // HALT - stop execution
}

pub struct MMixAssembler {
    source: String,
    pub labels: HashMap<String, u64>,
    pub instructions: Vec<(u64, MMixInstruction)>,
    current_addr: u64,
}

impl MMixAssembler {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            labels: HashMap::new(),
            instructions: Vec::new(),
            current_addr: 0,
        }
    }

    pub fn parse(&mut self) {
        match self.parse_internal() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Parse error: {}", e);
            }
        }
    }

    fn parse_internal(&mut self) -> Result<(), String> {
        use pest::Parser;

        let source = self.source.clone();
        let pairs = MMixalParser::parse(Rule::program, &source).map_err(|e| format!("{}", e))?;

        for pair in pairs {
            if pair.as_rule() == Rule::program {
                for stmt_pair in pair.into_inner() {
                    if stmt_pair.as_rule() == Rule::statement {
                        self.parse_statement(stmt_pair)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_statement(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut label_name: Option<String> = None;
        let mut inst: Option<MMixInstruction> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let ident = inner_pair.into_inner().next().unwrap();
                    label_name = Some(ident.as_str().to_string());
                }
                Rule::instruction => {
                    inst = Some(self.parse_instruction(inner_pair)?);
                }
                Rule::data_directive => {
                    inst = Some(self.parse_data_directive(inner_pair)?);
                }
                _ => {}
            }
        }

        if let Some(label) = label_name {
            self.labels.insert(label, self.current_addr);
        }

        if let Some(instruction) = inst {
            let size = Self::instruction_size(&instruction);
            self.instructions.push((self.current_addr, instruction));
            self.current_addr += size;
        }

        Ok(())
    }

    fn parse_instruction(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let inner = pair.into_inner().next().ok_or("Empty instruction")?;

        match inner.as_rule() {
            Rule::inst_set => self.parse_inst_set(inner),
            Rule::inst_setl => self.parse_inst_setl(inner),
            Rule::inst_seth => self.parse_inst_seth(inner),
            Rule::inst_setmh => self.parse_inst_setmh(inner),
            Rule::inst_setml => self.parse_inst_setml(inner),
            Rule::inst_incl => self.parse_inst_incl(inner),
            Rule::inst_load_store_rrr => self.parse_inst_load_store_rrr(inner),
            Rule::inst_load_store_rri => self.parse_inst_load_store_rri(inner),
            Rule::inst_arith_rrr => self.parse_inst_arith_rrr(inner),
            Rule::inst_arith_rri => self.parse_inst_arith_rri(inner),
            Rule::inst_neg_rrr => self.parse_inst_neg_rrr(inner),
            Rule::inst_neg_rri => self.parse_inst_neg_rri(inner),
            Rule::inst_bitwise_rrr => self.parse_inst_bitwise_rrr(inner),
            Rule::inst_bitwise_rri => self.parse_inst_bitwise_rri(inner),
            Rule::inst_bitfiddle_rrr => self.parse_inst_bitfiddle_rrr(inner),
            Rule::inst_bitfiddle_rri => self.parse_inst_bitfiddle_rri(inner),
            Rule::inst_shift_rrr => self.parse_inst_shift_rrr(inner),
            Rule::inst_shift_rri => self.parse_inst_shift_rri(inner),
            Rule::inst_branch => self.parse_inst_branch(inner),
            Rule::inst_jmp => self.parse_inst_jmp(inner),
            Rule::inst_halt => Ok(MMixInstruction::HALT),
            _ => Err(format!("Unhandled instruction: {:?}", inner.as_rule())),
        }
    }

    fn parse_inst_set(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = Self::parse_register(ops.next().unwrap())?;
        let val = Self::parse_number(ops.next().unwrap())?;
        Ok(MMixInstruction::SET(reg, val))
    }

    fn parse_inst_setl(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = Self::parse_register(ops.next().unwrap())?;
        let val = Self::parse_number(ops.next().unwrap())? as u16;
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
        let reg = Self::parse_register(ops.next().unwrap())?;
        let val = Self::parse_number(ops.next().unwrap())? as u16;
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
        let reg = Self::parse_register(ops.next().unwrap())?;
        let val = Self::parse_number(ops.next().unwrap())? as u16;
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
        let reg = Self::parse_register(ops.next().unwrap())?;
        let val = Self::parse_number(ops.next().unwrap())? as u16;
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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_register(ops.next().unwrap())?;
        Ok(MMixInstruction::INCL(x, y, z))
    }

    fn parse_inst_load_store_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_register(ops.next().unwrap())?;

        match mnem.as_str().to_uppercase().as_str() {
            "LDB" => Ok(MMixInstruction::LDB(x, y, z)),
            "LDBU" => Ok(MMixInstruction::LDBU(x, y, z)),
            "LDW" => Ok(MMixInstruction::LDW(x, y, z)),
            "LDWU" => Ok(MMixInstruction::LDWU(x, y, z)),
            "LDT" => Ok(MMixInstruction::LDT(x, y, z)),
            "LDTU" => Ok(MMixInstruction::LDTU(x, y, z)),
            "LDO" => Ok(MMixInstruction::LDO(x, y, z)),
            "LDOU" => Ok(MMixInstruction::LDOU(x, y, z)),
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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_number(ops.next().unwrap())? as u8;

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

    fn parse_inst_arith_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_register(ops.next().unwrap())?;

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
            "DIV" => Ok(MMixInstruction::DIV(x, y, z)),
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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_number(ops.next().unwrap())? as u8;

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
            "DIVI" => Ok(MMixInstruction::DIVI(x, y, z)),
            _ => Err(format!("Unknown arithmetic instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_neg_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let mnem = parts.next().unwrap();
        parts.next(); // skip operand wrapper
        let mut ops = parts;
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_number(ops.next().unwrap())? as u8;
        let z = Self::parse_register(ops.next().unwrap())?;

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
        parts.next(); // skip operand wrapper
        let mut ops = parts;
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_number(ops.next().unwrap())? as u8;
        let z = Self::parse_number(ops.next().unwrap())? as u8;

        match mnem.as_str().to_uppercase().as_str() {
            "NEGI" => Ok(MMixInstruction::NEGI(x, y, z)),
            "NEGUI" => Ok(MMixInstruction::NEGUI(x, y, z)),
            _ => Err(format!("Unknown NEG instruction: {}", mnem.as_str())),
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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_register(ops.next().unwrap())?;

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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_number(ops.next().unwrap())? as u8;

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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_register(ops.next().unwrap())?;

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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_number(ops.next().unwrap())? as u8;

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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_register(ops.next().unwrap())?;

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
        let x = Self::parse_register(ops.next().unwrap())?;
        let y = Self::parse_register(ops.next().unwrap())?;
        let z = Self::parse_number(ops.next().unwrap())? as u8;

        match mnem.as_str().to_uppercase().as_str() {
            "SLI" => Ok(MMixInstruction::SLI(x, y, z)),
            "SLUI" => Ok(MMixInstruction::SLUI(x, y, z)),
            "SRI" => Ok(MMixInstruction::SRI(x, y, z)),
            "SRUI" => Ok(MMixInstruction::SRUI(x, y, z)),
            _ => Err(format!("Unknown shift instruction: {}", mnem.as_str())),
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
        let x = Self::parse_register(ops.next().unwrap())?;
        let offset = Self::parse_number(ops.next().unwrap())? as u8;

        match mnem.as_str().to_uppercase().as_str() {
            "JE" => Ok(MMixInstruction::JE(x, offset)),
            "JNE" => Ok(MMixInstruction::JNE(x, offset)),
            "JL" => Ok(MMixInstruction::JL(x, offset)),
            "JG" => Ok(MMixInstruction::JG(x, offset)),
            _ => Err(format!("Unknown branch instruction: {}", mnem.as_str())),
        }
    }

    fn parse_inst_jmp(&self, pair: pest::iterators::Pair<Rule>) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let offset = Self::parse_number(ops.next().unwrap())? as u8;
        Ok(MMixInstruction::JMP(offset))
    }

    fn parse_data_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let directive = parts.next().unwrap();
        let value_pair = parts.next().unwrap();

        match directive.as_rule() {
            Rule::directive_byte => {
                let val = Self::parse_number(value_pair)? as u8;
                Ok(MMixInstruction::BYTE(val))
            }
            Rule::directive_wyde => {
                let val = Self::parse_number(value_pair)? as u16;
                Ok(MMixInstruction::WYDE(val))
            }
            Rule::directive_tetra => {
                let val = Self::parse_number(value_pair)? as u32;
                Ok(MMixInstruction::TETRA(val))
            }
            Rule::directive_octa => {
                let val = if value_pair.as_rule() == Rule::identifier {
                    self.labels.get(value_pair.as_str()).copied().unwrap_or(0)
                } else {
                    Self::parse_number(value_pair)?
                };
                Ok(MMixInstruction::OCTA(val))
            }
            _ => Err(format!("Unknown directive: {:?}", directive.as_rule())),
        }
    }

    fn parse_register(pair: pest::iterators::Pair<Rule>) -> Result<u8, String> {
        let text = pair.as_str();
        if !text.starts_with('$') {
            return Err(format!("Expected register, got: {}", text));
        }
        text[1..]
            .parse::<u8>()
            .map_err(|e| format!("Invalid register number: {}", e))
    }

    fn parse_number(pair: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let inner = pair.into_inner().next().unwrap();
        let text = inner.as_str();

        match inner.as_rule() {
            Rule::hex_number => u64::from_str_radix(&text[2..], 16)
                .map_err(|e| format!("Invalid hex number: {}", e)),
            Rule::dec_number => text
                .parse::<u64>()
                .map_err(|e| format!("Invalid decimal number: {}", e)),
            _ => Err(format!("Expected number, got: {:?}", inner.as_rule())),
        }
    }

    fn instruction_size(inst: &MMixInstruction) -> u64 {
        match inst {
            MMixInstruction::SET(_, _) => 16,
            MMixInstruction::BYTE(_) => 1,
            MMixInstruction::WYDE(_) => 2,
            MMixInstruction::TETRA(_) => 4,
            MMixInstruction::OCTA(_) => 8,
            _ => 4,
        }
    }

    /// Generate binary object code
    pub fn generate_object_code(&self) -> Vec<u8> {
        let mut code = Vec::new();

        for (_, instruction) in &self.instructions {
            match instruction {
                MMixInstruction::SET(x, value) => {
                    let b0 = (value >> 48) as u16;
                    let b1 = (value >> 32) as u16;
                    let b2 = (value >> 16) as u16;
                    let b3 = *value as u16;
                    code.extend_from_slice(&Self::encode_instruction(0xE1, *x, b0));
                    code.extend_from_slice(&Self::encode_instruction(0xE2, *x, b1));
                    code.extend_from_slice(&Self::encode_instruction(0xE3, *x, b2));
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
                    code.extend_from_slice(&[0xE6, *x, *y, *z]);
                }
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
                MMixInstruction::MUL(x, y, z) => {
                    code.extend_from_slice(&[0x18, *x, *y, *z]);
                }
                MMixInstruction::MULI(x, y, z) => {
                    code.extend_from_slice(&[0x19, *x, *y, *z]);
                }
                MMixInstruction::DIV(x, y, z) => {
                    code.extend_from_slice(&[0x1C, *x, *y, *z]);
                }
                MMixInstruction::DIVI(x, y, z) => {
                    code.extend_from_slice(&[0x1D, *x, *y, *z]);
                }
                // Bitwise operations (§10) - opcodes 0xC0-0xCF, 0xD8-0xD9
                MMixInstruction::OR(x, y, z) => {
                    code.extend_from_slice(&[0xC0, *x, *y, *z]);
                }
                MMixInstruction::ORI(x, y, z) => {
                    code.extend_from_slice(&[0xC1, *x, *y, *z]);
                }
                MMixInstruction::ORN(x, y, z) => {
                    code.extend_from_slice(&[0xC2, *x, *y, *z]);
                }
                MMixInstruction::ORNI(x, y, z) => {
                    code.extend_from_slice(&[0xC3, *x, *y, *z]);
                }
                MMixInstruction::NOR(x, y, z) => {
                    code.extend_from_slice(&[0xC4, *x, *y, *z]);
                }
                MMixInstruction::NORI(x, y, z) => {
                    code.extend_from_slice(&[0xC5, *x, *y, *z]);
                }
                MMixInstruction::XOR(x, y, z) => {
                    code.extend_from_slice(&[0xC6, *x, *y, *z]);
                }
                MMixInstruction::XORI(x, y, z) => {
                    code.extend_from_slice(&[0xC7, *x, *y, *z]);
                }
                MMixInstruction::AND(x, y, z) => {
                    code.extend_from_slice(&[0xC8, *x, *y, *z]);
                }
                MMixInstruction::ANDI(x, y, z) => {
                    code.extend_from_slice(&[0xC9, *x, *y, *z]);
                }
                MMixInstruction::ANDN(x, y, z) => {
                    code.extend_from_slice(&[0xCA, *x, *y, *z]);
                }
                MMixInstruction::ANDNI(x, y, z) => {
                    code.extend_from_slice(&[0xCB, *x, *y, *z]);
                }
                MMixInstruction::NAND(x, y, z) => {
                    code.extend_from_slice(&[0xCC, *x, *y, *z]);
                }
                MMixInstruction::NANDI(x, y, z) => {
                    code.extend_from_slice(&[0xCD, *x, *y, *z]);
                }
                MMixInstruction::NXOR(x, y, z) => {
                    code.extend_from_slice(&[0xCE, *x, *y, *z]);
                }
                MMixInstruction::NXORI(x, y, z) => {
                    code.extend_from_slice(&[0xCF, *x, *y, *z]);
                }
                MMixInstruction::MUX(x, y, z) => {
                    code.extend_from_slice(&[0xD8, *x, *y, *z]);
                }
                MMixInstruction::MUXI(x, y, z) => {
                    code.extend_from_slice(&[0xD9, *x, *y, *z]);
                }
                // Bit fiddling operations (§11-12)
                MMixInstruction::BDIF(x, y, z) => {
                    code.extend_from_slice(&[0xD0, *x, *y, *z]);
                }
                MMixInstruction::BDIFI(x, y, z) => {
                    code.extend_from_slice(&[0xD1, *x, *y, *z]);
                }
                MMixInstruction::WDIF(x, y, z) => {
                    code.extend_from_slice(&[0xD2, *x, *y, *z]);
                }
                MMixInstruction::WDIFI(x, y, z) => {
                    code.extend_from_slice(&[0xD3, *x, *y, *z]);
                }
                MMixInstruction::TDIF(x, y, z) => {
                    code.extend_from_slice(&[0xD4, *x, *y, *z]);
                }
                MMixInstruction::TDIFI(x, y, z) => {
                    code.extend_from_slice(&[0xD5, *x, *y, *z]);
                }
                MMixInstruction::ODIF(x, y, z) => {
                    code.extend_from_slice(&[0xD6, *x, *y, *z]);
                }
                MMixInstruction::ODIFI(x, y, z) => {
                    code.extend_from_slice(&[0xD7, *x, *y, *z]);
                }
                MMixInstruction::SADD(x, y, z) => {
                    code.extend_from_slice(&[0xDA, *x, *y, *z]);
                }
                MMixInstruction::SADDI(x, y, z) => {
                    code.extend_from_slice(&[0xDB, *x, *y, *z]);
                }
                MMixInstruction::MOR(x, y, z) => {
                    code.extend_from_slice(&[0xDC, *x, *y, *z]);
                }
                MMixInstruction::MORI(x, y, z) => {
                    code.extend_from_slice(&[0xDD, *x, *y, *z]);
                }
                MMixInstruction::MXOR(x, y, z) => {
                    code.extend_from_slice(&[0xDE, *x, *y, *z]);
                }
                MMixInstruction::MXORI(x, y, z) => {
                    code.extend_from_slice(&[0xDF, *x, *y, *z]);
                }
                // Shift instructions (§14)
                MMixInstruction::SL(x, y, z) => {
                    code.extend_from_slice(&[0x38, *x, *y, *z]);
                }
                MMixInstruction::SLI(x, y, z) => {
                    code.extend_from_slice(&[0x39, *x, *y, *z]);
                }
                MMixInstruction::SLU(x, y, z) => {
                    code.extend_from_slice(&[0x3A, *x, *y, *z]);
                }
                MMixInstruction::SLUI(x, y, z) => {
                    code.extend_from_slice(&[0x3B, *x, *y, *z]);
                }
                MMixInstruction::SR(x, y, z) => {
                    code.extend_from_slice(&[0x3C, *x, *y, *z]);
                }
                MMixInstruction::SRI(x, y, z) => {
                    code.extend_from_slice(&[0x3D, *x, *y, *z]);
                }
                MMixInstruction::SRU(x, y, z) => {
                    code.extend_from_slice(&[0x3E, *x, *y, *z]);
                }
                MMixInstruction::SRUI(x, y, z) => {
                    code.extend_from_slice(&[0x3F, *x, *y, *z]);
                }
                MMixInstruction::JMP(offset) => {
                    code.extend_from_slice(&[0xF0, 0, 0, *offset]);
                }
                MMixInstruction::JE(x, offset) => {
                    code.extend_from_slice(&[0x2E, *x, 0, *offset]);
                }
                MMixInstruction::JNE(x, offset) => {
                    code.extend_from_slice(&[0x2F, *x, 0, *offset]);
                }
                MMixInstruction::JL(x, offset) => {
                    code.extend_from_slice(&[0x32, *x, 0, *offset]);
                }
                MMixInstruction::JG(x, offset) => {
                    code.extend_from_slice(&[0x34, *x, 0, *offset]);
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
                _ => {
                    eprintln!(
                        "Warning: Unhandled instruction in code generation: {:?}",
                        instruction
                    );
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
}

// Keep all the existing tests - they should work unchanged
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_label() {
        let mut asm = MMixAssembler::new("LOOP: HALT");
        asm.parse();
        assert_eq!(asm.labels.get("LOOP"), Some(&0));
        assert_eq!(asm.instructions.len(), 1);
    }

    #[test]
    fn test_parse_octa_directive() {
        let mut asm = MMixAssembler::new("OCTA 0x123456789ABCDEF0");
        asm.parse();
        assert_eq!(asm.instructions.len(), 1);
        assert_eq!(
            asm.instructions[0].1,
            MMixInstruction::OCTA(0x123456789ABCDEF0)
        );
    }

    #[test]
    fn test_parse_node_structure() {
        let mut asm = MMixAssembler::new("NODE: OCTA 42\n      OCTA 0");
        asm.parse();
        assert_eq!(asm.labels.get("NODE"), Some(&0));
        assert_eq!(asm.instructions.len(), 2);
    }

    #[test]
    fn test_parse_set() {
        let mut asm = MMixAssembler::new("SET $2, 10");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SET(2, 10));
    }

    // Bitwise operation tests
    #[test]
    fn test_parse_and() {
        let mut asm = MMixAssembler::new("AND $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::AND(1, 2, 3));
    }

    #[test]
    fn test_parse_andi() {
        let mut asm = MMixAssembler::new("ANDI $1, $2, 0xFF");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 0xFF));
    }

    #[test]
    fn test_parse_or() {
        let mut asm = MMixAssembler::new("OR $10, $20, $30");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::OR(10, 20, 30));
    }

    #[test]
    fn test_parse_xor() {
        let mut asm = MMixAssembler::new("XOR $5, $6, $7");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::XOR(5, 6, 7));
    }

    #[test]
    fn test_parse_andn() {
        let mut asm = MMixAssembler::new("ANDN $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ANDN(1, 2, 3));
    }

    #[test]
    fn test_parse_nand() {
        let mut asm = MMixAssembler::new("NAND $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::NAND(1, 2, 3));
    }

    #[test]
    fn test_parse_nor() {
        let mut asm = MMixAssembler::new("NOR $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::NOR(1, 2, 3));
    }

    #[test]
    fn test_parse_nxor() {
        let mut asm = MMixAssembler::new("NXOR $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::NXOR(1, 2, 3));
    }

    #[test]
    fn test_parse_mux() {
        let mut asm = MMixAssembler::new("MUX $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MUX(1, 2, 3));
    }

    // Bit fiddling operations tests (§11-12)
    #[test]
    fn test_parse_bdif() {
        let mut asm = MMixAssembler::new("BDIF $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BDIF(1, 2, 3));
    }

    #[test]
    fn test_parse_bdifi() {
        let mut asm = MMixAssembler::new("BDIFI $1, $2, 0x10");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::BDIFI(1, 2, 0x10));
    }

    #[test]
    fn test_parse_wdif() {
        let mut asm = MMixAssembler::new("WDIF $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::WDIF(1, 2, 3));
    }

    #[test]
    fn test_parse_wdifi() {
        let mut asm = MMixAssembler::new("WDIFI $1, $2, 100");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::WDIFI(1, 2, 100));
    }

    #[test]
    fn test_parse_tdif() {
        let mut asm = MMixAssembler::new("TDIF $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::TDIF(1, 2, 3));
    }

    #[test]
    fn test_parse_tdifi() {
        let mut asm = MMixAssembler::new("TDIFI $1, $2, 50");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::TDIFI(1, 2, 50));
    }

    #[test]
    fn test_parse_odif() {
        let mut asm = MMixAssembler::new("ODIF $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ODIF(1, 2, 3));
    }

    #[test]
    fn test_parse_odifi() {
        let mut asm = MMixAssembler::new("ODIFI $1, $2, 255");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::ODIFI(1, 2, 255));
    }

    #[test]
    fn test_parse_sadd() {
        let mut asm = MMixAssembler::new("SADD $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SADD(1, 2, 3));
    }

    #[test]
    fn test_parse_saddi() {
        let mut asm = MMixAssembler::new("SADDI $1, $2, 0");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SADDI(1, 2, 0));
    }

    #[test]
    fn test_parse_mor() {
        let mut asm = MMixAssembler::new("MOR $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MOR(1, 2, 3));
    }

    #[test]
    fn test_parse_mori() {
        let mut asm = MMixAssembler::new("MORI $1, $2, 128");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MORI(1, 2, 128));
    }

    #[test]
    fn test_parse_mxor() {
        let mut asm = MMixAssembler::new("MXOR $1, $2, $3");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MXOR(1, 2, 3));
    }

    #[test]
    fn test_parse_mxori() {
        let mut asm = MMixAssembler::new("MXORI $1, $2, 64");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::MXORI(1, 2, 64));
    }

    // Shift instruction parsing tests (§14)
    #[test]
    fn test_parse_sl() {
        let mut asm = MMixAssembler::new("SL $3, $1, $2");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SL(3, 1, 2));
    }

    #[test]
    fn test_parse_sli() {
        let mut asm = MMixAssembler::new("SLI $3, $1, 8");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SLI(3, 1, 8));
    }

    #[test]
    fn test_parse_slu() {
        let mut asm = MMixAssembler::new("SLU $10, $20, $30");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SLU(10, 20, 30));
    }

    #[test]
    fn test_parse_slui() {
        let mut asm = MMixAssembler::new("SLUI $1, $2, 16");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SLUI(1, 2, 16));
    }

    #[test]
    fn test_parse_sr() {
        let mut asm = MMixAssembler::new("SR $5, $6, $7");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SR(5, 6, 7));
    }

    #[test]
    fn test_parse_sri() {
        let mut asm = MMixAssembler::new("SRI $3, $1, 4");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRI(3, 1, 4));
    }

    #[test]
    fn test_parse_sru() {
        let mut asm = MMixAssembler::new("SRU $8, $9, $10");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRU(8, 9, 10));
    }

    #[test]
    fn test_parse_srui() {
        let mut asm = MMixAssembler::new("SRUI $3, $1, 1");
        asm.parse();
        assert_eq!(asm.instructions[0].1, MMixInstruction::SRUI(3, 1, 1));
    }
}
