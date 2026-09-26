//! The TRAP/GET/PUT/SAVE/UNSAVE/RESUME/TRIP/SWYM/SYNC system family.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

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
                let mut ops = Children::of(operands);
                let x = self.parse_reg_or_byte(ops.required()?, mnem)?;
                let y = self.parse_reg_or_byte(ops.required()?, mnem)?;
                let z = self.parse_reg_or_byte(ops.required()?, mnem)?;
                Ok((x, y, z))
            }
            Rule::operand_list_two => {
                let mut ops = Children::of(operands);
                let x = self.parse_reg_or_byte(ops.required()?, mnem)?;
                let yz = self.imm_wyde(ops.required()?, mnem)?;
                let (y, z) = Self::split_hi_lo_byte(yz);
                Ok((x, y, z))
            }
            Rule::operand_list_one => {
                let mut ops = Children::of(operands);
                let xyz = self.imm_three_bytes(ops.required()?, mnem)?;
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
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let mut ops = Children::of(parts.required()?);
        let x = self.parse_register(ops.required()?)?;
        let z = self.special_register(ops.required()?, "GET")?;
        Ok(MMixInstruction::GET(x, z))
    }

    pub(super) fn parse_inst_put_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let mut ops = Children::of(parts.required()?);
        let x = self.special_register(ops.required()?, "PUT")?;
        match self.lower_z_operand(ops.required()?, "PUT")? {
            ZForm::Reg(z) => Ok(MMixInstruction::PUT(x, z)),
            ZForm::Imm(z) => Ok(MMixInstruction::PUTI(x, z)),
        }
    }

    pub(super) fn parse_inst_puti(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let mut ops = Children::of(parts.required()?);
        let x = self.special_register(ops.required()?, "PUTI")?;
        let z = self.imm_byte(ops.required()?, "PUTI")?;
        Ok(MMixInstruction::PUTI(x, z))
    }

    pub(super) fn parse_inst_save(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let mut ops = Children::of(parts.required()?);
        let x = self.parse_register(ops.required()?)?;
        let z = self.imm_byte(ops.required()?, "SAVE")?;
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
        let operands = parts.required()?;
        match operands.as_rule() {
            Rule::operand_list_two => {
                let mut ops = Children::of(operands);
                let x = self.imm_byte(ops.required()?, "UNSAVE")?;
                let z = self.parse_register(ops.required()?)?;
                Ok(MMixInstruction::UNSAVE(x, z))
            }
            Rule::operand_list_one => {
                let mut ops = Children::of(operands);
                let z = self.parse_register(ops.required()?)?;
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
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let xyz = match parts.next() {
            Some(op) => self.imm_three_bytes(Children::of(op).required()?, "RESUME")?,
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
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let xyz = match parts.next() {
            Some(op) => self.imm_three_bytes(Children::of(op).required()?, "SYNC")?,
            None => 0,
        };
        Ok(MMixInstruction::SYNC(xyz))
    }
}
