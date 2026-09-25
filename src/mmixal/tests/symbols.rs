//! Tests for symbol qualification, local labels, qualified references and predefined-symbol rules.

use super::*;

// ---- Multi-source assembly + global-':' symbol tests ----

#[test]
fn test_global_symbol_label_and_operand() {
    // `:Foo` parses both as a label definition and as an operand reference.
    let src = "\
            LOC #100\n\
            Main BNZ $1,:Foo\n\
            :Foo HALT\n";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    // The root prefix is `:`; `:Foo` and `Foo` name one symbol, keyed
    // without the colon.
    assert_eq!(asm.labels.get("Foo").copied(), Some(0x104));
    assert_eq!(asm.labels.get("Main").copied(), Some(0x100));
    assert!(!asm.labels.contains_key(":Foo"));
}

#[test]
fn test_multi_source_main_calls_lib() {
    let main_src = "\
            LOC #100\n\
            Main PUSHJ $0,:Lib\n\
                 HALT\n";
    let lib_src = "\
            LOC #200\n\
            :Lib POP 0,0\n";
    let mut asm = MMixAssembler::new(main_src, "main.mms");
    asm.add_source(lib_src, "lib.mms");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Main").copied(), Some(0x100));
    assert_eq!(asm.labels.get("Lib").copied(), Some(0x200));

    // Two LOC regions both produced instructions.
    let addrs: Vec<u64> = asm.instructions.iter().map(|(a, _)| *a).collect();
    assert!(addrs.contains(&0x100));
    assert!(addrs.contains(&0x200));
}

#[test]
fn test_multi_source_main_redefined() {
    let a = "\
            LOC #100\n\
            Main HALT\n";
    let b = "\
            LOC #200\n\
            Main HALT\n";
    let mut asm = MMixAssembler::new(a, "a.mms");
    asm.add_source(b, "b.mms");
    let err = asm.parse().expect_err("expected redefinition error");
    assert!(err.contains("'Main'"), "err: {}", err);
    assert!(err.contains("a.mms"), "err: {}", err);
    assert!(err.contains("b.mms"), "err: {}", err);
    assert!(err.contains("redefined"), "err: {}", err);
}

#[test]
fn test_multi_source_global_symbol_redefined() {
    let a = "\
            LOC #100\n\
            :Foo HALT\n";
    let b = "\
            LOC #200\n\
            :Foo HALT\n";
    let mut asm = MMixAssembler::new(a, "a.mms");
    asm.add_source(b, "b.mms");
    let err = asm.parse().expect_err("expected redefinition error");
    assert!(err.contains("'Foo'"), "err: {}", err);
    assert!(err.contains("a.mms"), "err: {}", err);
    assert!(err.contains("b.mms"), "err: {}", err);
}

#[test]
fn test_redefinition_across_label_and_is() {
    // A label and an IS-bound symbol with the same qualified name collide.
    let src = "\
            LOC #100\n\
            Foo HALT\n\
            Foo IS 5\n";
    let mut asm = MMixAssembler::new(src, "<test>");
    let err = asm.parse().expect_err("expected redefinition error");
    assert!(err.contains("'Foo'"), "err: {}", err);
    assert!(err.contains("redefined"), "err: {}", err);
}

// ---- Local symbols ------------------------------------------------

#[test]
fn test_local_labels_forward_and_backward_meet_in_the_middle() {
    // The reference's own idiom: the first jumps to the second and the
    // second jumps back to the first -- a same-line local reference
    // never resolves to that same line's own (not yet recorded)
    // occurrence.
    let mut asm = MMixAssembler::new("2H      JMP 2F\n2H      JMP 2B\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].0, 0x0);
    assert_eq!(asm.instructions[0].1, MMixInstruction::JMP(1));
    assert_eq!(asm.instructions[1].0, 0x4);
    assert_eq!(asm.instructions[1].1, MMixInstruction::JMPB(0xFFFFFF));
}

#[test]
fn test_local_label_on_a_greg_line_keeps_both_passes_in_step() {
    let parse = |src: &str| {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        asm.instructions
    };
    let local = parse("2H      GREG 0\nMain    SET $1,2B\n        SET $2,2F\n2H      HALT\n");
    let named = parse("R       GREG 0\nMain    SET $1,R\n        SET $2,L\nL       HALT\n");
    assert_eq!(local, named);
}

#[test]
fn test_local_back_reference_before_any_definition_is_zero() {
    // `2B` ahead of any `2H` is `0`, never an error.
    assert_first_instruction("OCTA 2B", MMixInstruction::OCTA(0));
}

#[test]
fn test_local_forward_reference_with_no_later_definition_is_undefined() {
    assert!(assemble_err("OCTA 2F").contains("Undefined symbol: 2F"));
}

#[test]
fn test_local_labels_are_independent_per_digit() {
    let mut asm = MMixAssembler::new(
        "1H      HALT\n\
             2H      HALT\n\
             Main    SET $1,1B\n\
                     SET $2,2B\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[2].1, MMixInstruction::SETL(1, 0x0));
    assert_eq!(asm.instructions[3].1, MMixInstruction::SETL(2, 0x4));
}

#[test]
fn test_second_local_label_redefines_rather_than_erroring() {
    let mut asm = MMixAssembler::new(
        "2H      HALT\n\
             2H      HALT\n\
             Main    SET $1,2B\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[2].1, MMixInstruction::SETL(1, 0x4));
}

#[test]
fn test_local_label_is_directive_counts_like_a_running_counter() {
    // The reference's own idiom: `9H IS 9B+1` twice leaves the counter
    // at 2 (0 -> 1 -> 2).
    assert_first_instruction(
        "9H IS 0\n\
                 9H IS 9B+1\n\
                 9H IS 9B+1\n\
                 Main SET $1,9B\n",
        MMixInstruction::SETL(1, 2),
    );
}

#[test]
fn test_local_label_alone_on_a_line_takes_the_unrounded_counter() {
    let mut asm = MMixAssembler::new(
        "Main    BYTE 1\n\
             2H\n\
                     OCTA 2B\n",
        "<test>",
    );
    asm.parse().unwrap();
    // 2H sits right after the one BYTE, unrounded (address 1); the
    // following OCTA still aligns to 8.
    let octa = asm
        .instructions
        .iter()
        .find(|(_, i)| matches!(i, MMixInstruction::OCTA(_)))
        .unwrap();
    assert_eq!(octa.1, MMixInstruction::OCTA(1));
    assert_eq!(octa.0, 8);
}

#[test]
fn test_lowercase_local_symbols_are_rejected() {
    assert!(assemble_err("2h SET $1,0").contains("syntax error"));
    assert!(assemble_err("Main SET $1,2b").contains("syntax error"));
}

#[test]
fn test_local_back_reference_after_loc_moves_backward_sees_source_order() {
    // Resolution is by source order, not by address: a LOC that moves
    // the counter backward still leaves a later `2B` seeing the
    // textually preceding `2H`.
    let mut asm = MMixAssembler::new(
        "2H      HALT\n\
             LOC #100\n\
             LOC #10\n\
             Main    SET $1,2B\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(1, 0));
}

#[test]
fn test_local_label_h_as_an_operand_is_an_error() {
    assert!(assemble_err("Main SET $1,2H").contains("syntax error"));
}

#[test]
fn test_local_ref_b_in_the_label_field_is_an_error() {
    assert!(assemble_err("2B JMP Main\nMain HALT\n").contains("unknown operation"));
}

#[test]
fn test_digit_literal_operand_forms_are_unaffected() {
    assert_first_instruction("SET $1,2", MMixInstruction::SETL(1, 2));
    assert_first_instruction("SET $1,#2B", MMixInstruction::SETL(1, 0x2B));
    assert_first_instruction("SET $1,0x2B", MMixInstruction::SETL(1, 0x2B));
    assert_first_instruction("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3));
    assert!(assemble_err("SETL $1,2abc").contains("syntax error"));
}

#[test]
fn test_set_1_2f_now_resolves_where_it_once_errored() {
    let mut asm = MMixAssembler::new("Main SET $1,2F\n2H HALT\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 0x4));
}

#[test]
fn test_local_forward_reference_resolves_where_named_ones_do() {
    let mut asm = MMixAssembler::new(
        "Main    JMP 2F-4\n\
                     OCTA 2F\n\
             2H      HALT\n",
        "<test>",
    );
    asm.parse().unwrap();
    // 2H sits at 0x10 (JMP at 0x0..0x3, OCTA aligned at 0x8..0xF).
    assert!(
        asm.instructions
            .iter()
            .any(|(_, i)| matches!(i, MMixInstruction::OCTA(v) if *v == 0x10))
    );
}

#[test]
fn test_local_forward_reference_as_is_loc_greg_operand_is_undefined() {
    assert!(assemble_err("Foo IS 2F\nMain HALT\n").contains("Undefined symbol: 2F"));
    assert!(assemble_err("2H IS 2F+1\nMain HALT\n").contains("Undefined symbol: 2F"));
    assert!(assemble_err("LOC 2F\nMain HALT\n").contains("Undefined symbol: 2F"));
    assert!(assemble_err("G1 GREG 2F\nMain HALT\n").contains("Undefined symbol: 2F"));
}

// ---- Qualified references -----------------------------------------

#[test]
fn test_qualified_reference_reads_a_prefix_definition_from_outside() {
    let mut asm = MMixAssembler::new(
        "PREFIX Foo:\n\
             Bar IS 5\n\
             PREFIX :\n\
             Main SET $1,Foo:Bar\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
}

#[test]
fn test_qualified_reference_three_part() {
    let mut asm = MMixAssembler::new(
        "PREFIX Foo:Bar:\n\
             Baz IS 7\n\
             PREFIX :\n\
             Main SET $1,Foo:Bar:Baz\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 7));
}

#[test]
fn test_leading_colon_qualified_reference_opts_out_of_prefix() {
    let mut asm = MMixAssembler::new(
        "Foo IS 3\n\
             PREFIX Sub_\n\
             Main SET $1,:Foo\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 3));
}

#[test]
fn test_label_with_trailing_colon_and_blank_still_defines_the_plain_name() {
    let mut asm = MMixAssembler::new("Main: SET $1,0\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Main"), Some(&0));
    assert!(!asm.labels.contains_key("Main:"));
}

#[test]
fn test_label_with_trailing_colon_and_no_blank_is_now_an_error() {
    // The accepted break: interior colons make `Main:SET` one symbol,
    // so `Main:SET $1,0` no longer defines `Main`.
    assert!(assemble_err("Main:SET $1,0").contains("syntax error"));
}

#[test]
fn test_qualified_definition_in_the_label_field() {
    let mut asm = MMixAssembler::new(
        "PREFIX Foo:\n\
             Bar HALT\n\
             PREFIX :\n\
             Main SET $1,Foo:Bar\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Foo:Bar"), Some(&0));
}

#[test]
fn test_prefix_colon_foo_colon_composes_like_a_relative_reference() {
    let mut asm = MMixAssembler::new(
        "PREFIX :Foo:\n\
             bar IS 5\n\
             PREFIX :\n\
             Main SET $1,Foo:bar\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
}

// ---- The predefined symbols -----------------------------------------

#[test]
fn test_seventeen_predefined_symbols_resolve_to_the_reference_table() {
    let cases: &[(&str, u64)] = &[
        ("Inf", 0x7FF0000000000000),
        ("D_BIT", 0x80),
        ("D_Handler", 0x10),
        ("V_BIT", 0x40),
        ("V_Handler", 0x20),
        ("W_BIT", 0x20),
        ("W_Handler", 0x30),
        ("I_BIT", 0x10),
        ("I_Handler", 0x40),
        ("O_BIT", 0x08),
        ("O_Handler", 0x50),
        ("U_BIT", 0x04),
        ("U_Handler", 0x60),
        ("Z_BIT", 0x02),
        ("Z_Handler", 0x70),
        ("X_BIT", 0x01),
        ("X_Handler", 0x80),
    ];
    for (name, value) in cases {
        assert_first_instruction(&format!("OCTA {name}"), MMixInstruction::OCTA(*value));
        // The root-colon spelling reaches the same value.
        assert_first_instruction(&format!("OCTA :{name}"), MMixInstruction::OCTA(*value));
    }
}

#[test]
fn test_text_segment_is_still_undefined() {
    // The reference's predefined-symbol table has no `Text_Segment`.
    assert!(assemble_err("OCTA Text_Segment").contains("Undefined symbol"));
}

// ---- The root prefix ------------------------------------------------

#[test]
fn test_root_prefix_row_prefix_pk_then_reset() {
    let mut asm = MMixAssembler::new("v IS 7\nPREFIX Pk:\nPREFIX :\nMain SET $0,v\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(0, 7));
}

#[test]
fn test_root_prefix_row_colon_x_reference() {
    let mut asm = MMixAssembler::new("x IS 5\nMain SET $1,:x\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
}

#[test]
fn test_root_prefix_row_prefix_foo_then_reset() {
    let mut asm = MMixAssembler::new(
        "PREFIX Foo\nbar IS 5\nPREFIX :\nMain SET $1,Foobar\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
}

#[test]
fn test_root_prefix_main_key_carries_no_colon() {
    let mut asm = MMixAssembler::new("PREFIX :\nMain HALT\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Main"), Some(&0));
    assert!(!asm.labels.contains_key(":Main"));
}

#[test]
fn test_root_prefix_x_then_colon_x_is_the_redefinition_error() {
    assert!(assemble_err("x IS 1\n:x IS 2\nMain HALT\n").contains("redefined"));
}

#[test]
fn test_root_prefix_labels_keys_colon_lib_without_colon() {
    let mut asm = MMixAssembler::new(":Lib HALT\nMain HALT\n", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Lib"), Some(&0));
    assert!(!asm.labels.contains_key(":Lib"));
}

// ---- Predefined names: a program's own definition wins --------------

#[test]
fn test_label_named_predefined_wins_for_a_later_reference() {
    let mut asm = MMixAssembler::new(
        "LOC #108\nFputs HALT\nLOC #100\nMain SET $1,Fputs\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(1, 0x108));
}

#[test]
fn test_label_named_predefined_reaches_a_pushj_target() {
    let mut asm = MMixAssembler::new(
        "LOC #108\nTime POP 0,0\nLOC #100\nMain PUSHJ $0,Time\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert!(matches!(asm.instructions[1].1, MMixInstruction::PUSHJ(..)));
}

#[test]
fn test_use_then_redefine_via_label_is_an_error() {
    let err = {
        let mut asm = MMixAssembler::new("Main SET $1,Fputs\nFputs HALT\n", "<test>");
        asm.parse().expect_err("expected used-then-redefined error")
    };
    assert!(
        err.contains("predefined symbol 'Fputs' redefined"),
        "err: {err}"
    );
    assert!(err.contains("its value was used at"), "err: {err}");
}

#[test]
fn test_use_then_redefine_via_is_is_an_error() {
    let err = {
        let mut asm = MMixAssembler::new("Main SET $1,Fputs\nFputs IS 3\n", "<test>");
        asm.parse().expect_err("expected used-then-redefined error")
    };
    assert!(
        err.contains("predefined symbol 'Fputs' redefined"),
        "err: {err}"
    );
    assert!(err.contains("its value was used at"), "err: {err}");
}

#[test]
fn test_second_different_definition_after_a_label_is_the_ordinary_redefinition_error() {
    assert!(
        assemble_err("Fputs HALT\nFputs IS 3\nMain HALT\n").contains("symbol 'Fputs' redefined",)
    );
}

// ---- Equal redefinition ----------------------------------------------

#[test]
fn test_equal_redefinition_is_then_label_same_value_assembles() {
    let mut asm = MMixAssembler::new("Here IS #104\nLOC #104\nHere HALT\n", "<test>");
    asm.parse().unwrap();
}

#[test]
fn test_equal_redefinition_is_then_label_different_value_is_an_error() {
    assert!(
        assemble_err("Here IS #104\nLOC #108\nHere HALT\n").contains("symbol 'Here' redefined",)
    );
}

#[test]
fn test_equal_redefinition_register_vs_pure_value_is_an_error() {
    assert!(assemble_err("x IS $1\nx IS 1\nMain HALT\n").contains("symbol 'x' redefined"));
}
