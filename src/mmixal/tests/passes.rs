//! Tests for LOCAL declarations and BSPEC/ESPEC special mode.

use super::*;

// ---- LOCAL ---------------------------------------------------------

#[test]
fn test_local_directive_with_a_global_register_assembles() {
    let mut asm = MMixAssembler::new("LOCAL $10\nMain HALT\n", "<test>");
    asm.parse().unwrap();
}

#[test]
fn test_local_directive_bare_value_draws_the_register_required_diagnostic() {
    assert!(
        assemble_err("LOCAL 10\nMain HALT\n")
            .contains("pure value 10 cannot be used where a register is required",)
    );
}

#[test]
fn test_local_directive_at_or_above_the_threshold_fails_naming_both() {
    let err = {
        let mut asm = MMixAssembler::new("G1 GREG 0\nLOCAL $254\nMain HALT\n", "<test>");
        asm.parse().expect_err("expected threshold error")
    };
    assert!(err.contains("$254"), "err: {err}");
    assert!(err.contains("threshold"), "err: {err}");
}

#[test]
fn test_local_directive_below_32_never_fails_regardless_of_gregs() {
    let mut asm = MMixAssembler::new("LOCAL $5\nMain HALT\n", "<test>");
    asm.parse().unwrap();
}

#[test]
fn test_local_directive_with_a_label_is_an_error() {
    assert!(assemble_err("Foo LOCAL $10\nMain HALT\n").contains("takes no label"));
}

// ---- BSPEC / ESPEC -------------------------------------------------

#[test]
fn test_espec_label_address_matches_the_block_deleted() {
    let mut with_block = MMixAssembler::new(
        "Main    SET $1,0\n\
             BSPEC 1\n\
             BYTE 1,2,3,4,5\n\
             ESPEC\n\
             After   SET $2,0\n",
        "<test>",
    );
    with_block.parse().unwrap();
    let mut without_block = MMixAssembler::new("Main SET $1,0\nAfter SET $2,0\n", "<test>");
    without_block.parse().unwrap();
    assert_eq!(
        with_block.labels.get("After"),
        without_block.labels.get("After")
    );
}

#[test]
fn test_bspec_block_emits_no_instructions() {
    let mut asm = MMixAssembler::new(
        "Main SET $1,0\nBSPEC 1\nBYTE 1,2,3\nESPEC\nHALT\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_bspec_allows_greg_and_is_with_full_effect() {
    let mut asm = MMixAssembler::new(
        "BSPEC 1\n\
             G1 GREG 0\n\
             Foo IS 5\n\
             ESPEC\n\
             Main SET $1,Foo\n",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
    assert!(asm.symbols.contains_key("G1"));
}

#[test]
fn test_bspec_rejects_an_instruction() {
    assert!(
        assemble_err("BSPEC 1\nSET $1,0\nESPEC\nMain HALT\n")
            .contains("not allowed inside BSPEC/ESPEC",)
    );
}

#[test]
fn test_bspec_rejects_loc() {
    assert!(
        assemble_err("BSPEC 1\nLOC #200\nESPEC\nMain HALT\n")
            .contains("not allowed inside BSPEC/ESPEC",)
    );
}

#[test]
fn test_bspec_with_no_espec_is_an_error() {
    assert!(assemble_err("BSPEC 1\nFoo IS 5\n").contains("BSPEC"));
}

#[test]
fn test_espec_with_no_bspec_is_an_error() {
    assert!(assemble_err("ESPEC\nMain HALT\n").contains("ESPEC"));
}

#[test]
fn test_bspec_does_not_nest() {
    assert!(assemble_err("BSPEC 1\nBSPEC 2\nESPEC\nESPEC\nMain HALT\n").contains("does not nest",));
}

#[test]
fn test_bspec_operand_wider_than_two_bytes_is_an_error() {
    assert!(
        assemble_err("BSPEC #10000\nESPEC\nMain HALT\n").contains("does not fit in two bytes",)
    );
}
