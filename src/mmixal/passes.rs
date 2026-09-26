//! The two-pass walk: `parse`, per-statement dispatch, and special-mode (`BSPEC`/`ESPEC`) handling.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

use super::MMixAssembler;
use super::MMixalParser;
use super::Rule;
use super::SourceLoc;
use super::SymbolType;
use super::instructions::MMixInstruction;
use super::tree::Children;
use tracing::{debug, instrument};

impl MMixAssembler {
    #[instrument(skip(self))]
    pub fn parse(&mut self) -> Result<(), String> {
        self.warnings.clear();
        if let Some((file, line, col)) = &self.debug_directive_overflow {
            return Err(format!(
                "{file}:{line}:{col}: error: too many `debug` directives in this \
                 program; the string table holds at most 256"
            ));
        }

        debug!("Starting MMIXAL parsing (two-pass)");
        match self.parse_two_pass() {
            Ok(_) => {
                debug!(
                    instruction_count = self.instructions.len(),
                    label_count = self.labels.len(),
                    symbol_count = self.symbols.len(),
                    "Parsing completed successfully"
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Two-pass assembler:
    /// Pass 1: Collect all labels and their addresses, process IS directives
    /// Pass 2: Generate instructions with resolved label references
    ///
    /// Each pass walks every translation unit in command-line order, threading
    /// `current_addr`, `current_prefix`, and the symbol tables across files so
    /// the result matches assembling the concatenation of the inputs. The
    /// PREFIX state is reset at the start of each pass.
    #[instrument(skip(self))]
    fn parse_two_pass(&mut self) -> Result<(), String> {
        use pest::Parser;

        let sources = self.sources.clone();
        debug!("Pass 1: Collecting labels and symbols");
        self.current_prefix.clear();
        self.local_occurrence = [0; 10];
        self.in_special_mode = false;
        self.bspec_open_site = None;

        for (index, unit) in sources.iter().enumerate() {
            self.current_filename = unit.filename.clone();
            self.current_unit_index = index;
            let pairs = MMixalParser::parse(Rule::program, &unit.preprocessed)
                .map_err(|e| Self::format_parse_error(&e, &unit.filename, &unit.preprocessed))?;
            for pair in pairs {
                if pair.as_rule() == Rule::program {
                    for line_pair in pair.into_inner() {
                        if line_pair.as_rule() == Rule::line {
                            self.first_pass_line(line_pair, &unit.preprocessed, &unit.filename)?;
                        }
                    }
                }
            }
        }

        if let Some((file, line, col)) = &self.bspec_open_site {
            return Err(format!(
                "{file}:{line}:{col}: syntax error: BSPEC has no matching ESPEC before end of input"
            ));
        }

        // LOCAL's threshold is fixed only once every GREG has allocated:
        // one above the lowest register GREG handed out, `$31` at the
        // lowest -- $0..$31 are local on every MMIX regardless of GREG
        // activity.
        let threshold = (self.next_greg as u16).saturating_add(1).max(32);
        for (reg, file, line, col) in &self.local_declarations {
            if u16::from(*reg) >= threshold {
                return Err(format!(
                    "{file}:{line}:{col}: LOCAL ${reg} is not below the global threshold ${threshold}"
                ));
            }
        }

        debug!(
            "Pass 1 complete: {} labels, {} symbols",
            self.labels.len(),
            self.symbols.len()
        );

        let saved_addr = self.current_addr;
        let saved_past_end = self.past_end;
        self.current_addr = 0;
        self.past_end = false;
        self.current_prefix.clear();
        self.local_occurrence = [0; 10];
        self.in_special_mode = false;
        self.greg_inits_seen = 0;

        debug!("Pass 2: Generating instructions");

        for (index, unit) in sources.iter().enumerate() {
            self.current_filename = unit.filename.clone();
            self.current_unit_index = index;
            let pairs = MMixalParser::parse(Rule::program, &unit.preprocessed)
                .map_err(|e| Self::format_parse_error(&e, &unit.filename, &unit.preprocessed))?;
            for pair in pairs {
                if pair.as_rule() == Rule::program {
                    for line_pair in pair.into_inner() {
                        if line_pair.as_rule() == Rule::line {
                            for stmt_pair in line_pair.into_inner() {
                                if stmt_pair.as_rule() == Rule::statement {
                                    let (line, col) = stmt_pair.line_col();
                                    self.second_pass_statement(stmt_pair)
                                        .map_err(|e| self.locate_fallback(e, line, col))?;
                                }
                            }
                        }
                    }
                }
            }
        }

        self.current_addr = saved_addr;
        self.past_end = saved_past_end;
        Ok(())
    }

    /// Pass 1's per-line walk. Tracks where the current `;`-delimited
    /// segment began and whether its statement was a bare label, so a
    /// segment's candidate remark is diagnosed by the matching function and
    /// never blamed on a sibling's parens. The rule that an indented line
    /// has no label field applies only to a line's own first segment -- a
    /// statement after `;` keeps reading a label, wherever the physical
    /// line started.
    fn first_pass_line(
        &mut self,
        line_pair: pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
    ) -> Result<(), String> {
        let mut segment_start = line_pair.as_span().start();
        let line_indented = Self::line_opens_indented(source, segment_start);
        let mut is_first_segment = true;
        let mut has_statement = false;
        // `Some(label_pair)` exactly when the current segment's statement
        // is a bare label with no instruction or directive attached.
        let mut bare_label: Option<pest::iterators::Pair<Rule>> = None;

        for stmt_pair in line_pair.into_inner() {
            match stmt_pair.as_rule() {
                Rule::statement => {
                    has_statement = true;
                    if is_first_segment
                        && line_indented
                        && let Some(label_pair) = Self::statement_label_pair(&stmt_pair)
                        && stmt_pair.as_span().end() > label_pair.as_span().end()
                    {
                        return Err(Self::indented_label_statement_error(
                            &stmt_pair,
                            &label_pair,
                            source,
                            filename,
                            segment_start,
                        ));
                    }
                    bare_label = Self::statement_is_label_only(&stmt_pair)
                        .then(|| stmt_pair.clone().into_inner().next())
                        .flatten();
                    let (line, col) = stmt_pair.line_col();
                    self.first_pass_statement(stmt_pair)
                        .map_err(|e| self.locate_fallback(e, line, col))?;
                }
                Rule::remark => {
                    if let Some(label_pair) = bare_label.as_ref()
                        && stmt_pair.as_str().is_empty()
                    {
                        if let Some(err) = Self::bare_reserved_mnemonic_error(
                            label_pair,
                            &stmt_pair,
                            source,
                            filename,
                            segment_start,
                        ) {
                            return Err(err);
                        }
                        if is_first_segment && line_indented {
                            return Err(Self::indented_bare_label_error(
                                label_pair,
                                &stmt_pair,
                                source,
                                filename,
                                segment_start,
                            ));
                        }
                    }
                    if bare_label.is_some() {
                        Self::diagnose_unrecognized_opcode(
                            &stmt_pair,
                            source,
                            filename,
                            segment_start,
                        )?;
                    } else {
                        Self::check_remark(
                            &stmt_pair,
                            source,
                            filename,
                            segment_start,
                            has_statement,
                        )?;
                    }
                    // Skip the `;` that follows, if any, so the next
                    // segment starts clean.
                    segment_start = stmt_pair.as_span().end() + 1;
                    is_first_segment = false;
                    has_statement = false;
                    bare_label = None;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// First pass: collect labels and process IS/PREFIX directives.
    /// Redefinition errors are reported here; the second pass overwrites
    /// silently because every label collected here will be re-encountered
    /// at the same address.
    #[instrument(skip(self, pair), fields(current_addr = format!("0x{:X}", self.current_addr)))]
    fn first_pass_statement(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut pending_label: Option<(String, usize, usize)> = None;
        let mut pending_local: Option<(u8, usize, usize)> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let (line, col) = inner_pair.line_col();
                    let ident = Children::of(inner_pair).required()?;
                    pending_label = Some((ident.as_str().to_string(), line, col));
                }
                Rule::local_label_def => {
                    let (line, col) = inner_pair.line_col();
                    let digit = Self::local_digit(inner_pair.as_str());
                    pending_local = Some((digit, line, col));
                    self.local_pending_digit = Some(digit);
                }
                Rule::instruction => {
                    if self.in_special_mode {
                        return Err(Self::special_mode_content_error(
                            &self.current_filename,
                            &inner_pair,
                            "an instruction",
                        ));
                    }
                    let item = inner_pair.line_col();
                    Self::require_addr(
                        self.align_current_addr(Self::INSTRUCTION_ALIGNMENT),
                        &self.current_filename,
                        Self::leftmost_site(&pending_label, &pending_local, item),
                    )?;
                    self.scan_uses_for_redefinition(&inner_pair);
                    let inst = self.peek_instruction_type(inner_pair)?;
                    let size = Self::instruction_size(&inst);
                    if let Some((raw, line, col)) = pending_label.take() {
                        self.define_label(&raw, self.current_addr, line, col)?;
                    }
                    if let Some((digit, _, _)) = pending_local.take() {
                        self.record_local_label(
                            digit,
                            SymbolType::Constant(self.current_addr),
                            true,
                        );
                    }
                    Self::require_addr(self.place_item(size), &self.current_filename, item)?;
                }
                Rule::directive => {
                    let directive_pair = Children::of(inner_pair).required()?;
                    match directive_pair.as_rule() {
                        Rule::data_directive => {
                            if self.in_special_mode {
                                // Discarded: no bytes, no address movement,
                                // but a label on the line still binds to
                                // the (unmoved) current address.
                                if let Some((raw, line, col)) = pending_label.take() {
                                    self.require_valid((line, col))?;
                                    self.define_label(&raw, self.current_addr, line, col)?;
                                }
                                if let Some((digit, line, col)) = pending_local.take() {
                                    self.require_valid((line, col))?;
                                    self.record_local_label(
                                        digit,
                                        SymbolType::Constant(self.current_addr),
                                        true,
                                    );
                                }
                            } else {
                                let item = directive_pair.line_col();
                                self.scan_uses_for_redefinition(&directive_pair);
                                let alignment = Self::data_directive_alignment(&directive_pair)?;
                                Self::require_addr(
                                    self.align_current_addr(alignment),
                                    &self.current_filename,
                                    Self::leftmost_site(&pending_label, &pending_local, item),
                                )?;
                                let size = self.data_directive_size(directive_pair.clone())?;
                                if let Some((raw, line, col)) = pending_label.take() {
                                    self.define_label(&raw, self.current_addr, line, col)?;
                                }
                                if let Some((digit, _, _)) = pending_local.take() {
                                    self.record_local_label(
                                        digit,
                                        SymbolType::Constant(self.current_addr),
                                        true,
                                    );
                                }
                                Self::require_addr(
                                    self.place_item(size),
                                    &self.current_filename,
                                    item,
                                )?;
                            }
                        }
                        Rule::loc_directive => {
                            if self.in_special_mode {
                                return Err(Self::special_mode_content_error(
                                    &self.current_filename,
                                    &directive_pair,
                                    "LOC",
                                ));
                            }
                            // A label on a LOC line names the location the
                            // counter held before LOC moves it, per the
                            // MMIXAL reference's `X LOC @+500`. Both past-end
                            // checks run before the operand is evaluated, so
                            // an operand that itself needs the address (`@`)
                            // never outranks this line's own label or local
                            // label for which site gets reported. The local
                            // label's own value is still recorded after the
                            // operand, so a same-digit reference in it never
                            // resolves to itself.
                            let addr_before = self.current_addr;
                            if let Some((raw, line, col)) = pending_label.take() {
                                self.require_valid((line, col))?;
                                self.define_label(&raw, addr_before, line, col)?;
                            }
                            if let Some(&(_, line, col)) = pending_local.as_ref() {
                                self.require_valid((line, col))?;
                            }
                            self.scan_uses_for_redefinition(&directive_pair);
                            self.parse_loc_directive(directive_pair)?;
                            if let Some((digit, _, _)) = pending_local.take() {
                                self.record_local_label(
                                    digit,
                                    SymbolType::Constant(addr_before),
                                    true,
                                );
                            }
                        }
                        Rule::greg_directive => {
                            // GREG allocates a global register; an attached
                            // label aliases the register, not an address.
                            // The reference starts the threshold at $255 and
                            // refuses a GREG once it reaches $32, so $32
                            // through $254 (223 registers) are the whole
                            // supply.
                            let allocated_reg = if self.next_greg < 32 {
                                let (line, col) = directive_pair.line_col();
                                return Err(format!(
                                    "{}:{}:{}: GREG has no global register left: \
                                     $32 through $254 are all allocated",
                                    self.current_filename, line, col
                                ));
                            } else {
                                let reg = self.next_greg;
                                self.next_greg -= 1;
                                reg
                            };

                            // A GREG with no operand -- the empty field is
                            // 0 -- holds a global register at 0, per the
                            // reference's own reading of an empty operand
                            // field.
                            let mut greg_parts = directive_pair.clone().into_inner();
                            let _directive = greg_parts.next();
                            let value = match greg_parts.next() {
                                Some(operand) => {
                                    self.scan_uses_for_redefinition(&operand);
                                    self.parse_number(operand)?
                                }
                                None => 0,
                            };
                            self.greg_inits.push((allocated_reg, value));

                            if let Some((raw, line, col)) = pending_label.take() {
                                self.define_symbol(
                                    &raw,
                                    SymbolType::Register(allocated_reg),
                                    line,
                                    col,
                                )?;
                            }
                            if let Some((digit, _, _)) = pending_local.take() {
                                self.record_local_label(
                                    digit,
                                    SymbolType::Register(allocated_reg),
                                    true,
                                );
                            }
                        }
                        Rule::is_directive => {
                            self.parse_is_directive(directive_pair, true)?;
                            // IS directive doesn't advance current_addr.
                        }
                        Rule::prefix_directive => {
                            self.parse_prefix_directive(directive_pair)?;
                        }
                        Rule::local_directive => {
                            Self::require_blank_label(
                                &self.current_filename,
                                "LOCAL",
                                Self::pending_label_loc(&pending_label, &pending_local),
                            )?;
                            self.handle_local_directive(directive_pair)?;
                        }
                        Rule::bspec_directive => {
                            Self::require_blank_label(
                                &self.current_filename,
                                "BSPEC",
                                Self::pending_label_loc(&pending_label, &pending_local),
                            )?;
                            self.open_special_mode(directive_pair)?;
                        }
                        Rule::espec_directive => {
                            Self::require_blank_label(
                                &self.current_filename,
                                "ESPEC",
                                Self::pending_label_loc(&pending_label, &pending_local),
                            )?;
                            self.close_special_mode(&directive_pair)?;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Standalone labels (no instruction or directive on the line)
        if let Some((raw, line, col)) = pending_label {
            self.require_valid((line, col))?;
            self.define_label(&raw, self.current_addr, line, col)?;
        }
        if let Some((digit, line, col)) = pending_local {
            self.require_valid((line, col))?;
            self.record_local_label(digit, SymbolType::Constant(self.current_addr), true);
        }
        self.local_pending_digit = None;

        Ok(())
    }

    /// The leftmost site to report if `item` turns out past the end: a
    /// statement has at most one of `pending_label`/`pending_local`, and
    /// either one's own site sits ahead of `item`'s.
    fn leftmost_site(
        pending_label: &Option<(String, usize, usize)>,
        pending_local: &Option<(u8, usize, usize)>,
        item: (usize, usize),
    ) -> (usize, usize) {
        pending_label
            .as_ref()
            .map(|&(_, l, c)| (l, c))
            .or_else(|| pending_local.as_ref().map(|&(_, l, c)| (l, c)))
            .unwrap_or(item)
    }

    /// Diagnostic for an instruction or `LOC` found between `BSPEC` and
    /// `ESPEC`: only `IS`, `PREFIX`, `GREG`, `LOCAL` and the four data
    /// directives are legal there.
    fn special_mode_content_error(
        filename: &str,
        pair: &pest::iterators::Pair<Rule>,
        what: &str,
    ) -> String {
        let (line, col) = pair.line_col();
        format!("{filename}:{line}:{col}: syntax error: {what} is not allowed inside BSPEC/ESPEC")
    }

    /// The line and column of whichever of a statement's label fields is
    /// pending, if any -- `pending_label` when both are somehow set, since
    /// grammar admits at most one.
    fn pending_label_loc(
        pending_label: &Option<(String, usize, usize)>,
        pending_local: &Option<(u8, usize, usize)>,
    ) -> Option<(usize, usize)> {
        pending_label
            .as_ref()
            .map(|&(_, line, col)| (line, col))
            .or_else(|| pending_local.as_ref().map(|&(_, line, col)| (line, col)))
    }

    /// `LOCAL`, `BSPEC` and `ESPEC` take no label field.
    fn require_blank_label(
        filename: &str,
        keyword: &str,
        pending_label_loc: Option<(usize, usize)>,
    ) -> Result<(), String> {
        if let Some((line, col)) = pending_label_loc {
            return Err(format!(
                "{filename}:{line}:{col}: syntax error: {keyword} takes no label"
            ));
        }
        Ok(())
    }

    /// `LOCAL expr`: `expr` must resolve to a register, checked at the
    /// close of assembly against the global threshold `next_greg` derives.
    fn handle_local_directive(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let mut parts = Children::of(pair);
        let _keyword = parts.next();
        let operand = parts.required()?;
        let (line, col) = operand.line_col();
        self.scan_uses_for_redefinition(&operand);
        let reg = self.parse_register(operand)?;
        self.local_declarations
            .push((reg, self.current_filename.clone(), line, col));
        Ok(())
    }

    /// `BSPEC expr`: opens special mode. `BSPEC` does not nest, and its
    /// operand must fit in two bytes.
    fn open_special_mode(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
        let (line, col) = pair.line_col();
        if self.in_special_mode {
            return Err(format!(
                "{}:{}:{}: syntax error: BSPEC does not nest",
                self.current_filename, line, col
            ));
        }
        let mut parts = Children::of(pair);
        let _keyword = parts.next();
        let operand = parts.required()?;
        let (op_line, op_col) = operand.line_col();
        self.scan_uses_for_redefinition(&operand);
        let value = self.parse_number(operand)?;
        if value > 0xFFFF {
            return Err(format!(
                "{}:{}:{}: syntax error: BSPEC operand {} does not fit in two bytes",
                self.current_filename, op_line, op_col, value
            ));
        }
        self.in_special_mode = true;
        self.bspec_open_site = Some((self.current_filename.clone(), line, col));
        Ok(())
    }

    /// `ESPEC`: closes special mode; an `ESPEC` with no open `BSPEC` is an
    /// error.
    fn close_special_mode(&mut self, pair: &pest::iterators::Pair<Rule>) -> Result<(), String> {
        let (line, col) = pair.line_col();
        if !self.in_special_mode {
            return Err(format!(
                "{}:{}:{}: syntax error: ESPEC has no matching BSPEC",
                self.current_filename, line, col
            ));
        }
        self.in_special_mode = false;
        self.bspec_open_site = None;
        Ok(())
    }

    /// Second pass: generate actual instructions with resolved labels.
    /// Labels and IS-bound symbols are re-inserted (overwriting the pass-1
    /// values with the same value) without redefinition checking, since
    /// PREFIX state is replayed identically and produces the same names.
    #[instrument(skip(self, pair), fields(current_addr = format!("0x{:X}", self.current_addr)))]
    pub(super) fn second_pass_statement(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(), String> {
        // Captured before `into_inner()` consumes `pair`: the statement's
        // line in the ACTIVE translation unit's PREPROCESSED text.
        // `record_debug_info` maps it back to the original source line.
        let (line, _) = pair.line_col();
        let mut label_name: Option<(String, usize, usize)> = None;
        let mut pending_local: Option<u8> = None;
        let mut inst: Option<(MMixInstruction, (usize, usize))> = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::label_def => {
                    let ident = Children::of(inner_pair).required()?;
                    let (line, col) = ident.line_col();
                    label_name = Some((ident.as_str().to_string(), line, col));
                }
                Rule::local_label_def => {
                    let digit = Self::local_digit(inner_pair.as_str());
                    pending_local = Some(digit);
                    self.local_pending_digit = Some(digit);
                }
                Rule::instruction => {
                    let item = inner_pair.line_col();
                    Self::require_addr(
                        self.align_current_addr(Self::INSTRUCTION_ALIGNMENT),
                        &self.current_filename,
                        item,
                    )?;
                    if let Some((raw, _, _)) = label_name.take() {
                        let qualified = self.qualify_name(&raw);
                        self.labels.insert(qualified, self.current_addr);
                    }
                    // Evaluated before this line's own local label (if any)
                    // is recorded, so a same-digit reference in an operand
                    // never resolves to itself.
                    inst = Some((self.parse_instruction(inner_pair)?, item));
                    if let Some(digit) = pending_local.take() {
                        self.record_local_label(digit, SymbolType::Constant(0), false);
                    }
                }
                Rule::directive => {
                    let directive_pair = Children::of(inner_pair).required()?;
                    match directive_pair.as_rule() {
                        Rule::data_directive => {
                            if self.in_special_mode {
                                if let Some((raw, line, col)) = label_name.take() {
                                    self.require_valid((line, col))?;
                                    let qualified = self.qualify_name(&raw);
                                    self.labels.insert(qualified, self.current_addr);
                                }
                                if let Some(digit) = pending_local.take() {
                                    self.record_local_label(digit, SymbolType::Constant(0), false);
                                }
                            } else {
                                let item = directive_pair.line_col();
                                let alignment = Self::data_directive_alignment(&directive_pair)?;
                                Self::require_addr(
                                    self.align_current_addr(alignment),
                                    &self.current_filename,
                                    item,
                                )?;
                                if let Some((raw, _, _)) = label_name.take() {
                                    let qualified = self.qualify_name(&raw);
                                    self.labels.insert(qualified, self.current_addr);
                                }
                                let instructions = self.parse_data_directive(directive_pair)?;
                                if let Some(digit) = pending_local.take() {
                                    self.record_local_label(digit, SymbolType::Constant(0), false);
                                }
                                for instruction in instructions {
                                    let size = Self::instruction_size(&instruction);
                                    self.record_debug_info(self.current_addr, line);
                                    self.instructions.push((self.current_addr, instruction));
                                    Self::require_addr(
                                        self.place_item(size),
                                        &self.current_filename,
                                        item,
                                    )?;
                                }
                            }
                        }
                        Rule::loc_directive => {
                            // Mirrors first pass: the label names the
                            // location before LOC moves the counter, and
                            // the operand is evaluated before this line's
                            // own local label is recorded.
                            if let Some((raw, line, col)) = label_name.take() {
                                self.require_valid((line, col))?;
                                let qualified = self.qualify_name(&raw);
                                self.labels.insert(qualified, self.current_addr);
                            }
                            self.parse_loc_directive(directive_pair)?;
                            if let Some(digit) = pending_local.take() {
                                self.record_local_label(digit, SymbolType::Constant(0), false);
                            }
                        }
                        Rule::greg_directive => {
                            // GREG was already processed in first pass. The
                            // two-operand memory form's base-address search
                            // bounds itself to the GREGs seen by this point
                            // in source order, so pass 2 replays the count.
                            self.greg_inits_seen += 1;
                            if let Some((raw, _, _)) = label_name.take() {
                                let qualified = self.qualify_name(&raw);
                                if !self.symbols.contains_key(&qualified) {
                                    return Err(format!(
                                        "Internal error: GREG label '{}' not found in symbols from first pass",
                                        qualified
                                    ));
                                }
                            }
                            if let Some(digit) = pending_local.take() {
                                self.record_local_label(digit, SymbolType::Constant(0), false);
                            }
                        }
                        Rule::is_directive => {
                            self.parse_is_directive(directive_pair, false)?;
                        }
                        Rule::prefix_directive => {
                            self.parse_prefix_directive(directive_pair)?;
                        }
                        Rule::local_directive => {}
                        Rule::bspec_directive => {
                            self.in_special_mode = true;
                        }
                        Rule::espec_directive => {
                            self.in_special_mode = false;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some((instruction, item)) = inst {
            let size = Self::instruction_size(&instruction);
            debug!(inst = ?instruction, addr = format!("0x{:X}", self.current_addr), size, "Added instruction");
            self.record_debug_info(self.current_addr, line);
            self.instructions.push((self.current_addr, instruction));
            Self::require_addr(self.place_item(size), &self.current_filename, item)?;
        }

        // Standalone labels (no instruction or directive on the line)
        if let Some((raw, line, col)) = label_name {
            self.require_valid((line, col))?;
            let qualified = self.qualify_name(&raw);
            self.labels.insert(qualified, self.current_addr);
        }
        if let Some(digit) = pending_local {
            self.record_local_label(digit, SymbolType::Constant(0), false);
        }
        self.local_pending_digit = None;

        Ok(())
    }

    /// Record `addr`'s source location in the active translation unit:
    /// `line`, in the preprocessed text pest walked, is also the line the
    /// user wrote it on (`preprocess_debug` never changes a source's line
    /// count).
    fn record_debug_info(&mut self, addr: u64, line: usize) {
        let file = self.sources[self.current_unit_index].filename.clone();
        self.debug_info.insert(addr, SourceLoc { file, line });
    }
}
