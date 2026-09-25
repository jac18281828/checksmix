//! The SET/SETI/SETL/SETH/SETMH/SETML/INC*/OR*/ANDN* wyde-immediate family.

use super::super::MMixAssembler;
use super::super::Rule;
use super::super::expressions::ExprValue;
use super::MMixInstruction;

impl MMixAssembler {
    pub(super) fn parse_inst_set(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
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

    pub(super) fn parse_inst_seti(
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

    pub(super) fn parse_inst_setl(
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

    pub(super) fn parse_inst_seth(
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

    pub(super) fn parse_inst_setmh(
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

    pub(super) fn parse_inst_setml(
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

    pub(super) fn parse_inst_incl(
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

    pub(super) fn parse_inst_inch(
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

    pub(super) fn parse_inst_incmh(
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

    pub(super) fn parse_inst_incml(
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

    pub(super) fn parse_inst_orh(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ORH")?;
        Ok(MMixInstruction::ORH(reg, val))
    }

    pub(super) fn parse_inst_ormh(
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

    pub(super) fn parse_inst_orml(
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

    pub(super) fn parse_inst_orl(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let operands = parts.next().unwrap();
        let mut ops = operands.into_inner();
        let reg = self.parse_register(ops.next().unwrap())?;
        let val = self.imm_wyde(ops.next().unwrap(), "ORL")?;
        Ok(MMixInstruction::ORL(reg, val))
    }

    pub(super) fn parse_inst_andnh(
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

    pub(super) fn parse_inst_andnmh(
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

    pub(super) fn parse_inst_andnml(
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

    pub(super) fn parse_inst_andnl(
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
}
