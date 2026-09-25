//! Tests for expression evaluation, literals and data-item diagnostics.

use super::*;

// ---- Expressions (C9.1) -------------------------------------------

#[test]
fn test_expr_left_associative_weak_chain() {
    // a-b-c is (a-b)-c, not a-(b-c).
    assert_first_instruction("OCTA 10-3-2", MMixInstruction::OCTA(5));
}

#[test]
fn test_expr_strong_binds_tighter_than_weak() {
    assert_first_instruction("OCTA 2+3*4", MMixInstruction::OCTA(14));
}

#[test]
fn test_expr_reference_shift_and_add_chain() {
    // The MMIXAL reference's o<<24+x<<16+y<<8+z, left-associated:
    // (o<<24)+(x<<16)+(y<<8)+z.
    let mut asm = MMixAssembler::new(
        "o IS 1\nx IS 2\ny IS 3\nz IS 4\nOCTA o<<24+x<<16+y<<8+z",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions[0].1,
        MMixInstruction::OCTA((1 << 24) + (2 << 16) + (3 << 8) + 4)
    );
}

#[test]
fn test_expr_wraps_subtraction_below_zero() {
    assert_first_instruction("OCTA 0-1", MMixInstruction::OCTA(u64::MAX));
}

#[test]
fn test_expr_wraps_addition_above_max() {
    assert_first_instruction("OCTA #FFFFFFFFFFFFFFFF+1", MMixInstruction::OCTA(0));
}

#[test]
fn test_expr_floor_fraction_operator() {
    // 1//2 is floor(2^64 * 1/2) = 2^63.
    assert_first_instruction("OCTA 1//2", MMixInstruction::OCTA(1u64 << 63));
}

#[test]
fn test_expr_shift_by_64_or_more_is_zero() {
    assert_first_instruction("OCTA 1<<64", MMixInstruction::OCTA(0));
    assert_first_instruction("OCTA 1>>64", MMixInstruction::OCTA(0));
}

#[test]
fn test_expr_register_plus_pure_selects_register_form() {
    // x IS $1, y IS $2: ADD x,y,y+1 is ADD $1,$2,$3 (register form).
    let mut asm = MMixAssembler::new("x IS $1\ny IS $2\nADD x,y,y+1", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 3));
}

#[test]
fn test_expr_register_minus_register_selects_immediate_form() {
    // ADD $1,$2,y-x is the immediate form, since register-register
    // subtraction is a pure value.
    let mut asm = MMixAssembler::new("x IS $1\ny IS $2\nADD $1,$2,y-x", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 1));
}

#[test]
fn test_expr_is_records_a_register_from_register_arithmetic() {
    // x IS $1+1 records a register, not a pure constant.
    let mut asm = MMixAssembler::new("x IS $1+1", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.symbols.get("x"), Some(&SymbolType::Register(2)));
}

#[test]
fn test_expr_set_register_arithmetic_copies() {
    // SET $1,$2+1 copies register $3 (the register NAMED $2+1),
    // never an arithmetic add on $2's runtime value.
    assert_first_instruction("SET $1,$2+1", MMixInstruction::SETRR(1, 3));
}

#[test]
fn test_expr_register_plus_register_is_error() {
    assert!(
        assemble_err("x IS $1\ny IS $2\nADD $1,$2,x+y")
            .contains("+ cannot apply to a register operand")
    );
}

#[test]
fn test_expr_pure_minus_register_is_error() {
    assert!(assemble_err("x IS $1\nOCTA 3-x").contains("- cannot apply to a register operand"));
}

#[test]
fn test_expr_strong_operator_on_register_is_error() {
    assert!(assemble_err("x IS $1\nOCTA x*2").contains("* cannot apply to a register operand"));
}

#[test]
fn test_expr_unary_minus_on_register_is_error() {
    assert!(assemble_err("x IS $1\nSET $2,-x").contains("unary - cannot apply to a register"));
}

#[test]
fn test_expr_unary_tilde_on_register_is_error() {
    assert!(assemble_err("x IS $1\nSET $2,~x").contains("unary ~ cannot apply to a register"));
}

#[test]
fn test_expr_unary_dollar_on_register_is_error() {
    assert!(assemble_err("x IS $1\nSET $2,$x").contains("unary $ cannot apply to a register"));
}

#[test]
fn test_expr_register_in_pure_site_is_error() {
    assert!(
        assemble_err("x IS $1\nLOC x").contains("cannot be used where a pure value is required")
    );
}

#[test]
fn test_expr_pure_value_in_register_site_is_error() {
    assert!(assemble_err("ADD 3,$1,$2").contains("cannot be used where a register is required"));
}

#[test]
fn test_expr_final_register_above_255_is_error() {
    assert!(assemble_err("SET $1,$260").contains("out of range 0..255"));
}

#[test]
fn test_expr_division_by_zero_is_error() {
    assert!(assemble_err("OCTA 5/0").contains("division by zero"));
}

#[test]
fn test_expr_percent_by_zero_is_error() {
    // `%` shares `/`'s zero-divisor check: it computes the remainder of
    // the same division, which is illegal at y=0.
    assert!(assemble_err("OCTA 5%0").contains("division by zero"));
}

#[test]
fn test_expr_illegal_fraction_is_error() {
    assert!(assemble_err("OCTA 2//1").contains("illegal fraction"));
}

#[test]
fn test_expr_unary_ampersand_is_unsupported() {
    assert!(
        assemble_err("Foo IS 1\nOCTA &Foo")
            .contains("unary & (a symbol's serial number) is unsupported")
    );
}

#[test]
fn test_expr_dangling_operator_is_syntax_error() {
    assert!(
        assemble_err("SETL $1,5+")
            .contains("a remark must be separated from the statement by a blank")
    );
}

#[test]
fn test_percent_inside_bare_expression_is_remainder() {
    assert_first_instruction("SET $1,5%3", MMixInstruction::SETL(1, 2));
}

#[test]
fn test_percent_after_space_opens_a_comment() {
    assert_first_instruction("SET $1,5 % 3", MMixInstruction::SETL(1, 5));
}

#[test]
fn test_percent_before_space_still_opens_a_comment() {
    assert_first_instruction("SET $1,5% 3", MMixInstruction::SETL(1, 5));
}

#[test]
fn test_percent_inside_a_group_is_remainder() {
    assert_first_instruction("SET $1,(5 % 3)", MMixInstruction::SETL(1, 2));
}

#[test]
fn test_percent_after_a_closed_group_opens_a_comment() {
    // `sum` is undefined; if this parsed as an operator the undefined
    // symbol would fail, so success proves the comment.
    assert_first_instruction("SET $1,(2 + 3) % sum", MMixInstruction::SETL(1, 5));
}

#[test]
fn test_semicolon_after_an_expression_starts_a_new_statement() {
    // `;` no longer opens a comment: `text` is a second statement, a
    // bare label needing no leading blank, defined at SET's address.
    let mut asm = MMixAssembler::new("SET $1,5;text", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    assert_eq!(asm.labels.get("text"), Some(&4));
}

#[test]
fn test_whitespace_after_a_weak_operator_is_a_syntax_error() {
    assert!(assemble_err("SETL $1,2 + 3").contains("a remark cannot begin with"));
}

#[test]
fn test_whitespace_after_unary_minus_is_a_syntax_error() {
    assert!(assemble_err("SET $1,- 5").contains("unknown operation"));
}

#[test]
fn test_bare_expression_closed_up_assembles() {
    assert_first_instruction("SETL $1,2+3", MMixInstruction::SETL(1, 5));
}

#[test]
fn test_parenthesized_group_may_hold_whitespace() {
    assert_first_instruction("SETL $1,(2 + 3)", MMixInstruction::SETL(1, 5));
}

#[test]
fn test_nested_groups_evaluate_innermost_first() {
    assert_first_instruction("SETL $1,((2 + 3) * 4)", MMixInstruction::SETL(1, 20));
}

#[test]
fn test_group_and_bare_operators_left_associate() {
    // Strong binds tighter than weak, and both are left-associative:
    // 2+(3*4)+5 is (2+(3*4))+5 = 2+12+5 = 19.
    assert_first_instruction("SETL $1,2+(3 * 4)+5", MMixInstruction::SETL(1, 19));
}

#[test]
fn test_unclosed_group_reports_unterminated_group() {
    let err = assemble_err("SETL $1,(2 + 3");
    assert!(
        err.contains("unterminated group"),
        "expected an unterminated-group diagnostic, got: {err}"
    );
}

#[test]
fn test_comma_inside_an_open_group_is_an_error() {
    assert!(assemble_err("SETL $1,(1 , 2)").contains("unknown operation"));
}

#[test]
fn test_newline_inside_an_open_group_is_an_error() {
    assert!(assemble_err("SETL $1,(1\n2)").contains("unterminated group"));
}

/// A negative `SET` source is an error whether the literal is decimal
/// or hex.
#[test]
fn test_set_negative_literal_is_an_error_for_decimal_and_hex() {
    assert_eq!(
        assemble_err("SET $1,-1"),
        "<test>:1:8: immediate operand -1 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
    );
    assert_eq!(
        assemble_err("SET $1,-5"),
        "<test>:1:8: immediate operand -5 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
    );
    assert_eq!(
        assemble_err("SET $1,-#10"),
        "<test>:1:8: immediate operand -16 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
    );
}

#[test]
fn test_at_in_an_instruction_is_the_aligned_address_after_byte() {
    let mut asm = MMixAssembler::new("BYTE 1\nSET $1,@", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::SETL(1, 4));
}

#[test]
fn test_at_at_in_a_data_directive_both_hold_the_aligned_address() {
    let mut asm = MMixAssembler::new("BYTE 1,1,1\nOCTA @,@", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[3].1, MMixInstruction::OCTA(8));
    assert_eq!(asm.instructions[4].1, MMixInstruction::OCTA(8));
}

#[test]
fn test_forward_reference_with_operator_resolves() {
    let mut asm = MMixAssembler::new(
        "JMP Later+4\nOCTA Later-8\nLater IS 100\nJMP Later",
        "<test>",
    );
    asm.parse()
        .unwrap_or_else(|e| panic!("forward reference with operator must resolve: {e}"));
    assert_eq!(asm.instructions[1].1, MMixInstruction::OCTA(92));
}

#[test]
fn test_loc_forward_reference_fails_like_today() {
    assert!(assemble_err("LOC Later+4\nLater IS 100").contains("Undefined symbol: Later"));
}

#[test]
fn test_is_forward_reference_fails_like_today() {
    assert!(assemble_err("Foo IS Later+1\nLater IS 100").contains("Undefined symbol: Later"));
}

#[test]
fn test_greg_forward_reference_fails_like_today() {
    assert!(assemble_err("GREG Later+1\nLater IS 100").contains("Undefined symbol: Later"));
}

#[test]
fn test_loc_label_takes_the_location_before_loc() {
    // After LOC #100 and one instruction, the counter is #104; `Gap`
    // must name #104, not the #300 the LOC on its own line jumps to.
    let mut asm = MMixAssembler::new("LOC #100\nMain TRAP 0,Halt,0\nGap LOC #300", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Gap"), Some(&0x104));
}

#[test]
fn test_loc_label_order_holds_within_pass_one() {
    // GREG's init value is computed once, in pass 1, and never
    // recomputed in pass 2 (which only checks the symbol is present),
    // so this is the one place a pass-1-only ordering bug survives to
    // the final state: `GREG Gap` must read `Gap`'s pre-move address.
    let mut asm = MMixAssembler::new("LOC #100\nGap LOC #300\nGREG Gap", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.greg_inits.last().map(|(_, v)| *v), Some(0x100));
}

#[test]
fn test_loc_at_plus_offset_names_the_prior_location() {
    // X LOC @+500 gives X the location before LOC, and leaves the
    // counter at X+500.
    let mut asm = MMixAssembler::new("LOC #100\nX LOC @+500\nBYTE 1", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("X"), Some(&0x100));
    assert_eq!(asm.instructions[0].0, 0x100 + 500);
}

#[test]
fn test_wyde_data_list_mixes_expression_items_and_a_string() {
    // The pass-agreement pattern of test_byte_string_pass1_pass2_agree:
    // a forward OCTA reads pass 1's size for the list, so pass 2 must
    // compute the same expression values or Next's address disagrees.
    let mut asm = MMixAssembler::new(
        "Base IS 2\nOCTA Next\nList WYDE Base+8,\"ab\",Base*10\nNext BYTE 99",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("List"), Some(&8));
    let list: Vec<_> = asm.instructions[1..5]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        list,
        vec![
            (8, MMixInstruction::WYDE(10)),
            (10, MMixInstruction::WYDE(b'a' as u16)),
            (12, MMixInstruction::WYDE(b'b' as u16)),
            (14, MMixInstruction::WYDE(20)),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&16));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(16));
}

#[test]
fn test_tetra_data_list_mixes_expression_items_and_a_string() {
    let mut asm = MMixAssembler::new(
        "Base IS 2\nOCTA Next\nList TETRA Base+8,\"ab\",Base*10\nNext BYTE 99",
        "<test>",
    );
    asm.parse().unwrap();
    let list: Vec<_> = asm.instructions[1..5]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        list,
        vec![
            (8, MMixInstruction::TETRA(10)),
            (12, MMixInstruction::TETRA(b'a' as u32)),
            (16, MMixInstruction::TETRA(b'b' as u32)),
            (20, MMixInstruction::TETRA(20)),
        ]
    );
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(24));
}

#[test]
fn test_octa_data_list_mixes_expression_items_and_a_string() {
    let mut asm = MMixAssembler::new(
        "Base IS 2\nOCTA Next\nList OCTA Base+8,\"ab\",Base*10\nNext BYTE 99",
        "<test>",
    );
    asm.parse().unwrap();
    let list: Vec<_> = asm.instructions[1..5]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        list,
        vec![
            (8, MMixInstruction::OCTA(10)),
            (16, MMixInstruction::OCTA(b'a' as u64)),
            (24, MMixInstruction::OCTA(b'b' as u64)),
            (32, MMixInstruction::OCTA(20)),
        ]
    );
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(40));
}

#[test]
fn test_expr_weak_bitwise_or_and_xor() {
    assert_first_instruction("OCTA 0xF0|0x0F", MMixInstruction::OCTA(0xFF));
    assert_first_instruction("OCTA 0xFF^0x0F", MMixInstruction::OCTA(0xF0));
}

#[test]
fn test_expr_strong_bitwise_and() {
    assert_first_instruction("OCTA 0xFF&0x0F", MMixInstruction::OCTA(0x0F));
}

// ---- Character and string literals, and wide numeric constants -----

#[test]
fn test_char_literal_takes_the_reference_form() {
    // One quote, one character, one quote -- the character may itself
    // be a quote, so `'''` is the apostrophe and `'\'` the backslash.
    // An ordinary letter, digit or operator character between the
    // quotes is its own ASCII value.
    assert_first_instruction("SET $1,'''", MMixInstruction::SETL(1, 39));
    assert_first_instruction("SET $1,'\\'", MMixInstruction::SETL(1, 92));
    assert_first_instruction("SET $1,'A'", MMixInstruction::SETL(1, 65));
    assert_first_instruction("SET $1,'0'", MMixInstruction::SETL(1, 48));
    assert_first_instruction("SET $1,'%'", MMixInstruction::SETL(1, 37));
    assert_first_instruction("SET $1,';'", MMixInstruction::SETL(1, 59));
}

#[test]
fn test_char_literal_takes_any_characters_unicode_scalar_value() {
    // p. 37 rule 2(c): a character constant is the Unicode value of
    // the quoted character.
    assert_first_instruction("SET $1,'é'", MMixInstruction::SETL(1, 0xE9));
    assert_first_instruction("SET $1,'π'", MMixInstruction::SETL(1, 0x3C0));
    assert_first_instruction("SET $1,'Ω'", MMixInstruction::SETL(1, 0x3A9));
}

#[test]
fn test_wyde_char_and_string_literals_take_their_code_point() {
    // p. 37, the paragraph after rule 2: a string stands for the
    // character constants of its characters.
    assert_first_instruction("WYDE '算'", MMixInstruction::WYDE(0x7B97));
    assert_first_instruction("WYDE \"π\"", MMixInstruction::WYDE(0x03C0));
}

#[test]
fn test_octa_string_takes_the_characters_full_code_point() {
    assert_first_instruction("OCTA \"€\"", MMixInstruction::OCTA(0x20AC));
}

#[test]
fn test_wyde_string_combines_its_code_point_with_an_operator() {
    assert_first_instruction("WYDE \"π\"+1", MMixInstruction::WYDE(0x03C1));
}

#[test]
fn test_byte_string_char_below_0x100_takes_its_code_point() {
    // A BYTE string's character takes its code point value; below
    // #100 that value fits the byte directly.
    assert_first_instruction("BYTE \"é\"", MMixInstruction::BYTE(0xE9));
}

#[test]
fn test_wyde_string_label_offset_counts_characters_not_utf8_bytes() {
    // A string contributes one item per character, not per UTF-8
    // byte: "πé" is two WYDE items, six bytes, regardless of either
    // character's own value. Checks the label against the address
    // `L`'s own instruction actually lands at, not only the label
    // map.
    let mut asm = MMixAssembler::new("W WYDE \"πé\",0\nL BYTE 1", "<test>");
    asm.parse().unwrap();
    let w = *asm.labels.get("W").unwrap();
    let l = *asm.labels.get("L").unwrap();
    assert_eq!(l, w + 6);
    assert_eq!(
        asm.instructions.iter().find(|(addr, _)| *addr == l),
        Some(&(l, MMixInstruction::BYTE(1)))
    );
}

#[test]
fn test_byte_string_backslash_is_an_ordinary_byte() {
    let mut asm = MMixAssembler::new("BYTE \"a\\nb\"", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![
            MMixInstruction::BYTE(b'a'),
            MMixInstruction::BYTE(b'\\'),
            MMixInstruction::BYTE(b'n'),
            MMixInstruction::BYTE(b'b'),
        ]
    );
}

#[test]
fn test_debug_string_holds_a_backslash_as_four_ordinary_bytes() {
    let source = "        LOC     #100\nMain    debug \"a\\tb\"\n        TRAP    0,Halt,0\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.debug_strings(), &[b"a\\tb".to_vec()]);
}

#[test]
fn test_abutting_string_after_a_backslash_constant_is_a_remark_error() {
    // '\'' is a whole constant (the backslash); the string that abuts
    // it with no blank between reads as continuing the statement, not
    // as a remark -- the same rule any other abutting text follows.
    assert_eq!(
        assemble_err("SET $1,'\\'\"(\""),
        "<test>:1:11: syntax error: a remark must be separated from the \
             statement by a blank"
    );
}

#[test]
fn test_hex_and_decimal_constants_reduce_mod_2_64() {
    assert_first_instruction(
        "OCTA #112233445566778899",
        MMixInstruction::OCTA(0x2233445566778899),
    );
    assert_first_instruction("OCTA 18446744073709551621", MMixInstruction::OCTA(5));
    assert_first_instruction("OCTA 0x10000000000000005", MMixInstruction::OCTA(5));
    assert_first_instruction(
        "OCTA 340282366920938463463374607431768211461",
        MMixInstruction::OCTA(5),
    );
    let src = format!("OCTA #1{}5", "0".repeat(32));
    assert_first_instruction(&src, MMixInstruction::OCTA(5));
}

#[test]
fn test_byte_list_string_in_expression_splits_at_its_boundary_characters() {
    // MMIX.md's example: an operator before the string binds to its
    // first character, one after it to its last, and the characters
    // between stand alone. The forward OCTA is resolved in pass 1, so
    // it must agree with the label pass 2 actually places.
    let mut asm = MMixAssembler::new("OCTA Next\nBYTE 1+\"ace\"+2,0\nNext BYTE 99", "<test>");
    asm.parse().unwrap();
    let items: Vec<_> = asm.instructions[1..5]
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        items,
        vec![
            MMixInstruction::BYTE(b'b'),
            MMixInstruction::BYTE(b'c'),
            MMixInstruction::BYTE(b'g'),
            MMixInstruction::BYTE(0),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&12));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(12));
}

#[test]
fn test_byte_list_two_strings_joined_by_an_operator_merge_at_the_seam() {
    let mut asm = MMixAssembler::new("OCTA Next\nBYTE \"ab\"+\"cd\"\nNext BYTE 99", "<test>");
    asm.parse().unwrap();
    let items: Vec<_> = asm.instructions[1..4]
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        items,
        vec![
            MMixInstruction::BYTE(b'a'),
            MMixInstruction::BYTE(197), // 'b' (98) + 'c' (99)
            MMixInstruction::BYTE(b'd'),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&11));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(11));
}

#[test]
fn test_wyde_single_char_string_combines_with_its_operator() {
    assert_first_instruction("WYDE \"a\"+1", MMixInstruction::WYDE(0x0062));
}

#[test]
fn test_byte_list_parenthesized_single_char_string_is_its_value() {
    assert_first_instruction("BYTE (\"a\")", MMixInstruction::BYTE(97));
}

#[test]
fn test_byte_list_parenthesized_multi_char_string_is_an_error() {
    assert!(assemble_err("BYTE (\"ab\")").contains("not a single value"));
}

#[test]
fn test_byte_list_empty_string_inside_an_expression_is_an_error() {
    assert!(
        assemble_err("BYTE 1+\"\"+2")
            .contains("an empty string is not a value inside an expression")
    );
}

#[test]
fn test_set_operand_string_is_an_error() {
    assemble_err("SET $1,\"a\"");
}

#[test]
fn test_byte_list_strong_operator_binds_to_the_strings_near_character() {
    // A strong operator before a string combines with its first
    // character only; one after combines with its last. The characters
    // between stand alone, as MMIX.md's rule requires whatever operator
    // surrounds a string.
    let mut asm = MMixAssembler::new("BYTE 2*\"ab\"", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(194), MMixInstruction::BYTE(98)]
    );

    let mut asm = MMixAssembler::new("BYTE \"ab\"*2", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(97), MMixInstruction::BYTE(196)]
    );
}

#[test]
fn test_byte_list_weak_and_strong_operators_both_reach_the_string() {
    let mut asm = MMixAssembler::new("BYTE 1+\"ab\"*2", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(98), MMixInstruction::BYTE(196)]
    );

    let mut asm = MMixAssembler::new("BYTE 2*\"ab\"+1", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(194), MMixInstruction::BYTE(99)]
    );
}

#[test]
fn test_byte_list_unary_operator_binds_to_the_strings_first_character() {
    // A unary operator applies to the string's first character only,
    // exactly like a strong or weak operator before it.
    let mut asm = MMixAssembler::new("BYTE -\"ab\"", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(159), MMixInstruction::BYTE(98)]
    );

    let mut asm = MMixAssembler::new("BYTE ~\"ab\"", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(158), MMixInstruction::BYTE(98)]
    );

    let mut asm = MMixAssembler::new("BYTE 1+-\"ab\"", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![MMixInstruction::BYTE(160), MMixInstruction::BYTE(98)]
    );
}

#[test]
fn test_byte_list_single_char_string_combines_with_a_strong_operator() {
    // A single-character string is a primary like any other: a strong
    // operator on either side reaches it whole, and division truncates.
    assert_first_instruction("BYTE \"a\"<<1", MMixInstruction::BYTE(194));

    let mut asm = MMixAssembler::new("BYTE \"abc\"/2", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        bytes,
        vec![
            MMixInstruction::BYTE(97),
            MMixInstruction::BYTE(98),
            MMixInstruction::BYTE(49),
        ]
    );
}

#[test]
fn test_wyde_list_two_strings_each_meet_the_operator_between_them() {
    // Only the seam characters ('b' and 'c') combine with the shared
    // `*`; the outer characters combine with their own neighbor.
    let mut asm = MMixAssembler::new("WYDE 2*\"ab\"*\"cd\"*3", "<test>");
    asm.parse().unwrap();
    let words: Vec<_> = asm
        .instructions
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        words,
        vec![
            MMixInstruction::WYDE(194),
            MMixInstruction::WYDE(9702),
            MMixInstruction::WYDE(300),
        ]
    );
}

#[test]
fn test_byte_list_parenthesized_string_combines_with_a_single_operator() {
    // Inside parentheses a string must reduce to one value; a single
    // character does, whatever operator reaches it.
    assert_first_instruction("BYTE (2*\"a\")", MMixInstruction::BYTE(194));
    assert_first_instruction("BYTE (-\"a\")", MMixInstruction::BYTE(159));
}

#[test]
fn test_byte_list_parenthesized_multi_char_string_is_an_error_with_any_operator() {
    // A multi-character string inside parentheses can never reduce to
    // one value, whatever operator surrounds it.
    assert_eq!(
        assemble_err("BYTE (2*\"ab\")"),
        "<test>:1:9: a 2-character string is not a single value here"
    );
    assert_eq!(
        assemble_err("BYTE (-\"ab\")"),
        "<test>:1:8: a 2-character string is not a single value here"
    );
}

#[test]
fn test_byte_list_empty_string_beside_any_operator_is_an_error() {
    assert_eq!(
        assemble_err("BYTE 2*\"\""),
        "<test>:1:8: an empty string is not a value inside an expression"
    );
    assert_eq!(
        assemble_err("BYTE -\"\""),
        "<test>:1:7: an empty string is not a value inside an expression"
    );
}

#[test]
fn test_byte_list_parenthesized_empty_string_is_an_error() {
    // An empty string inside parentheses gives the same message as one
    // beside any operator, whatever surrounds it.
    assert_eq!(
        assemble_err("BYTE (\"\")"),
        "<test>:1:7: an empty string is not a value inside an expression"
    );
    assert_eq!(
        assemble_err("BYTE (2*\"\")"),
        "<test>:1:9: an empty string is not a value inside an expression"
    );
}

#[test]
fn test_byte_list_strong_op_string_label_offset_agrees_across_passes() {
    // `2*"abc"` combines the leading `2*` with only `"abc"`'s first
    // character; the other two characters stand as their own items, so
    // the whole item is three units. The forward OCTA resolves in
    // pass 1, so it must agree with the label pass 2 actually places.
    let mut asm = MMixAssembler::new("OCTA Next\nBYTE 2*\"abc\",0\nNext BYTE 7", "<test>");
    asm.parse().unwrap();
    let items: Vec<_> = asm.instructions[1..6]
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        items,
        vec![
            MMixInstruction::BYTE(194),
            MMixInstruction::BYTE(b'b'),
            MMixInstruction::BYTE(b'c'),
            MMixInstruction::BYTE(0),
            MMixInstruction::BYTE(7),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&12));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(12));
}

#[test]
fn test_byte_list_two_strings_meeting_at_a_strong_op_label_offset_agrees_across_passes() {
    // `-"ab"*"cd"` combines only `"ab"`'s last character with `"cd"`'s
    // first through the `*` between them; `"ab"`'s first character
    // (under the unary `-`) and `"cd"`'s last stand on their own, so the
    // whole item is three units, not four.
    let mut asm = MMixAssembler::new("OCTA Next\nBYTE -\"ab\"*\"cd\",0\nNext BYTE 7", "<test>");
    asm.parse().unwrap();
    let items: Vec<_> = asm.instructions[1..6]
        .iter()
        .map(|(_, inst)| inst.clone())
        .collect();
    assert_eq!(
        items,
        vec![
            MMixInstruction::BYTE(159),
            MMixInstruction::BYTE(230),
            MMixInstruction::BYTE(100),
            MMixInstruction::BYTE(0),
            MMixInstruction::BYTE(7),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&12));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(12));
}

// ---- A string-free data item diagnoses exactly as `expr` does -------

#[test]
fn test_byte_list_unary_with_no_operand_reports_expected_primary() {
    assert_eq!(
        assemble_err("Main\tBYTE\t-\n\tTRAP\t0,Halt,0"),
        "<test>:1:12: syntax error: expected primary"
    );
}

#[test]
fn test_byte_list_group_missing_its_second_term_reports_expected_group_primary() {
    assert_eq!(
        assemble_err("Main\tBYTE\t(1+)\n\tTRAP\t0,Halt,0"),
        "<test>:1:14: syntax error: expected group_primary"
    );
}

#[test]
fn test_byte_list_leading_comma_reports_expected_data_value() {
    assert_eq!(
        assemble_err("Main\tBYTE\t,1\n\tTRAP\t0,Halt,0"),
        "<test>:1:11: syntax error: expected data_value"
    );
}

// ---- The bare empty string --------------------------------------------

#[test]
fn test_bare_empty_string_assembles_one_zero_unit_and_warns() {
    let mut asm = MMixAssembler::new(r#"BYTE """#, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0));
    assert_eq!(
        asm.warnings(),
        ["<test>:1:6: warning: an empty string assembles as one zero byte"]
    );

    let mut asm = MMixAssembler::new(r#"WYDE """#, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::WYDE(0));

    let mut asm = MMixAssembler::new(r#"OCTA """#, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(0));

    let mut asm = MMixAssembler::new(r#"BYTE "",1"#, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions[0..2]
            .iter()
            .map(|(_, i)| i.clone())
            .collect::<Vec<_>>(),
        vec![MMixInstruction::BYTE(0), MMixInstruction::BYTE(1)]
    );
}

/// A bare `""` sizes to one unit in pass 1 (`data_value_unit_count`)
/// the same as pass 2's synthesized zero, so a forward reference past
/// it lands at the same address either pass computes -- the same
/// property `test_byte_string_pass1_pass2_agree` proves for a string.
#[test]
fn test_bare_empty_string_pass1_pass2_agree() {
    let mut asm = MMixAssembler::new("OCTA Label\nBYTE \"\"\nLabel BYTE 7", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("Label"), Some(&9));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(9));
    assert_eq!(asm.instructions[2].1, MMixInstruction::BYTE(7));
}

#[test]
fn test_bare_empty_string_beside_an_operator_or_in_parens_is_an_error() {
    assert_eq!(
        assemble_err(r#"BYTE 2*"""#),
        "<test>:1:8: an empty string is not a value inside an expression"
    );
    assert_eq!(
        assemble_err(r#"BYTE ("")"#),
        "<test>:1:7: an empty string is not a value inside an expression"
    );
}
