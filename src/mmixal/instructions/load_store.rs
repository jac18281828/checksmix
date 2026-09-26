//! The load/store instruction family, including the LDA and uncached/uncommon forms.
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
    pub(super) fn parse_inst_load_store_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem_pair = parts.required()?;
        let mnem = mnem_pair.as_str().to_uppercase();
        let operands = parts.required()?;
        let (x, y, z) = match operands.as_rule() {
            Rule::operand_list_three => {
                let mut ops = Children::of(operands);
                let x = self.parse_register(ops.required()?)?;
                let y = self.parse_register(ops.required()?)?;
                let z = self.lower_z_operand(ops.required()?, &mnem)?;
                (x, y, z)
            }
            Rule::operand_list_two => {
                // The two-operand memory form: the second operand is a
                // register (an offset of zero) or a base address resolved
                // against a preceding GREG.
                let mut ops = Children::of(operands);
                let x = self.parse_register(ops.required()?)?;
                let (y, offset) = self.resolve_memory_base_operand(ops.required()?)?;
                (x, y, ZForm::Imm(offset))
            }
            _ => return Err(parts.unexpected(&operands)),
        };

        match (mnem.as_str(), z) {
            ("LDB", ZForm::Reg(z)) => Ok(MMixInstruction::LDB(x, y, z)),
            ("LDB", ZForm::Imm(z)) => Ok(MMixInstruction::LDBI(x, y, z)),
            ("LDBU", ZForm::Reg(z)) => Ok(MMixInstruction::LDBU(x, y, z)),
            ("LDBU", ZForm::Imm(z)) => Ok(MMixInstruction::LDBUI(x, y, z)),
            ("LDW", ZForm::Reg(z)) => Ok(MMixInstruction::LDW(x, y, z)),
            ("LDW", ZForm::Imm(z)) => Ok(MMixInstruction::LDWI(x, y, z)),
            ("LDWU", ZForm::Reg(z)) => Ok(MMixInstruction::LDWU(x, y, z)),
            ("LDWU", ZForm::Imm(z)) => Ok(MMixInstruction::LDWUI(x, y, z)),
            ("LDT", ZForm::Reg(z)) => Ok(MMixInstruction::LDT(x, y, z)),
            ("LDT", ZForm::Imm(z)) => Ok(MMixInstruction::LDTI(x, y, z)),
            ("LDTU", ZForm::Reg(z)) => Ok(MMixInstruction::LDTU(x, y, z)),
            ("LDTU", ZForm::Imm(z)) => Ok(MMixInstruction::LDTUI(x, y, z)),
            ("LDO", ZForm::Reg(z)) => Ok(MMixInstruction::LDO(x, y, z)),
            ("LDO", ZForm::Imm(z)) => Ok(MMixInstruction::LDOI(x, y, z)),
            ("LDOU", ZForm::Reg(z)) => Ok(MMixInstruction::LDOU(x, y, z)),
            ("LDOU", ZForm::Imm(z)) => Ok(MMixInstruction::LDOUI(x, y, z)),
            ("STB", ZForm::Reg(z)) => Ok(MMixInstruction::STB(x, y, z)),
            ("STB", ZForm::Imm(z)) => Ok(MMixInstruction::STBI(x, y, z)),
            ("STBU", ZForm::Reg(z)) => Ok(MMixInstruction::STBU(x, y, z)),
            ("STBU", ZForm::Imm(z)) => Ok(MMixInstruction::STBUI(x, y, z)),
            ("STW", ZForm::Reg(z)) => Ok(MMixInstruction::STW(x, y, z)),
            ("STW", ZForm::Imm(z)) => Ok(MMixInstruction::STWI(x, y, z)),
            ("STWU", ZForm::Reg(z)) => Ok(MMixInstruction::STWU(x, y, z)),
            ("STWU", ZForm::Imm(z)) => Ok(MMixInstruction::STWUI(x, y, z)),
            ("STT", ZForm::Reg(z)) => Ok(MMixInstruction::STT(x, y, z)),
            ("STT", ZForm::Imm(z)) => Ok(MMixInstruction::STTI(x, y, z)),
            ("STTU", ZForm::Reg(z)) => Ok(MMixInstruction::STTU(x, y, z)),
            ("STTU", ZForm::Imm(z)) => Ok(MMixInstruction::STTUI(x, y, z)),
            ("STO", ZForm::Reg(z)) => Ok(MMixInstruction::STO(x, y, z)),
            ("STO", ZForm::Imm(z)) => Ok(MMixInstruction::STOI(x, y, z)),
            ("STOU", ZForm::Reg(z)) => Ok(MMixInstruction::STOU(x, y, z)),
            ("STOU", ZForm::Imm(z)) => Ok(MMixInstruction::STOUI(x, y, z)),
            ("LDUNC", ZForm::Reg(z)) => Ok(MMixInstruction::LDUNC(x, y, z)),
            ("LDUNC", ZForm::Imm(z)) => Ok(MMixInstruction::LDUNCI(x, y, z)),
            ("STUNC", ZForm::Reg(z)) => Ok(MMixInstruction::STUNC(x, y, z)),
            ("STUNC", ZForm::Imm(z)) => Ok(MMixInstruction::STUNCI(x, y, z)),
            ("LDHT", ZForm::Reg(z)) => Ok(MMixInstruction::LDHT(x, y, z)),
            ("LDHT", ZForm::Imm(z)) => Ok(MMixInstruction::LDHTI(x, y, z)),
            ("STHT", ZForm::Reg(z)) => Ok(MMixInstruction::STHT(x, y, z)),
            ("STHT", ZForm::Imm(z)) => Ok(MMixInstruction::STHTI(x, y, z)),
            ("LDSF", ZForm::Reg(z)) => Ok(MMixInstruction::LDSF(x, y, z)),
            ("LDSF", ZForm::Imm(z)) => Ok(MMixInstruction::LDSFI(x, y, z)),
            ("STSF", ZForm::Reg(z)) => Ok(MMixInstruction::STSF(x, y, z)),
            ("STSF", ZForm::Imm(z)) => Ok(MMixInstruction::STSFI(x, y, z)),
            ("LDVTS", ZForm::Reg(z)) => Ok(MMixInstruction::LDVTS(x, y, z)),
            ("LDVTS", ZForm::Imm(z)) => Ok(MMixInstruction::LDVTSI(x, y, z)),
            ("CSWAP", ZForm::Reg(z)) => Ok(MMixInstruction::CSWAP(x, y, z)),
            ("CSWAP", ZForm::Imm(z)) => Ok(MMixInstruction::CSWAPI(x, y, z)),
            _ => Err(format!("Unknown load/store instruction: {}", mnem)),
        }
    }

    pub(super) fn parse_inst_load_store_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?;
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let y = self.parse_register(ops.required()?)?;
        let z = self.imm_byte(ops.required()?, mnem.as_str())?;

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

    /// `LDA $X,$Y,$Z` is `ADDU $X,$Y,$Z` and `LDA $X,$Y,Z` is `ADDU $X,$Y,Z`,
    /// so Z selects the same pair of opcodes ADDU selects.
    pub(super) fn parse_inst_lda_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let y = self.parse_register(ops.required()?)?;

        match self.lower_z_operand(ops.required()?, "LDA")? {
            ZForm::Reg(z) => Ok(MMixInstruction::LDA(x, y, z)),
            ZForm::Imm(z) => Ok(MMixInstruction::LDAI(x, y, z)),
        }
    }

    pub(super) fn parse_inst_lda_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDAI", MMixInstruction::LDAI)
    }

    /// `LDA $X,addr` and `LDAI $X,addr` are the same address form: `addr`
    /// resolves against a preceding `GREG` base exactly as the memory
    /// forms' two-operand shape does, always one tetra.
    pub(super) fn parse_inst_lda_ri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let operands = parts.required()?;
        let mut ops = Children::of(operands);
        let x = self.parse_register(ops.required()?)?;
        let (y, offset) = self.resolve_memory_base_operand(ops.required()?)?;
        Ok(MMixInstruction::LDAI(x, y, offset))
    }

    pub(super) fn parse_inst_ldunc_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDUNCI", MMixInstruction::LDUNCI)
    }

    pub(super) fn parse_inst_stunc_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "STUNCI", MMixInstruction::STUNCI)
    }

    pub(super) fn parse_inst_ldht_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDHTI", MMixInstruction::LDHTI)
    }

    pub(super) fn parse_inst_stht_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "STHTI", MMixInstruction::STHTI)
    }

    pub(super) fn parse_inst_ldsf_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDSFI", MMixInstruction::LDSFI)
    }

    pub(super) fn parse_inst_stsf_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "STSFI", MMixInstruction::STSFI)
    }

    pub(super) fn parse_inst_ldvts_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "LDVTSI", MMixInstruction::LDVTSI)
    }

    pub(super) fn parse_inst_cswap_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "CSWAPI", MMixInstruction::CSWAPI)
    }

    /// STCO's X is a pure byte or a register, the same bytes; only Z
    /// auto-selects register or immediate.
    pub(super) fn parse_inst_stco_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let operands = parts.required()?;
        let is_three = operands.as_rule() == Rule::operand_list_three;
        let mut ops = Children::of(operands);
        let x = self.parse_reg_or_byte(ops.required()?, "STCO")?;
        let (y, z) = if is_three {
            let y = self.parse_register(ops.required()?)?;
            let z = self.lower_z_operand(ops.required()?, "STCO")?;
            (y, z)
        } else {
            let (y, offset) = self.resolve_memory_base_operand(ops.required()?)?;
            (y, ZForm::Imm(offset))
        };
        match z {
            ZForm::Reg(z) => Ok(MMixInstruction::STCO(x, y, z)),
            ZForm::Imm(z) => Ok(MMixInstruction::STCOI(x, y, z)),
        }
    }

    pub(super) fn parse_inst_stco_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let _mnem = parts.next();
        let mut ops = Children::of(parts.required()?);
        let x = self.imm_byte(ops.required()?, "STCOI")?;
        let y = self.parse_register(ops.required()?)?;
        let z = self.imm_byte(ops.required()?, "STCOI")?;
        Ok(MMixInstruction::STCOI(x, y, z))
    }

    /// `PRELD`/`PREGO`/`PREST`/`SYNCD`/`SYNCID`'s X is a pure byte or a
    /// register, the same bytes; only Z auto-selects register or immediate.
    pub(super) fn parse_inst_cache_auto(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let mut parts = Children::of(pair);
        let mnem = parts.required()?.as_str().to_uppercase();
        let operands = parts.required()?;
        let is_three = operands.as_rule() == Rule::operand_list_three;
        let mut ops = Children::of(operands);
        let x = self.parse_reg_or_byte(ops.required()?, &mnem)?;
        let (y, z) = if is_three {
            let y = self.parse_register(ops.required()?)?;
            let z = self.lower_z_operand(ops.required()?, &mnem)?;
            (y, z)
        } else {
            let (y, offset) = self.resolve_memory_base_operand(ops.required()?)?;
            (y, ZForm::Imm(offset))
        };

        match (mnem.as_str(), z) {
            ("PRELD", ZForm::Reg(z)) => Ok(MMixInstruction::PRELD(x, y, z)),
            ("PRELD", ZForm::Imm(z)) => Ok(MMixInstruction::PRELDI(x, y, z)),
            ("PREGO", ZForm::Reg(z)) => Ok(MMixInstruction::PREGO(x, y, z)),
            ("PREGO", ZForm::Imm(z)) => Ok(MMixInstruction::PREGOI(x, y, z)),
            ("PREST", ZForm::Reg(z)) => Ok(MMixInstruction::PREST(x, y, z)),
            ("PREST", ZForm::Imm(z)) => Ok(MMixInstruction::PRESTI(x, y, z)),
            ("SYNCD", ZForm::Reg(z)) => Ok(MMixInstruction::SYNCD(x, y, z)),
            ("SYNCD", ZForm::Imm(z)) => Ok(MMixInstruction::SYNCDI(x, y, z)),
            ("SYNCID", ZForm::Reg(z)) => Ok(MMixInstruction::SYNCID(x, y, z)),
            ("SYNCID", ZForm::Imm(z)) => Ok(MMixInstruction::SYNCIDI(x, y, z)),
            _ => Err(format!("Unknown cache control instruction: {}", mnem)),
        }
    }

    pub(super) fn parse_inst_preld_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PRELDI", MMixInstruction::PRELDI)
    }

    pub(super) fn parse_inst_prego_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PREGOI", MMixInstruction::PREGOI)
    }

    pub(super) fn parse_inst_prest_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "PRESTI", MMixInstruction::PRESTI)
    }

    pub(super) fn parse_inst_syncd_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "SYNCDI", MMixInstruction::SYNCDI)
    }

    pub(super) fn parse_inst_syncid_rri(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        self.parse_rri(pair, "SYNCIDI", MMixInstruction::SYNCIDI)
    }
}
