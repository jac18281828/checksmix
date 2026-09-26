//! The TRAP/GET/PUT/SAVE/UNSAVE/RESUME/TRIP/SWYM/SYNC system family.

use super::super::MMixAssembler;
use super::super::Rule;
use super::super::operands::ZForm;
use super::super::tree::Children;
use super::MMixInstruction;

impl MMixAssembler {
    /// `TRAP`/`TRIP`/`SWYM`'s shared operand shapes: three fields (each a
    /// register or a pure byte), two fields (X alone that way, YZ a pure
    /// 16-bit value split into Y and Z), one field (a pure 24-bit value
    /// split into X, Y and Z), or none (every field 0).
    fn parse_trap_family_operands(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<(u8, u8, u8), String> {
        let mut parts = Children::of(pair);
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
            _ => Err(parts.unexpected(&operands)),
        }
    }

    pub(super) fn parse_inst_trap(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (x, y, z) = self.parse_trap_family_operands(pair, "TRAP")?;
        Ok(MMixInstruction::TRAP(x, y, z))
    }

    pub(super) fn parse_inst_get(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = pair.into_inner();
        let _mnem = parts.next();
        let mut ops = parts.next().unwrap().into_inner();
        let x = self.parse_register(ops.next().unwrap())?;
        let z = self.special_register(ops.next().unwrap(), "GET")?;
        Ok(MMixInstruction::GET(x, z))
    }

    pub(super) fn parse_inst_put_auto(
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

    pub(super) fn parse_inst_puti(
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

    pub(super) fn parse_inst_save(
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
    pub(super) fn parse_inst_unsave(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
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
            _ => Err(parts.unexpected(&operands)),
        }
    }

    /// Bare `RESUME`: XYZ=0. `RESUME` takes a 24-bit `XYZ`, the MMIXAL
    /// definition's spelling; all three bytes reach the encoding.
    pub(super) fn parse_inst_resume(
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

    pub(super) fn parse_inst_trip(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (x, y, z) = self.parse_trap_family_operands(pair, "TRIP")?;
        Ok(MMixInstruction::TRIP(x, y, z))
    }

    pub(super) fn parse_inst_swym(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (x, y, z) = self.parse_trap_family_operands(pair, "SWYM")?;
        Ok(MMixInstruction::SWYM(x, y, z))
    }

    /// Bare `SYNC`: XYZ=0. `SYNC` takes a 24-bit `XYZ`, the MMIXAL
    /// definition's spelling; all three bytes reach the encoding, though
    /// the machine halts on any code above 7.
    pub(super) fn parse_inst_sync(
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
}
