//! Tests for instruction dispatch: operand counts, arity, and every field edge.

use super::*;

// ---- Operand counts and kinds ----------------------------------------

#[test]
fn test_trap_two_operand_form_splits_yz() {
    assert_first_instruction("TRAP 1,#0203", MMixInstruction::TRAP(1, 2, 3));
}

#[test]
fn test_trap_one_operand_form_splits_xyz() {
    assert_first_instruction("TRAP #010203", MMixInstruction::TRAP(1, 2, 3));
}

#[test]
fn test_trip_two_and_one_operand_forms_split_the_same_way() {
    assert_first_instruction("TRIP 1,#0203", MMixInstruction::TRIP(1, 2, 3));
    assert_first_instruction("TRIP #010203", MMixInstruction::TRIP(1, 2, 3));
}

#[test]
fn test_swym_two_and_one_operand_forms_split_the_same_way() {
    assert_first_instruction("SWYM 1,#0203", MMixInstruction::SWYM(1, 2, 3));
    assert_first_instruction("SWYM #010203", MMixInstruction::SWYM(1, 2, 3));
}

#[test]
fn test_swym_one_operand_is_xyz() {
    assert_first_instruction("SWYM 1", MMixInstruction::SWYM(0, 0, 1));
}

#[test]
fn test_swym_two_operands_splits_yz() {
    assert_first_instruction("SWYM 1,2", MMixInstruction::SWYM(1, 0, 2));
}

#[test]
fn test_pop_one_operand_is_xyz() {
    assert_first_instruction("POP 1", MMixInstruction::POP(0, 0, 1));
}

#[test]
fn test_unsave_one_operand_matches_the_two_operand_spelling() {
    assert_first_instruction("UNSAVE $2", MMixInstruction::UNSAVE(0, 2));
}

#[test]
fn test_neg_two_operand_form_omits_y() {
    assert_first_instruction("NEG $1,5", MMixInstruction::NEGI(1, 0, 5));
    assert_first_instruction("NEGU $1,$0", MMixInstruction::NEGU(1, 0, 0));
}

#[test]
fn test_bare_greg_allocates_a_register_holding_zero() {
    let mut asm = MMixAssembler::new("g GREG\nMain HALT", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.greg_inits.last(), Some(&(254, 0)));
    assert_eq!(asm.symbols.get("g"), Some(&SymbolType::Register(254)));
}

#[test]
fn test_swym_three_registers_assembles() {
    assert_first_instruction("SWYM $5,$6,$7", MMixInstruction::SWYM(5, 6, 7));
}

#[test]
fn test_trap_three_registers_assembles() {
    assert_first_instruction("TRAP $1,$2,$3", MMixInstruction::TRAP(1, 2, 3));
}

#[test]
fn test_preld_and_stco_accept_a_pure_x() {
    assert_first_instruction("PRELD 7,$2,0", MMixInstruction::PRELDI(7, 2, 0));
    assert_first_instruction("STCO $1,$2,0", MMixInstruction::STCOI(1, 2, 0));
}

#[test]
fn test_pushj_pure_x_and_register_x_assemble_the_same_bytes() {
    assert_first_instruction("PUSHJ 0,Sub\nSub HALT", MMixInstruction::PUSHJ(0, 0, 1));
    let by_number = {
        let mut asm = MMixAssembler::new("PUSHJ 2,Sub\nSub HALT", "<test>");
        asm.parse().unwrap();
        asm.instructions[0].1.clone()
    };
    let by_register = {
        let mut asm = MMixAssembler::new("PUSHJ $2,Sub\nSub HALT", "<test>");
        asm.parse().unwrap();
        asm.instructions[0].1.clone()
    };
    assert_eq!(by_number, by_register);
}

#[test]
fn test_pushgo_pure_x_matches_register_x() {
    // Z=0 is a pure value, which auto-selects the immediate opcode
    // regardless of X's spelling.
    assert_first_instruction("PUSHGO 2,$3,0", MMixInstruction::PUSHGOI(2, 3, 0));
    assert_first_instruction("PUSHGO $2,$3,0", MMixInstruction::PUSHGOI(2, 3, 0));
}

#[test]
fn test_go_pure_x_is_still_an_error() {
    assert!(
        assemble_err("GO 2,$3,0")
            .contains("pure value 2 cannot be used where a register is required")
    );
}

// ---- No bare mnemonic is a silent label --------------------------------

#[test]
fn test_bare_pop_between_instructions_is_the_zero_form() {
    let mut asm = MMixAssembler::new("SET $1,0\n\tPOP\nSET $2,0", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::POP(0, 0, 0));
    assert!(!asm.labels.contains_key("POP"));
}

#[test]
fn test_bare_resume_between_instructions_is_the_zero_form() {
    let mut asm = MMixAssembler::new("SET $1,0\n\tRESUME\nSET $2,0", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::RESUME(0));
    assert!(!asm.labels.contains_key("RESUME"));
}

#[test]
fn test_bare_sync_between_instructions_is_the_zero_form() {
    let mut asm = MMixAssembler::new("SET $1,0\n\tSYNC\nSET $2,0", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SYNC(0));
    assert!(!asm.labels.contains_key("SYNC"));
}

#[test]
fn test_bare_trap_between_instructions_is_the_zero_form() {
    let mut asm = MMixAssembler::new("SET $1,0\n\tTRAP\nSET $2,0", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::TRAP(0, 0, 0));
    assert!(!asm.labels.contains_key("TRAP"));
}

#[test]
fn test_bare_trip_between_instructions_is_the_zero_form() {
    let mut asm = MMixAssembler::new("SET $1,0\n\tTRIP\nSET $2,0", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::TRIP(0, 0, 0));
    assert!(!asm.labels.contains_key("TRIP"));
}

#[test]
fn test_bare_save_between_instructions_is_unknown_operation() {
    assert_eq!(
        assemble_err("SET $1,0\n\tSAVE\nSET $2,0"),
        "<test>:2:2: syntax error: unknown operation: SAVE"
    );
}

#[test]
fn test_bare_unsave_between_instructions_requires_a_register() {
    assert_eq!(
        assemble_err("SET $1,0\n\tUNSAVE\nSET $2,0"),
        "<test>:2:2: pure value 0 cannot be used where a register is required"
    );
}

#[test]
fn test_save_after_semicolon_is_unknown_operation_not_a_label() {
    assert!(assemble_err("SET $2,2 ; SAVE").contains("unknown operation: SAVE"));
}

#[test]
fn test_column_one_lone_pop_is_a_pop_not_a_label() {
    let mut asm = MMixAssembler::new("POP\nSET $1,0", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[0].1, MMixInstruction::POP(0, 0, 0));
    assert!(!asm.labels.contains_key("POP"));
}

// ---- Upper-case opcodes and the indented line --------------------------

#[test]
fn test_lowercase_loc_prefix_defines_a_register_label() {
    assert_first_instruction("loc GREG 0\nSET loc,5", MMixInstruction::SETL(254, 5));
}

#[test]
fn test_mixed_case_sync_defines_a_label_not_the_instruction() {
    let mut asm = MMixAssembler::new("Sync BNZ $1,Main\nMain HALT", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.labels.contains_key("Sync"));
}

#[test]
fn test_indented_lowercase_mnemonic_is_unknown_operation() {
    assert!(assemble_err("SET $1,0\n\tset $1,2").contains("unknown operation: set $1,2"));
}

#[test]
fn test_indented_label_with_instruction_is_unknown_operation() {
    assert!(assemble_err("SET $1,0\n\tFoo SET $2,9").contains("unknown operation: Foo SET $2,9"));
}

#[test]
fn test_indented_label_with_is_directive_is_unknown_operation() {
    assert!(assemble_err("SET $1,0\n\tFoo IS 5").contains("unknown operation: Foo IS 5"));
}

#[test]
fn test_tab_then_carriage_return_still_opens_indented() {
    assert!(assemble_err("SET $1,0\n\t\rFoo SET $1,0").contains("unknown operation: Foo SET $1,0"));
}

#[test]
fn test_indented_lone_word_is_unknown_operation() {
    assert_eq!(
        assemble_err("SET $1,0\n\tFoo\nSET $2,0"),
        "<test>:2:2: syntax error: unknown operation: Foo"
    );
}

#[test]
fn test_indented_lone_local_label_is_unknown_operation() {
    assert_eq!(
        assemble_err("\tSET $1,0\n\t2H\nSET $2,0"),
        "<test>:2:2: syntax error: unknown operation: 2H"
    );
}

#[test]
fn test_column_one_local_label_with_text_is_unknown_operation() {
    assert_eq!(
        assemble_err("2H note text\n\tTRAP 0,Halt,0"),
        "<test>:1:4: syntax error: unknown operation: 2H note text"
    );
}

#[test]
fn test_column_one_local_label_with_a_digit_is_unknown_operation() {
    assert_eq!(
        assemble_err("2H 5\n\tTRAP 0,Halt,0"),
        "<test>:1:4: syntax error: unknown operation: 2H 5"
    );
}

#[test]
fn test_column_one_local_label_with_a_remark_marker_is_unknown_operation() {
    assert_eq!(
        assemble_err("2H * note\n\tTRAP 0,Halt,0"),
        "<test>:1:4: syntax error: unknown operation: 2H * note"
    );
}

#[test]
fn test_indented_lone_word_with_trailing_blanks_drops_them() {
    assert_eq!(
        assemble_err("SET $1,0\n\tFoo  \nSET $2,0"),
        "<test>:2:2: syntax error: unknown operation: Foo"
    );
}

#[test]
fn test_bare_save_with_a_colon_keeps_it() {
    assert_eq!(
        assemble_err("SET $1,0\n\tSAVE:\nSET $2,0"),
        "<test>:2:2: syntax error: unknown operation: SAVE:"
    );
}

#[test]
fn test_semicolon_lone_word_still_defines_a_label() {
    let mut asm = MMixAssembler::new("SET $2,2 ; loop\nSET $3,loop", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.labels.contains_key("loop"));
}

#[test]
fn test_column_one_lone_word_is_still_a_label() {
    let mut asm = MMixAssembler::new("Loop\nSET $1,Loop", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert!(asm.labels.contains_key("Loop"));
}

// ---- Longest form wins; a partial operand list is an error -------------

#[test]
fn test_trap_three_operand_form_is_not_swallowed_by_shorter_forms() {
    assert_first_instruction("TRAP 0,1,2", MMixInstruction::TRAP(0, 1, 2));
}

#[test]
fn test_pop_two_operand_form_unchanged_from_base() {
    assert_first_instruction("POP 1,2", MMixInstruction::POP(1, 0, 2));
}

#[test]
fn test_trap_partial_operand_list_is_an_error() {
    assert!(
        assemble_err("TRAP 0,")
            .contains("a remark must be separated from the statement by a blank")
    );
}

#[test]
fn test_pop_partial_operand_list_is_an_error() {
    assert!(
        assemble_err("POP 1,").contains("a remark must be separated from the statement by a blank")
    );
}

// ---- The remark boundary against multi-operand counts -------------------

#[test]
fn test_swym_one_operand_then_digit_is_a_dropped_operand_error() {
    assert!(assemble_err("SWYM 1 2").contains("a remark cannot begin with `2`"));
}

#[test]
fn test_trap_two_operand_then_digit_is_a_dropped_operand_error() {
    assert!(assemble_err("TRAP 0,1 2").contains("a remark cannot begin with `2`"));
}

#[test]
fn test_trap_blanks_around_comma_still_parse() {
    assert_first_instruction("TRAP 0,1 ,2", MMixInstruction::TRAP(0, 1, 2));
}

#[test]
fn test_swym_one_operand_then_note_is_ignored() {
    assert_first_instruction("SWYM 1 note", MMixInstruction::SWYM(0, 0, 1));
}

#[test]
fn test_bare_swym_then_undefined_word_is_undefined_symbol() {
    assert!(assemble_err("SWYM do nothing").contains("Undefined symbol: do"));
}

#[test]
fn test_bare_swym_then_a_defined_symbol_is_its_address() {
    let mut asm = MMixAssembler::new("JMP Skip\nMain HALT\nSkip SWYM Main", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[2].1, MMixInstruction::SWYM(0, 0, 4));
}

// ---- Arity unchanged for instructions this unit does not widen ---------

#[test]
fn test_save_one_operand_is_still_an_error() {
    assert!(assemble_err("SAVE $2").contains("unknown operation: SAVE $2"));
}

#[test]
fn test_get_with_a_predefined_special_register_name_still_assembles() {
    assert_first_instruction("GET $1,rA", MMixInstruction::GET(1, 21));
}

#[test]
fn test_lowercase_greg_is_not_the_directive() {
    // If lower-case `greg` matched the directive, this would allocate a
    // register holding 0 instead of erroring on the digit-led text
    // after the label `greg`.
    assert!(assemble_err("greg 0").contains("unknown operation: greg 0"));
}

// ---- Every instruction field's edge, one field past it -------------

#[test]
fn test_instruction_field_edges_by_range_table() {
    // Each of the sixteen wyde immediates is its own function.
    assert_first_instruction("SETL $1,#FFFF", MMixInstruction::SETL(1, 0xFFFF));
    assert_eq!(
        assemble_err("SETL $1,#10000"),
        "<test>:1:9: immediate operand 65536 out of range 0..65535 for SETL"
    );
    assert_first_instruction("SETH $1,#FFFF", MMixInstruction::SETH(1, 0xFFFF));
    assert_eq!(
        assemble_err("SETH $1,#10000"),
        "<test>:1:9: immediate operand 65536 out of range 0..65535 for SETH"
    );
    assert_first_instruction("SETMH $1,#FFFF", MMixInstruction::SETMH(1, 0xFFFF));
    assert_eq!(
        assemble_err("SETMH $1,#10000"),
        "<test>:1:10: immediate operand 65536 out of range 0..65535 for SETMH"
    );
    assert_first_instruction("SETML $1,#FFFF", MMixInstruction::SETML(1, 0xFFFF));
    assert_eq!(
        assemble_err("SETML $1,#10000"),
        "<test>:1:10: immediate operand 65536 out of range 0..65535 for SETML"
    );
    assert_first_instruction("INCL $1,#FFFF", MMixInstruction::INCL(1, 0xFFFF));
    assert_eq!(
        assemble_err("INCL $1,#10000"),
        "<test>:1:9: immediate operand 65536 out of range 0..65535 for INCL"
    );
    assert_first_instruction("INCH $1,#FFFF", MMixInstruction::INCH(1, 0xFFFF));
    assert_eq!(
        assemble_err("INCH $1,#1FFFF"),
        "<test>:1:9: immediate operand 131071 out of range 0..65535 for INCH"
    );
    assert_first_instruction("INCMH $1,#FFFF", MMixInstruction::INCMH(1, 0xFFFF));
    assert_eq!(
        assemble_err("INCMH $1,#10000"),
        "<test>:1:10: immediate operand 65536 out of range 0..65535 for INCMH"
    );
    assert_first_instruction("INCML $1,#FFFF", MMixInstruction::INCML(1, 0xFFFF));
    assert_eq!(
        assemble_err("INCML $1,#10000"),
        "<test>:1:10: immediate operand 65536 out of range 0..65535 for INCML"
    );
    assert_first_instruction("ORH $1,#FFFF", MMixInstruction::ORH(1, 0xFFFF));
    assert_eq!(
        assemble_err("ORH $1,#10000"),
        "<test>:1:8: immediate operand 65536 out of range 0..65535 for ORH"
    );
    assert_first_instruction("ORMH $1,#FFFF", MMixInstruction::ORMH(1, 0xFFFF));
    assert_eq!(
        assemble_err("ORMH $1,#10000"),
        "<test>:1:9: immediate operand 65536 out of range 0..65535 for ORMH"
    );
    assert_first_instruction("ORML $1,#FFFF", MMixInstruction::ORML(1, 0xFFFF));
    assert_eq!(
        assemble_err("ORML $1,#10000"),
        "<test>:1:9: immediate operand 65536 out of range 0..65535 for ORML"
    );
    assert_first_instruction("ORL $1,#FFFF", MMixInstruction::ORL(1, 0xFFFF));
    assert_eq!(
        assemble_err("ORL $1,#10000"),
        "<test>:1:8: immediate operand 65536 out of range 0..65535 for ORL"
    );
    assert_first_instruction("ANDNH $1,#FFFF", MMixInstruction::ANDNH(1, 0xFFFF));
    assert_eq!(
        assemble_err("ANDNH $1,#10000"),
        "<test>:1:10: immediate operand 65536 out of range 0..65535 for ANDNH"
    );
    assert_first_instruction("ANDNMH $1,#FFFF", MMixInstruction::ANDNMH(1, 0xFFFF));
    assert_eq!(
        assemble_err("ANDNMH $1,#10000"),
        "<test>:1:11: immediate operand 65536 out of range 0..65535 for ANDNMH"
    );
    assert_first_instruction("ANDNML $1,#FFFF", MMixInstruction::ANDNML(1, 0xFFFF));
    assert_eq!(
        assemble_err("ANDNML $1,#10000"),
        "<test>:1:11: immediate operand 65536 out of range 0..65535 for ANDNML"
    );
    assert_first_instruction("ANDNL $1,#FFFF", MMixInstruction::ANDNL(1, 0xFFFF));
    assert_eq!(
        assemble_err("ANDNL $1,#10000"),
        "<test>:1:10: immediate operand 65536 out of range 0..65535 for ANDNL"
    );

    // `parse_inst_arith_rri` (ADDI and its auto-immediate kin) and
    // `parse_inst_load_store_rri` (LDBI and its kin): each a distinct
    // function, its own byte-field check.
    assert_first_instruction("ADDI $1,$2,255", MMixInstruction::ADDI(1, 2, 255));
    assert_eq!(
        assemble_err("ADDI $1,$2,256"),
        "<test>:1:12: immediate operand 256 out of range 0..255 for ADDI"
    );
    assert_first_instruction("LDBI $1,$2,255", MMixInstruction::LDBI(1, 2, 255));
    assert_eq!(
        assemble_err("LDBI $1,$2,999"),
        "<test>:1:12: immediate operand 999 out of range 0..255 for LDBI"
    );

    // `parse_rri`, the helper `LDUNCI` and fifteen other explicit `*I`
    // three-operand spellings share.
    assert_first_instruction("LDUNCI $1,$2,255", MMixInstruction::LDUNCI(1, 2, 255));
    assert_eq!(
        assemble_err("LDUNCI $1,$2,256"),
        "<test>:1:14: immediate operand 256 out of range 0..255 for LDUNCI"
    );

    // `parse_inst_bitfiddle_rri`, `parse_inst_conditional_set_rri` and
    // `parse_inst_zero_or_set_rri`: each its own Z check.
    assert_first_instruction("BDIFI $1,$2,255", MMixInstruction::BDIFI(1, 2, 255));
    assert_eq!(
        assemble_err("BDIFI $1,$2,256"),
        "<test>:1:13: immediate operand 256 out of range 0..255 for BDIFI"
    );
    assert_first_instruction("CSNI $1,$2,255", MMixInstruction::CSNI(1, 2, 255));
    assert_eq!(
        assemble_err("CSNI $1,$2,256"),
        "<test>:1:12: immediate operand 256 out of range 0..255 for CSNI"
    );
    assert_first_instruction("ZSNI $1,$2,255", MMixInstruction::ZSNI(1, 2, 255));
    assert_eq!(
        assemble_err("ZSNI $1,$2,256"),
        "<test>:1:12: immediate operand 256 out of range 0..255 for ZSNI"
    );

    // NEG/NEGU's Y (the auto-immediate path) and NEGI/NEGUI's own Y
    // (the explicit-immediate path): two distinct functions.
    assert_first_instruction("NEG $1,255,$2", MMixInstruction::NEG(1, 255, 2));
    assert_eq!(
        assemble_err("NEG $1,256,$2"),
        "<test>:1:8: immediate operand 256 out of range 0..255 for NEG"
    );
    assert_first_instruction("NEGI $1,255,5", MMixInstruction::NEGI(1, 255, 5));
    assert_eq!(
        assemble_err("NEGI $1,256,$2"),
        "<test>:1:9: immediate operand 256 out of range 0..255 for NEGI"
    );

    // The float rounding-mode forms: `FIX`'s explicit `Y`, `FLOT`'s
    // forced `Y`, `FLOTI`'s three- and two-operand `Y`/`Z`.
    assert_first_instruction("FIX $1,255,$2", MMixInstruction::FIX(1, 255, 2));
    assert_eq!(
        assemble_err("FIX $1,256,$2"),
        "<test>:1:8: immediate operand 256 out of range 0..255 for FIX"
    );
    assert_first_instruction("FLOT $1,255,$2", MMixInstruction::FLOT(1, 255, 2));
    assert_eq!(
        assemble_err("FLOT $1,256,$2"),
        "<test>:1:9: immediate operand 256 out of range 0..255 for FLOT"
    );
    assert_first_instruction("FLOTI $1,255,5", MMixInstruction::FLOTI(1, 255, 5));
    assert_eq!(
        assemble_err("FLOTI $1,256,5"),
        "<test>:1:10: immediate operand 256 out of range 0..255 for FLOTI"
    );
    assert_first_instruction("FLOTI $1,1,255", MMixInstruction::FLOTI(1, 1, 255));
    assert_eq!(
        assemble_err("FLOTI $1,1,256"),
        "<test>:1:12: immediate operand 256 out of range 0..255 for FLOTI"
    );
    assert_first_instruction("FLOTI $1,255", MMixInstruction::FLOTI(1, 0, 255));
    assert_eq!(
        assemble_err("FLOTI $1,256"),
        "<test>:1:10: immediate operand 256 out of range 0..255 for FLOTI"
    );

    // GET's Z and PUT's X: a special register, 0..=31.
    assert_first_instruction("GET $1,31", MMixInstruction::GET(1, 31));
    assert_eq!(
        assemble_err("GET $1,32"),
        "<test>:1:8: immediate operand 32 out of range 0..31 for GET"
    );
    assert_first_instruction("PUT 31,$1", MMixInstruction::PUT(31, 1));
    assert_eq!(
        assemble_err("PUT 32,$1"),
        "<test>:1:5: immediate operand 32 out of range 0..31 for PUT"
    );

    // PUTI's X (special register) and Z (byte) check independently.
    assert_first_instruction("PUTI 31,255", MMixInstruction::PUTI(31, 255));
    assert_eq!(
        assemble_err("PUTI 32,1"),
        "<test>:1:6: immediate operand 32 out of range 0..31 for PUTI"
    );
    assert_eq!(
        assemble_err("PUTI 1,256"),
        "<test>:1:8: immediate operand 256 out of range 0..255 for PUTI"
    );

    // SAVE's Z and UNSAVE's X.
    assert_first_instruction("SAVE $1,255", MMixInstruction::SAVE(1, 255));
    assert_eq!(
        assemble_err("SAVE $255,256"),
        "<test>:1:11: immediate operand 256 out of range 0..255 for SAVE"
    );
    assert_first_instruction("UNSAVE 255,$1", MMixInstruction::UNSAVE(255, 1));
    assert_eq!(
        assemble_err("UNSAVE 256,$1"),
        "<test>:1:8: immediate operand 256 out of range 0..255 for UNSAVE"
    );

    // `STCOI`'s X and Z check independently.
    assert_first_instruction("STCOI 255,$2,5", MMixInstruction::STCOI(255, 2, 5));
    assert_eq!(
        assemble_err("STCOI 256,$2,5"),
        "<test>:1:7: immediate operand 256 out of range 0..255 for STCOI"
    );
    assert_first_instruction("STCOI 5,$2,255", MMixInstruction::STCOI(5, 2, 255));
    assert_eq!(
        assemble_err("STCOI 5,$2,256"),
        "<test>:1:12: immediate operand 256 out of range 0..255 for STCOI"
    );

    // RESUME and SYNC take the full 24-bit XYZ.
    assert_first_instruction("RESUME #FFFFFF", MMixInstruction::RESUME(0xFFFFFF));
    assert_eq!(
        assemble_err("RESUME #1000000"),
        "<test>:1:8: immediate operand 16777216 out of range 0..16777215 for RESUME"
    );
    assert_first_instruction("SYNC #FFFFFF", MMixInstruction::SYNC(0xFFFFFF));
    assert_eq!(
        assemble_err("SYNC #1000000"),
        "<test>:1:6: immediate operand 16777216 out of range 0..16777215 for SYNC"
    );

    // POP's X (byte), yz (wyde) and xyz (three bytes), both operand
    // forms.
    assert_first_instruction("POP 255,#FFFF", MMixInstruction::POP(255, 255, 255));
    assert_eq!(
        assemble_err("POP 256,0"),
        "<test>:1:5: immediate operand 256 out of range 0..255 for POP"
    );
    assert_eq!(
        assemble_err("POP 0,#10000"),
        "<test>:1:7: immediate operand 65536 out of range 0..65535 for POP"
    );
    assert_first_instruction("POP #FFFFFF", MMixInstruction::POP(255, 255, 255));
    assert_eq!(
        assemble_err("POP #1000000"),
        "<test>:1:5: immediate operand 16777216 out of range 0..16777215 for POP"
    );

    // TRAP's yz (wyde) and xyz (three bytes); both fit at their edge.
    assert_eq!(
        assemble_err("TRAP 0,#10000"),
        "<test>:1:8: immediate operand 65536 out of range 0..65535 for TRAP"
    );
    assert_eq!(
        assemble_err("TRAP #1000000"),
        "<test>:1:6: immediate operand 16777216 out of range 0..16777215 for TRAP"
    );
    assert_first_instruction("TRAP 0,#FFFF", MMixInstruction::TRAP(0, 0xFF, 0xFF));
    assert_first_instruction("TRAP #FFFFFF", MMixInstruction::TRAP(0xFF, 0xFF, 0xFF));
}

#[test]
fn test_resume_and_sync_encode_all_of_xyz() {
    let asm = MMixAssembler::new("", "<test>");
    assert_eq!(
        asm.encode_instruction_bytes(&MMixInstruction::SYNC(300)),
        vec![0xFC, 0x00, 0x01, 0x2C]
    );
    assert_eq!(
        asm.encode_instruction_bytes(&MMixInstruction::RESUME(0x10203)),
        vec![0xF9, 0x01, 0x02, 0x03]
    );
}

// `SET $1,-1`/`-5`/`-#10` are covered by
// `test_set_negative_literal_is_an_error_for_decimal_and_hex`.
#[test]
fn test_set_wide_immediate_diagnostic_and_positive_forms() {
    assert_eq!(
        assemble_err("SET $1,#10000"),
        "<test>:1:8: immediate operand 65536 out of range 0..65535 for SET; \
             use SETI for a wider constant"
    );
    assert_first_instruction("SET $1,#FFFF", MMixInstruction::SETL(1, 0xFFFF));
    assert_first_instruction("SETI $1,-1", MMixInstruction::SET(1, u64::MAX));
}
