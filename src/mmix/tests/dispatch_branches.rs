//! Conditional branches, JMP, GETA, PUSHJ/GO targets.

use super::*;

#[test]
fn test_bn_taken() {
    let mut mmix = MMix::new();
    // BN $1, 0, 5 - Branch if $1 is negative, offset = 5
    mmix.set_register(1, (-42i64) as u64);
    mmix.write_tetra(0, 0x40010005); // BN $1,0,5
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 20); // PC = 0 + 5*4 = 20
}

#[test]
fn test_bn_not_taken() {
    let mut mmix = MMix::new();
    // BN $1, 0, 5 - Branch if $1 is negative, offset = 5
    mmix.set_register(1, 42);
    mmix.write_tetra(0, 0x40010005); // BN $1,0,5
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advances normally
}

#[test]
fn test_bnb_taken() {
    let mut mmix = MMix::new();
    // BNB $1, 0xFFFD - Branch backward if $1 is negative (-3 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, (-42i64) as u64);
    mmix.write_tetra(100, 0x4101FFFD); // BNB $1,0,0xFFFD (offset -3)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 88); // PC = 100 + 4*(-3) = 88
}

#[test]
fn test_bz_taken() {
    let mut mmix = MMix::new();
    // BZ $1, 0, 10 - Branch if $1 is zero
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x4201000A); // BZ $1,0,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
}

#[test]
fn test_bz_not_taken() {
    let mut mmix = MMix::new();
    // BZ $1, 0, 10 - Branch if $1 is zero
    mmix.set_register(1, 1);
    mmix.write_tetra(0, 0x4201000A); // BZ $1,0,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bzb_taken() {
    let mut mmix = MMix::new();
    // BZB $1, 0xFFFB - Branch backward if $1 is zero (-5 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x4301FFFB); // BZB $1,0,0xFFFB (offset -5)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 80); // PC = 100 + 4*(-5) = 80
}

#[test]
fn test_bp_taken() {
    let mut mmix = MMix::new();
    // BP $1, 0, 8 - Branch if $1 is positive
    mmix.set_register(1, 42);
    mmix.write_tetra(0, 0x44010008); // BP $1,0,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 32); // PC = 0 + 8*4 = 32
}

#[test]
fn test_bp_not_taken_zero() {
    let mut mmix = MMix::new();
    // BP $1, 0, 8 - Branch if $1 is positive (zero is not positive)
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x44010008); // BP $1,0,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bp_not_taken_negative() {
    let mut mmix = MMix::new();
    // BP $1, 0, 8 - Branch if $1 is positive
    mmix.set_register(1, (-1i64) as u64);
    mmix.write_tetra(0, 0x44010008); // BP $1,0,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bpb_taken() {
    let mut mmix = MMix::new();
    // BPB $1, 0xFFFE - Branch backward if $1 is positive (-2 tetras)
    mmix.set_pc(200);
    mmix.set_register(1, 100);
    mmix.write_tetra(200, 0x4501FFFE); // BPB $1,0,0xFFFE (offset -2)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 192); // PC = 200 + 4*(-2) = 192
}

#[test]
fn test_bod_taken() {
    let mut mmix = MMix::new();
    // BOD $1, 0, 3 - Branch if $1 is odd
    mmix.set_register(1, 7);
    mmix.write_tetra(0, 0x46010003); // BOD $1,0,3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 12); // PC = 0 + 3*4 = 12
}

#[test]
fn test_bod_not_taken() {
    let mut mmix = MMix::new();
    // BOD $1, 0, 3 - Branch if $1 is odd
    mmix.set_register(1, 8);
    mmix.write_tetra(0, 0x46010003); // BOD $1,0,3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bodb_taken() {
    let mut mmix = MMix::new();
    // BODB $1, 0xFFFC - Branch backward if $1 is odd (-4 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 15);
    mmix.write_tetra(100, 0x4701FFFC); // BODB $1,0,0xFFFC (offset -4)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 84); // PC = 100 + 4*(-4) = 84
}

#[test]
fn test_bnn_taken_positive() {
    let mut mmix = MMix::new();
    // BNN $1, 0, 6 - Branch if $1 is non-negative (>= 0)
    mmix.set_register(1, 42);
    mmix.write_tetra(0, 0x48010006); // BNN $1,0,6
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 24); // PC = 0 + 6*4 = 24
}

#[test]
fn test_bnn_taken_zero() {
    let mut mmix = MMix::new();
    // BNN $1, 0, 6 - Branch if $1 is non-negative (includes zero)
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x48010006); // BNN $1,0,6
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 24);
}

#[test]
fn test_bnn_not_taken() {
    let mut mmix = MMix::new();
    // BNN $1, 0, 6 - Branch if $1 is non-negative
    mmix.set_register(1, (-1i64) as u64);
    mmix.write_tetra(0, 0x48010006); // BNN $1,0,6
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bnnb_taken() {
    let mut mmix = MMix::new();
    // BNNB $1, 0xFFFD - Branch backward if $1 is non-negative (-3 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x4901FFFD); // BNNB $1,0,0xFFFD (offset -3)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 88); // PC = 100 + 4*(-3) = 88
}

#[test]
fn test_bnz_taken() {
    let mut mmix = MMix::new();
    // BNZ $1, 0, 7 - Branch if $1 is non-zero
    mmix.set_register(1, 1);
    mmix.write_tetra(0, 0x4A010007); // BNZ $1,0,7
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 28); // PC = 0 + 7*4 = 28
}

#[test]
fn test_bnz_not_taken() {
    let mut mmix = MMix::new();
    // BNZ $1, 0, 7 - Branch if $1 is non-zero
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x4A010007); // BNZ $1,0,7
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bnzb_taken() {
    let mut mmix = MMix::new();
    // BNZB $1, 0xFFF6 - Branch backward if $1 is non-zero (-10 tetras)
    mmix.set_pc(200);
    mmix.set_register(1, 99);
    mmix.write_tetra(200, 0x4B01FFF6); // BNZB $1,0,0xFFF6 (offset -10)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 160); // PC = 200 + 4*(-10) = 160
}

#[test]
fn test_bnp_taken_negative() {
    let mut mmix = MMix::new();
    // BNP $1, 0, 4 - Branch if $1 is non-positive (<= 0)
    mmix.set_register(1, (-5i64) as u64);
    mmix.write_tetra(0, 0x4C010004); // BNP $1,0,4
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 16); // PC = 0 + 4*4 = 16
}

#[test]
fn test_bnp_taken_zero() {
    let mut mmix = MMix::new();
    // BNP $1, 0, 4 - Branch if $1 is non-positive (includes zero)
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x4C010004); // BNP $1,0,4
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 16);
}

#[test]
fn test_bnp_not_taken() {
    let mut mmix = MMix::new();
    // BNP $1, 0, 4 - Branch if $1 is non-positive
    mmix.set_register(1, 1);
    mmix.write_tetra(0, 0x4C010004); // BNP $1,0,4
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bnpb_taken() {
    let mut mmix = MMix::new();
    // BNPB $1, 0xFFFF - Branch backward if $1 is non-positive (-1 tetra)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x4D01FFFF); // BNPB $1,0,0xFFFF (offset -1)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 96); // PC = 100 + 4*(-1) = 96
}

#[test]
fn test_bev_taken() {
    let mut mmix = MMix::new();
    // BEV $1, 0, 12 - Branch if $1 is even
    mmix.set_register(1, 8);
    mmix.write_tetra(0, 0x4E01000C); // BEV $1,0,12
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 48); // PC = 0 + 12*4 = 48
}

#[test]
fn test_bev_not_taken() {
    let mut mmix = MMix::new();
    // BEV $1, 0, 12 - Branch if $1 is even
    mmix.set_register(1, 7);
    mmix.write_tetra(0, 0x4E01000C); // BEV $1,0,12
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_bevb_taken() {
    let mut mmix = MMix::new();
    // BEVB $1, 0xFFFE - Branch backward if $1 is even (-2 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x4F01FFFE); // BEVB $1,0,0xFFFE (offset -2)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 92); // PC = 100 + 4*(-2) = 92
}

#[test]
fn test_pbn_taken() {
    let mut mmix = MMix::new();
    // PBN $1, 0, 5 - Probable branch if $1 is negative
    mmix.set_register(1, (-10i64) as u64);
    mmix.write_tetra(0, 0x50010005); // PBN $1,0,5
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 20); // PC = 0 + 5*4 = 20
}

#[test]
fn test_pbnb_taken() {
    let mut mmix = MMix::new();
    // PBNB $1, 0xFFFD - Probable branch backward if $1 is negative (-3 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, (-1i64) as u64);
    mmix.write_tetra(100, 0x5101FFFD); // PBNB $1,0,0xFFFD (offset -3)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 88); // PC = 100 + 4*(-3) = 88
}

#[test]
fn test_pbz_taken() {
    let mut mmix = MMix::new();
    // PBZ $1, 0, 6 - Probable branch if $1 is zero
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x52010006); // PBZ $1,0,6
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 24); // PC = 0 + 6*4 = 24
}

#[test]
fn test_pbzb_taken() {
    let mut mmix = MMix::new();
    // PBZB $1, 0xFFFC - Probable branch backward if $1 is zero (-4 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x5301FFFC); // PBZB $1,0,0xFFFC (offset -4)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 84); // PC = 100 + 4*(-4) = 84
}

#[test]
fn test_pbp_taken() {
    let mut mmix = MMix::new();
    // PBP $1, 0, 8 - Probable branch if $1 is positive
    mmix.set_register(1, 50);
    mmix.write_tetra(0, 0x54010008); // PBP $1,0,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 32); // PC = 0 + 8*4 = 32
}

#[test]
fn test_pbpb_taken() {
    let mut mmix = MMix::new();
    // PBPB $1, 0xFFFE - Probable branch backward if $1 is positive (-2 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 1);
    mmix.write_tetra(100, 0x5501FFFE); // PBPB $1,0,0xFFFE (offset -2)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 92); // PC = 100 + 4*(-2) = 92
}

#[test]
fn test_pbod_taken() {
    let mut mmix = MMix::new();
    // PBOD $1, 0, 3 - Probable branch if $1 is odd
    mmix.set_register(1, 11);
    mmix.write_tetra(0, 0x56010003); // PBOD $1,0,3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 12); // PC = 0 + 3*4 = 12
}

#[test]
fn test_pbodb_taken() {
    let mut mmix = MMix::new();
    // PBODB $1, 0xFFFB - Probable branch backward if $1 is odd (-5 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 99);
    mmix.write_tetra(100, 0x5701FFFB); // PBODB $1,0,0xFFFB (offset -5)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 80); // PC = 100 + 4*(-5) = 80
}

#[test]
fn test_pbnn_taken() {
    let mut mmix = MMix::new();
    // PBNN $1, 0, 7 - Probable branch if $1 is non-negative
    mmix.set_register(1, 100);
    mmix.write_tetra(0, 0x58010007); // PBNN $1,0,7
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 28); // PC = 0 + 7*4 = 28
}

#[test]
fn test_pbnnb_taken() {
    let mut mmix = MMix::new();
    // PBNNB $1, 0xFFFF - Probable branch backward if $1 is non-negative (-1 tetra)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x5901FFFF); // PBNNB $1,0,0xFFFF (offset -1)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 96); // PC = 100 + 4*(-1) = 96
}

#[test]
fn test_pbnz_taken() {
    let mut mmix = MMix::new();
    // PBNZ $1, 0, 9 - Probable branch if $1 is non-zero
    mmix.set_register(1, 42);
    mmix.write_tetra(0, 0x5A010009); // PBNZ $1,0,9
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 36); // PC = 0 + 9*4 = 36
}

#[test]
fn test_pbnzb_taken() {
    let mut mmix = MMix::new();
    // PBNZB $1, 0xFFFA - Probable branch backward if $1 is non-zero (-6 tetras)
    mmix.set_pc(200);
    mmix.set_register(1, 1);
    mmix.write_tetra(200, 0x5B01FFFA); // PBNZB $1,0,0xFFFA (offset -6)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 176); // PC = 200 + 4*(-6) = 176
}

#[test]
fn test_pbnp_taken() {
    let mut mmix = MMix::new();
    // PBNP $1, 0, 4 - Probable branch if $1 is non-positive
    mmix.set_register(1, (-100i64) as u64);
    mmix.write_tetra(0, 0x5C010004); // PBNP $1,0,4
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 16); // PC = 0 + 4*4 = 16
}

#[test]
fn test_pbnpb_taken() {
    let mut mmix = MMix::new();
    // PBNPB $1, 0xFFF8 - Probable branch backward if $1 is non-positive (-8 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x5D01FFF8); // PBNPB $1,0,0xFFF8 (offset -8)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 68); // PC = 100 + 4*(-8) = 68
}

#[test]
fn test_pbev_taken() {
    let mut mmix = MMix::new();
    // PBEV $1, 0, 10 - Probable branch if $1 is even
    mmix.set_register(1, 100);
    mmix.write_tetra(0, 0x5E01000A); // PBEV $1,0,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
}

#[test]
fn test_pbevb_taken() {
    let mut mmix = MMix::new();
    // PBEVB $1, 0xFFF9 - Probable branch backward if $1 is even (-7 tetras)
    mmix.set_pc(100);
    mmix.set_register(1, 0);
    mmix.write_tetra(100, 0x5F01FFF9); // PBEVB $1,0,0xFFF9 (offset -7)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 72); // PC = 100 + 4*(-7) = 72
}

#[test]
fn test_jmp_forward() {
    let mut mmix = MMix::new();
    // JMP +10 (offset = 10)
    mmix.write_tetra(0, 0xF000000A); // JMP 0,0,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
}

#[test]
fn test_jmp_large_forward_offset() {
    let mut mmix = MMix::new();
    mmix.set_pc(100);
    // XYZ is unsigned in the forward opcode: 0xFFFFFB is 16777211, not -5.
    mmix.write_tetra(100, 0xF0FFFFFB); // JMP 0xFFFFFB
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 100 + 16777211 * 4); // PC = 100 + 16777211*4
}

#[test]
fn test_jmpb() {
    let mut mmix = MMix::new();
    mmix.set_pc(100);
    // XYZ = 0xFFFFFB is 0xFFFFFB - 2^24 = -5 tetras.
    mmix.write_tetra(100, 0xF1FFFFFB); // JMPB 0xFFFFFB
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 80); // PC = 100 - 5*4 = 80
}

#[test]
fn test_geta() {
    let mut mmix = MMix::new();
    mmix.set_pc(100);
    // GETA $1, 0, 10 - Get address at relative offset 10
    mmix.write_tetra(100, 0xF401000A); // GETA $1,0,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 140); // Addr = 100 + 10*4 = 140
    assert_eq!(mmix.get_pc(), 104); // PC advances normally
}

#[test]
fn test_getab() {
    let mut mmix = MMix::new();
    mmix.set_pc(100);
    // YZ = 0xFFFB is 0xFFFB - 65536 = -5 tetras.
    mmix.write_tetra(100, 0xF501FFFB); // GETAB $1,0xFFFB
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 80); // Addr = 100 - 5*4 = 80
    assert_eq!(mmix.get_pc(), 104);
}

#[test]
fn pushjb_reaches_a_backward_callee() {
    // The PUSHJB sits ten tetras past AddFunc, so YZ is 65536 - 10.
    // Read as a magnitude that lands in zeroed memory and halts with 0.
    let source = "\
\tLOC\t#100
AddFunc\tADDU\t$0,$0,$1
\tPOP\t1,0
Main\tSETI\t$1,40
\tSETI\t$2,2
\tPUSHJB\t$0,AddFunc
\tSET\t$255,$0
\tTRAP\t0,Halt,0
";
    assert_eq!(run_to_halt(source), 42);
}

#[test]
fn forward_branch_past_half_the_field_still_goes_forward() {
    // BZ's YZ is 32768; sign extension reads it as -32768.
    let source = "\
\tLOC\t#100
Start\tSETI\t$1,0
\tBZ\t$1,Far
\tSETI\t$255,1
\tTRAP\t0,Halt,0
\tLOC\t#20110
Far\tSETI\t$255,42
\tTRAP\t0,Halt,0
";
    assert_eq!(run_to_halt(source), 42);
}

#[test]
fn backward_branch_past_half_the_field_still_goes_backward() {
    // BZB's YZ is 32764, which is 32764 - 65536 = -32772 tetras.
    // Sign extension reads it as +32764.
    let source = "\
\tLOC\t#100
Back\tSETI\t$255,42
\tTRAP\t0,Halt,0
\tLOC\t#20100
Main\tSETI\t$1,0
\tBZB\t$1,Back
\tSETI\t$255,1
\tTRAP\t0,Halt,0
";
    assert_eq!(run_to_halt(source), 42);
}

#[test]
fn pushj_forward_past_half_the_field_still_goes_forward() {
    // PUSHJ's YZ is 32772; sign extension reads it as -32764.
    let source = "\
\tLOC\t#100
Main\tSETI\t$1,40
\tPUSHJ\t$0,Far
\tSET\t$255,$0
\tTRAP\t0,Halt,0
\tLOC\t#20120
Far\tSETI\t$0,42
\tPOP\t1,0
";
    assert_eq!(run_to_halt(source), 42);
}

#[test]
fn test_geta_forward_field_above_half_the_range() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    // YZ = 0xC000 is 49152 tetras forward, not -16384.
    mmix.write_tetra(0x100, 0xF401C000); // GETA $1,0xC000
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x100 + 49152 * 4);
}

#[test]
fn test_jmpb_small_field_is_a_far_backward_jump() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x4000000);
    // XYZ = 5 is 5 - 2^24 tetras, the far end of JMPB's reach.
    mmix.write_tetra(0x4000000, 0xF1000005); // JMPB 5
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x4000000 - (16777216 - 5) * 4);
}

#[test]
fn test_getab_small_field_is_a_far_backward_address() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x40000);
    // YZ = 5 is 5 - 65536 tetras, the far end of GETAB's reach.
    mmix.write_tetra(0x40000, 0xF5010005); // GETAB $1,5
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x40000 - (65536 - 5) * 4);
}

#[test]
fn test_go() {
    let mut mmix = MMix::new();
    // GO $1, $2, $3 - Go to location
    mmix.set_register(2, 1000);
    mmix.set_register(3, 24);
    mmix.write_tetra(0, 0x9E010203); // GO $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 4); // Return address
    assert_eq!(mmix.get_pc(), 1024); // Jump to 1000 + 24
}

#[test]
fn test_goi() {
    let mut mmix = MMix::new();
    // GOI $1, $2, 200 - Go to location immediate
    mmix.set_register(2, 5000);
    mmix.write_tetra(0, 0x9F0102C8); // GOI $1,$2,200
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 4); // Return address
    assert_eq!(mmix.get_pc(), 5200); // Jump to 5000 + 200
}

#[test]
fn test_go_at_top_of_memory_wraps_return_address() {
    let mut mmix = MMix::new();
    mmix.set_pc(0xFFFFFFFFFFFFFFFC);
    mmix.set_register(2, 1000);
    mmix.set_register(3, 24);
    mmix.write_tetra(0xFFFFFFFFFFFFFFFC, 0x9E010203); // GO $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // @+4 wraps to 0
    assert_eq!(mmix.get_pc(), 1024); // Jump to 1000 + 24
}

#[test]
fn test_goi_at_top_of_memory_wraps_return_address() {
    let mut mmix = MMix::new();
    mmix.set_pc(0xFFFFFFFFFFFFFFFC);
    mmix.set_register(2, 5000);
    mmix.write_tetra(0xFFFFFFFFFFFFFFFC, 0x9F0102C8); // GOI $1,$2,200
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // @+4 wraps to 0
    assert_eq!(mmix.get_pc(), 5200); // Jump to 5000 + 200
}
