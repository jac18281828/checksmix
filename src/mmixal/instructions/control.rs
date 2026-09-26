//! The branch, jump, GETA/GETAB and PUSHJ/PUSHGO/POP/GO control-flow family.
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
use tracing::debug;

/// A PC-relative displacement in the instruction encoding: the forward opcode
/// carries `field` directly, the backward opcode carries `2^bits - magnitude`.
#[derive(Debug, Clone, Copy)]
struct RelativeField {
    backward: bool,
    field: u32,
}

impl MMixAssembler {
    /// Resolve a PC-relative target into the field a `bits`-wide operand
    /// carries. Forward reaches `0..=2^bits - 1` tetras and backward
    /// `1..=2^bits`, so a displacement of zero takes the forward opcode. A
    /// mnemonic spelled with a trailing `B` asserts a backward target and
    /// rejects a forward one; every other mnemonic takes its direction from
    /// the sign of the displacement. `absolute_alternative` names an absolute
    /// instruction to suggest when the target is unreachable, or is empty.
    fn relative_field(
        &self,
        mnemonic: &str,
        target: u64,
        bits: u32,
        (line, col): (usize, usize),
        absolute_alternative: &str,
    ) -> Result<RelativeField, String> {
        let delta = target.wrapping_sub(self.current_addr) as i64;
        if delta % 4 != 0 {
            return Err(format!(
                "{}:{}:{}: {} target 0x{:X} is not 4-byte aligned relative to the current instruction (byte delta {})",
                self.current_filename, line, col, mnemonic, target, delta
            ));
        }
        let tetras = delta / 4;
        if let Some(forward_sibling) = mnemonic.strip_suffix('B')
            && tetras >= 0
        {
            return Err(format!(
                "{}:{}:{}: {} target 0x{:X} is not behind the current instruction ({} only encodes backward addresses; use {} instead)",
                self.current_filename, line, col, mnemonic, target, mnemonic, forward_sibling
            ));
        }
        let span = 1i64 << bits;
        let out_of_range = |direction: &str, reach_tetras: i64| {
            format!(
                "{}:{}:{}: {} target 0x{:X} is out of range: {} byte delta {} exceeds {}'s {}-byte {} reach{}",
                self.current_filename,
                line,
                col,
                mnemonic,
                target,
                direction,
                delta.unsigned_abs(),
                mnemonic,
                reach_tetras * 4,
                direction,
                absolute_alternative
            )
        };
        if tetras >= 0 {
            if tetras >= span {
                return Err(out_of_range("forward", span - 1));
            }
            return Ok(RelativeField {
                backward: false,
                field: tetras as u32,
            });
        }
        if -tetras > span {
            return Err(out_of_range("backward", span));
        }
        Ok(RelativeField {
            backward: true,
            field: (span + tetras) as u32,
        })
    }

    pub(super) fn parse_inst_branch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let target = self.parse_number(ops.required()?)?;

        // Each mnemonic names the variant to emit for a forward target and
        // the one for a backward target.
        type Branch = fn(u8, u16) -> MMixInstruction;
        let mnem = mnem.as_str().to_uppercase();
        let (forward, backward): (Branch, Branch) = match mnem.as_str() {
            "BN" | "BNB" => (MMixInstruction::BN, MMixInstruction::BNB),
            "BZ" | "BZB" => (MMixInstruction::BZ, MMixInstruction::BZB),
            "BP" | "BPB" => (MMixInstruction::BP, MMixInstruction::BPB),
            "BOD" | "BODB" => (MMixInstruction::BOD, MMixInstruction::BODB),
            "BNN" | "BNNB" => (MMixInstruction::BNN, MMixInstruction::BNNB),
            "BNZ" | "BNZB" => (MMixInstruction::BNZ, MMixInstruction::BNZB),
            "BNP" | "BNPB" => (MMixInstruction::BNP, MMixInstruction::BNPB),
            "BEV" | "BEVB" => (MMixInstruction::BEV, MMixInstruction::BEVB),
            _ => return Err(format!("Unknown branch instruction: {mnem}")),
        };

        let resolved = self.relative_field(&mnem, target, 16, (line, col), "")?;
        let field = resolved.field as u16;
        Ok(if resolved.backward {
            backward(x, field)
        } else {
            forward(x, field)
        })
    }

    pub(super) fn parse_inst_jmp(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let mnem = parts.next();
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let target = self.parse_number(ops.required()?)?;
        let mnem = mnem.map_or_else(|| "JMP".to_string(), |m| m.as_str().to_uppercase());
        let resolved = self.relative_field(&mnem, target, 24, (line, col), " (use GO instead)")?;
        Ok(if resolved.backward {
            MMixInstruction::JMPB(resolved.field)
        } else {
            MMixInstruction::JMP(resolved.field)
        })
    }

    pub(super) fn parse_inst_pbranch(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let target = self.parse_number(ops.required()?)?;

        type ProbableBranch = fn(u8, u8, u8) -> MMixInstruction;
        let mnem = mnem.as_str().to_uppercase();
        let (forward, backward): (ProbableBranch, ProbableBranch) = match mnem.as_str() {
            "PBN" | "PBNB" => (MMixInstruction::PBN, MMixInstruction::PBNB),
            "PBZ" | "PBZB" => (MMixInstruction::PBZ, MMixInstruction::PBZB),
            "PBP" | "PBPB" => (MMixInstruction::PBP, MMixInstruction::PBPB),
            "PBOD" | "PBODB" => (MMixInstruction::PBOD, MMixInstruction::PBODB),
            "PBNN" | "PBNNB" => (MMixInstruction::PBNN, MMixInstruction::PBNNB),
            "PBNZ" | "PBNZB" => (MMixInstruction::PBNZ, MMixInstruction::PBNZB),
            "PBNP" | "PBNPB" => (MMixInstruction::PBNP, MMixInstruction::PBNPB),
            "PBEV" | "PBEVB" => (MMixInstruction::PBEV, MMixInstruction::PBEVB),
            _ => return Err(format!("Unknown probable branch instruction: {mnem}")),
        };

        let resolved = self.relative_field(&mnem, target, 16, (line, col), "")?;
        let y = (resolved.field >> 8) as u8;
        let z = (resolved.field & 0xFF) as u8;
        Ok(if resolved.backward {
            backward(x, y, z)
        } else {
            forward(x, y, z)
        })
    }

    pub(super) fn parse_inst_geta(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let _mnem = parts.next(); // Skip mnemonic
        let operand = parts.required()?; // Get operand_list_two

        let mut operand_parts = Children::of(operand);
        let reg_pair = operand_parts.required()?;
        let addr_pair = operand_parts.required()?;

        let x = self.parse_register(reg_pair)?;
        let addr = self.parse_number(addr_pair)?;

        debug!(
            "GETA: current_addr=0x{:X}, target_addr=0x{:X}",
            self.current_addr, addr
        );

        // GETA reaches 65535 tetras forward; a backward target takes GETAB.
        let resolved = self.relative_field(
            "GETA",
            addr,
            16,
            (line, col),
            " (use SETI, or LDA against a GREG base, for longer-range addresses)",
        )?;
        let y = (resolved.field >> 8) as u8;
        let z = (resolved.field & 0xFF) as u8;

        debug!(
            "GETA: backward={}, field=0x{:X}, y=0x{:X}, z=0x{:X}",
            resolved.backward, resolved.field, y, z
        );

        Ok(if resolved.backward {
            MMixInstruction::GETAB(x, y, z)
        } else {
            MMixInstruction::GETA(x, y, z)
        })
    }

    pub(super) fn parse_inst_getab(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let _mnem = parts.next(); // Skip mnemonic
        let operand = parts.required()?; // Get operand_list_two

        let mut operand_parts = Children::of(operand);
        let reg_pair = operand_parts.required()?;
        let addr_pair = operand_parts.required()?;

        let x = self.parse_register(reg_pair)?;
        let addr = self.parse_number(addr_pair)?;

        let resolved = self.relative_field(
            "GETAB",
            addr,
            16,
            (line, col),
            " (use SETI, or LDA against a GREG base, for longer-range addresses)",
        )?;
        let y = (resolved.field >> 8) as u8;
        let z = (resolved.field & 0xFF) as u8;

        Ok(MMixInstruction::GETAB(x, y, z))
    }

    // PUSHJ/PUSHJB: format (reg-or-byte, imm) where imm is 16-bit offset
    pub(super) fn parse_inst_pushj(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let operand = parts.required()?;
        let mut ops = Children::of(operand);
        let x = self.parse_reg_or_byte(ops.required()?, "PUSHJ")?;
        let addr = self.parse_number(ops.required()?)?;
        let resolved = self.relative_field("PUSHJ", addr, 16, (line, col), "")?;
        let (y, z) = Self::split_hi_lo_byte(resolved.field as u16);
        Ok(if resolved.backward {
            MMixInstruction::PUSHJB(x, y, z)
        } else {
            MMixInstruction::PUSHJ(x, y, z)
        })
    }

    pub(super) fn parse_inst_pushjb(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let (line, col) = pair.line_col();
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let operand = parts.required()?;
        let mut ops = Children::of(operand);
        let x = self.parse_reg_or_byte(ops.required()?, "PUSHJB")?;
        let addr = self.parse_number(ops.required()?)?;
        let resolved = self.relative_field("PUSHJB", addr, 16, (line, col), "")?;
        let (y, z) = Self::split_hi_lo_byte(resolved.field as u16);
        Ok(MMixInstruction::PUSHJB(x, y, z))
    }

    /// `GO`'s X stays a register; `PUSHGO`'s X is also a pure byte, the
    /// same bytes as the register spelling.
    pub(super) fn parse_inst_go_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?.as_str().to_uppercase();
        let operands = parts.required()?;
        let is_three = operands.as_rule() == Rule::operand_list_three;
        let mut ops = Children::of(operands);
        let x_pair = ops.required()?;
        let x = if mnem == "PUSHGO" {
            self.parse_reg_or_byte(x_pair, &mnem)?
        } else {
            self.parse_register(x_pair)?
        };
        let (y, z) = if is_three {
            let y = self.parse_register(ops.required()?)?;
            let z = self.lower_z_operand(ops.required()?, &mnem)?;
            (y, z)
        } else {
            // The two-operand memory form: the second operand is a register
            // (an offset of zero) or a base address resolved against a
            // preceding GREG.
            let (y, offset) = self.resolve_memory_base_operand(ops.required()?)?;
            (y, ZForm::Imm(offset))
        };

        match (mnem.as_str(), z) {
            ("GO", ZForm::Reg(z)) => Ok(MMixInstruction::GO(x, y, z)),
            ("GO", ZForm::Imm(z)) => Ok(MMixInstruction::GOI(x, y, z)),
            ("PUSHGO", ZForm::Reg(z)) => Ok(MMixInstruction::PUSHGO(x, y, z)),
            ("PUSHGO", ZForm::Imm(z)) => Ok(MMixInstruction::PUSHGOI(x, y, z)),
            _ => Err(format!("Unknown GO instruction: {}", mnem)),
        }
    }

    pub(super) fn parse_inst_pushgo_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PUSHGOI", MMixInstruction::PUSHGOI)
    }

    /// `POP p,yz`: X=p, YZ=yz. `POP xyz`: XYZ=xyz, so `POP 1` is
    /// `POP(0,0,1)`. Bare `POP`: every field 0.
    pub(super) fn parse_inst_pop(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let Some(operands) = parts.next() else {
            return Ok(MMixInstruction::POP(0, 0, 0));
        };
        match operands.as_rule() {
            Rule::operand_list_two => {
                let mut ops = Children::of(operands);
                let x = self.imm_byte(ops.required()?, "POP")?;
                let yz = self.imm_wyde(ops.required()?, "POP")?;
                let (y, z) = Self::split_hi_lo_byte(yz);
                Ok(MMixInstruction::POP(x, y, z))
            }
            Rule::operand_list_one => {
                let mut ops = Children::of(operands);
                let xyz = self.imm_three_bytes(ops.required()?, "POP")?;
                let (x, y, z) = Self::split_xyz_bytes(xyz);
                Ok(MMixInstruction::POP(x, y, z))
            }
            _ => Err(parts.unexpected(&operands)),
        }
    }

    pub(super) fn parse_inst_go_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "GOI", MMixInstruction::GOI)
    }
}
