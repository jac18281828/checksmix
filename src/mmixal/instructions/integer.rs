//! The arithmetic, negate, bitwise, bit-fiddling, shift, conditional-set and zero-or-set families.

use super::super::MMixAssembler;
use super::super::Rule;
use super::super::operands::ZForm;
use super::super::tree::Children;
use super::MMixInstruction;

impl MMixAssembler {
    pub(super) fn parse_inst_arith_auto(
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

    pub(super) fn parse_inst_arith_rri(
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

    pub(super) fn parse_inst_neg_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
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
            _ => return Err(parts.unexpected(&operands)),
        };

        match (mnem.as_str(), z) {
            ("NEG", ZForm::Reg(z)) => Ok(MMixInstruction::NEG(x, y, z)),
            ("NEG", ZForm::Imm(z)) => Ok(MMixInstruction::NEGI(x, y, z)),
            ("NEGU", ZForm::Reg(z)) => Ok(MMixInstruction::NEGU(x, y, z)),
            ("NEGU", ZForm::Imm(z)) => Ok(MMixInstruction::NEGUI(x, y, z)),
            _ => Err(format!("Unknown NEG instruction: {}", mnem)),
        }
    }

    pub(super) fn parse_inst_neg_rri(
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

    pub(super) fn parse_inst_bitwise_auto(
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

    pub(super) fn parse_inst_bitwise_rri(
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

    pub(super) fn parse_inst_bitfiddle_auto(
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

    pub(super) fn parse_inst_bitfiddle_rri(
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

    pub(super) fn parse_inst_shift_auto(
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

    pub(super) fn parse_inst_shift_rri(
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

    pub(super) fn parse_inst_conditional_set_auto(
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

    pub(super) fn parse_inst_conditional_set_rri(
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

    pub(super) fn parse_inst_zero_or_set_auto(
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

    pub(super) fn parse_inst_zero_or_set_rri(
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
}
