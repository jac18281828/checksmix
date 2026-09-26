//! The floating-point instruction family.
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
    pub(super) fn parse_inst_float_rrr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let y = self.parse_register(ops.required()?)?;
        let z = self.parse_register(ops.required()?)?;

        match mnem.as_str().to_uppercase().as_str() {
            "FCMP" => Ok(MMixInstruction::FCMP(x, y, z)),
            "FUN" => Ok(MMixInstruction::FUN(x, y, z)),
            "FEQL" => Ok(MMixInstruction::FEQL(x, y, z)),
            "FCMPE" => Ok(MMixInstruction::FCMPE(x, y, z)),
            "FUNE" => Ok(MMixInstruction::FUNE(x, y, z)),
            "FEQLE" => Ok(MMixInstruction::FEQLE(x, y, z)),
            "FADD" => Ok(MMixInstruction::FADD(x, y, z)),
            "FSUB" => Ok(MMixInstruction::FSUB(x, y, z)),
            "FMUL" => Ok(MMixInstruction::FMUL(x, y, z)),
            "FDIV" => Ok(MMixInstruction::FDIV(x, y, z)),
            "FREM" => Ok(MMixInstruction::FREM(x, y, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FIX`/`FIXU`/`FSQRT`/`FINT`, 3-operand form: `Y` is a rounding-mode
    /// value (`0..=4`, `ROUND_CURRENT`/`ROUND_OFF`/`ROUND_UP`/`ROUND_DOWN`/
    /// `ROUND_NEAR`). Only its byte field is checked here, the same as
    /// `NEG`'s own value-typed `Y`; `Y > 4` halts at run time.
    pub(super) fn parse_inst_float_round_rrz(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let mut ops = Children::of(parts.required()?);
        let x = self.parse_register(ops.required()?)?;
        let y = self.imm_byte(ops.required()?, mnem.as_str())?;
        let z = self.parse_register(ops.required()?)?;

        match mnem.as_str().to_uppercase().as_str() {
            "FIX" => Ok(MMixInstruction::FIX(x, y, z)),
            "FIXU" => Ok(MMixInstruction::FIXU(x, y, z)),
            "FSQRT" => Ok(MMixInstruction::FSQRT(x, y, z)),
            "FINT" => Ok(MMixInstruction::FINT(x, y, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FIX`/`FIXU`/`FSQRT`/`FINT`, 2-operand form: `Y` is implicitly 0
    /// (`ROUND_CURRENT` — no override, use rA's mode).
    pub(super) fn parse_inst_float_round_rr(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let z = self.parse_register(ops.required()?)?;

        match mnem.as_str().to_uppercase().as_str() {
            "FIX" => Ok(MMixInstruction::FIX(x, 0, z)),
            "FIXU" => Ok(MMixInstruction::FIXU(x, 0, z)),
            "FSQRT" => Ok(MMixInstruction::FSQRT(x, 0, z)),
            "FINT" => Ok(MMixInstruction::FINT(x, 0, z)),
            _ => Err(format!(
                "Unknown floating point instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FLOT`/`FLOTU`/`SFLOT`/`SFLOTU`, 3-operand form: `Y` forces a
    /// rounding mode, `Z` auto-selects register or immediate.
    pub(super) fn parse_inst_flot_round(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?.as_str().to_uppercase();
        let mut ops = Children::of(parts.required()?);
        let x = self.parse_register(ops.required()?)?;
        let y = self.imm_byte(ops.required()?, &mnem)?;
        let z = self.lower_z_operand(ops.required()?, &mnem)?;

        match (mnem.as_str(), z) {
            ("FLOT", ZForm::Reg(z)) => Ok(MMixInstruction::FLOT(x, y, z)),
            ("FLOT", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTI(x, y, z)),
            ("FLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::FLOTU(x, y, z)),
            ("FLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTUI(x, y, z)),
            ("SFLOT", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOT(x, y, z)),
            ("SFLOT", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTI(x, y, z)),
            ("SFLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOTU(x, y, z)),
            ("SFLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTUI(x, y, z)),
            _ => Err(format!("Unknown float conversion instruction: {}", mnem)),
        }
    }

    /// `FLOT`/`FLOTU`/`SFLOT`/`SFLOTU`, 2-operand form: `Y` implicitly 0.
    pub(super) fn parse_inst_flot_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?.as_str().to_uppercase();
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let z = self.lower_z_operand(ops.required()?, &mnem)?;

        match (mnem.as_str(), z) {
            ("FLOT", ZForm::Reg(z)) => Ok(MMixInstruction::FLOT(x, 0, z)),
            ("FLOT", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTI(x, 0, z)),
            ("FLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::FLOTU(x, 0, z)),
            ("FLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::FLOTUI(x, 0, z)),
            ("SFLOT", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOT(x, 0, z)),
            ("SFLOT", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTI(x, 0, z)),
            ("SFLOTU", ZForm::Reg(z)) => Ok(MMixInstruction::SFLOTU(x, 0, z)),
            ("SFLOTU", ZForm::Imm(z)) => Ok(MMixInstruction::SFLOTUI(x, 0, z)),
            _ => Err(format!("Unknown float conversion instruction: {}", mnem)),
        }
    }

    /// `FLOTI`/`FLOTUI`/`SFLOTI`/`SFLOTUI`, 3-operand form: `Y` forces a
    /// rounding mode, `Z` stays immediate-only.
    pub(super) fn parse_inst_float_round_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let mut ops = Children::of(parts.required()?);
        let x = self.parse_register(ops.required()?)?;
        let y = self.imm_byte(ops.required()?, mnem.as_str())?;
        let z = self.imm_byte(ops.required()?, mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FLOTI" => Ok(MMixInstruction::FLOTI(x, y, z)),
            "FLOTUI" => Ok(MMixInstruction::FLOTUI(x, y, z)),
            "SFLOTI" => Ok(MMixInstruction::SFLOTI(x, y, z)),
            "SFLOTUI" => Ok(MMixInstruction::SFLOTUI(x, y, z)),
            _ => Err(format!(
                "Unknown floating point immediate instruction: {}",
                mnem.as_str()
            )),
        }
    }

    /// `FLOTI`/`FLOTUI`/`SFLOTI`/`SFLOTUI`, 2-operand form: `Y` implicitly 0.
    pub(super) fn parse_inst_float_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let z = self.imm_byte(ops.required()?, mnem.as_str())?;

        match mnem.as_str().to_uppercase().as_str() {
            "FLOTI" => Ok(MMixInstruction::FLOTI(x, 0, z)),
            "FLOTUI" => Ok(MMixInstruction::FLOTUI(x, 0, z)),
            "SFLOTI" => Ok(MMixInstruction::SFLOTI(x, 0, z)),
            "SFLOTUI" => Ok(MMixInstruction::SFLOTUI(x, 0, z)),
            _ => Err(format!(
                "Unknown floating point immediate instruction: {}",
                mnem.as_str()
            )),
        }
    }
}
