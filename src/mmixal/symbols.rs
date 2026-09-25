//! Symbol qualification, redefinition rules, and local-label bookkeeping.

use super::MMixAssembler;
use super::Rule;
use super::SymbolType;
use super::expressions::ExprValue;

impl MMixAssembler {
    /// Apply the active PREFIX to a raw identifier. The root prefix is the
    /// empty string; a name beginning with ':' opts out of the active
    /// PREFIX and is stored at the root, one leading colon stripped -- so
    /// `x` and `:x` name the same root-level symbol.
    pub(super) fn qualify_name(&self, raw: &str) -> String {
        match raw.strip_prefix(':') {
            Some(rest) => rest.to_string(),
            None => format!("{}{}", self.current_prefix, raw),
        }
    }

    /// The value currently stored under `name`, checking `labels` before
    /// `symbols` -- the order that lets a program's own label win over a
    /// predefined symbol of the same name.
    fn existing_value(&self, name: &str) -> Option<SymbolType> {
        self.labels
            .get(name)
            .map(|&addr| SymbolType::Constant(addr))
            .or_else(|| self.symbols.get(name).copied())
    }

    /// Checks whether `name` may be bound to `candidate`, per the
    /// redefinition rules: an existing user definition must match
    /// `candidate` exactly (a differing one is the ordinary redefinition
    /// error); a still-predefined, not-yet-shadowed name may be redefined
    /// only if no earlier statement has used its predefined value.
    /// `Ok(true)` means the caller should record the new origin and store
    /// `candidate`; `Ok(false)` means an equal redefinition needs no
    /// further action.
    fn check_definable(
        &self,
        name: &str,
        candidate: SymbolType,
        line: usize,
        col: usize,
    ) -> Result<bool, String> {
        if let Some((prev_file, prev_line)) = self
            .label_origins
            .get(name)
            .or_else(|| self.symbol_origins.get(name))
        {
            return if self.existing_value(name) == Some(candidate) {
                Ok(false)
            } else {
                Err(format!(
                    "{}:{}:{}: symbol '{}' redefined (first defined at {}:{})",
                    self.current_filename, line, col, name, prev_file, prev_line
                ))
            };
        }
        if self.predefined_names.contains(name)
            && let Some((used_file, used_line)) = self.predefined_used_at.get(name)
        {
            return Err(format!(
                "{}:{}:{}: predefined symbol '{}' redefined after its value was used at {}:{}",
                self.current_filename, line, col, name, used_file, used_line
            ));
        }
        Ok(true)
    }

    /// Define a label (instruction/data/standalone) at the current address.
    pub(super) fn define_label(
        &mut self,
        raw: &str,
        addr: u64,
        line: usize,
        col: usize,
    ) -> Result<(), String> {
        let name = self.qualify_name(raw);
        if self.check_definable(&name, SymbolType::Constant(addr), line, col)? {
            self.label_origins
                .insert(name.clone(), (self.current_filename.clone(), line));
            self.labels.insert(name, addr);
        }
        Ok(())
    }

    /// Define an IS- or GREG-bound symbol.
    pub(super) fn define_symbol(
        &mut self,
        raw: &str,
        ty: SymbolType,
        line: usize,
        col: usize,
    ) -> Result<(), String> {
        let name = self.qualify_name(raw);
        if self.check_definable(&name, ty, line, col)? {
            self.symbol_origins
                .insert(name.clone(), (self.current_filename.clone(), line));
            self.symbols.insert(name, ty);
        }
        Ok(())
    }

    /// Record `name`'s first use site if it is still an unshadowed
    /// predefined symbol -- the site a later redefinition attempt cites.
    fn mark_predefined_use(&mut self, name: &str, line: usize) {
        if self.predefined_names.contains(name)
            && !self.label_origins.contains_key(name)
            && !self.symbol_origins.contains_key(name)
            && !self.predefined_used_at.contains_key(name)
        {
            self.predefined_used_at
                .insert(name.to_string(), (self.current_filename.clone(), line));
        }
    }

    /// Walk `pair` and every descendant, marking each `global_id` leaf as a
    /// use. Callers scope `pair` to an operand -- never a definition's own
    /// name -- so every `global_id` found here is a reference, not a
    /// binding.
    pub(super) fn scan_uses_for_redefinition(&mut self, pair: &pest::iterators::Pair<Rule>) {
        if pair.as_rule() == Rule::global_id {
            let (line, _) = pair.line_col();
            let qualified = self.qualify_name(pair.as_str());
            self.mark_predefined_use(&qualified, line);
            return;
        }
        for inner in pair.clone().into_inner() {
            self.scan_uses_for_redefinition(&inner);
        }
    }

    /// The digit a local-symbol token (`dH`, `dB` or `dF`) opens with, as
    /// an index into the ten per-digit lists.
    pub(super) fn local_digit(text: &str) -> u8 {
        text.as_bytes()[0] - b'0'
    }

    fn symbol_type_to_expr_value(ty: SymbolType) -> ExprValue {
        match ty {
            SymbolType::Constant(v) => ExprValue::Pure(v),
            SymbolType::Register(r) => ExprValue::Register(r as u64),
        }
    }

    /// `dB`: the last `dH` of this digit at or before the referencing
    /// statement, or `0` when none has appeared yet -- never an error.
    pub(super) fn resolve_local_back(&self, digit: u8) -> ExprValue {
        let idx = digit as usize;
        let occurrence = self.local_occurrence[idx];
        if occurrence == 0 {
            ExprValue::Pure(0)
        } else {
            Self::symbol_type_to_expr_value(self.local_labels[idx][occurrence - 1])
        }
    }

    /// `dF`: the first `dH` of this digit after the referencing statement,
    /// an error naming the reference when none follows. In pass 1 the list
    /// for `digit` never holds more than `local_occurrence[digit]` entries
    /// (pass 1 is still building it), so this always reports undefined
    /// there, exactly as an undefined named forward reference does.
    /// `local_pending_digit` bumps the search past the statement's own
    /// not-yet-recorded occurrence, when it defines this same digit.
    pub(super) fn resolve_local_fwd(
        &self,
        digit: u8,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        let idx = digit as usize;
        let mut index = self.local_occurrence[idx];
        if self.local_pending_digit == Some(digit) {
            index += 1;
        }
        self.local_labels[idx]
            .get(index)
            .map(|&ty| Self::symbol_type_to_expr_value(ty))
            .ok_or_else(|| {
                format!(
                    "{}:{}:{}: Undefined symbol: {}F",
                    self.current_filename, line, col, digit
                )
            })
    }

    /// Records one `dH` occurrence. `building` is pass 1's own flag: pass 1
    /// appends `value` to the digit's list, pass 2 only advances the
    /// per-pass occurrence counter that keeps its `dB`/`dF` reads in step
    /// with pass 1's completed lists.
    pub(super) fn record_local_label(&mut self, digit: u8, value: SymbolType, building: bool) {
        let idx = digit as usize;
        if building {
            self.local_labels[idx].push(value);
        }
        self.local_occurrence[idx] += 1;
    }
}
