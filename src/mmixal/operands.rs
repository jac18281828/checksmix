//! Operand resolution: register and immediate parsing, and base-address search for the two-operand memory form.

use super::MMixAssembler;
use super::Rule;
use super::expressions::ExprValue;

/// Resolved Z operand for an auto-immediate base mnemonic. The parser uses
/// this to choose between the RRR and RRI variants of an instruction.
#[derive(Debug, Clone, Copy)]
pub(super) enum ZForm {
    Reg(u8),
    Imm(u8),
}

impl MMixAssembler {
    /// Resolve the Z operand of a base mnemonic (auto-immediate path) into
    /// either a register reference or an 8-bit immediate: a register-valued
    /// expression selects the register form, a pure value is range-checked
    /// against `0..=255` for the immediate form.
    pub(super) fn lower_z_operand(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<ZForm, String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => {
                let reg = self.require_register_in_range(r, line, col)?;
                Ok(ZForm::Reg(reg))
            }
            ExprValue::Pure(v) => self.imm_in_range(v, mnem, line, col),
        }
    }

    /// Range-check a resolved operand value against an instruction field's
    /// width, the one place every site in the range table calls: the value
    /// must fit `0..=max` or assembly fails naming the field's own maximum
    /// and the mnemonic as written. `v` prints as a signed 64-bit integer
    /// when it is `2^63` or more, so a negative literal reports negative.
    fn field_value(
        &self,
        v: u64,
        max: u64,
        mnem: &str,
        line: usize,
        col: usize,
    ) -> Result<u64, String> {
        if v <= max {
            Ok(v)
        } else {
            Err(format!(
                "{}:{}:{}: immediate operand {} out of range 0..{} for {}",
                self.current_filename, line, col, v as i64, max, mnem
            ))
        }
    }

    /// Evaluate `pair` and range-check it as an 8-bit instruction field
    /// (0..=255): every explicit `*I` spelling's own byte-sized operand.
    pub(super) fn imm_byte(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 0xFF, mnem, line, col)
            .map(|v| v as u8)
    }

    /// Evaluate `pair` and range-check it as a 16-bit instruction field
    /// (0..=65535): the wyde immediates and `TRAP`/`TRIP`/`SWYM`/`POP`'s
    /// `yz`.
    pub(super) fn imm_wyde(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u16, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 0xFFFF, mnem, line, col)
            .map(|v| v as u16)
    }

    /// Evaluate `pair` and range-check it as a 24-bit instruction field
    /// (0..=16777215): `TRAP`/`TRIP`/`SWYM`/`POP`'s `xyz`, `RESUME` and
    /// `SYNC`.
    pub(super) fn imm_three_bytes(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u32, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 0xFF_FFFF, mnem, line, col)
            .map(|v| v as u32)
    }

    /// Evaluate `pair` and range-check it as a special register number
    /// (0..=31): `GET`'s `Z`, `PUT`'s and `PUTI`'s `X`.
    pub(super) fn special_register(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        self.field_value(self.parse_number(pair)?, 31, mnem, line, col)
            .map(|v| v as u8)
    }

    /// Range-check an already-resolved Z value as an 8-bit immediate
    /// (0..=255), for [`Self::lower_z_operand`], which has already told
    /// register and pure values apart.
    fn imm_in_range(&self, v: u64, mnem: &str, line: usize, col: usize) -> Result<ZForm, String> {
        self.field_value(v, 0xFF, mnem, line, col)
            .map(|v| ZForm::Imm(v as u8))
    }

    /// Evaluate `pair` and accept either a register or a pure value as an
    /// 8-bit field: a register contributes its own number, range-checked
    /// the same as any other register operand; a pure value contributes its
    /// own magnitude, range-checked as an immediate. `TRAP`, `TRIP` and
    /// `SWYM`'s X, Y and Z read this way, as does the X byte `PUSHJ`,
    /// `PUSHGO`, the `PRELD` family and `STCO` take.
    pub(super) fn parse_reg_or_byte(
        &self,
        pair: pest::iterators::Pair<Rule>,
        mnem: &str,
    ) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => self.require_register_in_range(r, line, col),
            ExprValue::Pure(v) => self.field_value(v, 0xFF, mnem, line, col).map(|v| v as u8),
        }
    }

    /// The two-operand memory form's base-address search: among every
    /// `GREG` seen so far (in source order) whose initial value is nonzero,
    /// choose the largest value `b` with `b <= addr` and `addr - b < 256`,
    /// the earliest allocated on a tie between registers holding the same
    /// value. Returns the matched register and `addr - b`.
    fn resolve_base_address(&self, addr: u64, line: usize, col: usize) -> Result<(u8, u8), String> {
        let mut best: Option<(u8, u64)> = None;
        for &(reg, value) in &self.greg_inits[..self.greg_inits_seen] {
            if value == 0 || value > addr || addr - value >= 256 {
                continue;
            }
            if best.is_none_or(|(_, best_value)| value > best_value) {
                best = Some((reg, value));
            }
        }
        match best {
            Some((reg, value)) => Ok((reg, (addr - value) as u8)),
            None => Err(format!(
                "{}:{}:{}: no GREG before this instruction holds a base address 0 to 255 \
                 bytes below {addr:#x}",
                self.current_filename, line, col
            )),
        }
    }

    /// The two-operand memory form's second operand: a register operand is
    /// an offset of zero, following its value, not its spelling; a pure
    /// operand is a base address, resolved by [`Self::resolve_base_address`].
    /// Returns the register to place in Y and the offset to place in Z.
    pub(super) fn resolve_memory_base_operand(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(u8, u8), String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => {
                let reg = self.require_register_in_range(r, line, col)?;
                Ok((reg, 0))
            }
            ExprValue::Pure(addr) => self.resolve_base_address(addr, line, col),
        }
    }

    /// Range-check a register value carried inside an expression (up to
    /// 64 bits, per unary `$`) down to the 0..=255 a register field holds.
    pub(super) fn require_register_in_range(
        &self,
        r: u64,
        line: usize,
        col: usize,
    ) -> Result<u8, String> {
        u8::try_from(r).map_err(|_| {
            format!(
                "{}:{}:{}: register ${} is out of range 0..255",
                self.current_filename, line, col, r
            )
        })
    }

    /// Evaluate `pair` (an `expr`) and demand a pure value.
    pub(super) fn parse_number(&self, pair: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let (line, col) = pair.line_col();
        let value = self.eval_expr(pair)?;
        self.require_pure(value, line, col)
    }

    /// Demand a pure value out of an already-evaluated `ExprValue`, at
    /// `line`:`col` for the diagnostic. Shared by [`Self::parse_number`],
    /// which evaluates its own pair first, and `parse_data_directive`, which
    /// already holds one from [`Self::eval_data_value_items`].
    pub(super) fn require_pure(
        &self,
        value: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<u64, String> {
        match value {
            ExprValue::Pure(v) => Ok(v),
            ExprValue::Register(r) => Err(format!(
                "{}:{}:{}: register ${} cannot be used where a pure value is required",
                self.current_filename, line, col, r
            )),
        }
    }

    /// Evaluate `pair` (an `expr`) and demand a register, range-checked
    /// 0..=255.
    pub(super) fn parse_register(&self, pair: pest::iterators::Pair<Rule>) -> Result<u8, String> {
        let (line, col) = pair.line_col();
        match self.eval_expr(pair)? {
            ExprValue::Register(r) => self.require_register_in_range(r, line, col),
            ExprValue::Pure(v) => Err(format!(
                "{}:{}:{}: pure value {} cannot be used where a register is required",
                self.current_filename, line, col, v
            )),
        }
    }
}
