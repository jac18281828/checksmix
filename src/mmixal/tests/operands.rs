//! Tests for the two-operand memory form base-address search.

use super::*;

// ---- The two-operand memory form (base-address search) -------------

#[test]
fn test_base_address_form_resolves_against_preceding_greg() {
    assert_first_instruction(
        "Base GREG #1000\nData IS #1000\nLDO $1,Data",
        MMixInstruction::LDOI(1, 254, 0),
    );
}

#[test]
fn test_base_address_form_offset_255_is_the_widest_accepted() {
    assert_first_instruction(
        "Base GREG #1000\nData IS #10FF\nLDO $1,Data",
        MMixInstruction::LDOI(1, 254, 255),
    );
}

#[test]
fn test_base_address_form_offset_256_is_an_error() {
    assert!(
        assemble_err("Base GREG #1000\nData IS #1100\nLDO $1,Data")
            .contains("no GREG before this instruction holds a base address")
    );
}

#[test]
fn test_base_address_form_closer_greg_wins() {
    // Far allocates $254, Near allocates $253; Near's base (#1080) is
    // closer to Data (#1090) than Far's (#1000).
    assert_first_instruction(
        "Far GREG #1000\nNear GREG #1080\nData IS #1090\nLDO $1,Data",
        MMixInstruction::LDOI(1, 253, 16),
    );
}

#[test]
fn test_base_address_form_tie_takes_the_earliest_allocated() {
    assert_first_instruction(
        "A GREG #1000\nB GREG #1000\nData IS #1000\nLDO $1,Data",
        MMixInstruction::LDOI(1, 254, 0),
    );
}

#[test]
fn test_base_address_form_ignores_a_greg_after_the_instruction() {
    assert!(
        assemble_err("Data IS #1000\nLDO $1,Data\nLate GREG #1000")
            .contains("no GREG before this instruction holds a base address")
    );
}

#[test]
fn test_base_address_form_greg_zero_never_matches() {
    assert!(
        assemble_err("Zero GREG 0\nData IS #10\nLDO $1,Data")
            .contains("no GREG before this instruction holds a base address")
    );
}

#[test]
fn test_base_address_form_stb_takes_it() {
    assert_first_instruction(
        "Base GREG #1000\nData IS #1000\nSTB $1,Data",
        MMixInstruction::STBI(1, 254, 0),
    );
}

#[test]
fn test_base_address_form_go_takes_it() {
    assert_first_instruction(
        "Base GREG #1000\nData IS #1000\nGO $1,Data",
        MMixInstruction::GOI(1, 254, 0),
    );
}

#[test]
fn test_base_address_form_preld_takes_it() {
    assert_first_instruction(
        "Base GREG #1000\nData IS #1000\nPRELD 3,Data",
        MMixInstruction::PRELDI(3, 254, 0),
    );
}

#[test]
fn test_base_address_form_stco_takes_it() {
    assert_first_instruction(
        "Base GREG #1000\nData IS #1000\nSTCO 5,Data",
        MMixInstruction::STCOI(5, 254, 0),
    );
}

#[test]
fn test_base_address_form_forward_reference_resolves_and_is_one_tetra() {
    let mut asm = MMixAssembler::new("Base GREG #1000\nLDO $1,Data\nData IS #1000", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    assert_eq!(asm.instructions[0].1, MMixInstruction::LDOI(1, 254, 0));
    assert_eq!(
        MMixAssembler::instruction_size(&asm.instructions[0].1),
        4,
        "the base-address form is always one tetra"
    );
}

#[test]
fn test_memory_two_operand_register_is_offset_zero() {
    assert_first_instruction("LDO $1,$2", MMixInstruction::LDOI(1, 2, 0));
}

#[test]
fn test_memory_two_operand_register_follows_value_not_spelling() {
    assert_first_instruction("x IS $2\nLDO $1,x", MMixInstruction::LDOI(1, 2, 0));
}

#[test]
fn test_lda_two_operand_form_always_takes_the_base_address_path() {
    // LDA resolves against Base exactly as LDO does, whatever the
    // address's own value.
    assert_first_instruction(
        "Base GREG #1000\nLDA $1,Data\nData IS #1000",
        MMixInstruction::LDAI(1, 254, 0),
    );
}

#[test]
fn test_forward_lda_keeps_every_label_in_place() {
    // LDA costs one tetra regardless of whether its operand has
    // resolved yet, so pass 1 and pass 2 agree on every label after it
    // (the scan's `lda_fwd2`). Main's SET reads After before pass 2
    // revisits it, so it still carries pass 1's own estimate -- equal
    // to After's own SET only if that estimate already matches.
    let mut asm = MMixAssembler::new(
        "LOC #100\nBase GREG 1\nMain SET $3,After\nLDA $1,K\n\
             After SET $2,After\nTRAP 0,Halt,0\nK IS 5\n",
        "<test>",
    );
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    let after_addr = *asm.labels.get("After").expect("After label");
    assert_eq!(
        asm.instructions[0].1,
        MMixInstruction::SETL(3, after_addr as u16)
    );
    assert_eq!(
        asm.instructions[2].1,
        MMixInstruction::SETL(2, after_addr as u16)
    );
    assert_eq!(asm.instructions[1].1, MMixInstruction::LDAI(1, 254, 4));
}

#[test]
fn test_lda_pure_value_never_encodes_addu_register_form() {
    // With no GREG in scope, a pure second operand is the base-address
    // error, never register form #22.
    assert_eq!(
        assemble_err("LDA $1,5"),
        "<test>:1:8: no GREG before this instruction holds a base \
             address 0 to 255 bytes below 0x5"
    );
    // With a base in scope, it's LDAI -- opcode #23, never #22.
    let mut asm = MMixAssembler::new("B GREG 1\nLDA $1,5", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::LDAI(1, 254, 4));
    assert_eq!(
        asm.encode_instruction_bytes(&asm.instructions[0].1)[0],
        0x23
    );
}

/// Assemble `src` and return its LDA/LDO/etc. instruction: the last
/// item in `instructions`, since every case here places exactly one
/// data directive (an `OCTA` base value) ahead of the instruction under
/// test.
fn last_instruction(src: &str) -> MMixInstruction {
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
    asm.instructions
        .last()
        .unwrap_or_else(|| panic!("no instructions produced for {src:?}"))
        .1
        .clone()
}

#[test]
fn test_lda_base_search_matches_the_memory_forms() {
    let source = |addr: &str| {
        format!("LOC Data_Segment\nBase GREG @\nX OCTA 7\nLOC #100\nMain LDA $1,{addr}")
    };
    // Offset 0: Base itself covers X.
    assert_eq!(
        last_instruction(&source("X")),
        MMixInstruction::LDAI(1, 254, 0)
    );
    // Offset 255 assembles; 256 is the base-address error.
    assert_eq!(
        last_instruction(&source("X+255")),
        MMixInstruction::LDAI(1, 254, 255)
    );
    assert_eq!(
        assemble_err(&source("X+256")),
        "<test>:5:13: no GREG before this instruction holds a base \
             address 0 to 255 bytes below 0x2000000000000100"
    );
    // LDAI matches LDA.
    assert_eq!(
        last_instruction(&source("X")),
        last_instruction(&source("X").replacen("LDA", "LDAI", 1))
    );
}

#[test]
fn test_lda_base_search_ignores_a_greg_appearing_after_the_instruction() {
    // Closer holds Y's own value (offset 0), but it comes after Main:
    // the search bounds itself to GREGs already seen, so Base (offset
    // 8) wins regardless.
    assert_eq!(
        last_instruction(
            "LOC Data_Segment\nBase GREG @\nX OCTA 7\nY OCTA 9\n\
                 LOC #100\nMain LDA $1,Y\nCloser GREG Y"
        ),
        MMixInstruction::LDAI(1, 254, 8)
    );
}

#[test]
fn test_lda_register_operand_is_offset_zero() {
    assert_first_instruction("LDA $3,$2", MMixInstruction::LDAI(3, 2, 0));
    assert_first_instruction("x IS $2\nLDA $3,x", MMixInstruction::LDAI(3, 2, 0));
}

#[test]
fn test_lda_one_tetra_even_with_an_unresolved_forward_operand() {
    // Far exceeds #FF and is a forward reference; LDA still costs one
    // tetra, so After sits exactly 4 bytes past Main. Pre's own operand
    // reads After before pass 2 revisits it, so it still carries pass
    // 1's estimate of After's address -- proving that estimate is
    // already exact, not merely that pass 2's own later walk is.
    let mut asm = MMixAssembler::new(
        "Base GREG #150\nPre SET $2,After\nMain LDA $1,Far\nAfter HALT\nFar IS #200",
        "<test>",
    );
    asm.parse().unwrap();
    let main_addr = *asm.labels.get("Main").unwrap();
    let after_addr = *asm.labels.get("After").unwrap();
    assert_eq!(after_addr, main_addr + 4);
    assert_eq!(
        asm.instructions[0].1,
        MMixInstruction::SETL(2, after_addr as u16)
    );
    assert_eq!(asm.instructions[1].1, MMixInstruction::LDAI(1, 254, 176));
}

#[test]
fn test_greg_limit_is_223_and_the_last_is_32() {
    let mut source = String::new();
    for i in 0..223 {
        source.push_str(&format!("G{i}\tGREG\t0\n"));
    }
    source.push_str("Main\tHALT\n");
    let mut asm = MMixAssembler::new(&source, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("223 GREGs must assemble: {e}"));
    assert_eq!(asm.greg_inits.last().map(|&(reg, _)| reg), Some(32));
}

#[test]
fn test_the_224th_greg_reports_the_limit_at_its_line_and_column() {
    let mut source = String::new();
    for i in 0..224 {
        source.push_str(&format!("G{i}\tGREG\t0\n"));
    }
    source.push_str("Main\tHALT\n");
    assert_eq!(
        assemble_err(&source),
        "<test>:224:6: GREG has no global register left: \
             $32 through $254 are all allocated"
    );
}

#[test]
fn test_generate_object_code_carries_greg_values_through_the_loader() {
    let mut asm = MMixAssembler::new("Base GREG #1234\nMain HALT\n", "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse: {e}"));
    let object_code = asm.generate_object_code();

    let decoder = MmoDecoder::new(object_code);
    let mut mmix = MMix::new();
    decoder
        .load(&mut mmix)
        .expect("generate_object_code's output must load");

    assert_eq!(mmix.get_special(SpecialReg::RG), 254);
    assert_eq!(mmix.get_register(254), 0x1234);
}
