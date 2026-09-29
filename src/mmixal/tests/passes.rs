//! Tests for a repeated `parse`, LOCAL declarations and BSPEC/ESPEC special mode.

use super::*;

// ---- repeated parse ------------------------------------------------

/// A forward reference, a named and a local-label `GREG`, a local label, a
/// data directive that warns, a `debug` directive and a second unit.
const STATEFUL_MAIN: &str = "\
        LOC     #100
Base    GREG    #2000
Main    JMP     Fwd
        BYTE    300,\"hi\"
1H      OCTA    Data_Segment
        GETA    $1,1B
        debug   \"seen\"
Fwd     SETL    $2,Limit
Limit   IS      7
        LDA     $3,Base,8
";

const STATEFUL_LIB: &str = "\
2H      GREG    5
Lib     SWYM
        GETA    $4,2F
2H      TETRA   Lib
";

fn stateful_assembler(main: &str) -> MMixAssembler {
    let mut asm = MMixAssembler::new(main, "main.mms");
    asm.add_source(STATEFUL_LIB, "lib.mms");
    asm
}

/// Every observable of `asm`'s public surface, in a comparable form.
fn surface(asm: &MMixAssembler) -> String {
    let labels: BTreeMap<_, _> = asm.labels.iter().collect();
    let symbols: BTreeMap<_, _> = asm.symbols.iter().collect();
    let locs: Vec<_> = (0..0x400).map(|addr| asm.source_loc(addr)).collect();
    let lines: Vec<_> = ["main.mms", "lib.mms"]
        .iter()
        .flat_map(|file| (1..=12).map(move |line| (file, line)))
        .map(|(file, line)| (asm.addr_for_line(file, line), asm.source_text(file, line)))
        .collect();
    format!(
        "{:?}\n{labels:?}\n{symbols:?}\n{:?}\n{:?}\n{:?}\n{locs:?}\n{lines:?}\n{:?}",
        asm.instructions,
        asm.greg_inits,
        asm.warnings(),
        asm.generate_object_code(),
        asm.debug_strings(),
    )
}

#[test]
fn a_second_parse_matches_a_single_parse() {
    let mut fresh = stateful_assembler(STATEFUL_MAIN);
    fresh.parse().unwrap();
    assert_eq!(fresh.greg_inits.len(), 2);
    assert_eq!(fresh.warnings().len(), 1);

    let mut repeated = stateful_assembler(STATEFUL_MAIN);
    repeated.parse().unwrap();
    repeated.parse().unwrap();
    assert_eq!(surface(&repeated), surface(&fresh));

    repeated.parse().unwrap();
    assert_eq!(surface(&repeated), surface(&fresh));
}

#[test]
fn a_second_parse_that_fails_in_pass_two_matches_a_single_failed_parse() {
    let failing = STATEFUL_MAIN.replace("SETL    $2,Limit", "SETL    $2,Missing");
    let mut fresh = stateful_assembler(&failing);
    let fresh_err = fresh.parse().unwrap_err();
    assert!(fresh_err.contains("Missing"), "err: {fresh_err}");
    assert!(!fresh.instructions.is_empty());
    assert!(!fresh.greg_inits.is_empty());

    let mut repeated = stateful_assembler(&failing);
    assert_eq!(repeated.parse().unwrap_err(), fresh_err);
    assert_eq!(repeated.parse().unwrap_err(), fresh_err);
    assert_eq!(surface(&repeated), surface(&fresh));
}

/// `source`'s single parse and its second parse must agree on the surface.
fn assert_second_parse_matches_first(source: &str) {
    let mut fresh = MMixAssembler::new(source, "main.mms");
    let fresh_result = fresh.parse();

    let mut repeated = MMixAssembler::new(source, "main.mms");
    let _ = repeated.parse();
    assert_eq!(repeated.parse(), fresh_result);
    assert_eq!(surface(&repeated), surface(&fresh));
}

#[test]
fn a_second_parse_of_a_source_without_loc_starts_at_zero() {
    let source = "Main JMP Fwd\n SWYM\nFwd SWYM\n";
    let mut repeated = MMixAssembler::new(source, "main.mms");
    repeated.parse().unwrap();
    repeated.parse().unwrap();

    assert_eq!(repeated.instructions[0], (0, MMixInstruction::JMP(2)));
    assert_second_parse_matches_first(source);
}

#[test]
fn a_second_parse_of_a_source_ending_at_the_top_of_memory_succeeds() {
    let source = " SWYM\n LOC #FFFFFFFFFFFFFFFC\n TETRA 0\n";
    let mut repeated = MMixAssembler::new(source, "main.mms");
    repeated.parse().unwrap();
    repeated.parse().unwrap();

    assert_second_parse_matches_first(source);
}

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
