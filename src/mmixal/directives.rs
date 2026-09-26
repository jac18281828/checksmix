//! Data and pseudo directives: `BYTE`/`WYDE`/`TETRA`/`OCTA`, `LOC`, `IS`, and `PREFIX`.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

use super::MMixAssembler;
use super::Rule;
use super::SymbolType;
use super::expressions::ExprValue;
use super::instructions::MMixInstruction;
use super::tree::Children;
use tracing::debug;

impl MMixAssembler {
    /// Alignment of a data directive, taken from the directive's kind and
    /// never from the number of bytes it emits. `BYTE` stays unaligned
    /// however long its operand list; `WYDE`, `TETRA` and `OCTA` round to
    /// their own width.
    pub(super) fn data_directive_alignment(
        pair: &pest::iterators::Pair<Rule>,
    ) -> Result<u64, String> {
        let directive = pair
            .clone()
            .into_inner()
            .next()
            .ok_or("Empty data directive")?;

        match directive.as_rule() {
            Rule::directive_byte => Ok(1),
            Rule::directive_wyde => Ok(2),
            Rule::directive_tetra => Ok(4),
            Rule::directive_octa => Ok(8),
            _ => Err(format!("Unknown data directive: {:?}", directive.as_rule())),
        }
    }

    /// Calculate the actual size of a data directive: its unit width times
    /// its unit count. A string contributes one unit per decoded byte;
    /// every other primary contributes one, matching what pass 2's
    /// `eval_data_value_items` emits (`n₁ + … + n_k − k + 1` items for a
    /// value holding k strings, per `MMIX.md`).
    pub(super) fn data_directive_size(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<u64, String> {
        let mut parts = pair.clone().into_inner();
        let directive = parts.next().ok_or("Empty data directive")?;

        let unit_width = Self::data_directive_unit_width(directive.as_rule())?;
        let values = parts.next().ok_or("Missing data values")?;

        let mut unit_count = 0u64;
        for value in values.into_inner() {
            unit_count += self.data_value_unit_count(value)?;
        }
        let total_size = unit_width * unit_count;
        debug!(
            "Data directive size: {} units x {} bytes = {} bytes",
            unit_count, unit_width, total_size
        );
        Ok(total_size)
    }

    /// The number of values one `data_value` contributes, without
    /// evaluating any of them. A string of n characters replaces its own
    /// primary with n values; every other primary contributes one, matching
    /// pass 2's `eval_data_value_items`. So a value holding k strings of
    /// n₁ … n_k characters contributes n₁ + … + n_k − k + 1, whatever
    /// operators surround them. An empty string is an error at its own
    /// position, the same rule pass 2 applies, unless it is the value's
    /// only content: a bare `""` assembles as one zero unit, so it
    /// contributes one, matching pass 2's synthesized zero.
    fn data_value_unit_count(&self, value: pest::iterators::Pair<Rule>) -> Result<u64, String> {
        let data_expr = Self::data_expr(value)?;
        if Self::is_bare_empty_string(&data_expr) {
            return Ok(1);
        }

        let mut char_total = 0u64;
        let mut string_count = 0u64;
        for term in data_expr.into_inner().step_by(2) {
            for primary in term.into_inner().step_by(2) {
                self.count_data_primary_strings(primary, &mut char_total, &mut string_count)?;
            }
        }
        Ok(1 + char_total - string_count)
    }

    /// [`Self::data_value_unit_count`]'s walk into one `data_primary`,
    /// descending through a unary wrap to the string or ordinary value it
    /// ultimately holds.
    fn count_data_primary_strings(
        &self,
        primary: pest::iterators::Pair<Rule>,
        char_total: &mut u64,
        string_count: &mut u64,
    ) -> Result<(), String> {
        let mut children = primary.into_inner();
        let first = children
            .next()
            .ok_or_else(|| "data_primary has a child".to_string())?;
        match first.as_rule() {
            Rule::unary_op => {
                let operand = children
                    .next()
                    .ok_or_else(|| "unary operator needs an operand".to_string())?;
                self.count_data_primary_strings(operand, char_total, string_count)
            }
            Rule::string_literal => {
                let (_, rest) = self.decode_data_string_literal(&first)?;
                *char_total += 1 + rest.len() as u64;
                *string_count += 1;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The unit width, in bytes, that a data directive assembles per value.
    fn data_directive_unit_width(directive_kind: Rule) -> Result<u64, String> {
        match directive_kind {
            Rule::directive_byte => Ok(1),
            Rule::directive_wyde => Ok(2),
            Rule::directive_tetra => Ok(4),
            Rule::directive_octa => Ok(8),
            _ => Err(format!("Unknown data directive: {:?}", directive_kind)),
        }
    }

    /// Build the data unit a directive assembles from one value, truncating
    /// to the directive's unit width.
    fn data_directive_unit(directive_kind: Rule, value: u64) -> Result<MMixInstruction, String> {
        match directive_kind {
            Rule::directive_byte => Ok(MMixInstruction::BYTE(value as u8)),
            Rule::directive_wyde => Ok(MMixInstruction::WYDE(value as u16)),
            Rule::directive_tetra => Ok(MMixInstruction::TETRA(value as u32)),
            Rule::directive_octa => Ok(MMixInstruction::OCTA(value)),
            _ => Err(format!("Unknown data directive: {:?}", directive_kind)),
        }
    }

    /// A data unit's name in a diagnostic, from its width in bytes.
    fn data_unit_name(width: u64) -> &'static str {
        match width {
            1 => "byte",
            2 => "wyde",
            4 => "tetra",
            _ => "octa",
        }
    }

    /// The bits a data unit of `width` bytes holds. A value outside this
    /// mask doesn't fit the unit; its low bytes assemble and it warns.
    /// `OCTA`'s mask is every 64-bit value, so it never warns.
    fn data_unit_mask(width: u64) -> u64 {
        if width >= 8 {
            u64::MAX
        } else {
            (1u64 << (width * 8)) - 1
        }
    }

    /// Record a warning at `line:col`, `message` following "warning: ".
    fn warn(&mut self, line: usize, col: usize, message: String) {
        self.warnings.push(format!(
            "{}:{}:{}: warning: {}",
            self.current_filename, line, col, message
        ));
    }

    /// Parse a data directive and expand its value list to one instruction
    /// per unit (e.g. `BYTE "Hello"` becomes five `BYTE` instructions). A
    /// value that doesn't fit its unit warns and keeps its low bytes, the
    /// MMIXAL reference's own rule for a data directive.
    pub(super) fn parse_data_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<MMixInstruction>, String> {
        let mut parts = pair.into_inner();
        let directive_kind = parts.next().ok_or("Empty data directive")?.as_rule();
        let values_pair = parts.next().ok_or("Missing data values")?;
        let unit_width = Self::data_directive_unit_width(directive_kind)?;
        let unit = Self::data_unit_name(unit_width);
        let mask = Self::data_unit_mask(unit_width);

        let mut result = Vec::new();
        for value in values_pair.into_inner() {
            let (line, col) = value.line_col();
            if Self::is_bare_empty_string(&Self::data_expr(value.clone())?) {
                self.warn(
                    line,
                    col,
                    format!("an empty string assembles as one zero {unit}"),
                );
            }
            for item in self.eval_data_value_items(value)? {
                let val = self.require_pure(item, line, col)?;
                if val & mask != val {
                    self.warn(
                        line,
                        col,
                        format!(
                            "value {} does not fit in a {unit}; its low {unit} assembles",
                            val as i64
                        ),
                    );
                }
                result.push(Self::data_directive_unit(directive_kind, val)?);
            }
        }
        Ok(result)
    }

    /// `LOC`'s operand may itself read `@`, which errors here (via
    /// `parse_number`) when the counter is already past the end. Otherwise
    /// a `LOC` always restores a valid counter, whatever state it found.
    pub(super) fn parse_loc_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(), String> {
        let mut parts = Children::of(pair);
        let _directive = parts.next(); // Skip "LOC" keyword
        let addr = self.parse_number(parts.required()?)?;
        self.current_addr = addr;
        self.past_end = false;
        Ok(())
    }

    pub(super) fn parse_is_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
        checking: bool,
    ) -> Result<(), String> {
        let mut parts = Children::of(pair);
        let lhs = parts.required()?;
        let lhs_rule = lhs.as_rule();
        let (line, col) = lhs.line_col();
        let raw_name = lhs.as_str().to_string();
        let _is_keyword = parts.next(); // Skip "IS" keyword
        let value_pair = parts.required()?;
        let (vline, vcol) = value_pair.line_col();

        self.scan_uses_for_redefinition(&value_pair);
        let symbol_type = match self.eval_expr(value_pair)? {
            ExprValue::Register(r) => {
                SymbolType::Register(self.require_register_in_range(r, vline, vcol)?)
            }
            ExprValue::Pure(value) => SymbolType::Constant(value),
        };

        if lhs_rule == Rule::local_label_def {
            self.record_local_label(Self::local_digit(&raw_name), symbol_type, checking);
        } else if checking {
            self.define_symbol(&raw_name, symbol_type, line, col)?;
        } else {
            let qualified = self.qualify_name(&raw_name);
            self.symbols.insert(qualified, symbol_type);
        }
        Ok(())
    }

    /// `PREFIX`'s operand is stored with one leading ':' stripped, so
    /// `PREFIX :` is the root (the empty prefix) and `PREFIX :Foo:` equals
    /// `PREFIX Foo:`. checksmix replaces the prefix outright rather than
    /// qualifying a relative operand against the current one.
    pub(super) fn parse_prefix_directive(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(), String> {
        let mut parts = Children::of(pair);
        let _directive = parts.next(); // Skip "PREFIX" keyword
        let arg = parts.required()?;
        let arg_str = arg.as_str();
        self.current_prefix = arg_str.strip_prefix(':').unwrap_or(arg_str).to_string();
        Ok(())
    }
}
