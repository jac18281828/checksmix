//! Tests for the PREFIX and data-unit-warning directives.

use super::*;

// ---- PREFIX directive tests ----

#[test]
fn test_prefix_qualifies_unqualified_symbol() {
    let src = "\
            PREFIX Sub_\n\
            Bar IS 5\n";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.symbols.get("Sub_Bar").copied(),
        Some(SymbolType::Constant(5))
    );
    assert!(!asm.symbols.contains_key("Bar"));
}

#[test]
fn test_prefix_colon_opts_out() {
    let src = "\
            PREFIX Sub_\n\
            :Foo IS 9\n";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    // A leading ':' opts out of the active PREFIX and, at the root, is
    // stored without the colon.
    assert_eq!(
        asm.symbols.get("Foo").copied(),
        Some(SymbolType::Constant(9))
    );
    assert!(!asm.symbols.contains_key(":Foo"));
    assert!(!asm.symbols.contains_key("Sub_:Foo"));
    assert!(!asm.symbols.contains_key("Sub_Foo"));
}

#[test]
fn test_prefix_persists_across_files() {
    // PREFIX set in file A applies to definitions in file B.
    let a = "PREFIX P_\n";
    let b = "\
            LOC #100\n\
            Bar HALT\n";
    let mut asm = MMixAssembler::new(a, "a.mms");
    asm.add_source(b, "b.mms");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("P_Bar").copied(), Some(0x100));
    assert!(!asm.labels.contains_key("Bar"));
}

#[test]
fn test_prefix_reset_to_global() {
    // `PREFIX :` makes unqualified names resolve under the global root.
    let src = "\
            PREFIX P_\n\
            Bar IS 1\n\
            PREFIX :\n\
            Baz IS 2\n";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.symbols.get("P_Bar").copied(),
        Some(SymbolType::Constant(1))
    );
    assert_eq!(
        asm.symbols.get("Baz").copied(),
        Some(SymbolType::Constant(2))
    );
}

// ---- The data-unit warning channel ----------------------------------

#[test]
fn test_data_unit_overflow_warns_and_keeps_low_bytes() {
    let mut asm = MMixAssembler::new("BYTE 300", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0x2C));
    assert_eq!(
        asm.warnings(),
        ["<test>:1:6: warning: value 300 does not fit in a byte; \
              its low byte assembles"]
    );

    let mut asm = MMixAssembler::new("WYDE #12345", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::WYDE(0x2345));
    assert_eq!(
        asm.warnings(),
        ["<test>:1:6: warning: value 74565 does not fit in a wyde; \
              its low wyde assembles"]
    );

    let mut asm = MMixAssembler::new("TETRA #100000000", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::TETRA(0));
    assert_eq!(
        asm.warnings(),
        [
            "<test>:1:7: warning: value 4294967296 does not fit in a tetra; \
              its low tetra assembles"
        ]
    );

    let mut asm = MMixAssembler::new("BYTE -1", "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.warnings(),
        ["<test>:1:6: warning: value -1 does not fit in a byte; \
              its low byte assembles"]
    );

    for src in ["BYTE 255", "OCTA -1"] {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse().unwrap();
        assert!(asm.warnings().is_empty(), "{src:?} should not warn");
    }
}

#[test]
fn test_data_list_two_overflowing_items_warn_in_order_once_each() {
    let mut asm = MMixAssembler::new("BYTE 300,300", "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.warnings(),
        [
            "<test>:1:6: warning: value 300 does not fit in a byte; \
                 its low byte assembles",
            "<test>:1:10: warning: value 300 does not fit in a byte; \
                 its low byte assembles",
        ]
    );
}

#[test]
fn test_warnings_reports_only_the_last_parse() {
    let mut asm = MMixAssembler::new("BYTE 300", "<test>");
    asm.parse().unwrap();
    asm.parse().unwrap();
    assert_eq!(asm.warnings().len(), 1);
}

#[test]
fn test_data_item_two_overflowing_values_warn_twice_at_its_column() {
    let mut asm = MMixAssembler::new(r#"BYTE 300+"ab"+300"#, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.warnings(),
        [
            "<test>:1:6: warning: value 397 does not fit in a byte; \
                 its low byte assembles",
            "<test>:1:6: warning: value 398 does not fit in a byte; \
                 its low byte assembles",
        ]
    );
}

#[test]
fn test_string_character_overflow_warns_at_its_opening_quote() {
    let mut asm = MMixAssembler::new(r#"BYTE "a"+300"#, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(141));
    assert_eq!(
        asm.warnings(),
        ["<test>:1:6: warning: value 397 does not fit in a byte; \
              its low byte assembles"]
    );
}

#[test]
fn test_string_character_above_ff_warns_as_a_byte_overflow() {
    let mut asm = MMixAssembler::new("BYTE \"€\"", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0xAC));
    assert_eq!(
        asm.warnings(),
        ["<test>:1:6: warning: value 8364 does not fit in a byte; \
              its low byte assembles"]
    );

    let mut asm = MMixAssembler::new("BYTE \"€€\"", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions.len(), 2);
    assert_eq!(asm.instructions[0].1, MMixInstruction::BYTE(0xAC));
    assert_eq!(asm.instructions[1].1, MMixInstruction::BYTE(0xAC));
    assert_eq!(
        asm.warnings(),
        [
            "<test>:1:6: warning: value 8364 does not fit in a byte; \
                 its low byte assembles",
            "<test>:1:6: warning: value 8364 does not fit in a byte; \
                 its low byte assembles",
        ]
    );
}
