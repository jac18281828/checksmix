//! Tests for the location counter and the top of the address space.

use super::*;

// ---- the address space ends at #FFFFFFFFFFFFFFFF ------------------

#[test]
fn test_swym_at_the_top_of_memory_assembles() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions,
        vec![(0xFFFFFFFFFFFFFFFC, MMixInstruction::SWYM(0, 0, 0))]
    );
}

#[test]
fn test_octa_at_the_top_of_memory_assembles() {
    let source = " LOC #FFFFFFFFFFFFFFF8\n OCTA 1\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions,
        vec![(0xFFFFFFFFFFFFFFF8, MMixInstruction::OCTA(1))]
    );
}

#[test]
fn test_two_bytes_fill_the_last_two_addresses_exactly() {
    let source = " LOC #FFFFFFFFFFFFFFFE\n BYTE 1\n BYTE 2\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions,
        vec![
            (0xFFFFFFFFFFFFFFFE, MMixInstruction::BYTE(1)),
            (0xFFFFFFFFFFFFFFFF, MMixInstruction::BYTE(2)),
        ]
    );
}

#[test]
fn test_an_item_after_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:2: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_an_item_extending_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFF\n BYTE 1,2\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:2:2: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_alignment_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFD\n SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:2:2: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_at_symbol_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd IS @\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:8: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_standalone_label_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_loc_after_past_end_restores_a_valid_counter() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n LOC #100\nMain SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Main"), Some(&0x100));
}

#[test]
fn test_past_end_resets_between_the_two_passes() {
    // Pass 1 ends past the end (the second SWYM fills the last byte);
    // pass 2 must start over clean rather than inherit that state, or
    // its own second SWYM would spuriously error.
    let source = " SWYM\n LOC #FFFFFFFFFFFFFFFC\n SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions,
        vec![
            (0, MMixInstruction::SWYM(0, 0, 0)),
            (0xFFFFFFFFFFFFFFFC, MMixInstruction::SWYM(0, 0, 0)),
        ]
    );
}

#[test]
fn test_standalone_local_label_past_the_end_is_an_error() {
    // A bare `1H` binds to the counter the same as a named label, so
    // one past the end is an error rather than silently binding to
    // the last item's address (`$0` reading `#FFFFFFFFFFFFFFF8`
    // through the `1B` reference below, never reached once this
    // errors).
    let source = " LOC #FFFFFFFFFFFFFFF8\n OCTA 7\n1H\n LOC #100\nMain GETA $0,1B\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_loc_lines_own_label_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd LOC #100\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_loc_lines_own_local_label_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n1H LOC #100\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_loc_lines_own_local_label_outranks_at_in_its_operand() {
    // The local label's own site is leftmost on the line, so it is
    // reported even though the operand's `@` needs the same missing
    // address.
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n1H LOC @+4\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_a_label_inside_bspec_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n BSPEC 0\nEnd BYTE 1\n ESPEC\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:4:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_a_local_label_inside_bspec_past_the_end_is_an_error() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n BSPEC 0\n1H BYTE 1\n ESPEC\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:4:1: address past #FFFFFFFFFFFFFFFF");
}

/// A label and its instruction both need the address past the end;
/// the label's own column (leftmost) is reported, not the mnemonic's.
#[test]
fn test_a_label_and_its_item_past_the_end_reports_the_labels_column() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_a_local_label_and_its_item_past_the_end_reports_its_column() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\n1H SWYM\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_a_label_and_its_data_item_past_the_end_reports_the_labels_column() {
    let source = " LOC #FFFFFFFFFFFFFFFC\n SWYM\nEnd BYTE 1\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert_eq!(err, "<test>:3:1: address past #FFFFFFFFFFFFFFFF");
}

/// Parses one bare statement for a direct `second_pass_statement` call,
/// bypassing `parse_two_pass`'s two-pass walk entirely.
fn lone_statement(source: &'static str) -> pest::iterators::Pair<'static, Rule> {
    use pest::Parser;
    MMixalParser::parse(Rule::statement, source)
        .unwrap()
        .next()
        .unwrap()
}

/// Two-operand `LDA` is one tetra whatever its operand, so pass 1 and
/// pass 2 always place every item at the same address and no source
/// program can make pass 2 alone reach a past-end statement pass 1
/// missed. The seven tests below drive `second_pass_statement` directly
/// instead, one per `require_addr`/`require_valid` call it makes, and
/// each still fails if its call is replaced with `.expect(...)`. The
/// instruction-align and data-directive-align tests below pin a
/// located error, never a panic, at the align call: `place_item`'s own
/// `past_end` guard would report the same error if the align call's
/// guard were skipped, so those two alone do not isolate the align
/// check.
///
/// Pins the instruction's own alignment check.
#[test]
fn test_pass_2_disagreeing_with_pass_1_on_size_is_a_located_error_not_a_panic() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.past_end = true;
    let err = asm
        .second_pass_statement(lone_statement("SWYM"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

/// `place_item` rejects a second item in the same data directive once
/// the first has already filled the address space's last byte, rather
/// than reusing its address: `current_addr` sits at the last address an
/// `OCTA` can start from, so the first of two exactly fills the top and
/// the second must still be rejected, not silently placed at the same
/// address.
#[test]
fn test_items_after_the_end_within_one_directive_never_overlap() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.current_addr = u64::MAX - 7;
    let err = asm
        .second_pass_statement(lone_statement("OCTA 1,2"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

/// Pins the standalone-label site at the end of `second_pass_statement`.
#[test]
fn test_pass_2_only_overrun_on_a_standalone_label_is_an_error() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.past_end = true;
    let err = asm
        .second_pass_statement(lone_statement("End"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

/// Pins `second_pass_statement`'s `loc_directive` label site.
#[test]
fn test_pass_2_only_overrun_on_a_locs_own_label_is_an_error() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.past_end = true;
    let err = asm
        .second_pass_statement(lone_statement("End LOC #100"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

/// Pins `second_pass_statement`'s special-mode label site.
#[test]
fn test_pass_2_only_overrun_on_a_label_inside_bspec_is_an_error() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.past_end = true;
    asm.in_special_mode = true;
    let err = asm
        .second_pass_statement(lone_statement("End BYTE 1"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

/// Pins the data-directive alignment's `require_addr` call, which
/// `.expect` also passed until now.
#[test]
fn test_pass_2_only_overrun_on_a_data_directives_alignment_is_an_error() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.past_end = true;
    let err = asm
        .second_pass_statement(lone_statement("OCTA 1"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

/// Pins an instruction's own `place_item` call: `SETI` is 16 bytes
/// though its alignment is 4, so the last 4-aligned address still
/// overruns placing it, where a plain 4-byte instruction never could.
#[test]
fn test_pass_2_only_overrun_on_the_final_instructions_place_is_an_error() {
    let mut asm = MMixAssembler::new("", "<test>");
    asm.current_addr = u64::MAX - 3;
    let err = asm
        .second_pass_statement(lone_statement("SETI $1,5"))
        .unwrap_err();
    assert_eq!(err, "<test>:1:1: address past #FFFFFFFFFFFFFFFF");
}

#[test]
fn test_standalone_label_line_is_not_rounded() {
    // Rounding happens when an item is assembled, not when a label is
    // defined alone, so a bare label line names an address up to 7 bytes
    // below the octabyte that follows it.
    let mut asm = MMixAssembler::new("BYTE 1\nDATA\nOCTA #FF", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("DATA"), Some(&1));
    assert_eq!(
        asm.instructions
            .last()
            .map(|(addr, inst)| (*addr, inst.clone())),
        Some((8, MMixInstruction::OCTA(0xFF)))
    );
}

#[test]
fn test_lda_rri_pass1_size_matches_pass2() {
    let mut asm = MMixAssembler::new("JMP LABEL\nLDA $1,$2,4\nLABEL: HALT", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::JMP(2));
}

#[test]
fn test_branch_offset_beyond_i16_bytes_not_truncated() {
    // Byte delta between BZ (at addr 0) and LABEL (at addr 0x10000, i.e.
    // 65536 bytes forward) exceeds i16::MAX (32767), so casting the raw
    // byte delta to i16 before dividing by 4 would silently wrap.
    let source = "BZ $0,LABEL\nLOC #10000\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    let expected_offset = 0x10000i64 / 4;
    assert_eq!(
        asm.instructions[0].1,
        MMixInstruction::BZ(0, expected_offset as u16)
    );
}

#[test]
fn test_jmp_backward_emits_jmpb() {
    // A JMP whose target is BEHIND the current instruction must assemble
    // to JMPB, whose 24-bit field is 2^24 - magnitude.
    let source = "LOC #100\nBACK: HALT\nJMP BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    // JMP is at addr 0x104 (BACK's HALT is 4 bytes), target 0x100:
    // magnitude = (0x104 - 0x100) / 4 = 1, field = 0x1000000 - 1.
    assert_eq!(asm.instructions[1].1, MMixInstruction::JMPB(0xFFFFFF));
}

#[test]
fn test_geta_offset_beyond_range_errors() {
    // Target is 65536 tetras forward, one past GETA's 0..=65535 reach.
    // Without the range check this source assembles to a wrapped field.
    let source = "GETA $0,LABEL\nLOC #40000\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("GETA"));
    assert!(err.contains("out of range"));
}

#[test]
fn test_geta_misaligned_target_errors() {
    // A bare label line takes the counter unrounded, so LABEL names the
    // byte after the BYTE -- address 5, and 5 bytes forward of the GETA.
    let source = "GETA $0,LABEL\nBYTE 1\nLABEL\nOCTA 0";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("not 4-byte aligned"), "{err}");
}

#[test]
fn test_geta_offset_within_range_succeeds() {
    // 65535 tetras forward is the last target GETA reaches.
    let source = "GETA $0,LABEL\nLOC #3FFFC\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    let MMixInstruction::GETA(_, y, z) = asm.instructions[0].1 else {
        panic!("expected GETA instruction");
    };
    let field = ((y as u16) << 8) | z as u16;
    assert_eq!(field, 65535);
}

#[test]
fn test_getab_forward_target_errors() {
    // Target is FORWARD of the GETAB, which cannot be encoded at all
    // (GETAB is a backward-only unsigned magnitude). Under the unfixed
    // code this assembles successfully with a semantically inverted
    // encoding.
    let source = "GETAB $0,LABEL\nLOC #100\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    assert!(asm.parse().is_err());
}

#[test]
fn test_getab_backward_target_encodes_knuth_field() {
    // GETAB sits at addr 0x104 (BACK's HALT is 4 bytes), target 0x100:
    // magnitude = (0x104 - 0x100) / 4 = 1, field = 65536 - 1 = 0xFFFF.
    let source = "LOC #100\nBACK: HALT\nGETAB $0,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::GETAB(0, 0xFF, 0xFF));
}

#[test]
fn test_geta_forward_reach_extends_past_i16() {
    // 32769 tetras forward is inside GETA's 0..=65535 reach and outside
    // the i16 range the old check enforced.
    let source = "GETA $0,LABEL\nLOC #20004\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::GETA(0, 0x80, 0x01));
}

#[test]
fn test_forward_mnemonic_at_backward_target_emits_backward_sibling() {
    // MMIXAL picks the opcode from the sign of the displacement, so a BNP
    // one tetra behind itself becomes BNPB with field 65536 - 1.
    let source = "LOC #100\nBACK: HALT\nBNP $1,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::BNPB(1, 0xFFFF));
}

#[test]
fn test_pushj_at_backward_target_emits_pushjb() {
    let source = "LOC #100\nBACK: HALT\nPUSHJ $1,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions[1].1,
        MMixInstruction::PUSHJB(1, 0xFF, 0xFF)
    );
}

#[test]
fn test_pbranch_at_backward_target_emits_knuth_field() {
    // parse_inst_pbranch is a separate function from parse_inst_branch and
    // needs its own coverage of both the auto-selection and the field.
    let source = "LOC #100\nBACK: HALT\nPBZ $1,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::PBZB(1, 0xFF, 0xFF));

    let explicit = "LOC #100\nBACK: HALT\nPBZB $1,BACK";
    let mut asm = MMixAssembler::new(explicit, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::PBZB(1, 0xFF, 0xFF));
}

#[test]
fn test_backward_mnemonic_at_forward_target_names_forward_sibling() {
    let source = "BZB $1,LABEL\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("BZB"), "{err}");
    assert!(err.contains("use BZ instead"), "{err}");
}

#[test]
fn test_zero_displacement_takes_the_forward_opcode() {
    // Forward reaches 0..=65535 tetras and backward 1..=65536, so a
    // branch to itself is forward with an empty field.
    let source = "HERE: BZ $1,HERE";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BZ(1, 0));
}

#[test]
fn test_branch_backward_reaches_65536_tetras() {
    // A backward field of 0 means -65536 tetras, the far end of the reach.
    let source = "LOC #0\nBACK: HALT\nLOC #40000\nBZ $1,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::BZB(1, 0));
}

#[test]
fn test_branch_beyond_reach_errors() {
    let forward = "BZ $1,LABEL\nLOC #40000\nLABEL: HALT";
    let mut asm = MMixAssembler::new(forward, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("out of range"), "{err}");

    let backward = "LOC #0\nBACK: HALT\nLOC #40004\nBZ $1,BACK";
    let mut asm = MMixAssembler::new(backward, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("out of range"), "{err}");
}

#[test]
fn test_branch_misaligned_target_errors() {
    // LABEL is a bare label line on data: address 5, not a multiple of 4.
    let source = "BZ $1,LABEL\nBYTE 1\nLABEL\nOCTA 0";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("not 4-byte aligned"), "{err}");
}

#[test]
fn test_jmp_misaligned_target_errors() {
    // LABEL is a bare label line on data: address 5, not a multiple of 4.
    let source = "JMP LABEL\nBYTE 1\nLABEL\nOCTA 0";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("not 4-byte aligned"), "{err}");
}

#[test]
fn test_jmp_beyond_reach_errors() {
    // JMP's field is 24 bits: 0..=16777215 tetras forward, 1..=16777216
    // backward. Without the check the extra bits were masked away.
    let forward = "JMP LABEL\nLOC #4000000\nLABEL: HALT";
    let mut asm = MMixAssembler::new(forward, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("out of range"), "{err}");

    let backward = "LOC #0\nBACK: HALT\nLOC #4000004\nJMP BACK";
    let mut asm = MMixAssembler::new(backward, "<test>");
    let err = asm.parse().unwrap_err();
    assert!(err.contains("out of range"), "{err}");
}

#[test]
fn test_incl_takes_a_16_bit_immediate() {
    // INCL adds YZ to $X, like its INCH/INCMH/INCML siblings.
    let mut asm = MMixAssembler::new("INCL $1,#203", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::INCL(1, 0x203));
}

#[test]
fn test_parse_char_literal_immediate() {
    let mut asm = MMixAssembler::new("ANDI $1, $2, 'A'", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 65));
}

#[test]
fn test_parse_char_literal_multi_char_error() {
    let mut asm = MMixAssembler::new("ANDI $1, $2, 'AB'", "<test>");
    assert!(asm.parse().is_err());
}

#[test]
fn test_char_literal_two_characters_reports_expected_primary() {
    assert_eq!(
        assemble_err("Main\tAND\t$1,$2,'AB'\n\tTRAP\t0,Halt,0"),
        "<test>:1:16: syntax error: expected primary"
    );
}

// Bitwise operation tests
#[test]
fn test_parse_and() {
    let mut asm = MMixAssembler::new("AND $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::AND(1, 2, 3));
}

#[test]
fn test_parse_andi() {
    let mut asm = MMixAssembler::new("ANDI $1, $2, #FF", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 0xFF));
}

#[test]
fn test_parse_or() {
    let mut asm = MMixAssembler::new("OR $10, $20, $30", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::OR(10, 20, 30));
}

#[test]
fn test_parse_xor() {
    let mut asm = MMixAssembler::new("XOR $5, $6, $7", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::XOR(5, 6, 7));
}

#[test]
fn test_parse_andn() {
    let mut asm = MMixAssembler::new("ANDN $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ANDN(1, 2, 3));
}

#[test]
fn test_parse_nand() {
    let mut asm = MMixAssembler::new("NAND $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::NAND(1, 2, 3));
}

#[test]
fn test_parse_nor() {
    let mut asm = MMixAssembler::new("NOR $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::NOR(1, 2, 3));
}

#[test]
fn test_parse_nxor() {
    let mut asm = MMixAssembler::new("NXOR $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::NXOR(1, 2, 3));
}

#[test]
fn test_parse_mux() {
    let mut asm = MMixAssembler::new("MUX $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::MUX(1, 2, 3));
}

// Bit fiddling operations tests
#[test]
fn test_parse_bdif() {
    let mut asm = MMixAssembler::new("BDIF $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BDIF(1, 2, 3));
}

#[test]
fn test_parse_bdifi() {
    let mut asm = MMixAssembler::new("BDIFI $1, $2, #10", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BDIFI(1, 2, 0x10));
}

#[test]
fn test_parse_wdif() {
    let mut asm = MMixAssembler::new("WDIF $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::WDIF(1, 2, 3));
}

#[test]
fn test_parse_wdifi() {
    let mut asm = MMixAssembler::new("WDIFI $1, $2, 100", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::WDIFI(1, 2, 100));
}

#[test]
fn test_parse_tdif() {
    let mut asm = MMixAssembler::new("TDIF $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::TDIF(1, 2, 3));
}

#[test]
fn test_parse_tdifi() {
    let mut asm = MMixAssembler::new("TDIFI $1, $2, 50", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::TDIFI(1, 2, 50));
}

#[test]
fn test_parse_odif() {
    let mut asm = MMixAssembler::new("ODIF $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ODIF(1, 2, 3));
}

#[test]
fn test_parse_odifi() {
    let mut asm = MMixAssembler::new("ODIFI $1, $2, 255", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ODIFI(1, 2, 255));
}

#[test]
fn test_parse_sadd() {
    let mut asm = MMixAssembler::new("SADD $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SADD(1, 2, 3));
}

#[test]
fn test_parse_saddi() {
    let mut asm = MMixAssembler::new("SADDI $1, $2, 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SADDI(1, 2, 0));
}

#[test]
fn test_parse_mor() {
    let mut asm = MMixAssembler::new("MOR $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::MOR(1, 2, 3));
}

#[test]
fn test_parse_mori() {
    let mut asm = MMixAssembler::new("MORI $1, $2, 128", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::MORI(1, 2, 128));
}

#[test]
fn test_parse_mxor() {
    let mut asm = MMixAssembler::new("MXOR $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::MXOR(1, 2, 3));
}

#[test]
fn test_parse_mxori() {
    let mut asm = MMixAssembler::new("MXORI $1, $2, 64", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::MXORI(1, 2, 64));
}

// Shift instruction parsing tests
#[test]
fn test_parse_sl() {
    let mut asm = MMixAssembler::new("SL $3, $1, $2", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SL(3, 1, 2));
}

#[test]
fn test_parse_sli() {
    let mut asm = MMixAssembler::new("SLI $3, $1, 8", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SLI(3, 1, 8));
}

#[test]
fn test_parse_slu() {
    let mut asm = MMixAssembler::new("SLU $10, $20, $30", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SLU(10, 20, 30));
}

#[test]
fn test_parse_slui() {
    let mut asm = MMixAssembler::new("SLUI $1, $2, 16", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SLUI(1, 2, 16));
}

#[test]
fn test_parse_sr() {
    let mut asm = MMixAssembler::new("SR $5, $6, $7", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SR(5, 6, 7));
}

#[test]
fn test_parse_sri() {
    let mut asm = MMixAssembler::new("SRI $3, $1, 4", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SRI(3, 1, 4));
}

#[test]
fn test_parse_sru() {
    let mut asm = MMixAssembler::new("SRU $8, $9, $10", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SRU(8, 9, 10));
}

#[test]
fn test_parse_srui() {
    let mut asm = MMixAssembler::new("SRUI $3, $1, 1", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SRUI(3, 1, 1));
}

#[test]
fn test_parse_fcmpe() {
    let mut asm = MMixAssembler::new("FCMPE $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::FCMPE(1, 2, 3));
}

#[test]
fn test_parse_fune() {
    let mut asm = MMixAssembler::new("FUNE $4, $5, $6", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::FUNE(4, 5, 6));
}

#[test]
fn test_parse_feqle() {
    let mut asm = MMixAssembler::new("FEQLE $7, $8, $9", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::FEQLE(7, 8, 9));
}

/// Verify the longest-first grammar still matches the shorter mnemonics.
#[test]
fn test_parse_fcmp_after_fcmpe_added() {
    let mut asm = MMixAssembler::new("FCMP $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::FCMP(1, 2, 3));
}

#[test]
fn test_parse_feql_after_feqle_added() {
    let mut asm = MMixAssembler::new("FEQL $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::FEQL(1, 2, 3));
}

#[test]
fn test_parse_fun_after_fune_added() {
    let mut asm = MMixAssembler::new("FUN $1, $2, $3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::FUN(1, 2, 3));
}
