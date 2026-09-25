//! The auto-immediate routing pin: base mnemonics selecting their register or immediate form, across every family it touches.

use super::*;

// -----------------------------------------------------------------
// Auto-immediate selection for base mnemonics (ADD, AND, SR, ...).
// -----------------------------------------------------------------
// The base mnemonic now accepts either a register or an in-range
// immediate as its third operand and emits the corresponding RRR or
// RRI MMixInstruction variant. The explicit *I mnemonics still work
// as before through their original code path.

#[test]
fn test_auto_arith_register_form_unchanged() {
    // Regression: ADD with register Z still emits ADD, not ADDI.
    let mut asm = MMixAssembler::new("ADD $1,$2,$3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 3));
}

#[test]
fn test_auto_arith_immediate_swap() {
    // ADD with a literal Z now selects the ADDI variant automatically.
    let mut asm = MMixAssembler::new("ADD $1,$2,5", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 5));
}

// -----------------------------------------------------------------
// SET selects its variant from the operand's kind, and the base
// load/store mnemonics select RRR/RRI the same way.
// -----------------------------------------------------------------

#[test]
fn test_set_immediate_selects_setl() {
    let mut asm = MMixAssembler::new("SET $1,5", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 5));
}

#[test]
fn test_set_accepts_a_wyde_wide_immediate() {
    // Above the 8-bit arithmetic Z field, still inside SET's own wyde.
    let mut asm = MMixAssembler::new("SET $1,20000", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETL(1, 20000));
}

#[test]
fn test_set_rejects_an_immediate_above_a_wyde() {
    let mut asm = MMixAssembler::new("SET $1,#10000", "<test>");
    let err = asm.parse().expect_err("expected out-of-range error");
    assert!(
        err.contains("SETI"),
        "error should name SETI as the wide form, got: {err}"
    );
}

#[test]
fn test_set_rejects_a_wide_symbol_operand() {
    let mut asm = MMixAssembler::new("C IS #12345\nSET $1,C", "<test>");
    let err = asm.parse().expect_err("expected out-of-range error");
    assert!(
        err.contains("SETI"),
        "error should name SETI as the wide form, got: {err}"
    );
}

#[test]
fn test_seti_accepts_an_immediate_above_a_wyde() {
    let mut asm = MMixAssembler::new("SETI $1,#10000", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SET(1, 0x10000));
}

#[test]
fn test_set_negative_literal_is_an_error() {
    assert_eq!(
        assemble_err("SET $1,-1"),
        "<test>:1:8: immediate operand -1 out of range 0..65535 for SET; use SETI or NEG for a negative constant"
    );
}

#[test]
fn test_set_symbol_register_alias_still_copies() {
    let mut asm = MMixAssembler::new("N IS $7\nSET $1,N", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETRR(1, 7));
}

#[test]
fn test_set_is_one_tetra_and_seti_is_four() {
    let mut set = MMixAssembler::new("SET $1,5", "<test>");
    set.parse().unwrap();
    let mut seti = MMixAssembler::new("SETI $1,5", "<test>");
    seti.parse().unwrap();

    assert_eq!(
        set.encode_instruction_bytes(&set.instructions[0].1).len(),
        4
    );
    assert_eq!(
        seti.encode_instruction_bytes(&seti.instructions[0].1).len(),
        16
    );
}

#[test]
fn test_auto_load_store_immediate_swap() {
    let mut base = MMixAssembler::new("LDO $1,$2,0\nSTO $1,$2,0", "<test>");
    base.parse().unwrap();
    let mut explicit = MMixAssembler::new("LDOI $1,$2,0\nSTOI $1,$2,0", "<test>");
    explicit.parse().unwrap();

    assert_eq!(base.instructions[0].1, explicit.instructions[0].1);
    assert_eq!(base.instructions[1].1, explicit.instructions[1].1);
    assert_eq!(base.instructions[0].1, MMixInstruction::LDOI(1, 2, 0));
    assert_eq!(base.instructions[1].1, MMixInstruction::STOI(1, 2, 0));
}

#[test]
fn test_auto_load_store_register_form_unchanged() {
    let mut asm = MMixAssembler::new("LDO $1,$2,$3\nSTBU $1,$2,$3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::LDO(1, 2, 3));
    assert_eq!(asm.instructions[1].1, MMixInstruction::STBU(1, 2, 3));
}

#[test]
fn test_auto_load_store_immediate_out_of_range() {
    // Z is an 8-bit field for loads and stores.
    let mut asm = MMixAssembler::new("LDO $1,$2,300", "<test>");
    let err = asm.parse().expect_err("expected out-of-range error");
    assert!(
        err.contains("out of range 0..255"),
        "error should mention range, got: {err}"
    );
}

#[test]
fn test_auto_arith_explicit_addi_still_works() {
    // The *I alias path is unchanged.
    let mut asm = MMixAssembler::new("ADDI $1,$2,5", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 5));
}

#[test]
fn test_auto_arith_addi_rejects_register_z() {
    // *I mnemonics must continue to reject a register third operand.
    let mut asm = MMixAssembler::new("ADDI $1,$2,$3", "<test>");
    assert!(asm.parse().is_err(), "ADDI with $Z should not parse");
}

#[test]
fn test_auto_arith_immediate_out_of_range() {
    // A literal Z above 255 produces an out-of-range error.
    let mut asm = MMixAssembler::new("ADD $1,$2,300", "<test>");
    let err = asm.parse().expect_err("expected out-of-range error");
    assert!(
        err.contains("out of range 0..255"),
        "error should mention range, got: {err}"
    );
}

#[test]
fn test_auto_arith_symbol_resolves_to_immediate() {
    // A symbol bound to a small constant should auto-select the RRI form.
    let src = "K IS 7\nADD $1,$2,K";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADDI(1, 2, 7));
}

#[test]
fn test_auto_arith_symbol_resolves_to_register_alias() {
    // A symbol bound to a register alias should keep the RRR form.
    let src = "R IS $4\nADD $1,$2,R";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ADD(1, 2, 4));
}

// Family coverage: one representative test per other family.

#[test]
fn test_auto_bitwise_and_with_hex_immediate() {
    let mut asm = MMixAssembler::new("AND $1,$2,#FF", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ANDI(1, 2, 0xFF));
}

#[test]
fn test_auto_shift_sr_with_decimal_immediate() {
    let mut asm = MMixAssembler::new("SR $1,$2,3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SRI(1, 2, 3));
}

#[test]
fn test_auto_bitfiddle_bdif_register_form() {
    // Bit-fiddle family still chooses RRR when Z is a register.
    let mut asm = MMixAssembler::new("BDIF $1,$2,$3", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::BDIF(1, 2, 3));
}

#[test]
fn test_auto_conditional_set_csz_with_immediate() {
    let mut asm = MMixAssembler::new("CSZ $1,$2,7", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::CSZI(1, 2, 7));
}

#[test]
fn test_auto_zero_or_set_zsp_with_immediate() {
    let mut asm = MMixAssembler::new("ZSP $1,$2,1", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::ZSPI(1, 2, 1));
}

// Regression: a program written with base mnemonics must assemble to
// the exact same bytes as the same program written with explicit *I
// mnemonics. Touches all six in-scope families.
#[test]
fn test_auto_immediate_byte_identical_to_explicit_i() {
    let auto_src = "\
ADD  $1,$2,5
SUBU $3,$4,#10
AND  $5,$6,#FF
OR   $7,$8,1
SR   $1,$2,3
SLU  $3,$4,16
BDIF $5,$6,7
SADD $7,$8,255
CSZ  $1,$2,42
CSNN $3,$4,1
ZSP  $5,$6,8
ZSEV $7,$8,128
";
    let explicit_src = "\
ADDI  $1,$2,5
SUBUI $3,$4,#10
ANDI  $5,$6,#FF
ORI   $7,$8,1
SRI   $1,$2,3
SLUI  $3,$4,16
BDIFI $5,$6,7
SADDI $7,$8,255
CSZI  $1,$2,42
CSNNI $3,$4,1
ZSPI  $5,$6,8
ZSEVI $7,$8,128
";

    let mut auto_asm = MMixAssembler::new(auto_src, "<auto>");
    auto_asm.parse().unwrap();
    let mut explicit_asm = MMixAssembler::new(explicit_src, "<explicit>");
    explicit_asm.parse().unwrap();

    assert_eq!(
        auto_asm.instructions.len(),
        explicit_asm.instructions.len(),
        "auto and explicit forms produced different instruction counts"
    );

    for (i, (auto, explicit)) in auto_asm
        .instructions
        .iter()
        .zip(explicit_asm.instructions.iter())
        .enumerate()
    {
        let auto_bytes = auto_asm.encode_instruction_bytes(&auto.1);
        let explicit_bytes = explicit_asm.encode_instruction_bytes(&explicit.1);
        assert_eq!(
            auto_bytes, explicit_bytes,
            "instruction {i}: auto form {:?} encoded to {:?}, explicit form {:?} encoded to {:?}",
            auto.1, auto_bytes, explicit.1, explicit_bytes
        );
    }
}

// -----------------------------------------------------------------
// Extensive validation for the auto-immediate path.
// -----------------------------------------------------------------
// The auto rules introduce backtracking through prefix collisions
// (e.g. AND vs ANDI, ADD vs ADDU, CSN vs CSNN). The tests below
// pin down the routing for every base mnemonic in scope, exercise
// boundary Z values, exercise symbol-Z resolution paths, and
// confirm cross-family non-interference.

/// Like `assert_first_instruction`, but for offset-bearing families
/// (branch/PB-branch offsets, GETA/GETAB and PUSHJ/PUSHJB PC-relative
/// addresses) where the exact computed value isn't what a prefix-
/// collision test is proving. Checks only the variant discriminant
/// (and any un-computed fields the predicate cares to check).
/// Assert the LAST instruction of `src`, letting a case prefix its source
/// with a label the instruction under test can reach backward.
fn assert_last_instruction_matches(src: &str, predicate: impl Fn(&MMixInstruction) -> bool) {
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
    let last = asm
        .instructions
        .last()
        .unwrap_or_else(|| panic!("no instructions produced for {src:?}"));
    assert!(
        predicate(&last.1),
        "wrong instruction variant for {src:?}: got {:?}",
        last.1
    );
}

// ---- Family-wide auto-immediate coverage ------------------------

#[test]
fn test_auto_arith_full_coverage() {
    // Every arithmetic base mnemonic + register form (RRR) and
    // immediate form (RRI). Z=5 for stability; mnemonic prefixes
    // (ADD/ADDU/2ADDU/...) must each route to their own variant.
    let cases: &[(&str, MMixInstruction)] = &[
        ("ADD $1,$2,$3", MMixInstruction::ADD(1, 2, 3)),
        ("ADD $1,$2,5", MMixInstruction::ADDI(1, 2, 5)),
        ("ADDU $1,$2,$3", MMixInstruction::ADDU(1, 2, 3)),
        ("ADDU $1,$2,5", MMixInstruction::ADDUI(1, 2, 5)),
        ("2ADDU $1,$2,$3", MMixInstruction::ADDU2(1, 2, 3)),
        ("2ADDU $1,$2,5", MMixInstruction::ADDU2I(1, 2, 5)),
        ("4ADDU $1,$2,$3", MMixInstruction::ADDU4(1, 2, 3)),
        ("4ADDU $1,$2,5", MMixInstruction::ADDU4I(1, 2, 5)),
        ("8ADDU $1,$2,$3", MMixInstruction::ADDU8(1, 2, 3)),
        ("8ADDU $1,$2,5", MMixInstruction::ADDU8I(1, 2, 5)),
        ("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3)),
        ("16ADDU $1,$2,5", MMixInstruction::ADDU16I(1, 2, 5)),
        ("SUB $1,$2,$3", MMixInstruction::SUB(1, 2, 3)),
        ("SUB $1,$2,5", MMixInstruction::SUBI(1, 2, 5)),
        ("SUBU $1,$2,$3", MMixInstruction::SUBU(1, 2, 3)),
        ("SUBU $1,$2,5", MMixInstruction::SUBUI(1, 2, 5)),
        ("MUL $1,$2,$3", MMixInstruction::MUL(1, 2, 3)),
        ("MUL $1,$2,5", MMixInstruction::MULI(1, 2, 5)),
        ("MULU $1,$2,$3", MMixInstruction::MULU(1, 2, 3)),
        ("MULU $1,$2,5", MMixInstruction::MULUI(1, 2, 5)),
        ("DIV $1,$2,$3", MMixInstruction::DIV(1, 2, 3)),
        ("DIV $1,$2,5", MMixInstruction::DIVI(1, 2, 5)),
        ("DIVU $1,$2,$3", MMixInstruction::DIVU(1, 2, 3)),
        ("DIVU $1,$2,5", MMixInstruction::DIVUI(1, 2, 5)),
        ("CMP $1,$2,$3", MMixInstruction::CMP(1, 2, 3)),
        ("CMP $1,$2,5", MMixInstruction::CMPI(1, 2, 5)),
        ("CMPU $1,$2,$3", MMixInstruction::CMPU(1, 2, 3)),
        ("CMPU $1,$2,5", MMixInstruction::CMPUI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_bitwise_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("AND $1,$2,$3", MMixInstruction::AND(1, 2, 3)),
        ("AND $1,$2,5", MMixInstruction::ANDI(1, 2, 5)),
        ("OR $1,$2,$3", MMixInstruction::OR(1, 2, 3)),
        ("OR $1,$2,5", MMixInstruction::ORI(1, 2, 5)),
        ("XOR $1,$2,$3", MMixInstruction::XOR(1, 2, 3)),
        ("XOR $1,$2,5", MMixInstruction::XORI(1, 2, 5)),
        ("ANDN $1,$2,$3", MMixInstruction::ANDN(1, 2, 3)),
        ("ANDN $1,$2,5", MMixInstruction::ANDNI(1, 2, 5)),
        ("ORN $1,$2,$3", MMixInstruction::ORN(1, 2, 3)),
        ("ORN $1,$2,5", MMixInstruction::ORNI(1, 2, 5)),
        ("NAND $1,$2,$3", MMixInstruction::NAND(1, 2, 3)),
        ("NAND $1,$2,5", MMixInstruction::NANDI(1, 2, 5)),
        ("NOR $1,$2,$3", MMixInstruction::NOR(1, 2, 3)),
        ("NOR $1,$2,5", MMixInstruction::NORI(1, 2, 5)),
        ("NXOR $1,$2,$3", MMixInstruction::NXOR(1, 2, 3)),
        ("NXOR $1,$2,5", MMixInstruction::NXORI(1, 2, 5)),
        ("MUX $1,$2,$3", MMixInstruction::MUX(1, 2, 3)),
        ("MUX $1,$2,5", MMixInstruction::MUXI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_bitfiddle_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("BDIF $1,$2,$3", MMixInstruction::BDIF(1, 2, 3)),
        ("BDIF $1,$2,5", MMixInstruction::BDIFI(1, 2, 5)),
        ("WDIF $1,$2,$3", MMixInstruction::WDIF(1, 2, 3)),
        ("WDIF $1,$2,5", MMixInstruction::WDIFI(1, 2, 5)),
        ("TDIF $1,$2,$3", MMixInstruction::TDIF(1, 2, 3)),
        ("TDIF $1,$2,5", MMixInstruction::TDIFI(1, 2, 5)),
        ("ODIF $1,$2,$3", MMixInstruction::ODIF(1, 2, 3)),
        ("ODIF $1,$2,5", MMixInstruction::ODIFI(1, 2, 5)),
        ("SADD $1,$2,$3", MMixInstruction::SADD(1, 2, 3)),
        ("SADD $1,$2,5", MMixInstruction::SADDI(1, 2, 5)),
        ("MOR $1,$2,$3", MMixInstruction::MOR(1, 2, 3)),
        ("MOR $1,$2,5", MMixInstruction::MORI(1, 2, 5)),
        ("MXOR $1,$2,$3", MMixInstruction::MXOR(1, 2, 3)),
        ("MXOR $1,$2,5", MMixInstruction::MXORI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_shift_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("SL $1,$2,$3", MMixInstruction::SL(1, 2, 3)),
        ("SL $1,$2,5", MMixInstruction::SLI(1, 2, 5)),
        ("SLU $1,$2,$3", MMixInstruction::SLU(1, 2, 3)),
        ("SLU $1,$2,5", MMixInstruction::SLUI(1, 2, 5)),
        ("SR $1,$2,$3", MMixInstruction::SR(1, 2, 3)),
        ("SR $1,$2,5", MMixInstruction::SRI(1, 2, 5)),
        ("SRU $1,$2,$3", MMixInstruction::SRU(1, 2, 3)),
        ("SRU $1,$2,5", MMixInstruction::SRUI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_conditional_set_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("CSN $1,$2,$3", MMixInstruction::CSN(1, 2, 3)),
        ("CSN $1,$2,5", MMixInstruction::CSNI(1, 2, 5)),
        ("CSZ $1,$2,$3", MMixInstruction::CSZ(1, 2, 3)),
        ("CSZ $1,$2,5", MMixInstruction::CSZI(1, 2, 5)),
        ("CSP $1,$2,$3", MMixInstruction::CSP(1, 2, 3)),
        ("CSP $1,$2,5", MMixInstruction::CSPI(1, 2, 5)),
        ("CSOD $1,$2,$3", MMixInstruction::CSOD(1, 2, 3)),
        ("CSOD $1,$2,5", MMixInstruction::CSODI(1, 2, 5)),
        ("CSNN $1,$2,$3", MMixInstruction::CSNN(1, 2, 3)),
        ("CSNN $1,$2,5", MMixInstruction::CSNNI(1, 2, 5)),
        ("CSNZ $1,$2,$3", MMixInstruction::CSNZ(1, 2, 3)),
        ("CSNZ $1,$2,5", MMixInstruction::CSNZI(1, 2, 5)),
        ("CSNP $1,$2,$3", MMixInstruction::CSNP(1, 2, 3)),
        ("CSNP $1,$2,5", MMixInstruction::CSNPI(1, 2, 5)),
        ("CSEV $1,$2,$3", MMixInstruction::CSEV(1, 2, 3)),
        ("CSEV $1,$2,5", MMixInstruction::CSEVI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_zero_or_set_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("ZSN $1,$2,$3", MMixInstruction::ZSN(1, 2, 3)),
        ("ZSN $1,$2,5", MMixInstruction::ZSNI(1, 2, 5)),
        ("ZSZ $1,$2,$3", MMixInstruction::ZSZ(1, 2, 3)),
        ("ZSZ $1,$2,5", MMixInstruction::ZSZI(1, 2, 5)),
        ("ZSP $1,$2,$3", MMixInstruction::ZSP(1, 2, 3)),
        ("ZSP $1,$2,5", MMixInstruction::ZSPI(1, 2, 5)),
        ("ZSOD $1,$2,$3", MMixInstruction::ZSOD(1, 2, 3)),
        ("ZSOD $1,$2,5", MMixInstruction::ZSODI(1, 2, 5)),
        ("ZSNN $1,$2,$3", MMixInstruction::ZSNN(1, 2, 3)),
        ("ZSNN $1,$2,5", MMixInstruction::ZSNNI(1, 2, 5)),
        ("ZSNZ $1,$2,$3", MMixInstruction::ZSNZ(1, 2, 3)),
        ("ZSNZ $1,$2,5", MMixInstruction::ZSNZI(1, 2, 5)),
        ("ZSNP $1,$2,$3", MMixInstruction::ZSNP(1, 2, 3)),
        ("ZSNP $1,$2,5", MMixInstruction::ZSNPI(1, 2, 5)),
        ("ZSEV $1,$2,$3", MMixInstruction::ZSEV(1, 2, 3)),
        ("ZSEV $1,$2,5", MMixInstruction::ZSEVI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

// ---- Canonical spellings for the remaining families -------------
// MMIXAL picks the immediate opcode from the operand, so a base
// mnemonic with an immediate Z emits what its *I spelling emits. The
// *I spellings stay accepted as a legacy surface.

fn first_instruction(src: &str) -> MMixInstruction {
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
    asm.instructions
        .first()
        .unwrap_or_else(|| panic!("no instructions produced for {src:?}"))
        .1
        .clone()
}

#[test]
fn test_auto_extended_load_store_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("LDUNC $1,$2,$3", MMixInstruction::LDUNC(1, 2, 3)),
        ("LDUNC $1,$2,5", MMixInstruction::LDUNCI(1, 2, 5)),
        ("STUNC $1,$2,$3", MMixInstruction::STUNC(1, 2, 3)),
        ("STUNC $1,$2,5", MMixInstruction::STUNCI(1, 2, 5)),
        ("LDHT $1,$2,$3", MMixInstruction::LDHT(1, 2, 3)),
        ("LDHT $1,$2,5", MMixInstruction::LDHTI(1, 2, 5)),
        ("STHT $1,$2,$3", MMixInstruction::STHT(1, 2, 3)),
        ("STHT $1,$2,5", MMixInstruction::STHTI(1, 2, 5)),
        ("LDSF $1,$2,$3", MMixInstruction::LDSF(1, 2, 3)),
        ("LDSF $1,$2,5", MMixInstruction::LDSFI(1, 2, 5)),
        ("STSF $1,$2,$3", MMixInstruction::STSF(1, 2, 3)),
        ("STSF $1,$2,5", MMixInstruction::STSFI(1, 2, 5)),
        ("LDVTS $1,$2,$3", MMixInstruction::LDVTS(1, 2, 3)),
        ("LDVTS $1,$2,5", MMixInstruction::LDVTSI(1, 2, 5)),
        ("CSWAP $1,$2,$3", MMixInstruction::CSWAP(1, 2, 3)),
        ("CSWAP $1,$2,5", MMixInstruction::CSWAPI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_cache_and_go_full_coverage() {
    let cases: &[(&str, MMixInstruction)] = &[
        ("PRELD $1,$2,$3", MMixInstruction::PRELD(1, 2, 3)),
        ("PRELD $1,$2,5", MMixInstruction::PRELDI(1, 2, 5)),
        ("PREGO $1,$2,$3", MMixInstruction::PREGO(1, 2, 3)),
        ("PREGO $1,$2,5", MMixInstruction::PREGOI(1, 2, 5)),
        ("PREST $1,$2,$3", MMixInstruction::PREST(1, 2, 3)),
        ("PREST $1,$2,5", MMixInstruction::PRESTI(1, 2, 5)),
        ("SYNCD $1,$2,$3", MMixInstruction::SYNCD(1, 2, 3)),
        ("SYNCD $1,$2,5", MMixInstruction::SYNCDI(1, 2, 5)),
        ("SYNCID $1,$2,$3", MMixInstruction::SYNCID(1, 2, 3)),
        ("SYNCID $1,$2,5", MMixInstruction::SYNCIDI(1, 2, 5)),
        ("GO $1,$2,$3", MMixInstruction::GO(1, 2, 3)),
        ("GO $1,$2,5", MMixInstruction::GOI(1, 2, 5)),
        ("PUSHGO $1,$2,$3", MMixInstruction::PUSHGO(1, 2, 3)),
        ("PUSHGO $1,$2,5", MMixInstruction::PUSHGOI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_float_conversion_full_coverage() {
    // Y is a rounding-mode value, not a register; 0 here exercises
    // Z's register/immediate auto-select, this test's point.
    let cases: &[(&str, MMixInstruction)] = &[
        ("FLOT $1,0,$3", MMixInstruction::FLOT(1, 0, 3)),
        ("FLOT $1,0,5", MMixInstruction::FLOTI(1, 0, 5)),
        ("FLOTU $1,0,$3", MMixInstruction::FLOTU(1, 0, 3)),
        ("FLOTU $1,0,5", MMixInstruction::FLOTUI(1, 0, 5)),
        ("SFLOT $1,0,$3", MMixInstruction::SFLOT(1, 0, 3)),
        ("SFLOT $1,0,5", MMixInstruction::SFLOTI(1, 0, 5)),
        ("SFLOTU $1,0,$3", MMixInstruction::SFLOTU(1, 0, 3)),
        ("SFLOTU $1,0,5", MMixInstruction::SFLOTUI(1, 0, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_auto_irregular_operand_shapes_full_coverage() {
    // STCO's X and NEG's Y stay immediate bytes; only Z auto-selects.
    let cases: &[(&str, MMixInstruction)] = &[
        ("STCO 5,$2,$3", MMixInstruction::STCO(5, 2, 3)),
        ("STCO 5,$2,7", MMixInstruction::STCOI(5, 2, 7)),
        ("NEG $1,0,$3", MMixInstruction::NEG(1, 0, 3)),
        ("NEG $1,0,7", MMixInstruction::NEGI(1, 0, 7)),
        ("NEGU $1,0,$3", MMixInstruction::NEGU(1, 0, 3)),
        ("NEGU $1,0,7", MMixInstruction::NEGUI(1, 0, 7)),
        ("PUT rA,$1", MMixInstruction::PUT(21, 1)),
        ("PUT rA,7", MMixInstruction::PUTI(21, 7)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_base_spelling_agrees_with_legacy_immediate_spelling() {
    let pairs: &[(&str, &str)] = &[
        ("LDHT $1,$2,5", "LDHTI $1,$2,5"),
        ("LDSF $1,$2,5", "LDSFI $1,$2,5"),
        ("LDUNC $1,$2,5", "LDUNCI $1,$2,5"),
        ("LDVTS $1,$2,5", "LDVTSI $1,$2,5"),
        ("STHT $1,$2,5", "STHTI $1,$2,5"),
        ("STSF $1,$2,5", "STSFI $1,$2,5"),
        ("STUNC $1,$2,5", "STUNCI $1,$2,5"),
        ("CSWAP $1,$2,5", "CSWAPI $1,$2,5"),
        ("PREGO $1,$2,5", "PREGOI $1,$2,5"),
        ("PRELD $1,$2,5", "PRELDI $1,$2,5"),
        ("PREST $1,$2,5", "PRESTI $1,$2,5"),
        ("SYNCD $1,$2,5", "SYNCDI $1,$2,5"),
        ("SYNCID $1,$2,5", "SYNCIDI $1,$2,5"),
        ("GO $1,$2,5", "GOI $1,$2,5"),
        ("PUSHGO $1,$2,5", "PUSHGOI $1,$2,5"),
        ("FLOT $1,0,5", "FLOTI $1,0,5"),
        ("FLOTU $1,0,5", "FLOTUI $1,0,5"),
        ("SFLOT $1,0,5", "SFLOTI $1,0,5"),
        ("SFLOTU $1,0,5", "SFLOTUI $1,0,5"),
        ("STCO 5,$2,7", "STCOI 5,$2,7"),
        ("NEG $1,0,7", "NEGI $1,0,7"),
        ("NEGU $1,0,7", "NEGUI $1,0,7"),
        ("PUT rA,7", "PUTI rA,7"),
    ];
    for (base, legacy) in pairs {
        assert_eq!(
            first_instruction(base),
            first_instruction(legacy),
            "{base:?} must emit what {legacy:?} emits"
        );
    }
}

#[test]
fn test_no_previously_accepted_operand_form_narrowed() {
    // Every spelling these families accepted before their base
    // mnemonics auto-selected. Widening must displace none of them.
    let forms: &[&str] = &[
        "LDHT $1,$2,$3",
        "LDHTI $1,$2,5",
        "LDSF $1,$2,$3",
        "LDSFI $1,$2,5",
        "LDUNC $1,$2,$3",
        "LDUNCI $1,$2,5",
        "LDVTS $1,$2,$3",
        "LDVTSI $1,$2,5",
        "STHT $1,$2,$3",
        "STHTI $1,$2,5",
        "STSF $1,$2,$3",
        "STSFI $1,$2,5",
        "STUNC $1,$2,$3",
        "STUNCI $1,$2,5",
        "CSWAP $1,$2,$3",
        "CSWAPI $1,$2,5",
        "PREGO $1,$2,$3",
        "PREGOI $1,$2,5",
        "PRELD $1,$2,$3",
        "PRELDI $1,$2,5",
        "PREST $1,$2,$3",
        "PRESTI $1,$2,5",
        "SYNCD $1,$2,$3",
        "SYNCDI $1,$2,5",
        "SYNCID $1,$2,$3",
        "SYNCIDI $1,$2,5",
        "GO $1,$2,$3",
        "GOI $1,$2,5",
        "PUSHGO $1,$2,$3",
        "PUSHGOI $1,$2,5",
        "FLOT $1,0,$3",
        "FLOTI $1,0,5",
        "FLOTU $1,0,$3",
        "FLOTUI $1,0,5",
        "SFLOT $1,0,$3",
        "SFLOTI $1,0,5",
        "SFLOTU $1,0,$3",
        "SFLOTUI $1,0,5",
        "STCO 5,$2,$3",
        "STCOI 5,$2,7",
        "NEG $1,0,$3",
        "NEGI $1,0,7",
        "NEGU $1,0,$3",
        "NEGUI $1,0,7",
        "PUT rA,$1",
        "PUTI rA,7",
        "LDA $1,$2,3",
        "LDAI $1,$2,3",
    ];
    for src in forms {
        let mut asm = MMixAssembler::new(src, "<test>");
        asm.parse()
            .unwrap_or_else(|e| panic!("{src:?} no longer assembles: {e}"));
    }
}

#[test]
fn test_flot_rejects_register_in_y_slot() {
    // Y is a rounding-mode value, never a register: every operand is an
    // `expr` now, so `$2` parses fine there, and the evaluator is what
    // rejects it -- a register where FLOT's Y demands a pure value.
    let mut asm = MMixAssembler::new("FLOT $1,$2,$3", "<test>");
    let err = asm
        .parse()
        .expect_err("a register in FLOT's Y slot must still be rejected");
    assert_eq!(
        err,
        "<test>:1:9: register $2 cannot be used where a pure value is required"
    );
}

#[test]
fn test_round_mode_symbols_resolve_to_documented_values() {
    // The predefined-symbol table (MMIXAL reference), independent of
    // rA's own persistent-mode numbering.
    let asm = MMixAssembler::new("", "<test>");
    for (name, value) in [
        ("ROUND_CURRENT", 0u64),
        ("ROUND_OFF", 1),
        ("ROUND_UP", 2),
        ("ROUND_DOWN", 3),
    ] {
        assert_eq!(
            asm.symbols.get(name).copied(),
            Some(SymbolType::Constant(value)),
            "{name} must resolve to {value}"
        );
    }
}

/// The reference's eleven TRAP codes, checksmix's three extensions, the
/// five `Fopen` modes, and the three standard handles, all at the
/// values the ABI table names. Reverting `TrapCode`'s numbering turns
/// one of these red.
#[test]
fn test_predefined_trap_symbols_match_the_reference_abi() {
    let asm = MMixAssembler::new("", "<test>");
    for (name, value) in [
        ("Halt", 0u64),
        ("Fopen", 1),
        ("Fclose", 2),
        ("Fread", 3),
        ("Fgets", 4),
        ("Fgetws", 5),
        ("Fwrite", 6),
        ("Fputs", 7),
        ("Fputws", 8),
        ("Fseek", 9),
        ("Ftell", 10),
        ("Fputc", 0x80),
        ("Time", 0x81),
        ("Debug", 0x82),
        ("TextRead", 0),
        ("TextWrite", 1),
        ("BinaryRead", 2),
        ("BinaryWrite", 3),
        ("BinaryReadWrite", 4),
        ("StdIn", 0),
        ("StdOut", 1),
        ("StdErr", 2),
    ] {
        assert_eq!(
            asm.symbols.get(name).copied(),
            Some(SymbolType::Constant(value)),
            "{name} must resolve to {value}"
        );
    }
    assert!(
        !asm.symbols.contains_key("Trip"),
        "Trip is not a symbol: the TRIP instruction does user trips now"
    );
}

#[test]
fn test_lda_selects_addus_two_opcodes() {
    assert_first_instruction("LDA $1,$2,$3", MMixInstruction::LDA(1, 2, 3));
    assert_first_instruction("LDA $1,$2,3", MMixInstruction::LDAI(1, 2, 3));
    assert_first_instruction("LDAI $1,$2,3", MMixInstruction::LDAI(1, 2, 3));
}

#[test]
fn test_lda_emits_the_same_bytes_as_addu() {
    // LDA carries no opcode of its own: it is ADDU under another name,
    // in both the register and the immediate operand form.
    let asm = MMixAssembler::new("", "<test>");
    for (lda, addu) in [
        ("LDA $1,$2,$3", "ADDU $1,$2,$3"),
        ("LDA $1,$2,3", "ADDU $1,$2,3"),
    ] {
        assert_eq!(
            asm.encode_instruction_bytes(&first_instruction(lda)),
            asm.encode_instruction_bytes(&first_instruction(addu)),
            "{lda:?} must emit the same bytes as {addu:?}"
        );
    }
}

#[test]
fn test_stco_accepts_a_register_x_operand() {
    // The reference warns on a register X but assembles its number;
    // refusing it here would narrow an accepted form.
    assert_first_instruction("STCO $1,$2,$3", MMixInstruction::STCO(1, 2, 3));
}

#[test]
fn test_jmpb_backward_target_encodes_knuth_field() {
    // JMPB sits at 0x104 (BACK's HALT is 4 bytes), target 0x100:
    // magnitude = 1 tetra, field = 2^24 - 1.
    let source = "LOC #100\nBACK: HALT\nJMPB BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[1].1, MMixInstruction::JMPB(0xFFFFFF));
}

#[test]
fn test_jmpb_forward_target_errors() {
    let source = "JMPB LABEL\nLOC #100\nLABEL: HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm.parse().expect_err("JMPB encodes only backward targets");
    assert!(
        err.contains("use JMP instead"),
        "error should name JMP as the forward form, got: {err}"
    );
}

// ---- Mnemonic prefix-collision tests ----------------------------
// These pin down PEG backtracking for prefix-overlapping mnemonics
// (ADD vs ADDU vs ADDUI; AND vs ANDN vs ANDNI; CSN vs CSNN; etc.).

#[test]
fn test_prefix_robust_arith_signed_vs_unsigned() {
    // ADD must not steal "ADDU"; ADDU must not steal "ADDUI" (the
    // *I path must still match through inst_arith_rri).
    assert_first_instruction("ADD $1,$2,$3", MMixInstruction::ADD(1, 2, 3));
    assert_first_instruction("ADDU $1,$2,$3", MMixInstruction::ADDU(1, 2, 3));
    assert_first_instruction("ADDI $1,$2,7", MMixInstruction::ADDI(1, 2, 7));
    assert_first_instruction("ADDUI $1,$2,7", MMixInstruction::ADDUI(1, 2, 7));
    assert_first_instruction("ADD $1,$2,7", MMixInstruction::ADDI(1, 2, 7));
    assert_first_instruction("ADDU $1,$2,7", MMixInstruction::ADDUI(1, 2, 7));
}

#[test]
fn test_prefix_robust_numeric_arith() {
    // 2ADDU through 16ADDU each must claim their own mnemonic and
    // their *I variant must still route via the rri path.
    assert_first_instruction("2ADDU $1,$2,$3", MMixInstruction::ADDU2(1, 2, 3));
    assert_first_instruction("4ADDU $1,$2,$3", MMixInstruction::ADDU4(1, 2, 3));
    assert_first_instruction("8ADDU $1,$2,$3", MMixInstruction::ADDU8(1, 2, 3));
    assert_first_instruction("16ADDU $1,$2,$3", MMixInstruction::ADDU16(1, 2, 3));
    assert_first_instruction("2ADDU $1,$2,5", MMixInstruction::ADDU2I(1, 2, 5));
    assert_first_instruction("4ADDU $1,$2,5", MMixInstruction::ADDU4I(1, 2, 5));
    assert_first_instruction("8ADDU $1,$2,5", MMixInstruction::ADDU8I(1, 2, 5));
    assert_first_instruction("16ADDU $1,$2,5", MMixInstruction::ADDU16I(1, 2, 5));
    assert_first_instruction("2ADDUI $1,$2,5", MMixInstruction::ADDU2I(1, 2, 5));
    assert_first_instruction("4ADDUI $1,$2,5", MMixInstruction::ADDU4I(1, 2, 5));
    assert_first_instruction("8ADDUI $1,$2,5", MMixInstruction::ADDU8I(1, 2, 5));
    assert_first_instruction("16ADDUI $1,$2,5", MMixInstruction::ADDU16I(1, 2, 5));
}

#[test]
fn test_prefix_robust_bitwise_and_andn() {
    // AND vs ANDN vs ANDI vs ANDNI: each must route to its own
    // variant. Particularly important because mnemonic_and matches
    // the "AND" prefix of all four; backtracking has to recover.
    assert_first_instruction("AND $1,$2,$3", MMixInstruction::AND(1, 2, 3));
    assert_first_instruction("ANDN $1,$2,$3", MMixInstruction::ANDN(1, 2, 3));
    assert_first_instruction("AND $1,$2,7", MMixInstruction::ANDI(1, 2, 7));
    assert_first_instruction("ANDN $1,$2,7", MMixInstruction::ANDNI(1, 2, 7));
    assert_first_instruction("ANDI $1,$2,7", MMixInstruction::ANDI(1, 2, 7));
    assert_first_instruction("ANDNI $1,$2,7", MMixInstruction::ANDNI(1, 2, 7));
    // ANDN also collides with the wyde-field ANDNH/ANDNMH/ANDNML/ANDNL
    // family (2-operand reg,imm shape, distinct from the 3-operand
    // AND/ANDN/ANDNI forms above).
    assert_first_instruction("ANDNH $1,5", MMixInstruction::ANDNH(1, 5));
    assert_first_instruction("ANDNMH $1,5", MMixInstruction::ANDNMH(1, 5));
    assert_first_instruction("ANDNML $1,5", MMixInstruction::ANDNML(1, 5));
    assert_first_instruction("ANDNL $1,5", MMixInstruction::ANDNL(1, 5));
}

#[test]
fn test_prefix_robust_bitwise_or_orn() {
    assert_first_instruction("OR $1,$2,$3", MMixInstruction::OR(1, 2, 3));
    assert_first_instruction("ORN $1,$2,$3", MMixInstruction::ORN(1, 2, 3));
    assert_first_instruction("OR $1,$2,7", MMixInstruction::ORI(1, 2, 7));
    assert_first_instruction("ORN $1,$2,7", MMixInstruction::ORNI(1, 2, 7));
    assert_first_instruction("ORI $1,$2,7", MMixInstruction::ORI(1, 2, 7));
    assert_first_instruction("ORNI $1,$2,7", MMixInstruction::ORNI(1, 2, 7));
    // OR also collides with the wyde-field ORH/ORMH/ORML/ORL family
    // (2-operand reg,imm shape, distinct from the 3-operand OR/ORN/ORNI
    // forms above).
    assert_first_instruction("ORH $1,5", MMixInstruction::ORH(1, 5));
    assert_first_instruction("ORMH $1,5", MMixInstruction::ORMH(1, 5));
    assert_first_instruction("ORML $1,5", MMixInstruction::ORML(1, 5));
    assert_first_instruction("ORL $1,5", MMixInstruction::ORL(1, 5));
}

#[test]
fn test_prefix_robust_set_family() {
    // SET must not steal SETI's longer literal, and SETI/SETL/SETH/
    // SETMH/SETML — five mnemonics sharing the "SET" prefix — must each
    // route to their own variant.
    assert_first_instruction("SET $1,$2", MMixInstruction::SETRR(1, 2));
    assert_first_instruction("SETI $1,5", MMixInstruction::SET(1, 5));
    assert_first_instruction("SETL $1,5", MMixInstruction::SETL(1, 5));
    assert_first_instruction("SETH $1,5", MMixInstruction::SETH(1, 5));
    assert_first_instruction("SETMH $1,5", MMixInstruction::SETMH(1, 5));
    assert_first_instruction("SETML $1,5", MMixInstruction::SETML(1, 5));
}

#[test]
fn test_prefix_robust_neg() {
    // NEG/NEGU/NEGI/NEGUI: NEG must not steal NEGU's, NEGI's, or
    // NEGUI's longer literal.
    assert_first_instruction("NEG $1,5,$3", MMixInstruction::NEG(1, 5, 3));
    assert_first_instruction("NEGU $1,5,$3", MMixInstruction::NEGU(1, 5, 3));
    assert_first_instruction("NEGI $1,5,7", MMixInstruction::NEGI(1, 5, 7));
    assert_first_instruction("NEGUI $1,5,7", MMixInstruction::NEGUI(1, 5, 7));
}

#[test]
fn test_prefix_robust_float_fix_flot() {
    // FIX/FIXU and FLOT/FLOTI/FLOTU/FLOTUI and SFLOT/SFLOTI/SFLOTU/
    // SFLOTUI: FIX must not steal FIXU's literal, FLOT must not steal
    // FLOTU's/FLOTI's/FLOTUI's, and likewise for SFLOT. Y is a
    // rounding-mode value, not a register; 0 here is orthogonal to
    // what this test exercises.
    assert_first_instruction("FIX $1,0,$3", MMixInstruction::FIX(1, 0, 3));
    assert_first_instruction("FIXU $1,0,$3", MMixInstruction::FIXU(1, 0, 3));
    assert_first_instruction("FLOT $1,0,$3", MMixInstruction::FLOT(1, 0, 3));
    assert_first_instruction("FLOTU $1,0,$3", MMixInstruction::FLOTU(1, 0, 3));
    assert_first_instruction("FLOTI $1,0,5", MMixInstruction::FLOTI(1, 0, 5));
    assert_first_instruction("FLOTUI $1,0,5", MMixInstruction::FLOTUI(1, 0, 5));
    assert_first_instruction("SFLOT $1,0,$3", MMixInstruction::SFLOT(1, 0, 3));
    assert_first_instruction("SFLOTU $1,0,$3", MMixInstruction::SFLOTU(1, 0, 3));
    assert_first_instruction("SFLOTI $1,0,5", MMixInstruction::SFLOTI(1, 0, 5));
    assert_first_instruction("SFLOTUI $1,0,5", MMixInstruction::SFLOTUI(1, 0, 5));
}

#[test]
fn test_prefix_robust_load_store_families() {
    // LDB/LDW/LDT/LDO and STB/STW/STT/STO each have a U sibling (LDBU,
    // ...) and an I sibling (LDBI, ...) and a UI sibling (LDBUI, ...);
    // the base mnemonic must not steal any of them.
    let cases: &[(&str, MMixInstruction)] = &[
        ("LDB $1,$2,$3", MMixInstruction::LDB(1, 2, 3)),
        ("LDBU $1,$2,$3", MMixInstruction::LDBU(1, 2, 3)),
        ("LDBI $1,$2,5", MMixInstruction::LDBI(1, 2, 5)),
        ("LDBUI $1,$2,5", MMixInstruction::LDBUI(1, 2, 5)),
        ("LDW $1,$2,$3", MMixInstruction::LDW(1, 2, 3)),
        ("LDWU $1,$2,$3", MMixInstruction::LDWU(1, 2, 3)),
        ("LDWI $1,$2,5", MMixInstruction::LDWI(1, 2, 5)),
        ("LDWUI $1,$2,5", MMixInstruction::LDWUI(1, 2, 5)),
        ("LDT $1,$2,$3", MMixInstruction::LDT(1, 2, 3)),
        ("LDTU $1,$2,$3", MMixInstruction::LDTU(1, 2, 3)),
        ("LDTI $1,$2,5", MMixInstruction::LDTI(1, 2, 5)),
        ("LDTUI $1,$2,5", MMixInstruction::LDTUI(1, 2, 5)),
        ("LDO $1,$2,$3", MMixInstruction::LDO(1, 2, 3)),
        ("LDOU $1,$2,$3", MMixInstruction::LDOU(1, 2, 3)),
        ("LDOI $1,$2,5", MMixInstruction::LDOI(1, 2, 5)),
        ("LDOUI $1,$2,5", MMixInstruction::LDOUI(1, 2, 5)),
        ("STB $1,$2,$3", MMixInstruction::STB(1, 2, 3)),
        ("STBU $1,$2,$3", MMixInstruction::STBU(1, 2, 3)),
        ("STBI $1,$2,5", MMixInstruction::STBI(1, 2, 5)),
        ("STBUI $1,$2,5", MMixInstruction::STBUI(1, 2, 5)),
        ("STW $1,$2,$3", MMixInstruction::STW(1, 2, 3)),
        ("STWU $1,$2,$3", MMixInstruction::STWU(1, 2, 3)),
        ("STWI $1,$2,5", MMixInstruction::STWI(1, 2, 5)),
        ("STWUI $1,$2,5", MMixInstruction::STWUI(1, 2, 5)),
        ("STT $1,$2,$3", MMixInstruction::STT(1, 2, 3)),
        ("STTU $1,$2,$3", MMixInstruction::STTU(1, 2, 3)),
        ("STTI $1,$2,5", MMixInstruction::STTI(1, 2, 5)),
        ("STTUI $1,$2,5", MMixInstruction::STTUI(1, 2, 5)),
        ("STO $1,$2,$3", MMixInstruction::STO(1, 2, 3)),
        ("STOU $1,$2,$3", MMixInstruction::STOU(1, 2, 3)),
        ("STOI $1,$2,5", MMixInstruction::STOI(1, 2, 5)),
        ("STOUI $1,$2,5", MMixInstruction::STOUI(1, 2, 5)),
    ];
    for (src, expected) in cases {
        assert_first_instruction(src, expected.clone());
    }
}

#[test]
fn test_prefix_robust_lda() {
    // LDA must not steal LDAI's longer literal. Both spellings select
    // the immediate opcode here; only LDA's register form selects 0x22.
    assert_first_instruction("LDA $1,$2,$3", MMixInstruction::LDA(1, 2, 3));
    assert_first_instruction("LDA $1,$2,5", MMixInstruction::LDAI(1, 2, 5));
    assert_first_instruction("LDAI $1,$2,5", MMixInstruction::LDAI(1, 2, 5));
}

#[test]
fn test_prefix_robust_get_geta_getab() {
    // GET must not swallow the "GET" prefix of GETA/GETAB, and GETA
    // must not swallow GETAB's. GETA/GETAB carry a computed PC-relative
    // offset, so only the variant discriminant (and the un-computed X
    // register) is checked here, per the offset-bearing-family note.
    assert_first_instruction("GET $1,0", MMixInstruction::GET(1, 0));
    assert_first_instruction_matches("GETA $0,4", |i| matches!(i, MMixInstruction::GETA(0, _, _)));
    let source = "LOC #100\nBACK: HALT\nGETAB $0,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
    assert!(matches!(
        asm.instructions[1].1,
        MMixInstruction::GETAB(0, _, _)
    ));
}

#[test]
fn test_prefix_robust_go_pushgo() {
    // GO/GOI and PUSHGO/PUSHGOI: the base mnemonic must not steal its
    // *I sibling's literal.
    assert_first_instruction("GO $1,$2,$3", MMixInstruction::GO(1, 2, 3));
    assert_first_instruction("GOI $1,$2,5", MMixInstruction::GOI(1, 2, 5));
    assert_first_instruction("PUSHGO $1,$2,$3", MMixInstruction::PUSHGO(1, 2, 3));
    assert_first_instruction("PUSHGOI $1,$2,5", MMixInstruction::PUSHGOI(1, 2, 5));
}

#[test]
fn test_prefix_robust_pushj() {
    // PUSHJ must not steal PUSHJB's longer literal. Both carry a
    // computed offset, so only the discriminant and X register are
    // checked (offset-bearing-family note).
    assert_first_instruction_matches("PUSHJ $1,4", |i| {
        matches!(i, MMixInstruction::PUSHJ(1, _, _))
    });
    let source = "LOC #100\nBACK: HALT\nPUSHJB $1,BACK";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
    assert!(matches!(
        asm.instructions[1].1,
        MMixInstruction::PUSHJB(1, _, _)
    ));
}

#[test]
fn test_prefix_robust_put() {
    // PUT must not steal PUTI's longer literal.
    assert_first_instruction("PUT 5,$1", MMixInstruction::PUT(5, 1));
    assert_first_instruction("PUTI 5,7", MMixInstruction::PUTI(5, 7));
}

#[test]
fn test_prefix_robust_prefetch_and_sync() {
    // PREGO/PRELD/PREST/SYNCD/SYNCID each have an *I sibling, and SYNC
    // itself is a literal prefix of SYNCD/SYNCDI/SYNCID/SYNCIDI.
    assert_first_instruction("PREGO $1,$2,$3", MMixInstruction::PREGO(1, 2, 3));
    assert_first_instruction("PREGOI $1,$2,5", MMixInstruction::PREGOI(1, 2, 5));
    assert_first_instruction("PRELD $1,$2,$3", MMixInstruction::PRELD(1, 2, 3));
    assert_first_instruction("PRELDI $1,$2,5", MMixInstruction::PRELDI(1, 2, 5));
    assert_first_instruction("PREST $1,$2,$3", MMixInstruction::PREST(1, 2, 3));
    assert_first_instruction("PRESTI $1,$2,5", MMixInstruction::PRESTI(1, 2, 5));
    assert_first_instruction("SYNCD $1,$2,$3", MMixInstruction::SYNCD(1, 2, 3));
    assert_first_instruction("SYNCDI $1,$2,5", MMixInstruction::SYNCDI(1, 2, 5));
    assert_first_instruction("SYNCID $1,$2,$3", MMixInstruction::SYNCID(1, 2, 3));
    assert_first_instruction("SYNCIDI $1,$2,5", MMixInstruction::SYNCIDI(1, 2, 5));
    assert_first_instruction("SYNC 5", MMixInstruction::SYNC(5));
}

#[test]
fn test_prefix_robust_extra_load_store() {
    // LDUNC/STUNC/LDHT/STHT/LDSF/STSF/LDVTS/CSWAP/STCO each have an *I
    // sibling; the base mnemonic must not steal it.
    assert_first_instruction("LDUNC $1,$2,$3", MMixInstruction::LDUNC(1, 2, 3));
    assert_first_instruction("LDUNCI $1,$2,5", MMixInstruction::LDUNCI(1, 2, 5));
    assert_first_instruction("STUNC $1,$2,$3", MMixInstruction::STUNC(1, 2, 3));
    assert_first_instruction("STUNCI $1,$2,5", MMixInstruction::STUNCI(1, 2, 5));
    assert_first_instruction("LDHT $1,$2,$3", MMixInstruction::LDHT(1, 2, 3));
    assert_first_instruction("LDHTI $1,$2,5", MMixInstruction::LDHTI(1, 2, 5));
    assert_first_instruction("STHT $1,$2,$3", MMixInstruction::STHT(1, 2, 3));
    assert_first_instruction("STHTI $1,$2,5", MMixInstruction::STHTI(1, 2, 5));
    assert_first_instruction("LDSF $1,$2,$3", MMixInstruction::LDSF(1, 2, 3));
    assert_first_instruction("LDSFI $1,$2,5", MMixInstruction::LDSFI(1, 2, 5));
    assert_first_instruction("STSF $1,$2,$3", MMixInstruction::STSF(1, 2, 3));
    assert_first_instruction("STSFI $1,$2,5", MMixInstruction::STSFI(1, 2, 5));
    assert_first_instruction("LDVTS $1,$2,$3", MMixInstruction::LDVTS(1, 2, 3));
    assert_first_instruction("LDVTSI $1,$2,5", MMixInstruction::LDVTSI(1, 2, 5));
    assert_first_instruction("CSWAP $1,$2,$3", MMixInstruction::CSWAP(1, 2, 3));
    assert_first_instruction("CSWAPI $1,$2,5", MMixInstruction::CSWAPI(1, 2, 5));

    assert_first_instruction("STCO 5,$1,$2", MMixInstruction::STCO(5, 1, 2));
    assert_first_instruction("STCOI 5,$1,7", MMixInstruction::STCOI(5, 1, 7));
}

/// Predicate over a parsed instruction's variant, paired with source in
/// the `branch`/`pbranch` family tests below (offset-bearing, so the
/// exact value isn't what those tests are proving).
type InstructionPredicate = fn(&MMixInstruction) -> bool;

#[test]
fn test_prefix_robust_branch_family() {
    // BN/BNB/BNN/BNNB/BNP/BNPB/BNZ/BNZB/BEV/BEVB/BOD/BODB/BP/BPB/BZ/BZB:
    // every short mnemonic in this family is a literal prefix of at
    // least one longer sibling. Offsets are computed, so only the
    // discriminant and X register are checked. A *B mnemonic needs a
    // target behind it, so those cases branch to a preceding label.
    let cases: &[(&str, InstructionPredicate)] = &[
        ("BN $1,4", |i| matches!(i, MMixInstruction::BN(1, _))),
        ("BACK: HALT\nBNB $1,BACK", |i| {
            matches!(i, MMixInstruction::BNB(1, _))
        }),
        ("BNN $1,4", |i| matches!(i, MMixInstruction::BNN(1, _))),
        ("BACK: HALT\nBNNB $1,BACK", |i| {
            matches!(i, MMixInstruction::BNNB(1, _))
        }),
        ("BNP $1,4", |i| matches!(i, MMixInstruction::BNP(1, _))),
        ("BACK: HALT\nBNPB $1,BACK", |i| {
            matches!(i, MMixInstruction::BNPB(1, _))
        }),
        ("BNZ $1,4", |i| matches!(i, MMixInstruction::BNZ(1, _))),
        ("BACK: HALT\nBNZB $1,BACK", |i| {
            matches!(i, MMixInstruction::BNZB(1, _))
        }),
        ("BEV $1,4", |i| matches!(i, MMixInstruction::BEV(1, _))),
        ("BACK: HALT\nBEVB $1,BACK", |i| {
            matches!(i, MMixInstruction::BEVB(1, _))
        }),
        ("BOD $1,4", |i| matches!(i, MMixInstruction::BOD(1, _))),
        ("BACK: HALT\nBODB $1,BACK", |i| {
            matches!(i, MMixInstruction::BODB(1, _))
        }),
        ("BP $1,4", |i| matches!(i, MMixInstruction::BP(1, _))),
        ("BACK: HALT\nBPB $1,BACK", |i| {
            matches!(i, MMixInstruction::BPB(1, _))
        }),
        ("BZ $1,4", |i| matches!(i, MMixInstruction::BZ(1, _))),
        ("BACK: HALT\nBZB $1,BACK", |i| {
            matches!(i, MMixInstruction::BZB(1, _))
        }),
    ];
    for (src, pred) in cases {
        assert_last_instruction_matches(src, pred);
    }
}

#[test]
fn test_prefix_robust_pbranch_family() {
    // Same collision shape as the branch family above, one level up
    // (PBN/PBNB/PBNN/...), with the same backward-target requirement.
    let cases: &[(&str, InstructionPredicate)] = &[
        ("PBN $1,4", |i| matches!(i, MMixInstruction::PBN(1, _, _))),
        ("BACK: HALT\nPBNB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBNB(1, _, _))
        }),
        ("PBNN $1,4", |i| matches!(i, MMixInstruction::PBNN(1, _, _))),
        ("BACK: HALT\nPBNNB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBNNB(1, _, _))
        }),
        ("PBNP $1,4", |i| matches!(i, MMixInstruction::PBNP(1, _, _))),
        ("BACK: HALT\nPBNPB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBNPB(1, _, _))
        }),
        ("PBNZ $1,4", |i| matches!(i, MMixInstruction::PBNZ(1, _, _))),
        ("BACK: HALT\nPBNZB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBNZB(1, _, _))
        }),
        ("PBEV $1,4", |i| matches!(i, MMixInstruction::PBEV(1, _, _))),
        ("BACK: HALT\nPBEVB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBEVB(1, _, _))
        }),
        ("PBOD $1,4", |i| matches!(i, MMixInstruction::PBOD(1, _, _))),
        ("BACK: HALT\nPBODB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBODB(1, _, _))
        }),
        ("PBP $1,4", |i| matches!(i, MMixInstruction::PBP(1, _, _))),
        ("BACK: HALT\nPBPB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBPB(1, _, _))
        }),
        ("PBZ $1,4", |i| matches!(i, MMixInstruction::PBZ(1, _, _))),
        ("BACK: HALT\nPBZB $1,BACK", |i| {
            matches!(i, MMixInstruction::PBZB(1, _, _))
        }),
    ];
    for (src, pred) in cases {
        assert_last_instruction_matches(src, pred);
    }
}

#[test]
fn test_prefix_robust_shift_signed_vs_unsigned() {
    assert_first_instruction("SL $1,$2,$3", MMixInstruction::SL(1, 2, 3));
    assert_first_instruction("SLU $1,$2,$3", MMixInstruction::SLU(1, 2, 3));
    assert_first_instruction("SR $1,$2,$3", MMixInstruction::SR(1, 2, 3));
    assert_first_instruction("SRU $1,$2,$3", MMixInstruction::SRU(1, 2, 3));
    assert_first_instruction("SL $1,$2,7", MMixInstruction::SLI(1, 2, 7));
    assert_first_instruction("SLU $1,$2,7", MMixInstruction::SLUI(1, 2, 7));
    assert_first_instruction("SR $1,$2,7", MMixInstruction::SRI(1, 2, 7));
    assert_first_instruction("SRU $1,$2,7", MMixInstruction::SRUI(1, 2, 7));
    assert_first_instruction("SLI $1,$2,7", MMixInstruction::SLI(1, 2, 7));
    assert_first_instruction("SRUI $1,$2,7", MMixInstruction::SRUI(1, 2, 7));
}

#[test]
fn test_prefix_robust_conditional_csn_csnn() {
    // CSN must not steal CSNN/CSNZ/CSNP. And the *I siblings must
    // route via the rri path (CSNI vs CSNNI etc.).
    assert_first_instruction("CSN $1,$2,$3", MMixInstruction::CSN(1, 2, 3));
    assert_first_instruction("CSNN $1,$2,$3", MMixInstruction::CSNN(1, 2, 3));
    assert_first_instruction("CSNZ $1,$2,$3", MMixInstruction::CSNZ(1, 2, 3));
    assert_first_instruction("CSNP $1,$2,$3", MMixInstruction::CSNP(1, 2, 3));
    assert_first_instruction("CSN $1,$2,7", MMixInstruction::CSNI(1, 2, 7));
    assert_first_instruction("CSNN $1,$2,7", MMixInstruction::CSNNI(1, 2, 7));
    assert_first_instruction("CSNZ $1,$2,7", MMixInstruction::CSNZI(1, 2, 7));
    assert_first_instruction("CSNP $1,$2,7", MMixInstruction::CSNPI(1, 2, 7));
    assert_first_instruction("CSNI $1,$2,7", MMixInstruction::CSNI(1, 2, 7));
    assert_first_instruction("CSNNI $1,$2,7", MMixInstruction::CSNNI(1, 2, 7));
}

#[test]
fn test_prefix_robust_zero_or_set_zsn_zsnn() {
    assert_first_instruction("ZSN $1,$2,$3", MMixInstruction::ZSN(1, 2, 3));
    assert_first_instruction("ZSNN $1,$2,$3", MMixInstruction::ZSNN(1, 2, 3));
    assert_first_instruction("ZSNZ $1,$2,$3", MMixInstruction::ZSNZ(1, 2, 3));
    assert_first_instruction("ZSNP $1,$2,$3", MMixInstruction::ZSNP(1, 2, 3));
    assert_first_instruction("ZSN $1,$2,7", MMixInstruction::ZSNI(1, 2, 7));
    assert_first_instruction("ZSNN $1,$2,7", MMixInstruction::ZSNNI(1, 2, 7));
    assert_first_instruction("ZSNZ $1,$2,7", MMixInstruction::ZSNZI(1, 2, 7));
    assert_first_instruction("ZSNP $1,$2,7", MMixInstruction::ZSNPI(1, 2, 7));
}

// ---- Boundary value tests ---------------------------------------

#[test]
fn test_immediate_boundary_zero() {
    assert_first_instruction("ADD $1,$2,0", MMixInstruction::ADDI(1, 2, 0));
    assert_first_instruction("AND $1,$2,0", MMixInstruction::ANDI(1, 2, 0));
    assert_first_instruction("SR $1,$2,0", MMixInstruction::SRI(1, 2, 0));
}

#[test]
fn test_immediate_boundary_max_decimal_255() {
    assert_first_instruction("ADD $1,$2,255", MMixInstruction::ADDI(1, 2, 255));
    assert_first_instruction("OR $1,$2,255", MMixInstruction::ORI(1, 2, 255));
    assert_first_instruction("ZSP $1,$2,255", MMixInstruction::ZSPI(1, 2, 255));
}

#[test]
fn test_immediate_boundary_max_hex_ff() {
    assert_first_instruction("ADD $1,$2,#FF", MMixInstruction::ADDI(1, 2, 0xFF));
    assert_first_instruction("XOR $1,$2,#FF", MMixInstruction::XORI(1, 2, 0xFF));
    assert_first_instruction("CSN $1,$2,#FF", MMixInstruction::CSNI(1, 2, 0xFF));
}

#[test]
fn test_immediate_boundary_overflow_decimal_256() {
    assert!(assemble_err("ADD $1,$2,256").contains("out of range 0..255"));
    assert!(assemble_err("AND $1,$2,256").contains("out of range 0..255"));
    assert!(assemble_err("SRU $1,$2,256").contains("out of range 0..255"));
}

#[test]
fn test_immediate_boundary_overflow_hex_100() {
    assert!(assemble_err("ADD $1,$2,#100").contains("out of range 0..255"));
}

#[test]
fn test_immediate_boundary_overflow_large_value() {
    // A genuinely large value must not silently truncate; it must
    // be rejected by the auto-immediate range check.
    assert!(assemble_err("ADD $1,$2,#10000").contains("out of range 0..255"));
    assert!(assemble_err("ADD $1,$2,1000000").contains("out of range 0..255"));
}

#[test]
fn test_immediate_boundary_negative_rejected_in_auto() {
    // The auto path is strict 0..=255. Negative literals (which
    // wrap to large u64s) must be rejected. The explicit *I path
    // keeps its silent-wrap behavior — see
    // `test_parse_negative_literal_8bit_wrap`.
    assert!(assemble_err("ADD $1,$2,-1").contains("out of range 0..255"));
    assert!(assemble_err("AND $1,$2,-128").contains("out of range 0..255"));
}

#[test]
fn test_immediate_register_max_255() {
    // $255 as Z must remain a register reference, not get
    // confused with the immediate 255.
    assert_first_instruction("ADD $1,$2,$255", MMixInstruction::ADD(1, 2, 255));
    assert_first_instruction("AND $1,$2,$255", MMixInstruction::AND(1, 2, 255));
}

#[test]
fn test_immediate_char_literal() {
    assert_first_instruction("ADD $1,$2,'A'", MMixInstruction::ADDI(1, 2, 65));
}

// ---- Symbol/label resolution at the Z slot ----------------------

#[test]
fn test_symbol_z_constant_in_range() {
    assert_first_instruction("K IS 0\nADD $1,$2,K", MMixInstruction::ADDI(1, 2, 0));
    assert_first_instruction("K IS 255\nAND $1,$2,K", MMixInstruction::ANDI(1, 2, 255));
    assert_first_instruction("K IS 42\nCSZ $1,$2,K", MMixInstruction::CSZI(1, 2, 42));
}

#[test]
fn test_symbol_z_constant_out_of_range() {
    assert!(assemble_err("K IS 256\nADD $1,$2,K").contains("out of range 0..255"));
    assert!(assemble_err("K IS 1000\nAND $1,$2,K").contains("out of range 0..255"));
}

#[test]
// NEG's immediate spellings carry Z as an 8-bit field: an operand above
// 255 is an error rather than a silent truncation, whether it is written
// as a literal or resolved from a symbol.
fn test_neg_immediate_spelling_range_checks_its_z() {
    assert!(assemble_err("NEGI $1,0,#300").contains("out of range 0..255"));
    assert!(assemble_err("NEGUI $1,0,#300").contains("out of range 0..255"));
    assert!(assemble_err("BigC IS #300\nNEGI $1,0,BigC").contains("out of range 0..255"));
    assert!(assemble_err("BigC IS #300\nNEGUI $1,0,BigC").contains("out of range 0..255"));
    assert!(assemble_err("NEGI $1,0,-1").contains("out of range 0..255"));
    assert!(assemble_err("NEGUI $1,0,-1").contains("out of range 0..255"));
    assert_first_instruction("NEGI $1,0,5", MMixInstruction::NEGI(1, 0, 5));
    assert_first_instruction("NEGUI $1,0,5", MMixInstruction::NEGUI(1, 0, 5));
    assert_first_instruction(
        "SmallC IS 5\nNEGI $1,0,SmallC",
        MMixInstruction::NEGI(1, 0, 5),
    );
}

#[test]
fn test_symbol_z_register_alias_zero() {
    assert_first_instruction("Z IS $0\nADD $1,$2,Z", MMixInstruction::ADD(1, 2, 0));
}

#[test]
fn test_symbol_z_register_alias_max() {
    assert_first_instruction("M IS $255\nAND $1,$2,M", MMixInstruction::AND(1, 2, 255));
}

#[test]
fn test_symbol_z_label_address_out_of_range() {
    // A label whose address is above 255 must error rather than
    // silently truncate, even though it grammatically parses as a
    // bare identifier in the Z slot.
    let src = "\
LOC #200
Foo  OCTA 0
Main ADD $1,$2,Foo
";
    assert!(assemble_err(src).contains("out of range 0..255"));
}

#[test]
fn test_symbol_z_undefined_errors() {
    assert!(assemble_err("ADD $1,$2,Nope").contains("Undefined symbol"));
    assert!(assemble_err("AND $1,$2,Nope").contains("Undefined symbol"));
}

// ---- Cross-family non-interference ------------------------------
// Multiple base mnemonics from different families in one source —
// each must route to its own family's auto rule.

#[test]
fn test_cross_family_routing_in_one_program() {
    let src = "\
ADD  $1,$2,5
AND  $3,$4,7
SR   $5,$6,3
BDIF $7,$8,9
CSZ  $1,$2,1
ZSP  $3,$4,2
";
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse().unwrap();
    assert_eq!(
        asm.instructions
            .iter()
            .map(|(_, i)| i.clone())
            .collect::<Vec<_>>(),
        vec![
            MMixInstruction::ADDI(1, 2, 5),
            MMixInstruction::ANDI(3, 4, 7),
            MMixInstruction::SRI(5, 6, 3),
            MMixInstruction::BDIFI(7, 8, 9),
            MMixInstruction::CSZI(1, 2, 1),
            MMixInstruction::ZSPI(3, 4, 2),
        ]
    );
}

// ---- Existing-test perturbation ---------------------------------
// Pick a handful of pre-existing register-form assertions and add
// matching auto-immediate assertions so that any future grammar
// change that breaks the auto path will be caught alongside the
// original tests.

#[test]
fn test_perturbation_and_xor_or() {
    // Mirrors test_parse_and / test_parse_xor / test_parse_or but
    // uses the auto-immediate path.
    assert_first_instruction("AND $1,$2,#FF", MMixInstruction::ANDI(1, 2, 0xFF));
    assert_first_instruction("XOR $5,$6,#0F", MMixInstruction::XORI(5, 6, 0x0F));
    assert_first_instruction("OR $10,$20,#80", MMixInstruction::ORI(10, 20, 0x80));
}

#[test]
fn test_perturbation_bitfiddle_family() {
    // Mirrors test_parse_bdif/wdif/tdif/odif/sadd/mor/mxor.
    assert_first_instruction("BDIF $1,$2,#10", MMixInstruction::BDIFI(1, 2, 0x10));
    assert_first_instruction("WDIF $1,$2,100", MMixInstruction::WDIFI(1, 2, 100));
    assert_first_instruction("TDIF $1,$2,50", MMixInstruction::TDIFI(1, 2, 50));
    assert_first_instruction("ODIF $1,$2,255", MMixInstruction::ODIFI(1, 2, 255));
    assert_first_instruction("SADD $1,$2,0", MMixInstruction::SADDI(1, 2, 0));
    assert_first_instruction("MOR $1,$2,128", MMixInstruction::MORI(1, 2, 128));
    assert_first_instruction("MXOR $1,$2,64", MMixInstruction::MXORI(1, 2, 64));
}

#[test]
fn test_perturbation_shift_family() {
    // Mirrors test_parse_sl / sli / slu / slui / sr / sri / sru / srui.
    assert_first_instruction("SL $3,$1,8", MMixInstruction::SLI(3, 1, 8));
    assert_first_instruction("SLU $1,$2,16", MMixInstruction::SLUI(1, 2, 16));
    assert_first_instruction("SR $1,$2,4", MMixInstruction::SRI(1, 2, 4));
    assert_first_instruction("SRU $1,$2,32", MMixInstruction::SRUI(1, 2, 32));
}

// ---- Comprehensive byte-identical regression --------------------
// Every base mnemonic in the six in-scope families paired with its
// explicit *I sibling. This is the strongest cross-validation that
// the auto path produces exactly the same encoded bytes as the
// legacy path.

#[test]
fn test_byte_identical_regression_all_families() {
    let pairs: &[(&str, &str)] = &[
        // Arithmetic
        ("ADD $1,$2,5", "ADDI $1,$2,5"),
        ("ADDU $1,$2,5", "ADDUI $1,$2,5"),
        ("2ADDU $1,$2,5", "2ADDUI $1,$2,5"),
        ("4ADDU $1,$2,5", "4ADDUI $1,$2,5"),
        ("8ADDU $1,$2,5", "8ADDUI $1,$2,5"),
        ("16ADDU $1,$2,5", "16ADDUI $1,$2,5"),
        ("SUB $1,$2,5", "SUBI $1,$2,5"),
        ("SUBU $1,$2,5", "SUBUI $1,$2,5"),
        ("MUL $1,$2,5", "MULI $1,$2,5"),
        ("MULU $1,$2,5", "MULUI $1,$2,5"),
        ("DIV $1,$2,5", "DIVI $1,$2,5"),
        ("DIVU $1,$2,5", "DIVUI $1,$2,5"),
        ("CMP $1,$2,5", "CMPI $1,$2,5"),
        ("CMPU $1,$2,5", "CMPUI $1,$2,5"),
        // Bitwise
        ("AND $1,$2,5", "ANDI $1,$2,5"),
        ("OR $1,$2,5", "ORI $1,$2,5"),
        ("XOR $1,$2,5", "XORI $1,$2,5"),
        ("ANDN $1,$2,5", "ANDNI $1,$2,5"),
        ("ORN $1,$2,5", "ORNI $1,$2,5"),
        ("NAND $1,$2,5", "NANDI $1,$2,5"),
        ("NOR $1,$2,5", "NORI $1,$2,5"),
        ("NXOR $1,$2,5", "NXORI $1,$2,5"),
        ("MUX $1,$2,5", "MUXI $1,$2,5"),
        // Bit-fiddle
        ("BDIF $1,$2,5", "BDIFI $1,$2,5"),
        ("WDIF $1,$2,5", "WDIFI $1,$2,5"),
        ("TDIF $1,$2,5", "TDIFI $1,$2,5"),
        ("ODIF $1,$2,5", "ODIFI $1,$2,5"),
        ("SADD $1,$2,5", "SADDI $1,$2,5"),
        ("MOR $1,$2,5", "MORI $1,$2,5"),
        ("MXOR $1,$2,5", "MXORI $1,$2,5"),
        // Shift
        ("SL $1,$2,5", "SLI $1,$2,5"),
        ("SLU $1,$2,5", "SLUI $1,$2,5"),
        ("SR $1,$2,5", "SRI $1,$2,5"),
        ("SRU $1,$2,5", "SRUI $1,$2,5"),
        // Conditional set
        ("CSN $1,$2,5", "CSNI $1,$2,5"),
        ("CSZ $1,$2,5", "CSZI $1,$2,5"),
        ("CSP $1,$2,5", "CSPI $1,$2,5"),
        ("CSOD $1,$2,5", "CSODI $1,$2,5"),
        ("CSNN $1,$2,5", "CSNNI $1,$2,5"),
        ("CSNZ $1,$2,5", "CSNZI $1,$2,5"),
        ("CSNP $1,$2,5", "CSNPI $1,$2,5"),
        ("CSEV $1,$2,5", "CSEVI $1,$2,5"),
        // Zero or set
        ("ZSN $1,$2,5", "ZSNI $1,$2,5"),
        ("ZSZ $1,$2,5", "ZSZI $1,$2,5"),
        ("ZSP $1,$2,5", "ZSPI $1,$2,5"),
        ("ZSOD $1,$2,5", "ZSODI $1,$2,5"),
        ("ZSNN $1,$2,5", "ZSNNI $1,$2,5"),
        ("ZSNZ $1,$2,5", "ZSNZI $1,$2,5"),
        ("ZSNP $1,$2,5", "ZSNPI $1,$2,5"),
        ("ZSEV $1,$2,5", "ZSEVI $1,$2,5"),
    ];

    for (auto_src, explicit_src) in pairs {
        let mut auto_asm = MMixAssembler::new(auto_src, "<auto>");
        auto_asm
            .parse()
            .unwrap_or_else(|e| panic!("auto src {auto_src:?} failed: {e}"));
        let mut explicit_asm = MMixAssembler::new(explicit_src, "<explicit>");
        explicit_asm
            .parse()
            .unwrap_or_else(|e| panic!("explicit src {explicit_src:?} failed: {e}"));
        let auto_bytes = auto_asm.encode_instruction_bytes(&auto_asm.instructions[0].1);
        let explicit_bytes = explicit_asm.encode_instruction_bytes(&explicit_asm.instructions[0].1);
        assert_eq!(
            auto_bytes, explicit_bytes,
            "byte mismatch: auto {auto_src:?} -> {:?} {:?} vs explicit {explicit_src:?} -> {:?} {:?}",
            auto_asm.instructions[0].1, auto_bytes, explicit_asm.instructions[0].1, explicit_bytes
        );
    }
}

/// The explicit `*I` spellings reject a negative `Z` exactly as the
/// auto (`ADD`) path does.
#[test]
fn test_explicit_i_negative_is_an_error() {
    assert_eq!(
        assemble_err("ADDI $1,$2,-1"),
        "<test>:1:12: immediate operand -1 out of range 0..255 for ADDI"
    );
    assert_eq!(
        assemble_err("ANDI $1,$2,-1"),
        "<test>:1:12: immediate operand -1 out of range 0..255 for ANDI"
    );
    assert_eq!(
        assemble_err("SLUI $1,$2,-1"),
        "<test>:1:12: immediate operand -1 out of range 0..255 for SLUI"
    );
    assert_eq!(
        assemble_err("ADD $1,$2,-1"),
        "<test>:1:11: immediate operand -1 out of range 0..255 for ADD"
    );
}
