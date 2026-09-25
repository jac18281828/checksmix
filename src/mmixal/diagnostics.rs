//! Error and warning message construction: parse-error formatting, remark ambiguity, and statement-shape diagnostics.

use super::MMixAssembler;
use super::MMixalParser;
use super::Rule;

/// Why a candidate remark is mistakable for part of the statement rather
/// than commentary a reader could set apart.
enum RemarkAmbiguity {
    /// No blank separates the remark from the statement, so nothing marks
    /// where the statement ended.
    Abutting,
    /// The remark opens with a digit or one of the twelve
    /// [`MMixAssembler::REMARK_CONTINUATION_CHARS`]. A leading digit is a
    /// guard against a dropped operand separator, not a claim that an
    /// expression wanted it; a leading continuation character could extend
    /// the expression or operand list it follows.
    LeadingChar(char),
}

impl MMixAssembler {
    /// Prefix a statement's error with its own `{file}:{line}:{col}: ` when
    /// it names no location of its own. Every reachable error already names
    /// its own precise column before reaching here; only a grammar-invariant
    /// arm (unreachable from valid source) is still unlocated when a
    /// statement's dispatch returns it.
    pub(super) fn locate_fallback(&self, err: String, line: usize, col: usize) -> String {
        if err.starts_with(&format!("{}:", self.current_filename)) {
            err
        } else {
            format!("{}:{}:{}: {}", self.current_filename, line, col, err)
        }
    }

    /// Format Pest parse errors in a user-friendly way. Pest reports a line
    /// in the preprocessed text, which is also the line the user wrote it
    /// on (`preprocess_debug` never changes a source's line count).
    /// `source` is the preprocessed text the failed parse walked: an
    /// unterminated group reports as pest expecting more operator content
    /// (`weak_op`/`strong_op`/`group_ws`), never a missing `)`, so naming it
    /// takes a look at the source line rather than at pest's own positives.
    pub(super) fn format_parse_error(
        error: &pest::error::Error<Rule>,
        filename: &str,
        source: &str,
    ) -> String {
        use pest::error::LineColLocation;

        let (line, col) = match error.line_col {
            LineColLocation::Pos((l, c)) => (l, c),
            LineColLocation::Span((l, c), _) => (l, c),
        };

        if let pest::error::ErrorVariant::ParsingError { positives, .. } = &error.variant
            && Self::expects_more_group_content(positives)
            && Self::line_has_unclosed_paren(source, line)
        {
            return format!(
                "{}:{}:{}: syntax error: unterminated group",
                filename, line, col
            );
        }

        let expected_msg = Self::describe_expected(&error.variant);
        format!(
            "{}:{}:{}: syntax error: expected {}",
            filename, line, col, expected_msg
        )
    }

    /// Render a pest error's `positives` (or custom message) as the
    /// user-facing "expected ..." fragment. Shared by `format_parse_error`
    /// and `format_reparse_error`, which builds the same kind of line from
    /// a sub-parse pest never attempted at the top level.
    fn describe_expected(variant: &pest::error::ErrorVariant<Rule>) -> String {
        match variant {
            pest::error::ErrorVariant::ParsingError { positives, .. } => {
                if positives.is_empty() {
                    "valid MMIX instruction or directive".to_string()
                } else {
                    // Try to make the expected rules more user-friendly
                    let friendly: Vec<String> = positives
                        .iter()
                        .map(|r| match r {
                            Rule::instruction => "instruction".to_string(),
                            Rule::directive => "directive".to_string(),
                            Rule::directive_is => "IS directive (symbol definition)".to_string(),
                            Rule::directive_loc => "LOC directive".to_string(),
                            Rule::expr => "number or expression".to_string(),
                            Rule::global_id => "label or symbol name".to_string(),
                            Rule::identifier => "label or symbol name".to_string(),
                            // A data-list item holding no string parses through
                            // the same grammar as an instruction operand; its
                            // own rule names must never leak into a diagnostic
                            // that `expr` would report identically.
                            Rule::data_term => "data_value".to_string(),
                            Rule::data_primary => "primary".to_string(),
                            Rule::data_group_primary => "group_primary".to_string(),
                            _ => format!("{:?}", r),
                        })
                        .collect();

                    if friendly.len() == 1 {
                        friendly[0].clone()
                    } else {
                        format!("one of: {}", friendly.join(", "))
                    }
                }
            }
            pest::error::ErrorVariant::CustomError { message } => message.clone(),
        }
    }

    /// Format a re-parse `error` (from testing a substring of `source` in
    /// isolation) as a diagnostic in `source`'s own coordinates.
    /// `base_offset` is where that substring began in `source`; pest's own
    /// `error` reports a position relative to the substring, not `source`.
    fn format_reparse_error(
        error: &pest::error::Error<Rule>,
        filename: &str,
        source: &str,
        base_offset: usize,
    ) -> String {
        let sub_pos = match error.location {
            pest::error::InputLocation::Pos(p) => p,
            pest::error::InputLocation::Span((s, _)) => s,
        };
        let (line, col) = pest::Position::new(source, base_offset + sub_pos)
            .map(|p| p.line_col())
            .unwrap_or((1, 1));
        let expected_msg = Self::describe_expected(&error.variant);
        format!("{filename}:{line}:{col}: syntax error: expected {expected_msg}")
    }

    /// True when pest's positives suggest it was still trying to extend an
    /// expression -- the shape an unterminated group's failure takes.
    fn expects_more_group_content(positives: &[Rule]) -> bool {
        positives
            .iter()
            .any(|r| matches!(r, Rule::weak_op | Rule::strong_op | Rule::group_ws))
    }

    /// True when `line` (1-based, in `source`) has more `(` than `)`.
    /// Used only by `format_parse_error`, where pest has already failed to
    /// parse the whole input and there is no statement span to scope to.
    fn line_has_unclosed_paren(source: &str, line: usize) -> bool {
        let Some(text) = source.lines().nth(line - 1) else {
            return false;
        };
        Self::segment_has_unclosed_paren(text)
    }

    /// True when `segment` has more `(` than `)`, skipping the contents of
    /// any string or character literal so a quoted `(` or `)` is never
    /// counted. `diagnose_unrecognized_opcode` scopes `segment` to one
    /// statement's own span, never the whole physical line, so a sibling
    /// statement's parens (on either side of a `;`) can't be blamed on this
    /// one.
    fn segment_has_unclosed_paren(segment: &str) -> bool {
        let mut depth: i32 = 0;
        let mut chars = segment.char_indices();
        while let Some((_, ch)) = chars.next() {
            if Self::skip_literal(&mut chars, ch) {
                continue;
            }
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        depth > 0
    }

    /// Advances `chars` past a string or character literal `ch` opens --
    /// a `"` consumes to the next `"`, a `'` consumes exactly one character
    /// plus the closing quote -- returning whether `ch` opened one. Shared
    /// by every scan that walks a segment's raw text ignoring literal
    /// contents, so a quoted `(`, `)` or `%` is never mistaken for this
    /// release's own syntax.
    fn skip_literal(chars: &mut std::str::CharIndices, ch: char) -> bool {
        match ch {
            '"' => {
                for (_, c) in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                }
                true
            }
            '\'' => {
                chars.next(); // the character
                chars.next(); // the closing quote, if present
                true
            }
            _ => false,
        }
    }

    /// Byte offset of the first `%` in `segment` that sits outside a
    /// string or character literal, or `segment.len()` when there is
    /// none. The unknown-operation diagnostic prints `segment[..cut]` as
    /// the offending statement, so a literal's own `%` (`BYTE "50%"`) is
    /// never mistaken for this release's comment opener and truncated
    /// mid-literal.
    fn segment_comment_start(segment: &str) -> usize {
        let mut chars = segment.char_indices();
        while let Some((idx, ch)) = chars.next() {
            if Self::skip_literal(&mut chars, ch) {
                continue;
            }
            if ch == '%' {
                return idx;
            }
        }
        segment.len()
    }

    /// Characters that could extend a bare expression or an operand list.
    /// A remark opening with one of these, right after the blank that ends
    /// the operand field, would read as continuing the statement rather
    /// than as commentary -- dropping an operand in silence if it were
    /// ignored.
    const REMARK_CONTINUATION_CHARS: [char; 12] =
        [',', '+', '-', '*', '/', '~', '&', '|', '^', '<', '>', '$'];

    /// `None` when `text`, preceded by `blank_before`, reads as a remark;
    /// `Some` naming the ambiguity otherwise. Rule 1 -- `EXPR` is greedy -- has
    /// already run by the time this is called: `text` is only ever what
    /// `EXPR` left behind.
    fn remark_ambiguity(text: &str, blank_before: bool) -> Option<RemarkAmbiguity> {
        if !blank_before {
            return Some(RemarkAmbiguity::Abutting);
        }
        let first = text.chars().next()?;
        (first.is_ascii_digit() || Self::REMARK_CONTINUATION_CHARS.contains(&first))
            .then_some(RemarkAmbiguity::LeadingChar(first))
    }

    /// Formats the diagnostic for `ambiguity`, at `filename:line:col`.
    fn format_remark_ambiguity(
        ambiguity: &RemarkAmbiguity,
        filename: &str,
        line: usize,
        col: usize,
    ) -> String {
        match ambiguity {
            RemarkAmbiguity::Abutting => format!(
                "{filename}:{line}:{col}: syntax error: a remark must be separated from the \
                 statement by a blank"
            ),
            RemarkAmbiguity::LeadingChar(c) => format!(
                "{filename}:{line}:{col}: syntax error: a remark cannot begin with `{c}` — it \
                 reads as part of the statement; start a comment with `%`"
            ),
        }
    }

    /// Formats "unknown operation: {statement}" for a bare word in the OP
    /// field, or for a candidate remark with no statement ahead of it:
    /// `segment`, its `%` comment stripped (raw inside the atomic `remark`
    /// capture, so the grammar never trims it) and the blanks an indented
    /// or post-`;` statement carries trimmed off, is the statement the
    /// reader wrote.
    fn unknown_operation_error(segment: &str, filename: &str, line: usize, col: usize) -> String {
        let statement = segment[..Self::segment_comment_start(segment)].trim();
        format!("{filename}:{line}:{col}: syntax error: unknown operation: {statement}")
    }

    /// Rule 2: what `EXPR` (rule 1) left behind is a remark unless it is
    /// mistakable for part of the statement. `has_statement` is false when
    /// no `Rule::statement` preceded `pair` in its segment; a remark
    /// presupposes a statement to follow, so an ambiguity there reports an
    /// unknown operation instead of a remark diagnostic, and `segment_start` bounds that statement
    /// text to this segment alone -- never a sibling statement's text on
    /// the same line.
    pub(super) fn check_remark(
        pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
        has_statement: bool,
    ) -> Result<(), String> {
        let text = pair.as_str();
        if text.is_empty() {
            return Ok(());
        }
        let (line, col) = pair.line_col();
        let start = pair.as_span().start();
        let blank_before = start > 0 && matches!(source.as_bytes()[start - 1], b' ' | b'\t');

        let Some(ambiguity) = Self::remark_ambiguity(text, blank_before) else {
            return Ok(());
        };

        if !has_statement {
            let segment = &source[segment_start..pair.as_span().end()];
            return Err(Self::unknown_operation_error(segment, filename, line, col));
        }
        Err(Self::format_remark_ambiguity(
            &ambiguity, filename, line, col,
        ))
    }

    /// A bare label whose statement position holds a word naming no
    /// instruction or directive -- not an ambiguity test on a remark, but a
    /// diagnosis of that word: an unclosed group that swallowed a real
    /// instruction whole, a known mnemonic or directive missing or
    /// malformed its operand, or, failing both, the statement itself so the
    /// reader can place the fault. Naming which specific word is at fault
    /// is ambiguous in general (a valid label followed by a bad mnemonic
    /// and a bad mnemonic swallowed as a label are the same shape).
    /// `segment_start` bounds the unclosed-group check to this statement's
    /// own text, from wherever it began (the line's start, or just past the
    /// previous `;`) to `pair`'s own end -- never a sibling statement's text
    /// on the same line.
    pub(super) fn diagnose_unrecognized_opcode(
        pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> Result<(), String> {
        let text = pair.as_str();
        if text.is_empty() {
            return Ok(());
        }
        let (line, col) = pair.line_col();

        // An unclosed group makes every instruction alternative fail deep
        // inside its operand, so `statement` falls back to reading the
        // mnemonic as a bare label and leaves the rest for `remark` --
        // trading the real problem for a confusing one unless caught here.
        // A comment after a *successful* match never reaches this
        // function, so a stray `(` in commentary is never mistaken for an
        // unterminated group.
        let segment = &source[segment_start..pair.as_span().end()];
        if Self::segment_has_unclosed_paren(segment) {
            return Err(format!(
                "{filename}:{line}:{col}: syntax error: unterminated group"
            ));
        }
        let remark_start = pair.as_span().start();
        if let Some((error, base_offset)) =
            Self::recognized_keyword_error(text, remark_start, segment, segment_start)
        {
            return Err(Self::format_reparse_error(
                &error,
                filename,
                source,
                base_offset,
            ));
        }
        Err(Self::unknown_operation_error(segment, filename, line, col))
    }

    /// Directive keyword-only rules, paired with the full directive rule
    /// that gives a meaningful "missing/malformed operand" diagnostic once
    /// the keyword itself is confirmed present. `directive_is` isn't here:
    /// it is the one directive whose own grammar folds in the preceding
    /// label, so it needs the whole segment, not `remark_text` alone.
    const DIRECTIVE_KEYWORD_RULES: [(Rule, Rule); 10] = [
        (Rule::directive_loc, Rule::loc_directive),
        (Rule::directive_greg, Rule::greg_directive),
        (Rule::directive_prefix, Rule::prefix_directive),
        (Rule::directive_byte, Rule::data_directive),
        (Rule::directive_wyde, Rule::data_directive),
        (Rule::directive_tetra, Rule::data_directive),
        (Rule::directive_octa, Rule::data_directive),
        (Rule::directive_local, Rule::local_directive),
        (Rule::directive_bspec, Rule::bspec_directive),
        (Rule::directive_espec, Rule::espec_directive),
    ];

    /// When `remark_text` opens with a recognized mnemonic or directive
    /// keyword, re-parse the construct that keyword belongs to and return
    /// its own error, plus the byte offset (into the original source) that
    /// error's position is relative to -- so a known keyword with a
    /// missing or malformed operand (`Foo IS`, `Foo GREG`, `Foo SET`)
    /// reports what pest actually expected there, never a made-up
    /// "unknown operation". Every alternative in `instruction` opens with
    /// a literal mnemonic, so a failure at position 0 there means none
    /// matched even a prefix; a failure past position 0 means a mnemonic
    /// matched and only the operand is missing or malformed. `segment`
    /// (from `segment_start`) is `remark_text`'s own statement, label
    /// included, the only span `directive_is` can be re-parsed against,
    /// since its grammar requires that label as part of the rule itself.
    fn recognized_keyword_error(
        remark_text: &str,
        remark_start: usize,
        segment: &str,
        segment_start: usize,
    ) -> Option<(pest::error::Error<Rule>, usize)> {
        use pest::Parser;

        if MMixalParser::parse(Rule::directive_is, remark_text).is_ok()
            && let Err(e) = MMixalParser::parse(Rule::directive, segment)
        {
            return Some((e, segment_start));
        }
        for (keyword, full_rule) in Self::DIRECTIVE_KEYWORD_RULES {
            if MMixalParser::parse(keyword, remark_text).is_ok()
                && let Err(e) = MMixalParser::parse(full_rule, remark_text)
            {
                return Some((e, remark_start));
            }
        }
        if let Err(e) = MMixalParser::parse(Rule::instruction, remark_text) {
            let pos = match e.location {
                pest::error::InputLocation::Pos(p) => p,
                pest::error::InputLocation::Span((s, _)) => s,
            };
            if pos > 0 {
                return Some((e, remark_start));
            }
        }
        None
    }

    /// True when `pair` (a `Rule::statement`) is a bare label with no
    /// instruction or directive attached -- the one shape whose candidate
    /// remark `diagnose_unrecognized_opcode` diagnoses rather than reading
    /// as a remark.
    pub(super) fn statement_is_label_only(pair: &pest::iterators::Pair<Rule>) -> bool {
        let mut inner = pair.clone().into_inner();
        matches!(
            inner.next().map(|p| p.as_rule()),
            Some(Rule::label_def | Rule::local_label_def)
        ) && inner.next().is_none()
    }

    /// Whether the line whose `Rule::line` pair starts at byte `start` in
    /// `source` opens with a blank or a tab. `program` is not atomic, so
    /// pest already skipped the run of blanks, tabs and carriage returns
    /// `WHITESPACE` consumes ahead of the pair's own span; walking that run
    /// back from `start` to the previous newline (or the start of file)
    /// finds the line's own first byte, with no rescan of the line's text
    /// beyond its own leading run.
    pub(super) fn line_opens_indented(source: &str, start: usize) -> bool {
        let bytes = source.as_bytes();
        let mut first = start;
        while first > 0 && matches!(bytes[first - 1], b' ' | b'\t' | b'\r') {
            first -= 1;
        }
        first < bytes.len() && matches!(bytes[first], b' ' | b'\t')
    }

    /// The label-shaped pair that opens `stmt_pair`'s match, and the pair
    /// right after it if the label was not the whole match: `label_def` or
    /// `local_label_def` from `statement`'s own label alternatives, paired
    /// with the instruction or directive that followed; or the local label
    /// / global id `is_directive` reads ahead of its own `IS` keyword --
    /// the one directive whose grammar folds a label into itself -- paired
    /// with `IS`'s own keyword pair. `(None, _)` when `stmt_pair` matched
    /// via `instruction` or a directive that carries no label.
    fn statement_label_split<'i>(
        stmt_pair: &pest::iterators::Pair<'i, Rule>,
    ) -> (
        Option<pest::iterators::Pair<'i, Rule>>,
        Option<pest::iterators::Pair<'i, Rule>>,
    ) {
        let mut inner = stmt_pair.clone().into_inner();
        let Some(first) = inner.next() else {
            return (None, None);
        };
        match first.as_rule() {
            Rule::label_def | Rule::local_label_def => {
                let after = inner.next();
                (Some(first), after)
            }
            Rule::directive => {
                let Some(d) = first.into_inner().next() else {
                    return (None, None);
                };
                if d.as_rule() == Rule::is_directive {
                    let mut is_inner = d.into_inner();
                    let label = is_inner.next();
                    let after = is_inner.next();
                    (label, after)
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        }
    }

    /// The label-shaped pair that opens `stmt_pair`'s match, if any. See
    /// `statement_label_split`.
    pub(super) fn statement_label_pair<'i>(
        stmt_pair: &pest::iterators::Pair<'i, Rule>,
    ) -> Option<pest::iterators::Pair<'i, Rule>> {
        Self::statement_label_split(stmt_pair).0
    }

    /// The pair right after `stmt_pair`'s opening label, if the label was
    /// not the whole match. See `statement_label_split`.
    fn statement_after_label<'i>(
        stmt_pair: &pest::iterators::Pair<'i, Rule>,
    ) -> Option<pest::iterators::Pair<'i, Rule>> {
        Self::statement_label_split(stmt_pair).1
    }

    /// An indented line has no label field, so a statement that opened by
    /// reading one is an unknown operation even when a real instruction or
    /// directive followed it (`\tFoo SET $2,9`, `\t2H JMP 2F`, an indented
    /// `Foo IS 5`). `{col}` lands on whatever
    /// followed the label -- the instruction, the directive, or `IS`'s own
    /// keyword -- matching where the same diagnostic already lands for a
    /// bare word followed by trailing text.
    pub(super) fn indented_label_statement_error(
        stmt_pair: &pest::iterators::Pair<Rule>,
        label_pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> String {
        let segment = &source[segment_start..stmt_pair.as_span().end()];
        let (line, col) = Self::statement_after_label(stmt_pair)
            .map(|p| p.line_col())
            .unwrap_or_else(|| label_pair.line_col());
        Self::unknown_operation_error(segment, filename, line, col)
    }

    /// An indented line's lone word, with nothing at all following it, is
    /// an unknown operation rather than a silently defined label. `{col}`
    /// is the word itself, since nothing follows it to point at; `{statement}`
    /// is formed the way the opcode-field diagnosis forms it -- the source
    /// text from the line's start through `remark_pair`'s end, trimmed --
    /// since `label_pair`'s own span can carry trailing blanks that a
    /// following optional token left unconsumed.
    pub(super) fn indented_bare_label_error(
        label_pair: &pest::iterators::Pair<Rule>,
        remark_pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> String {
        let segment = &source[segment_start..remark_pair.as_span().end()];
        let (line, col) = label_pair.line_col();
        Self::unknown_operation_error(segment, filename, line, col)
    }

    /// `SAVE` and `UNSAVE` are the two of the seven bare mnemonics the
    /// reference's empty-field-is-0 rule does not cover: `SAVE` takes
    /// exactly two operands, so the one implicit operand an empty field
    /// gives is not enough to assemble it, and `UNSAVE`'s one-operand form
    /// reads that implicit operand as a register, which 0 is not.
    /// Both are errors in every position a bare word can
    /// appear -- indented, column 1, or after `;` -- never a silently
    /// defined label. `None` when `label_pair`'s name is neither. `SAVE`'s
    /// diagnostic names the statement as written -- `{statement}`, formed
    /// as `indented_bare_label_error` forms it -- so a trailing colon
    /// shows; `UNSAVE`'s does not name the word at all.
    pub(super) fn bare_reserved_mnemonic_error(
        label_pair: &pest::iterators::Pair<Rule>,
        remark_pair: &pest::iterators::Pair<Rule>,
        source: &str,
        filename: &str,
        segment_start: usize,
    ) -> Option<String> {
        let name = label_pair
            .clone()
            .into_inner()
            .next()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
        let (line, col) = label_pair.line_col();
        match name.as_str() {
            "SAVE" => {
                let segment = &source[segment_start..remark_pair.as_span().end()];
                Some(Self::unknown_operation_error(segment, filename, line, col))
            }
            "UNSAVE" => Some(format!(
                "{filename}:{line}:{col}: pure value 0 cannot be used where a register is required"
            )),
            _ => None,
        }
    }
}
