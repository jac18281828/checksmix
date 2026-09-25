//! Tests for lexical conformance, comment/remark diagnostics and located error messages.

use super::*;

// ---- Lexical conformance (C9.2) ------------------------------------

/// One of the three diagnostics a dangling operator's abutting text may
/// raise -- an unterminated group, a missing blank, or an operator that
/// opens the remark -- rather than a specific one of them.
fn assert_abutting_text_is_rejected(source: &str) {
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm
        .parse()
        .err()
        .unwrap_or_else(|| panic!("expected an error for {source:?}, parse succeeded"));
    assert!(
        err.contains("separated from the statement by a blank")
            || err.contains("a remark cannot begin with")
            || err.contains("unterminated group"),
        "error for {source:?} does not name the remark rule: {err}"
    );
}

#[test]
fn test_semicolon_separates_two_statements_on_one_line() {
    let mut asm = MMixAssembler::new("SETL $1,1; ADD $1,$1,1", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
    assert_eq!(asm.instructions[1].1, MMixInstruction::ADDI(1, 1, 1));
}

#[test]
fn test_semicolon_needs_no_surrounding_blank() {
    let mut asm = MMixAssembler::new("SETL $1,1;ADD $1,$1,1", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
    assert_eq!(asm.instructions[1].1, MMixInstruction::ADDI(1, 1, 1));
}

#[test]
fn test_label_after_semicolon_is_defined_at_that_statements_address() {
    let mut asm = MMixAssembler::new("SETL $1,1;Here ADD $1,$1,1", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.labels.get("Here"), Some(&4));
}

#[test]
fn test_three_statements_on_one_line() {
    let mut asm = MMixAssembler::new("SETL $1,1;SETL $2,2;SETL $3,3", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 3);
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 2));
    assert_eq!(asm.instructions[2].1, MMixInstruction::SETL(3, 3));
}

#[test]
fn test_empty_statements_between_and_around_semicolons_parse() {
    for source in [";;", "SETL $1,1;"] {
        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
    }
}

#[test]
fn test_indented_leading_semicolon_starts_an_empty_statement_not_a_comment() {
    // Indentation means the whole-line comment rule doesn't cover this
    // `;`: it opens an empty first statement, and `SETL` is the second.
    // Under `;`-as-comment, this line assembled nothing at all.
    let mut asm = MMixAssembler::new("   ;SETL $1,1", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 1);
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
}

#[test]
fn test_percent_comment_wins_over_a_later_semicolon() {
    let mut asm = MMixAssembler::new("SETL $1,1 % note; ADD $1,$1,1", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 1);
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
}

#[test]
fn test_semicolon_inside_a_string_literal_is_ordinary_text() {
    let mut asm = MMixAssembler::new("BYTE \";\"", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 1);
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0x3B));
}

#[test]
fn test_semicolon_inside_a_char_literal_is_ordinary_text() {
    assert_first_instruction("SET $1,';'", MMixInstruction::SETL(1, 0x3B));
}

#[test]
fn test_whole_line_comment_openers_contribute_nothing() {
    for opener in [";", "*", "#", "/", "-"] {
        let source = format!("{opener} not a statement\nSETL $1,1");
        let mut asm = MMixAssembler::new(&source, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
        assert_eq!(asm.instructions.len(), 1, "for opener {opener:?}");
        assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 1));
    }
}

#[test]
fn test_column_one_colon_and_underscore_still_open_a_label() {
    let mut asm = MMixAssembler::new(":Foo HALT\n_Bar HALT", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    // `:Foo` opts out of (the empty) active prefix and is keyed at the
    // root without its colon.
    assert_eq!(asm.labels.get("Foo"), Some(&0));
    assert_eq!(asm.labels.get("_Bar"), Some(&4));
}

#[test]
fn test_line_starting_with_a_digit_still_parses() {
    assert_first_instruction("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3));
}

#[test]
fn test_indented_percent_comment_still_a_comment() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    % note\nSETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_leading_zero_literal_is_decimal_not_octal() {
    assert_first_instruction("SETL $1,0100", MMixInstruction::SETL(1, 100));
}

/// The error keeps the decimal reading of a leading-zero literal
/// (`-010` reads `-10`); a leading zero is never octal.
#[test]
fn test_negative_leading_zero_literal_is_an_error_reading_decimal() {
    assert_eq!(
        assemble_err("SET $1,-010"),
        "<test>:1:8: immediate operand -10 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
    );
}

#[test]
fn test_hex_literal_forms_unaffected_by_octal_removal() {
    assert_first_instruction("SET $1,0x10", MMixInstruction::SETL(1, 16));
    assert_first_instruction("SET $1,#10", MMixInstruction::SETL(1, 16));
}

#[test]
fn test_leading_zero_literal_with_a_single_trailing_digit() {
    assert_first_instruction("SETL $1,08", MMixInstruction::SETL(1, 8));
}

#[test]
fn test_remark_after_an_operand_is_ignored() {
    assert_first_instruction(
        "ADD $1,$2,$3 sum of the parts",
        MMixInstruction::ADD(1, 2, 3),
    );
    assert_first_instruction("SET $1,5 the answer", MMixInstruction::SETL(1, 5));
    assert_first_instruction("SET $1,(2 + 3) ) stray", MMixInstruction::SETL(1, 5));
}

#[test]
fn test_remark_after_an_empty_operand_list_is_ignored() {
    assert_first_instruction("HALT exit here", MMixInstruction::HALT);
}

#[test]
fn test_trailing_operator_after_a_blank_is_rejected_naming_the_rule() {
    for source in [
        "SETL $1,2 + 3",
        "SET $1,2 , 3",
        "SET $1,2 * 3",
        "SET $1,2 - 3",
        "SET $1,$2 $3",
        "SET $1,2 / 3",
    ] {
        assert!(assemble_err(source).contains("a remark cannot begin with"));
    }
}

#[test]
fn test_abutting_text_is_rejected() {
    for source in ["SETL $1,2abc", "SET $1,5)", "SET $1,3/", "SET $1,4//"] {
        assert_abutting_text_is_rejected(source);
    }
}

#[test]
fn test_division_inside_an_expression_or_group_is_untouched() {
    assert_first_instruction("SET $1,3/4", MMixInstruction::SETL(1, 0));
    assert_first_instruction("SET $1,(3 / 4)", MMixInstruction::SETL(1, 0));
    assert_first_instruction("SET $1,8/4", MMixInstruction::SETL(1, 2));
}

#[test]
fn test_whitespace_around_commas_in_operand_lists_still_parses() {
    assert_first_instruction_matches("TRAP 0, Time, 2", |inst| {
        matches!(inst, MMixInstruction::TRAP(0, _, 2))
    });
    assert_first_instruction("SETI $2, 10", MMixInstruction::SET(2, 10));
}

#[test]
fn test_unrecognized_opcode_after_a_label_is_rejected() {
    // `ADDx` fails `word_end`, so it never matches ADD; nothing follows
    // it but `a,b,1` (register aliases via IS), so `ADDx` reads as a
    // bare label. A label statement holds nothing but blanks and a
    // comment, so this is a syntax error -- but `ADDx` is a perfectly
    // valid label, and `a` alone is just as plausible a culprit as
    // `ADDx`, so the diagnostic prints the whole statement and leaves
    // the reader to place the fault, rather than guess a single word.
    let source = "a IS $1\nb IS $2\nADDx a,b,1";
    assert!(assemble_err(source).contains("unknown operation: ADDx a,b,1"));
}

#[test]
fn test_known_directive_missing_its_operand_is_not_unknown_operation() {
    // `IS`, `LOC` and `SET` are all real keywords; each is just
    // missing what must follow it. The branch that rejects a truly
    // unrecognized opcode must not fire here -- these are malformed,
    // not unknown -- so pest's own "expected ..." diagnostic surfaces
    // instead, the same shape base reports. `GREG`'s operand is
    // optional (an empty field holds 0), so `Foo GREG` assembles and
    // does not belong in this list.
    for source in ["Foo IS", "Foo LOC", "Foo SET"] {
        let err = assemble_err(source);
        assert!(
            !err.contains("unknown operation"),
            "{source:?} must not be misreported as an unknown operation: {err}"
        );
    }
}

#[test]
fn test_debug_directive_followed_by_a_semicolon_is_rejected() {
    // The closing quote must be followed by nothing but blanks, a `%`
    // comment, or end of line; `; HALT` fails that, so the preprocessor
    // leaves the line untouched and the statement -- `Main` plus the
    // unrecognized `debug "hi"` -- is printed whole rather than
    // silently dropped.
    let source = "\tLOC\t#100\nMain\tdebug \"hi\" ; HALT\n";
    assert!(assemble_err(source).contains("unknown operation: Main\tdebug \"hi\""));
}

#[test]
fn test_unclosed_group_check_ignores_parens_in_other_statements() {
    // "ADDx a,b" is unrecognized on its own -- independent of the
    // second statement's unclosed group in "(4+5" -- so the first
    // statement's own diagnostic must print its own text alone, not
    // borrow "unterminated group" from a sibling's parens that
    // whole-line scanning would have seen and this statement's own
    // (paren-free) text does not have.
    let err = assemble_err("ADDx a,b;SET $2,(4+5");
    assert!(
        err.contains("unknown operation: ADDx a,b"),
        "expected the first statement's own diagnostic, got: {err}"
    );
    assert!(
        !err.contains("unterminated group"),
        "must not borrow the second statement's unclosed group, got: {err}"
    );
}

#[test]
fn test_unknown_operation_statement_keeps_a_percent_inside_a_literal() {
    // A `%` inside a string or character literal is ordinary text,
    // not this release's comment opener -- the same rule `BYTE ";"`
    // relies on for `;`. Cutting the printed statement at the first
    // `%` anywhere, ignoring literal contents, truncates it mid-quote
    // and shows the reader an unterminated string they never wrote.
    let err = assemble_err(r#"ADDx "50%",b"#);
    assert!(
        err.contains(r#"unknown operation: ADDx "50%",b"#),
        "expected the literal's `%` to survive intact, got: {err}"
    );
    let err = assemble_err("ADDx '%',b");
    assert!(
        err.contains("unknown operation: ADDx '%',b"),
        "expected the literal's `%` to survive intact, got: {err}"
    );
    // A character literal's scan must consume exactly one character
    // plus its closing quote: it must not over-consume into whatever
    // follows just because that character happens to be a backslash.
    let err = assemble_err("ADDx '\\'\"50%\"");
    assert!(
        err.contains("unknown operation: ADDx '\\'\"50%\""),
        "expected both literals to survive intact, got: {err}"
    );
    let err = assemble_err("ADDx '\\''%'");
    assert!(
        err.contains("unknown operation: ADDx '\\''%'"),
        "expected both literals to survive intact, got: {err}"
    );
}

#[test]
fn test_unclosed_group_check_reaches_a_statement_after_a_valid_one() {
    // The first statement is fully valid ("Main HALT"), its own
    // trailing "note)" a balanced-looking but net-closing paren that
    // ignored commentary never checks; the second, past the `;`,
    // opens a group it never closes. A whole-line scan sums both
    // statements' parens together (0 opens, 1 close, then 1 open) to
    // a NET-BALANCED total and misses the real problem entirely;
    // scoped to its own statement, the second statement's own text
    // alone is unclosed and is reported as such, proving both that
    // `segment_start` correctly advances past a successful first
    // statement and that the check is genuinely per-statement, not
    // whole-line.
    let err = assemble_err("Main HALT note);SET $2,(4+5");
    assert!(
        err.contains("<test>:1:21:") && err.contains("unterminated group"),
        "expected an unterminated-group diagnostic at column 21, got: {err}"
    );
}

#[test]
fn test_unclosed_paren_in_ignored_commentary_is_not_an_unterminated_group() {
    // The guard that catches a real unclosed group only applies to a
    // label that never resolved to an instruction or directive.
    // "HALT" fully matches on its own, so "note (see below" is
    // ordinary ignored commentary -- a stray `(` there is not a
    // group, and must not be reported as one.
    let mut asm = MMixAssembler::new("Main HALT note (see below", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[0].1, MMixInstruction::HALT);
}

#[test]
fn test_unclosed_group_check_ignores_parens_in_a_string_literal() {
    // A `(` quoted inside a string is data, not a group opener; the
    // unclosed-group check must not count it.
    let mut asm = MMixAssembler::new("Main SET $1,5 stray;BYTE \"(\"", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    assert_eq!(asm.instructions[1].1, MMixInstruction::BYTE(b'('));
}

#[test]
fn test_unclosed_group_check_ignores_parens_in_a_string_literal_reduced() {
    let mut asm = MMixAssembler::new("Main BYTE \"(\" stray", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(b'('));
}

// ---- Comment and ignored-remark map (C9.2) --------------------------
//
// Three independent mechanisms produce this map:
// `blank_whole_line_comments` decides, from column 1 alone, whether a
// line is a label candidate at all; the grammar's `;` separator decides
// where one statement ends and the next begins; and the remark check
// decides whether a statement's candidate remark is permitted
// commentary or a fault. A cell below is named for the mechanism that
// decides it.

// -- Column 1: every marker discards the line -------------------------
//
// Each payload abuts its operand (`2abc`), which is a syntax error if
// parsed at all, so a passing assertion proves the line was discarded,
// never merely tolerated.

#[test]
fn test_percent_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new("%SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_hash_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new("#SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_bang_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new("!SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_dot_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new(".SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_at_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new("@SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_semicolon_in_column_one_discards_the_line_as_a_label_rule_not_a_comment_rule() {
    // `;` opens no comment syntax of its own; it is discarded here only
    // because it is not a letter, digit, `:` or `_` -- the same reason
    // `#` and `*` are discarded on this row.
    let mut asm = MMixAssembler::new(";SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_asterisk_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new("*SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_slash_in_column_one_discards_the_line() {
    let mut asm = MMixAssembler::new("/SETL $1,2abc", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.instructions.is_empty());
}

#[test]
fn test_a_letter_colon_or_underscore_in_column_one_still_opens_a_label() {
    // The pairing that makes column 1 a label rule rather than a
    // comment rule: the set that opens a statement here is exactly the
    // set the blanking predicate keeps.
    let mut asm = MMixAssembler::new("Main HALT\n:Foo HALT\n_Bar HALT", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.labels.get("Main"), Some(&0));
    // `:Foo` names the same root symbol as `Foo`, keyed without the
    // colon.
    assert_eq!(asm.labels.get("Foo"), Some(&4));
    assert_eq!(asm.labels.get("_Bar"), Some(&8));
}

// -- Indented alone: a marker line between two real instructions -----
//
// Indentation puts the line past `blank_whole_line_comments`'s reach --
// that mechanism decides from column 1 alone. What happens to the line
// instead is decided by pest's `COMMENT` for `%`, the grammar's `;`
// separator, and the remark check for `*`, `/` and no marker. `#`, `!`,
// `.` and `@` carry a `; SETL $2,2` tail and assert the second
// statement still assembles -- without the tail these assertions would
// stay green even if the marker became a true comment character.

#[test]
fn test_percent_indented_alone_is_a_comment_the_tailed_statement_is_lost() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    % note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 1);
}

#[test]
fn test_hash_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    # note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_bang_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    ! note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_dot_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    . note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_at_indented_alone_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    @ note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_semicolon_indented_alone_opens_an_empty_statement_then_an_unknown_operation() {
    // The indented `;` opens an empty first statement, per the
    // grammar's `;` separator, not a comment; the prose after it is
    // then read as its own statement, a bare word read as a label with
    // text trailing it.
    let err = assemble_err("SETL $1,1\n    ; note text\nSETL $3,3");
    assert!(
        err.contains("unknown operation: note text"),
        "expected the prose after the indented `;` to be an unknown \
             operation, got: {err}"
    );
}

#[test]
fn test_semicolon_indented_alone_lone_word_defines_a_label() {
    // a lone word after ';' is defined as a label
    let mut asm = MMixAssembler::new("SETL $1,1\n    ; counter\nSET $2,counter", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.labels.get("counter"), Some(&4));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 4));
}

#[test]
fn test_semicolon_indented_alone_defines_an_is_constant() {
    let mut asm = MMixAssembler::new("SETL $1,1\n    ; offset IS 8\nSET $2,offset", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 8));
}

#[test]
fn test_asterisk_indented_alone_is_rejected_as_a_dropped_operator() {
    // No statement precedes this line's candidate remark, so a failed
    // remark reports an unknown operation, not a remark diagnostic.
    assert!(
        assemble_err("SETL $1,1\n    * note text\nSETL $3,3")
            .contains("unknown operation: * note text",)
    );
}

#[test]
fn test_slash_indented_alone_is_rejected_as_a_dropped_operator() {
    assert!(
        assemble_err("SETL $1,1\n    / note text\nSETL $3,3")
            .contains("unknown operation: / note text",)
    );
}

#[test]
fn test_no_marker_indented_alone_is_an_unknown_operation() {
    // No marker at all: the indented prose's first word reads as a
    // label, and the second word is text a label statement cannot
    // carry, so together they are an unknown operation.
    assert!(
        assemble_err("SETL $1,1\n    note text\nSETL $3,3")
            .contains("unknown operation: note text",)
    );
}

// -- Trailing: the same run after a complete statement ---------------
//
// The trailing `;` cell (four outcomes) is pinned separately below; it
// is not one of these.

#[test]
fn test_percent_wins_over_a_later_semicolon_dropping_the_second_statement() {
    // `%` beats a later `;` because it is a pest implicit comment,
    // consumed before the `;`-loop ever runs.
    let mut asm = MMixAssembler::new("SETL $1,1 % note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 1);
}

#[test]
fn test_hash_trailing_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1 # note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_bang_trailing_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1 ! note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_dot_trailing_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1 . note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_at_trailing_is_ignored_the_tailed_statement_still_assembles() {
    let mut asm = MMixAssembler::new("SETL $1,1 @ note; SETL $2,2", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_asterisk_trailing_is_rejected_as_a_dropped_operator() {
    assert!(assemble_err("SETL $1,1 * note").contains("a remark cannot begin with"));
}

#[test]
fn test_slash_trailing_is_rejected_as_a_dropped_operator() {
    assert!(assemble_err("SETL $1,1 / note").contains("a remark cannot begin with"));
}

// -- The three ambiguities that disqualify a remark --------------------

#[test]
fn test_abutting_remark_must_be_separated_by_a_blank() {
    assert!(assemble_err("SETL $1,2abc").contains("separated from the statement by a blank"));
}

#[test]
fn test_operator_led_remark_errors_for_every_operator_char() {
    for c in [',', '+', '-', '*', '/', '~', '&', '|', '^', '<', '>', '$'] {
        assert!(assemble_err(&format!("SETL $1,2 {c} 3")).contains("a remark cannot begin with"));
    }
}

#[test]
fn test_digit_led_remark_errors_as_a_dropped_separator() {
    assert!(assemble_err("HALT 2 apples").contains("a remark cannot begin with"));
}

#[test]
fn test_remark_opening_with_a_letter_is_ignored() {
    assert_first_instruction(
        "ADD $1,$2,$3 sum of the parts",
        MMixInstruction::ADD(1, 2, 3),
    );
}

#[test]
fn test_remark_opening_with_underscore_or_colon_is_ignored_too() {
    assert_first_instruction("ADD $1,$2,$3 _underscore", MMixInstruction::ADD(1, 2, 3));
    assert_first_instruction("ADD $1,$2,$3 :colon", MMixInstruction::ADD(1, 2, 3));
}

// -- The trailing `;`: four outcomes -----------------------------------

#[test]
fn test_trailing_semicolon_lone_word_defines_a_label() {
    // a lone word after ';' is defined as a label
    let mut asm = MMixAssembler::new("SET $1,0 ; counter\nSET $2,counter", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.labels.get("counter"), Some(&4));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 4));

    // Control: behind a real comment, `counter` is never defined.
    assert!(
        assemble_err("SET $1,0 % counter\nSET $2,counter").contains("Undefined symbol: counter",)
    );
}

#[test]
fn test_trailing_semicolon_lone_word_defines_an_is_constant() {
    // IS matches in upper case only; lower-case `is` is not the
    // directive.
    let mut asm = MMixAssembler::new("SET $1,0 ; offset IS 8\nSET $2,offset", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(2, 8));

    // Control: behind a real comment, `offset` is never defined.
    assert!(
        assemble_err("SET $1,0 % offset IS 8\nSET $2,offset").contains("Undefined symbol: offset",)
    );
}

#[test]
fn test_trailing_semicolon_prose_that_reads_as_a_bad_expression_is_an_error() {
    // IS matches in upper case only.
    assert!(assemble_err("SET $1,0 ; this IS invalid").contains("Undefined symbol: invalid"));
}

#[test]
fn test_trailing_semicolon_prose_that_reads_as_an_unknown_operation_is_an_error() {
    assert!(
        assemble_err("SET $1,0 ; set the counter").contains("unknown operation: set the counter",)
    );
}

// -- The shield: `#` carries no comment meaning of its own -----------

#[test]
fn test_hash_shield_ignores_arbitrary_trailing_prose() {
    assert_first_instruction(
        "SET $1,0 # anything at all here",
        MMixInstruction::SETL(1, 0),
    );
}

#[test]
fn test_bare_trailing_hash_assembles_like_no_remark_at_all() {
    assert_first_instruction("SET $1,0 #", MMixInstruction::SETL(1, 0));
}

#[test]
fn test_hash_shield_ignores_digit_led_prose_that_would_otherwise_error() {
    assert_first_instruction("SET $1,0 # 2 apples", MMixInstruction::SETL(1, 0));
    // Without the shield, a digit-led run is the deliberate exception:
    // an error, not a warning.
    assert!(assemble_err("SET $1,0 2 apples").contains("a remark cannot begin with"));
}

// -- The three remaining pins -----------------------------------------

#[test]
fn test_percent_inside_a_literal_is_ordinary_text_not_a_comment() {
    let mut asm = MMixAssembler::new(r#"BYTE "50%""#, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(
        asm.instructions
            .iter()
            .map(|(_, i)| i.clone())
            .collect::<Vec<_>>(),
        vec![
            MMixInstruction::BYTE(b'5'),
            MMixInstruction::BYTE(b'0'),
            MMixInstruction::BYTE(b'%'),
        ]
    );

    // The same literal survives intact in the unknown-operation
    // diagnostic rather than truncating at the `%`.
    assert!(assemble_err(r#"ADDx "50%",b"#).contains(r#"unknown operation: ADDx "50%",b"#));
}

#[test]
fn test_at_is_a_valid_operand_but_bang_and_dot_have_no_grammar_token() {
    assert_first_instruction("SET $1,@", MMixInstruction::SETL(1, 0));
    assert!(assemble_err("SET $1,!").contains("unknown operation: SET $1,!"));
    assert!(assemble_err("SET $1,.").contains("unknown operation: SET $1,."));
}

// ---- Remark diagnostics: full message, every position row ----

#[test]
fn test_remark_ambiguity_messages_pin_position_and_text() {
    assert_eq!(
        assemble_err("SETL $1,2abc"),
        "<test>:1:10: syntax error: a remark must be separated from the statement by a blank"
    );
    assert_eq!(
        assemble_err("HALT+3"),
        "<test>:1:5: syntax error: a remark must be separated from the statement by a blank"
    );
    assert_eq!(
        assemble_err("SETL $1,2 + 3"),
        "<test>:1:11: syntax error: a remark cannot begin with `+` — it reads as part of \
             the statement; start a comment with `%`"
    );
    assert_eq!(
        assemble_err("HALT + 3"),
        "<test>:1:6: syntax error: a remark cannot begin with `+` — it reads as part of \
             the statement; start a comment with `%`"
    );
    assert_eq!(
        assemble_err("SET $1,2 , 3"),
        "<test>:1:10: syntax error: a remark cannot begin with `,` — it reads as part of \
             the statement; start a comment with `%`"
    );
    assert_eq!(
        assemble_err("HALT 2 apples"),
        "<test>:1:6: syntax error: a remark cannot begin with `2` — it reads as part of \
             the statement; start a comment with `%`"
    );
}

#[test]
fn test_unknown_operation_with_no_statement_ahead_pins_position_and_text() {
    assert_eq!(
        assemble_err("9Bar\tSETL\t$1,1"),
        "<test>:1:1: syntax error: unknown operation: 9Bar\tSETL\t$1,1"
    );
    assert_eq!(
        assemble_err("SETL $1,1;9foo"),
        "<test>:1:11: syntax error: unknown operation: 9foo"
    );
    assert_eq!(
        assemble_err("SETL $1,1 ;+3"),
        "<test>:1:12: syntax error: unknown operation: +3"
    );
    assert_eq!(
        assemble_err("\tSETL $1,1\n    * note"),
        "<test>:2:5: syntax error: unknown operation: * note"
    );
    assert_eq!(
        assemble_err("  2 apples"),
        "<test>:1:3: syntax error: unknown operation: 2 apples"
    );
}

// ---- Locations: file:line:col on every assembler error ----------------

#[test]
fn test_symbol_redefined_names_file_line_and_column() {
    assert_eq!(
        assemble_err("X IS 1\nMain HALT;X IS 2\n"),
        "<test>:2:11: symbol 'X' redefined (first defined at <test>:1)"
    );
}

#[test]
fn test_predefined_symbol_redefined_after_use_names_file_line_and_column() {
    assert_eq!(
        assemble_err("Main SET $1,Fputs\nSetup HALT;Fputs IS 9\n"),
        "<test>:2:12: predefined symbol 'Fputs' redefined after its \
             value was used at <test>:1"
    );
}

#[test]
fn test_too_many_debug_directives_names_file_line_and_column() {
    let mut source = String::new();
    for _ in 0..256 {
        source.push_str("debug \"x\"\n");
    }
    source.push_str("L debug \"overflow\"\n");
    assert_eq!(
        assemble_err(&source),
        "<test>:257:3: error: too many `debug` directives in this \
             program; the string table holds at most 256"
    );
}

#[test]
fn test_bspec_unterminated_names_the_bspec_keywords_column() {
    assert_eq!(
        assemble_err("Main HALT\n\tBSPEC 1\nBYTE 1\n"),
        "<test>:2:2: syntax error: BSPEC has no matching ESPEC before end of input"
    );
}

#[test]
fn test_local_over_threshold_names_the_operands_column() {
    assert_eq!(
        assemble_err("G1 GREG 0\nLOCAL $254\nMain HALT\n"),
        "<test>:2:7: LOCAL $254 is not below the global threshold $254"
    );
}

#[test]
fn test_bspec_content_error_names_the_offending_opcodes_column() {
    assert_eq!(
        assemble_err("BSPEC 1\nMain HALT\nESPEC\n"),
        "<test>:2:6: syntax error: an instruction is not allowed inside BSPEC/ESPEC"
    );
}

#[test]
fn test_takes_no_label_names_the_labels_column() {
    assert_eq!(
        assemble_err("Main HALT;Foo ESPEC\n"),
        "<test>:1:11: syntax error: ESPEC takes no label"
    );
}

#[test]
fn test_bspec_does_not_nest_names_the_inner_keywords_column() {
    assert_eq!(
        assemble_err("BSPEC 1\n\tBSPEC 2\nESPEC\nESPEC\nMain HALT\n"),
        "<test>:2:2: syntax error: BSPEC does not nest"
    );
}

#[test]
fn test_bspec_operand_too_wide_names_the_operands_column() {
    assert_eq!(
        assemble_err("BSPEC #10000\nMain HALT\nESPEC\n"),
        "<test>:1:7: syntax error: BSPEC operand 65536 does not fit in two bytes"
    );
}

#[test]
fn test_espec_unmatched_names_the_espec_keywords_column() {
    assert_eq!(
        assemble_err("Main HALT\n\tESPEC\n"),
        "<test>:2:2: syntax error: ESPEC has no matching BSPEC"
    );
}

#[test]
fn test_include_cycle_names_the_including_files_own_include_line() {
    let reader = fixture_reader(vec![
        ("a.mms", "INCLUDE b.mms\n"),
        ("b.mms", "  INCLUDE a.mms\n"),
    ]);
    let err = MMixAssembler::resolve_includes(
        "INCLUDE a.mms\n",
        "driver.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap_err();
    assert_eq!(
        err,
        "b.mms:1:3: include cycle detected: a.mms -> b.mms -> a.mms"
    );
}

#[test]
fn test_include_unreadable_file_names_the_including_files_own_include_line() {
    let reader = fixture_reader(vec![]);
    let err = MMixAssembler::resolve_includes(
        "\n\tINCLUDE missing.mms\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap_err();
    assert_eq!(
        err,
        "root.mms:2:2: cannot read included file 'missing.mms': no such fixture file"
    );
}

#[test]
fn test_locate_fallback_prefixes_an_unlocated_error_and_passes_a_located_one_through() {
    let asm = MMixAssembler::new("", "<test>");
    assert_eq!(
        asm.locate_fallback("Empty instruction".to_string(), 3, 5),
        "<test>:3:5: Empty instruction"
    );
    let located = "<test>:1:1: symbol 'X' redefined (first defined at <test>:1)".to_string();
    assert_eq!(asm.locate_fallback(located.clone(), 3, 5), located);
}
