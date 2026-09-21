//! General/special register access and the PUT family guard rails.

use super::*;

#[test]
fn test_general_registers() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 0x123456789ABCDEF0);
    assert_eq!(mmix.get_register(1), 0x123456789ABCDEF0);
    assert_eq!(mmix.get_register(2), 0);
}

#[test]
fn test_special_registers() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RR, 42);
    assert_eq!(mmix.get_special(SpecialReg::RR), 42);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_put_get() {
    let mut mmix = MMix::new();
    // PUT rR, $1 - Put value from $1 into rR (special register 6)
    mmix.set_register(1, 0x123456789ABCDEF0);
    mmix.write_tetra(0, 0xF6060001); // PUT X=6 (rR), Y=0, Z=1 ($1)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RR), 0x123456789ABCDEF0);

    // GET $2, rR - Get value from rR into $2
    mmix.write_tetra(4, 0xFE020006); // GET X=2 ($2), Y=0, Z=6 (rR)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(2), 0x123456789ABCDEF0);
}

#[test]
fn test_puti_stores_z_alone() {
    let mut mmix = MMix::new();
    // PUTI rH, Y=0, Z=0x34 - only Z reaches rH.
    mmix.write_tetra(0, 0xF7030034); // PUTI X=3 (rH), Y=0, Z=0x34
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RH), 0x34);
}

#[test]
fn test_put_x_at_32_names_no_special_register() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(1, 42);
    mmix.write_tetra(0, 0xF6200001); // PUT X=32,$1 -- no register above 31
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(mmix.get_exit_code(), 1);
}

/// VAL-1: `PUT`'s Y must be zero.
#[test]
fn test_put_y_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(1, 1);

    // PUT rA,1,$1 -- Y=1 must be zero.
    mmix.write_tetra(0, 0xF6150101);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(
        mmix.get_special(SpecialReg::RA),
        0,
        "the write did not land"
    );
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "PUT Y=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000000"
    );
}

/// VAL-1: `PUTI`'s Y must be zero.
#[test]
fn test_puti_y_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);

    // PUTI rA,1,1 -- Y=1 must be zero.
    mmix.write_tetra(0, 0xF7150101);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(
        mmix.get_special(SpecialReg::RA),
        0,
        "the write did not land"
    );
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "PUTI Y=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000000"
    );
}

#[test]
fn test_put_rc_is_rejected_with_a_privileged_operation_interrupt() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(1, 222);
    mmix.write_tetra(0, 0xF6080001); // PUT X=8 (rC), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RC),
        0,
        "the write did not land"
    );
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rC"));
    assert!(handle.diagnostics()[0].contains("privileged"));
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_rn_is_rejected_with_an_illegal_instruction_interrupt() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RN, 111);
    mmix.set_register(1, 222);
    mmix.write_tetra(0, 0xF6090001); // PUT X=9 (rN), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RN),
        111,
        "the write did not land"
    );
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rN"));
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_ro_is_rejected_with_an_illegal_instruction_interrupt() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    let ro = mmix.get_special(SpecialReg::RO);
    mmix.set_register(1, 222);
    mmix.write_tetra(0, 0xF60A0001); // PUT X=10 (rO), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RO),
        ro,
        "the write did not land"
    );
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rO"));
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_rs_is_rejected_with_an_illegal_instruction_interrupt() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    let rs = mmix.get_special(SpecialReg::RS);
    mmix.set_register(1, 222);
    mmix.write_tetra(0, 0xF60B0001); // PUT X=11 (rS), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RS),
        rs,
        "the write did not land"
    );
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rS"));
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_rg_below_32_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(1, 31);
    mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RG), 32, "rG unchanged");
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rG"));
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_rg_below_rl_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RL, 40);
    mmix.set_register(1, 35); // >= 32, but < rL
    mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RG), 32, "rG unchanged");
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_rg_above_255_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(1, 256);
    mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RG), 32, "rG unchanged");
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_rg_accepts_the_low_boundary_32() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RG, 60);
    mmix.set_register(1, 32);
    mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RG), 32);
}

#[test]
fn test_put_rg_accepts_the_high_boundary_255() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 255);
    mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RG), 255);
}

#[test]
fn test_put_rg_accepts_a_value_equal_to_rl() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RL, 40);
    mmix.set_register(1, 40);
    mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RG), 40);
}

#[test]
fn test_put_rg_raising_zeroes_the_newly_local_span() {
    let mut mmix = MMix::new();
    mmix.set_register(40, 0xDEAD); // global while rG = 32
    mmix.set_register(70, 777); // outside the raised span; stays global

    mmix.write_tetra(0, 0xF713003C); // PUTI rG,60
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RG), 60);
    assert_eq!(
        mmix.get_register(40),
        0,
        "reclassified into the local/marginal range reads zero"
    );
    assert_eq!(
        mmix.get_register(70),
        777,
        "outside the span keeps its value"
    );
}

#[test]
fn test_put_rg_lowering_zeroes_the_newly_global_span() {
    let mut mmix = MMix::new();
    mmix.set_register(40, 0xDEAD); // global while rG = 32
    mmix.set_special(SpecialReg::RG, 60); // raw raise: $40 goes stale-marginal
    mmix.set_register(70, 777); // outside the lowered span; stays global

    mmix.write_tetra(0, 0xF7130020); // PUTI rG,32
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RG), 32);
    assert_eq!(
        mmix.get_register(40),
        0,
        "reclassified back into the global range reads zero"
    );
    assert_eq!(
        mmix.get_register(70),
        777,
        "outside the span keeps its value"
    );
}

#[test]
fn test_put_rg_zeroes_a_reclassified_register_end_to_end() {
    // SET $40,#DEAD / PUT rG,60 / PUT rG,32 / ADD $5,$40,$0 -- C6's
    // adversarial review (2026-09-17) found this left $5 = #DEAD;
    // MMIXware gives 0.
    let mut mmix = MMix::new();
    mmix.set_register(40, 0xDEAD); // global while rG = 32

    mmix.write_tetra(0, 0xF713003C); // PUTI rG,60
    assert!(mmix.execute_instruction());
    mmix.write_tetra(4, 0xF7130020); // PUTI rG,32
    assert!(mmix.execute_instruction());
    mmix.write_tetra(8, 0x20052800); // ADD $5,$40,$0
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_register(5), 0);
}

#[test]
fn test_set_register_zeros_a_stale_marginal_gap() {
    // Raising rG is a raw write with no rL rules of its own, so it can
    // leave a stale value in a register that becomes marginal. Writing
    // a higher register must still zero it out when the write claims
    // the range.
    let mut mmix = MMix::new();
    mmix.set_register(40, 0xDEAD); // global while rG = 32
    mmix.set_special(SpecialReg::RG, 50); // $40 is now marginal, still 0xDEAD

    mmix.set_register(45, 123);

    assert_eq!(mmix.get_register(40), 0, "the gap zeros out");
    assert_eq!(mmix.get_register(45), 123);
    assert_eq!(mmix.get_special(SpecialReg::RL), 46);
}

#[test]
fn test_put_rl_with_a_larger_z_leaves_rl_unchanged() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RL, 5);

    // PUTI rL,10 - z > rL, so rL stays at min(10, 5) = 5.
    mmix.write_tetra(0, 0xF714000A);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RL), 5);
}

#[test]
fn test_put_rl_zeros_the_registers_it_drops() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RL, 6);
    mmix.set_register(4, 777);

    // PUTI rL,3 - drops $3..$5 out of the local range.
    mmix.write_tetra(0, 0xF7140003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RL), 3);

    // ADD $1,$4,$0 reads the dropped $4 with no intervening write: zero.
    // $1 is already local (1 < 3), so its own destination rise is a
    // no-op and does not confound this read.
    mmix.write_tetra(4, 0x20010400);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_put_rl_survives_an_rl_beyond_the_register_file() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RL, u64::MAX);
    mmix.set_register(200, 42); // global at rG = 32

    // PUTI rL,3
    mmix.write_tetra(0, 0xF7140003);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    assert_eq!(mmix.get_register(200), 42, "a global is not a local");
}

/// `UNSAVE` can no longer produce an rG/rL pair this far out of range —
/// its own validation now refuses a packed rG above 255 or a local
/// count above the packed rG — so this state is planted directly
/// through `set_special`, which stays raw by design.
#[test]
fn test_put_rl_after_an_out_of_range_state_naming_more_registers_than_the_file_holds() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RG, 1000); // beyond the register file
    mmix.set_special(SpecialReg::RL, 500);

    // PUTI rL,3
    mmix.write_tetra(0, 0xF7140003);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
}

/// UNSAVE restores rG from guest memory, so rG can name a register the
/// file does not have. `claim_local` must compare against it at full
/// width: a byte-narrowed comparison turns a real local, sitting below
/// the true rG but above its truncated low byte, into a global that is
/// never claimed. `PUT rG` carries no validation yet, so this state is
/// reachable today; C7's validation may close it later.
#[test]
fn test_register_claims_local_when_rg_names_no_real_register() {
    let mut mmix = MMix::new();
    mmix.set_register(150, 0xDEAD); // global while rG = 32 (the default)
    mmix.set_special(SpecialReg::RG, 300); // beyond the register file
    mmix.set_special(SpecialReg::RL, 3);

    // $150 is now marginal (3 <= 150 < 300); claiming $200 sweeps it.
    mmix.set_register(200, 777);

    assert_eq!(mmix.get_special(SpecialReg::RL), 201);
    assert_eq!(
        mmix.get_register(150),
        0,
        "the claim zeroed the marginal range"
    );
    assert_eq!(mmix.get_register(200), 777);
}

/// `writes_general_register_x` decides, for every opcode byte, whether
/// the instruction's X field becomes a live local before the
/// instruction runs. This table is derived independently from
/// `MMIX.md` and from `Opcode`, never from the predicate itself, so a
/// single misclassified byte fails as itself rather than surviving in
/// an aggregate comparison.
#[test]
fn test_destination_predicate_matches_an_independent_table_at_every_byte() {
    use crate::mmixal::Opcode;

    // (byte, mnemonic, is a general-register destination before the
    // instruction runs). PRELD, PREGO, PREST, SYNCD and SYNCID parse X
    // as a register in checksmix but take an immediate byte count in
    // the specification, so all ten forms are false. PUSHJ and PUSHGO
    // write nothing to $X directly, but $X is the call hole and must be
    // local first, so both are true. PUT's X names a special register,
    // POP's X is a count, UNSAVE's X is fixed at 0, and TRAP/TRIP/SYNC/
    // SWYM/RESUME/JMP/JMPB/HALT carry no general-register operand in X.
    let table: [(u8, Opcode, bool); 256] = [
        (0x00, Opcode::TRAP, false),
        (0x01, Opcode::FCMP, true),
        (0x02, Opcode::FUN, true),
        (0x03, Opcode::FEQL, true),
        (0x04, Opcode::FADD, true),
        (0x05, Opcode::FIX, true),
        (0x06, Opcode::FSUB, true),
        (0x07, Opcode::FIXU, true),
        (0x08, Opcode::FLOT, true),
        (0x09, Opcode::FLOTI, true),
        (0x0A, Opcode::FLOTU, true),
        (0x0B, Opcode::FLOTUI, true),
        (0x0C, Opcode::SFLOT, true),
        (0x0D, Opcode::SFLOTI, true),
        (0x0E, Opcode::SFLOTU, true),
        (0x0F, Opcode::SFLOTUI, true),
        (0x10, Opcode::FMUL, true),
        (0x11, Opcode::FCMPE, true),
        (0x12, Opcode::FUNE, true),
        (0x13, Opcode::FEQLE, true),
        (0x14, Opcode::FDIV, true),
        (0x15, Opcode::FSQRT, true),
        (0x16, Opcode::FREM, true),
        (0x17, Opcode::FINT, true),
        (0x18, Opcode::MUL, true),
        (0x19, Opcode::MULI, true),
        (0x1A, Opcode::MULU, true),
        (0x1B, Opcode::MULUI, true),
        (0x1C, Opcode::DIV, true),
        (0x1D, Opcode::DIVI, true),
        (0x1E, Opcode::DIVU, true),
        (0x1F, Opcode::DIVUI, true),
        (0x20, Opcode::ADD, true),
        (0x21, Opcode::ADDI, true),
        (0x22, Opcode::ADDU, true),
        (0x23, Opcode::ADDUI, true),
        (0x24, Opcode::SUB, true),
        (0x25, Opcode::SUBI, true),
        (0x26, Opcode::SUBU, true),
        (0x27, Opcode::SUBUI, true),
        (0x28, Opcode::ADDU2, true),
        (0x29, Opcode::ADDU2I, true),
        (0x2A, Opcode::ADDU4, true),
        (0x2B, Opcode::ADDU4I, true),
        (0x2C, Opcode::ADDU8, true),
        (0x2D, Opcode::ADDU8I, true),
        (0x2E, Opcode::ADDU16, true),
        (0x2F, Opcode::ADDU16I, true),
        (0x30, Opcode::CMP, true),
        (0x31, Opcode::CMPI, true),
        (0x32, Opcode::CMPU, true),
        (0x33, Opcode::CMPUI, true),
        (0x34, Opcode::NEG, true),
        (0x35, Opcode::NEGI, true),
        (0x36, Opcode::NEGU, true),
        (0x37, Opcode::NEGUI, true),
        (0x38, Opcode::SL, true),
        (0x39, Opcode::SLI, true),
        (0x3A, Opcode::SLU, true),
        (0x3B, Opcode::SLUI, true),
        (0x3C, Opcode::SR, true),
        (0x3D, Opcode::SRI, true),
        (0x3E, Opcode::SRU, true),
        (0x3F, Opcode::SRUI, true),
        (0x40, Opcode::BN, false),
        (0x41, Opcode::BNB, false),
        (0x42, Opcode::BZ, false),
        (0x43, Opcode::BZB, false),
        (0x44, Opcode::BP, false),
        (0x45, Opcode::BPB, false),
        (0x46, Opcode::BOD, false),
        (0x47, Opcode::BODB, false),
        (0x48, Opcode::BNN, false),
        (0x49, Opcode::BNNB, false),
        (0x4A, Opcode::BNZ, false),
        (0x4B, Opcode::BNZB, false),
        (0x4C, Opcode::BNP, false),
        (0x4D, Opcode::BNPB, false),
        (0x4E, Opcode::BEV, false),
        (0x4F, Opcode::BEVB, false),
        (0x50, Opcode::PBN, false),
        (0x51, Opcode::PBNB, false),
        (0x52, Opcode::PBZ, false),
        (0x53, Opcode::PBZB, false),
        (0x54, Opcode::PBP, false),
        (0x55, Opcode::PBPB, false),
        (0x56, Opcode::PBOD, false),
        (0x57, Opcode::PBODB, false),
        (0x58, Opcode::PBNN, false),
        (0x59, Opcode::PBNNB, false),
        (0x5A, Opcode::PBNZ, false),
        (0x5B, Opcode::PBNZB, false),
        (0x5C, Opcode::PBNP, false),
        (0x5D, Opcode::PBNPB, false),
        (0x5E, Opcode::PBEV, false),
        (0x5F, Opcode::PBEVB, false),
        (0x60, Opcode::CSN, true),
        (0x61, Opcode::CSNI, true),
        (0x62, Opcode::CSZ, true),
        (0x63, Opcode::CSZI, true),
        (0x64, Opcode::CSP, true),
        (0x65, Opcode::CSPI, true),
        (0x66, Opcode::CSOD, true),
        (0x67, Opcode::CSODI, true),
        (0x68, Opcode::CSNN, true),
        (0x69, Opcode::CSNNI, true),
        (0x6A, Opcode::CSNZ, true),
        (0x6B, Opcode::CSNZI, true),
        (0x6C, Opcode::CSNP, true),
        (0x6D, Opcode::CSNPI, true),
        (0x6E, Opcode::CSEV, true),
        (0x6F, Opcode::CSEVI, true),
        (0x70, Opcode::ZSN, true),
        (0x71, Opcode::ZSNI, true),
        (0x72, Opcode::ZSZ, true),
        (0x73, Opcode::ZSZI, true),
        (0x74, Opcode::ZSP, true),
        (0x75, Opcode::ZSPI, true),
        (0x76, Opcode::ZSOD, true),
        (0x77, Opcode::ZSODI, true),
        (0x78, Opcode::ZSNN, true),
        (0x79, Opcode::ZSNNI, true),
        (0x7A, Opcode::ZSNZ, true),
        (0x7B, Opcode::ZSNZI, true),
        (0x7C, Opcode::ZSNP, true),
        (0x7D, Opcode::ZSNPI, true),
        (0x7E, Opcode::ZSEV, true),
        (0x7F, Opcode::ZSEVI, true),
        (0x80, Opcode::LDB, true),
        (0x81, Opcode::LDBI, true),
        (0x82, Opcode::LDBU, true),
        (0x83, Opcode::LDBUI, true),
        (0x84, Opcode::LDW, true),
        (0x85, Opcode::LDWI, true),
        (0x86, Opcode::LDWU, true),
        (0x87, Opcode::LDWUI, true),
        (0x88, Opcode::LDT, true),
        (0x89, Opcode::LDTI, true),
        (0x8A, Opcode::LDTU, true),
        (0x8B, Opcode::LDTUI, true),
        (0x8C, Opcode::LDO, true),
        (0x8D, Opcode::LDOI, true),
        (0x8E, Opcode::LDOU, true),
        (0x8F, Opcode::LDOUI, true),
        (0x90, Opcode::LDSF, true),
        (0x91, Opcode::LDSFI, true),
        (0x92, Opcode::LDHT, true),
        (0x93, Opcode::LDHTI, true),
        (0x94, Opcode::CSWAP, true),
        (0x95, Opcode::CSWAPI, true),
        (0x96, Opcode::LDUNC, true),
        (0x97, Opcode::LDUNCI, true),
        (0x98, Opcode::LDVTS, true),
        (0x99, Opcode::LDVTSI, true),
        (0x9A, Opcode::PRELD, false),
        (0x9B, Opcode::PRELDI, false),
        (0x9C, Opcode::PREGO, false),
        (0x9D, Opcode::PREGOI, false),
        (0x9E, Opcode::GO, true),
        (0x9F, Opcode::GOI, true),
        (0xA0, Opcode::STB, false),
        (0xA1, Opcode::STBI, false),
        (0xA2, Opcode::STBU, false),
        (0xA3, Opcode::STBUI, false),
        (0xA4, Opcode::STW, false),
        (0xA5, Opcode::STWI, false),
        (0xA6, Opcode::STWU, false),
        (0xA7, Opcode::STWUI, false),
        (0xA8, Opcode::STT, false),
        (0xA9, Opcode::STTI, false),
        (0xAA, Opcode::STTU, false),
        (0xAB, Opcode::STTUI, false),
        (0xAC, Opcode::STO, false),
        (0xAD, Opcode::STOI, false),
        (0xAE, Opcode::STOU, false),
        (0xAF, Opcode::STOUI, false),
        (0xB0, Opcode::STSF, false),
        (0xB1, Opcode::STSFI, false),
        (0xB2, Opcode::STHT, false),
        (0xB3, Opcode::STHTI, false),
        (0xB4, Opcode::STCO, false),
        (0xB5, Opcode::STCOI, false),
        (0xB6, Opcode::STUNC, false),
        (0xB7, Opcode::STUNCI, false),
        (0xB8, Opcode::SYNCD, false),
        (0xB9, Opcode::SYNCDI, false),
        (0xBA, Opcode::PREST, false),
        (0xBB, Opcode::PRESTI, false),
        (0xBC, Opcode::SYNCID, false),
        (0xBD, Opcode::SYNCIDI, false),
        (0xBE, Opcode::PUSHGO, true),
        (0xBF, Opcode::PUSHGOI, true),
        (0xC0, Opcode::OR, true),
        (0xC1, Opcode::ORI, true),
        (0xC2, Opcode::ORN, true),
        (0xC3, Opcode::ORNI, true),
        (0xC4, Opcode::NOR, true),
        (0xC5, Opcode::NORI, true),
        (0xC6, Opcode::XOR, true),
        (0xC7, Opcode::XORI, true),
        (0xC8, Opcode::AND, true),
        (0xC9, Opcode::ANDI, true),
        (0xCA, Opcode::ANDN, true),
        (0xCB, Opcode::ANDNI, true),
        (0xCC, Opcode::NAND, true),
        (0xCD, Opcode::NANDI, true),
        (0xCE, Opcode::NXOR, true),
        (0xCF, Opcode::NXORI, true),
        (0xD0, Opcode::BDIF, true),
        (0xD1, Opcode::BDIFI, true),
        (0xD2, Opcode::WDIF, true),
        (0xD3, Opcode::WDIFI, true),
        (0xD4, Opcode::TDIF, true),
        (0xD5, Opcode::TDIFI, true),
        (0xD6, Opcode::ODIF, true),
        (0xD7, Opcode::ODIFI, true),
        (0xD8, Opcode::MUX, true),
        (0xD9, Opcode::MUXI, true),
        (0xDA, Opcode::SADD, true),
        (0xDB, Opcode::SADDI, true),
        (0xDC, Opcode::MOR, true),
        (0xDD, Opcode::MORI, true),
        (0xDE, Opcode::MXOR, true),
        (0xDF, Opcode::MXORI, true),
        (0xE0, Opcode::SETH, true),
        (0xE1, Opcode::SETMH, true),
        (0xE2, Opcode::SETML, true),
        (0xE3, Opcode::SETL, true),
        (0xE4, Opcode::INCH, true),
        (0xE5, Opcode::INCMH, true),
        (0xE6, Opcode::INCML, true),
        (0xE7, Opcode::INCL, true),
        (0xE8, Opcode::ORH, true),
        (0xE9, Opcode::ORMH, true),
        (0xEA, Opcode::ORML, true),
        (0xEB, Opcode::ORL, true),
        (0xEC, Opcode::ANDNH, true),
        (0xED, Opcode::ANDNMH, true),
        (0xEE, Opcode::ANDNML, true),
        (0xEF, Opcode::ANDNL, true),
        (0xF0, Opcode::JMP, false),
        (0xF1, Opcode::JMPB, false),
        (0xF2, Opcode::PUSHJ, true),
        (0xF3, Opcode::PUSHJB, true),
        (0xF4, Opcode::GETA, true),
        (0xF5, Opcode::GETAB, true),
        (0xF6, Opcode::PUT, false),
        (0xF7, Opcode::PUTI, false),
        (0xF8, Opcode::POP, false),
        (0xF9, Opcode::RESUME, false),
        (0xFA, Opcode::SAVE, true),
        (0xFB, Opcode::UNSAVE, false),
        (0xFC, Opcode::SYNC, false),
        (0xFD, Opcode::SWYM, false),
        (0xFE, Opcode::GET, true),
        (0xFF, Opcode::TRIP, false),
    ];

    assert_eq!(table.len(), 256, "the table must cover every opcode byte");

    for (i, (byte, mnemonic, expected)) in table.iter().enumerate() {
        assert_eq!(
            *byte, i as u8,
            "row {i} is out of place: carries byte {byte:#04X}"
        );
        assert_eq!(
            Opcode::try_from(*byte).unwrap(),
            *mnemonic,
            "row {byte:#04X} names {mnemonic:?}, but Opcode disagrees"
        );
        let actual = MMix::writes_general_register_x(*byte);
        assert_eq!(
            actual, *expected,
            "byte {byte:#04X} ({mnemonic:?}): predicate says {actual}, table says {expected}"
        );
    }
}

#[test]
fn test_put_rl_keeps_the_globals_in_a_state_mmix_rejects() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RG, 60);
    mmix.set_register(55, 777); // local while rG = 60, so rL rises to 56
    mmix.set_special(SpecialReg::RG, 32); // $55 is global now

    // PUTI rL,3
    mmix.write_tetra(0, 0xF7140003);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    assert_eq!(mmix.get_register(55), 777, "a global keeps its value");
}

#[test]
fn test_get_of_rl_into_a_marginal_destination_sees_the_raised_value() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RG, 32);
    mmix.set_special(SpecialReg::RL, 5);

    // GET $10,rL
    mmix.write_tetra(0, 0xFE0A0014);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_register(10), 11);
    assert_eq!(mmix.get_special(SpecialReg::RL), 11);
}

#[test]
fn test_get_z_at_32_and_255_are_rejected_with_no_special_register_above_31() {
    for z in [0x20u32, 0xFF] {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RB, 111);
        mmix.set_special(SpecialReg::RJ, 222);
        mmix.set_special(SpecialReg::RG, 40);
        mmix.set_special(SpecialReg::RL, 5);
        mmix.set_special(SpecialReg::RA, 333);

        // GET $1,Z
        mmix.write_tetra(0, 0xFE010000 | z);
        assert!(!mmix.execute_instruction());

        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(mmix.get_register(1), 0, "$X did not change");
        assert_eq!(mmix.get_special(SpecialReg::RB), 111);
        assert_eq!(mmix.get_special(SpecialReg::RJ), 222);
        assert_eq!(mmix.get_special(SpecialReg::RG), 40);
        assert_eq!(mmix.get_special(SpecialReg::RL), 5);
        assert_eq!(mmix.get_special(SpecialReg::RA), 333);
        assert_eq!(handle.diagnostics().len(), 1);
        assert_eq!(mmix.get_exit_code(), 1);
    }
}

/// VAL-1: `GET`'s Y must be zero. `$X` starts marginal, so a claim before
/// this check would raise `rL` and show up here.
#[test]
fn test_get_y_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RG, 40);
    mmix.set_special(SpecialReg::RL, 0);

    // GET $1,1,rZZ ($1 is marginal; Y=1 must be zero)
    mmix.write_tetra(0, 0xFE01011F);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_register(1), 0, "$X did not change");
    assert_eq!(mmix.get_special(SpecialReg::RL), 0, "rL did not rise");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "GET Y=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000000"
    );
}

#[test]
fn test_get_z_ge_32_into_a_marginal_destination_leaves_rl_unchanged() {
    let (host, _handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RG, 40);
    mmix.set_special(SpecialReg::RL, 5);

    // GET $10,32 -- $10 is marginal (rL = 5), Z = 32 names no register
    mmix.write_tetra(0, 0xFE0A0020);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RL), 5, "rL did not rise");
    assert_eq!(
        mmix.get_register(10),
        0,
        "the marginal destination reads zero"
    );
}

#[test]
fn test_get_z_at_31_still_succeeds() {
    let mut mmix = MMix::new();

    // GET $1,31 (rZZ)
    mmix.write_tetra(0, 0xFE01001F);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 4);
    assert_eq!(mmix.get_register(1), mmix.get_special(SpecialReg::RZZ));
}

#[test]
fn test_arithmetic_destination_rise_leaves_a_marginal_source_reading_zero() {
    let mut mmix = MMix::new();
    mmix.set_register(40, 0xDEAD); // global while rG = 32
    mmix.set_special(SpecialReg::RG, 50);
    mmix.set_special(SpecialReg::RL, 3);

    // ADD $45,$40,$0
    mmix.write_tetra(0, 0x202D2800);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_register(45), 0);
    assert_eq!(mmix.get_register(40), 0);
    assert_eq!(mmix.get_special(SpecialReg::RL), 46);
}

#[test]
fn test_a_conditional_set_that_stores_nothing_still_raises_rl() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RL, 3);

    // CSN $8,$6,$7 - $6 is zero, so nothing is stored.
    mmix.write_tetra(0, 0x60080607);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RL), 9);
}

#[test]
fn test_a_read_modify_write_destination_reads_its_own_rise_as_zero() {
    let mut mmix = MMix::new();
    mmix.set_register(40, 777); // global while rG = 32
    mmix.set_special(SpecialReg::RG, 50);
    mmix.set_special(SpecialReg::RL, 3);

    // INCL $40,5
    mmix.write_tetra(0, 0xE7280005);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_register(40), 5);
    assert_eq!(mmix.get_special(SpecialReg::RL), 41);
}

#[test]
fn test_go_claims_its_destination_before_reading_its_address() {
    let mut mmix = MMix::new();
    mmix.set_register(40, 0xDEAD); // global while rG = 32
    mmix.set_special(SpecialReg::RG, 50);
    mmix.set_special(SpecialReg::RL, 3);

    // GO $45,$40,4
    mmix.write_tetra(0, 0x9F2D2804);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 4, "$40 reads zero, so the target is 0+4");
    assert_eq!(mmix.get_register(45), 4, "the tetra after the GO");
    assert_eq!(mmix.get_special(SpecialReg::RL), 46);
}

#[test]
fn test_a_store_does_not_claim_its_x_register() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RL, 3);

    // STO $8,$0,0
    mmix.write_tetra(0, 0xAD080000);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
}

// ============== rA layout and the integer events ==============

#[test]
fn test_ra_bits_match_knuth_predefs() {
    // The predefined symbols: D_BIT=#80 V_BIT=#40 W_BIT=#20 I_BIT=#10
    //                         O_BIT=#08 U_BIT=#04 Z_BIT=#02 X_BIT=#01
    assert_eq!(RA_D, 0x80);
    assert_eq!(RA_V, 0x40);
    assert_eq!(RA_W, 0x20);
    assert_eq!(RA_I, 0x10);
    assert_eq!(RA_O, 0x08);
    assert_eq!(RA_U, 0x04);
    assert_eq!(RA_Z, 0x02);
    assert_eq!(RA_X, 0x01);
    assert_eq!(RA_ROUND_SHIFT, 16);
    assert_eq!(RA_MAX, 0x3FFFF);
}

#[test]
fn test_put_ra_writes_the_rounding_mode_at_bits_17_16() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 1 << RA_ROUND_SHIFT); // ROUND_OFF
    mmix.write_tetra(0, 0xF6150001); // PUT rA,$1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA), 1 << RA_ROUND_SHIFT);

    // The mode is read from those bits: ROUND_OFF truncates 42.9 to 42.
    mmix.set_register(3, 42.9f64.to_bits());
    mmix.write_tetra(4, 0x05020003); // FIX $2,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(2), 42);

    // Raising an event leaves the mode field alone.
    let ra = mmix.get_special(SpecialReg::RA);
    assert_eq!(ra & RA_X, RA_X, "the conversion was inexact");
    assert_eq!(ra >> RA_ROUND_SHIFT, 1, "the mode survives an event");
}

#[test]
fn test_put_ra_rejects_a_value_wider_than_18_bits() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT);
    mmix.set_register(1, RA_MAX + 1);
    mmix.write_tetra(0, 0xF6150001); // PUT rA,$1
    assert!(!mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RA),
        2 << RA_ROUND_SHIFT,
        "a rejected write leaves rA unchanged"
    );
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rA"));
    assert_eq!(mmix.get_exit_code(), 1);
}

#[test]
fn test_put_ra_accepts_the_widest_legal_value() {
    let mut mmix = MMix::new();
    mmix.set_register(1, RA_MAX);
    mmix.write_tetra(0, 0xF6150001); // PUT rA,$1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_MAX);
}

#[test]
fn test_put_other_specials_is_not_capped() {
    let mut mmix = MMix::new();
    mmix.set_register(1, u64::MAX);
    mmix.write_tetra(0, 0xF6010001); // PUT rD,$1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RD), u64::MAX);
}
