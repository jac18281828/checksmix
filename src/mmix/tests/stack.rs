//! PUSHJ/PUSHGO/POP frames and SAVE/UNSAVE contexts.

use super::*;

#[test]
fn test_register_stack_initialization() {
    let mmix = MMix::new();
    assert_eq!(mmix.get_special(SpecialReg::RO), STACK_SEGMENT_START);
    assert_eq!(mmix.get_special(SpecialReg::RS), STACK_SEGMENT_START);
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    assert_eq!(mmix.get_special(SpecialReg::RG), 32);
    assert_eq!(mmix.call_depth(), 0);
}

#[test]
fn test_pushj_basic() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 100);
    mmix.set_register(1, 101);
    mmix.set_register(2, 102);
    mmix.set_special(SpecialReg::RL, 3);
    let ro = mmix.get_special(SpecialReg::RO);

    // PUSHJ $3, +1 pushes X+1 = 4 entries: $0..$2 and the hole marker.
    mmix.write_tetra(0x100, 0xF2030001);
    mmix.execute_instruction();

    // rO and rS both advance by X+1 = 4 octas.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 32);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro + 32);
    // new rL = max(0, rL_old - X - 1) = max(0, 3-3-1) = 0
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);
    assert_eq!(mmix.get_pc(), 0x104);

    // Saved frame in memory: $0, $1, $2, then the hole marker = X.
    assert_eq!(mmix.read_octa(ro), 100);
    assert_eq!(mmix.read_octa(ro + 8), 101);
    assert_eq!(mmix.read_octa(ro + 16), 102);
    assert_eq!(mmix.read_octa(ro + 24), 3); // hole marker = X

    // Live register file: caller's $0..$2 zeroed (no slide source above $X here).
    assert_eq!(mmix.get_register(0), 0);
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_register(2), 0);

    assert_eq!(mmix.call_depth(), 1);
}

/// The only test pinning part 1's destination rise through PUSHJ:
/// writing to a marginal $X raises rL and zeroes $rL..$X before
/// push_frame runs, so the spill holds zeros, not whatever the flat
/// array held from an earlier, unrelated frame.
#[test]
fn test_pushj_spills_zero_not_stale_content_when_caller_rl_is_zero() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    // Dirty the register file directly, then drop rL to 0 without
    // going through claim_local, so $0..$2 are marginal but the
    // physical array still holds the dirty values.
    mmix.set_register(0, 0xDEAD);
    mmix.set_register(1, 0xBEEF);
    mmix.set_register(2, 0xCAFE);
    mmix.set_special(SpecialReg::RL, 0);
    let ro = mmix.get_special(SpecialReg::RO);

    // PUSHJ $2, +1: X=2, caller's rL = 0.
    mmix.write_tetra(0x100, 0xF2020001);
    mmix.execute_instruction();

    // The destination rise zeroes $0..$2 before the spill runs, so
    // memory holds zeros, not the dirty values.
    assert_eq!(mmix.read_octa(ro), 0);
    assert_eq!(mmix.read_octa(ro + 8), 0);
    assert_eq!(mmix.read_octa(ro + 16), 2); // hole marker = X
    // The entries are fixed by X, not by the raised rL: the new rL
    // settles back at 0 either way.
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);
}

/// `POP` reads its hole from `M8[rO-8]` at the moment it runs, not from
/// any count `PUSHJ` cached. Rewriting that word between the two changes
/// how far `POP` retracts: no implementation that ignores memory can
/// pass this.
#[test]
fn test_pop_reads_the_hole_from_memory_even_when_rewritten() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 100);
    mmix.set_register(1, 101);
    mmix.set_register(2, 102);
    mmix.set_special(SpecialReg::RL, 3);
    let ro = mmix.get_special(SpecialReg::RO);

    // PUSHJ $3, +1: hole marker 3 lands at ro+24; rO advances to ro+32.
    mmix.write_tetra(0x100, 0xF2030001);
    mmix.execute_instruction();
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 32);

    // Overwrite the hole in memory: POP must read this value, 1, not
    // the pushed x = 3.
    mmix.write_octa(ro + 24, 1);

    // POP 0, 0
    mmix.write_tetra(0x104, 0xF8000000);
    mmix.execute_instruction();

    // Reading x = 3 (the pushed value) would retract by 4 octas,
    // landing back at ro. Reading the rewritten hole (1) retracts by
    // only 2, landing short of it -- proof POP read memory, not a
    // cached count.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 16);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro + 16);
}

#[test]
fn test_pushj_window_slide_argument_passing() {
    // Stage an arg at $5; PUSHJ $4 should make it visible to the callee at $0.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(5, 0xAA);
    mmix.set_special(SpecialReg::RL, 8); // 8 active locals

    // PUSHJ $4, +1
    mmix.write_tetra(0x100, 0xF2040001);
    mmix.execute_instruction();

    // Callee sees caller's $5 as $0 (slid down by X+1 = 5).
    assert_eq!(mmix.get_register(0), 0xAA);
    // rL = 8 - 5 = 3
    assert_eq!(mmix.get_special(SpecialReg::RL), 3);

    // POP 0,0 — no output reaches $5, so it is marginal and reads zero.
    mmix.write_tetra(0x104, 0xF8000000);
    mmix.execute_instruction();
    assert_eq!(mmix.get_special(SpecialReg::RL), 4);
    assert_eq!(mmix.get_register(5), 0);
    // $4 was the marginal hole — it's consumed by PUSHJ and reads as zero
    // after POP 0 since no return value lands there.
    assert_eq!(mmix.get_register(4), 0);
}

#[test]
fn test_pushj_window_slide_return_value() {
    // POP 1 lands the single return value at the caller's hole position $X.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RL, 5);

    // PUSHJ $4, +1
    mmix.write_tetra(0x100, 0xF2040001);
    mmix.execute_instruction();

    // Callee writes 0xBB into $0 as the return value.
    mmix.set_register(0, 0xBB);

    // POP 1, 0
    mmix.write_tetra(0x104, 0xF8010000);
    mmix.execute_instruction();

    // Return value lands at caller's $4 (the hole).
    assert_eq!(mmix.get_register(4), 0xBB);
    // rL = min(x+n, rG) = min(4+1, 32) = 5
    assert_eq!(mmix.get_special(SpecialReg::RL), 5);
}

#[test]
fn test_pushj_zeros_freshly_allocated_locals() {
    // After PUSHJ, callee locals beyond the slide window must read as zero.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    for i in 0..8u8 {
        mmix.set_register(i, 1000 + i as u64);
    }
    mmix.set_special(SpecialReg::RL, 8);

    // PUSHJ $4, +1
    mmix.write_tetra(0x100, 0xF2040001);
    mmix.execute_instruction();

    // Slid-down values: caller $5..$7 → callee $0..$2.
    assert_eq!(mmix.get_register(0), 1005);
    assert_eq!(mmix.get_register(1), 1006);
    assert_eq!(mmix.get_register(2), 1007);
    // Vacated tail must be zero.
    assert_eq!(mmix.get_register(3), 0);
    assert_eq!(mmix.get_register(4), 0);
    assert_eq!(mmix.get_register(5), 0);
    assert_eq!(mmix.get_register(7), 0);
}

#[test]
fn test_pop_with_return_value_shift() {
    // PUSHJ $3 + POP 2: the last output lands in the hole.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RL, 4);

    // PUSHJ $3, +1
    mmix.write_tetra(0x100, 0xF2030001);
    mmix.execute_instruction();

    mmix.set_register(0, 0x111);
    mmix.set_register(1, 0x222);

    // POP 2, 0
    mmix.write_tetra(0x104, 0xF8020000);
    mmix.execute_instruction();

    // x=3, n=2: the hole $3 gets the last output (0x222); $4 gets 0x111.
    assert_eq!(mmix.get_register(3), 0x222);
    assert_eq!(mmix.get_register(4), 0x111);
    // rL = min(x+n, rG) = min(3+2, 32) = 5
    assert_eq!(mmix.get_special(SpecialReg::RL), 5);
    // Caller's $0..$2 are restored from memory (originally zero).
    assert_eq!(mmix.get_register(0), 0);
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_register(2), 0);
}

#[test]
fn test_pushgo_pop_basic() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 10);
    mmix.set_register(1, 11);
    // Target address: $3 + $4 = 0x200
    mmix.set_register(3, 0x200);
    mmix.set_register(4, 0);
    mmix.set_special(SpecialReg::RL, 2);
    let ro = mmix.get_special(SpecialReg::RO);

    // PUSHGO $2, $3, $4
    mmix.write_tetra(0x100, 0xBE020304);
    mmix.execute_instruction();

    // X=2: rO and rS both advance by X+1 = 3 octas.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro + 24);
    // new rL = saturating_sub(2, 3) = 0
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);
    assert_eq!(mmix.get_pc(), 0x200);

    // Saved $0, $1, then the hole marker = X.
    assert_eq!(mmix.read_octa(ro), 10);
    assert_eq!(mmix.read_octa(ro + 8), 11);
    assert_eq!(mmix.read_octa(ro + 16), 2);
    assert_eq!(mmix.call_depth(), 1);

    // POP 0,0 at target
    mmix.write_tetra(0x200, 0xF8000000);
    mmix.execute_instruction();

    // Caller's $0,$1 restored from memory.
    assert_eq!(mmix.get_register(0), 10);
    assert_eq!(mmix.get_register(1), 11);
    assert_eq!(mmix.get_special(SpecialReg::RO), ro);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro);
    assert_eq!(mmix.get_special(SpecialReg::RL), 2);
    assert_eq!(mmix.get_pc(), 0x104);
    assert_eq!(mmix.call_depth(), 0);
}

#[test]
fn test_pop_basic() {
    // PUSHJ $3 + POP 0: caller's $0..$2 fully restored, rL back to 3.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 100);
    mmix.set_register(1, 101);
    mmix.set_register(2, 102);
    mmix.set_special(SpecialReg::RL, 3);
    let ro = mmix.get_special(SpecialReg::RO);

    // PUSHJ $3, +1
    mmix.write_tetra(0x100, 0xF2030001);
    mmix.execute_instruction();

    // Callee scribbles over its locals.
    mmix.set_register(0, 200);
    mmix.set_register(1, 201);
    mmix.set_register(2, 202);

    // POP 0, 0
    mmix.write_tetra(0x104, 0xF8000000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 100);
    assert_eq!(mmix.get_register(1), 101);
    assert_eq!(mmix.get_register(2), 102);
    assert_eq!(mmix.get_special(SpecialReg::RO), ro);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro);
    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    assert_eq!(mmix.get_pc(), 0x104);
    assert_eq!(mmix.call_depth(), 0);
}

#[test]
fn test_pop_with_return_values() {
    // PUSHJ $4 + POP 3: callee's $0..$2 land at caller's $4..$6.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    for i in 0..8u8 {
        mmix.set_register(i, 100 + i as u64);
    }
    mmix.set_special(SpecialReg::RL, 8);

    // PUSHJ $4, +1
    mmix.write_tetra(0x100, 0xF2040001);
    mmix.execute_instruction();

    // Callee sets 3 return values.
    mmix.set_register(0, 300);
    mmix.set_register(1, 301);
    mmix.set_register(2, 302);

    // POP 3, 0
    mmix.write_tetra(0x104, 0xF8030000);
    mmix.execute_instruction();

    // The hole $4 gets the last output (302); $5, $6 get 300, 301 in order.
    assert_eq!(mmix.get_register(4), 302);
    assert_eq!(mmix.get_register(5), 300);
    assert_eq!(mmix.get_register(6), 301);
    // Caller's $0..$3 restored from memory.
    assert_eq!(mmix.get_register(0), 100);
    assert_eq!(mmix.get_register(1), 101);
    assert_eq!(mmix.get_register(2), 102);
    assert_eq!(mmix.get_register(3), 103);
    // rL = min(x+n, rG) = min(4+3, 32) = 7
    assert_eq!(mmix.get_special(SpecialReg::RL), 7);
}

#[test]
fn test_pop_no_return_values() {
    // PUSHJ $5 + POP 0: caller's $0..$4 are restored; $5..$9 are
    // marginal after POP and read zero.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    for i in 0..10u8 {
        mmix.set_register(i, 100 + i as u64);
    }
    mmix.set_special(SpecialReg::RL, 10);

    // PUSHJ $5, +1
    mmix.write_tetra(0x100, 0xF2050001);
    mmix.execute_instruction();

    // Callee scribbles over its locals.
    mmix.set_register(0, 999);
    mmix.set_register(1, 888);
    mmix.set_register(2, 777);

    // POP 0, 0
    mmix.write_tetra(0x104, 0xF8000000);
    mmix.execute_instruction();

    // Caller's $0..$4 restored from memory.
    for i in 0..5u8 {
        assert_eq!(mmix.get_register(i), 100 + i as u64);
    }
    // $5 was the marginal hole — it's consumed by PUSHJ and reads as zero
    // after POP 0 since no return value lands there.
    assert_eq!(mmix.get_register(5), 0);
    // Slots above the hole are marginal after POP — they read zero, not
    // their stale pre-call values.
    for i in 6..10u8 {
        assert_eq!(mmix.get_register(i), 0);
    }
    assert_eq!(mmix.get_special(SpecialReg::RL), 5);
}

#[test]
fn test_pop_mmixware_program1_matches_measured_values() {
    // Measured on MMIXware and checksmix at 91d207f (see MMIX.md's
    // register stack table). Caller's $1..$5 read zero after POP, and
    // the callee's sole return value lands at the hole $0.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(1, 111);
    mmix.set_register(2, 222);
    mmix.set_register(3, 333);
    mmix.set_register(4, 444);
    mmix.set_register(5, 555);

    // PUSHJ $0, +1
    mmix.write_tetra(0x100, 0xF2000001);
    mmix.execute_instruction();

    // Callee sets its return values.
    mmix.set_register(0, 999);
    mmix.set_register(1, 777);

    // POP 1, 0
    mmix.write_tetra(0x104, 0xF8010000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 999);
    for i in 1..=5u8 {
        assert_eq!(mmix.get_register(i), 0);
    }
    assert_eq!(mmix.get_special(SpecialReg::RL), 1);
}

#[test]
fn test_pop_mmixware_program2_matches_measured_values() {
    // Measured on MMIXware and checksmix at 91d207f. POP 2 puts the
    // callee's last output ($1) in the hole and the first ($0) above
    // it, and registers above the outputs read zero rather than their
    // pre-call values.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 10);
    mmix.set_register(1, 20);
    mmix.set_register(2, 30);
    mmix.set_register(3, 40);
    mmix.set_register(4, 50);
    mmix.set_register(5, 60);
    mmix.set_register(6, 70);

    // PUSHJ $3, +1
    mmix.write_tetra(0x100, 0xF2030001);
    mmix.execute_instruction();

    // Callee sets its three return values.
    mmix.set_register(0, 801);
    mmix.set_register(1, 802);
    mmix.set_register(2, 803);

    // POP 2, 0
    mmix.write_tetra(0x104, 0xF8020000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 10);
    assert_eq!(mmix.get_register(1), 20);
    assert_eq!(mmix.get_register(2), 30);
    assert_eq!(mmix.get_register(3), 802);
    assert_eq!(mmix.get_register(4), 801);
    assert_eq!(mmix.get_register(5), 0);
    assert_eq!(mmix.get_register(6), 0);
    assert_eq!(mmix.get_special(SpecialReg::RL), 5);
}

#[test]
fn test_pop_0_0_leaves_hole_and_everything_above_it_zero() {
    // POP 0,0 leaves the hole marginal: rL becomes x, and every
    // register from there through rG-1 reads zero.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 10);
    mmix.set_register(1, 20);
    mmix.set_register(2, 30);
    mmix.set_register(3, 40);
    mmix.set_special(SpecialReg::RL, 4);

    // PUSHJ $2, +1
    mmix.write_tetra(0x100, 0xF2020001);
    mmix.execute_instruction();

    // Callee scribbles over its one local.
    mmix.set_register(0, 999);

    // POP 0, 0
    mmix.write_tetra(0x104, 0xF8000000);
    mmix.execute_instruction();

    // Caller's $0, $1 restored; $2 (the hole) and everything above it,
    // through rG-1, is marginal and reads zero.
    assert_eq!(mmix.get_register(0), 10);
    assert_eq!(mmix.get_register(1), 20);
    for i in 2..32u8 {
        assert_eq!(mmix.get_register(i), 0);
    }
    assert_eq!(mmix.get_special(SpecialReg::RL), 2);
}

#[test]
fn test_pop_x_greater_than_l_clamps_and_zeros_the_hole() {
    // If X > L, X becomes L+1 and the hole gets zero regardless of what
    // the callee left there.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 100);
    mmix.set_register(2, 200);
    mmix.set_special(SpecialReg::RL, 3);

    // PUSHJ $1, +1 — callee's rL is 1 (only $0 is a valid local).
    mmix.write_tetra(0x100, 0xF2010001);
    mmix.execute_instruction();
    assert_eq!(mmix.get_special(SpecialReg::RL), 1);

    // Callee sets its one local as a return value.
    mmix.set_register(0, 555);

    // POP 3, 0 — X (3) exceeds L (1), so X clamps to L+1 = 2.
    mmix.write_tetra(0x104, 0xF8030000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 100);
    // The hole reads zero, not the callee's $0.
    assert_eq!(mmix.get_register(1), 0);
    // The clamp still delivers the callee's one real output, one slot up.
    assert_eq!(mmix.get_register(2), 555);
    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
}

#[test]
fn test_pushj_x_at_or_above_rg_saves_all_locals_and_pops_at_the_hole() {
    // PUSHJ $X with X >= rG pushes $0..$(rL-1), the callee starts at
    // rL = 0, and the hole for POP is the caller's rL, not X.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RG, 10);
    mmix.set_register(0, 1000);
    mmix.set_register(1, 1001);
    mmix.set_register(2, 1002);
    mmix.set_register(3, 1003);
    mmix.set_register(4, 1004);
    mmix.set_special(SpecialReg::RL, 5);
    mmix.set_register(50, 0xBEEF); // a global, above rG

    let rs_before = mmix.get_special(SpecialReg::RS);

    // PUSHJ $255, +1
    mmix.write_tetra(0x100, 0xF2FF0001);
    mmix.execute_instruction();
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);

    // Callee computes its result in its one local.
    mmix.set_register(0, 777);

    // POP 1, 0
    mmix.write_tetra(0x104, 0xF8010000);
    mmix.execute_instruction();

    // Caller's $0..$4 restored, the output lands at $rL (the hole), and
    // rL becomes min(rL+1, rG).
    assert_eq!(mmix.get_register(0), 1000);
    assert_eq!(mmix.get_register(1), 1001);
    assert_eq!(mmix.get_register(2), 1002);
    assert_eq!(mmix.get_register(3), 1003);
    assert_eq!(mmix.get_register(4), 1004);
    assert_eq!(mmix.get_register(5), 777);
    assert_eq!(mmix.get_special(SpecialReg::RL), 6);
    // Globals are untouched throughout.
    assert_eq!(mmix.get_register(50), 0xBEEF);
    // POP retracts rO and rS to exactly the address PUSHJ found them
    // at: a spill sized by rL, not the hole read back from memory,
    // would retract by the wrong amount and land somewhere else.
    assert_eq!(mmix.get_special(SpecialReg::RS), rs_before);
}

#[test]
fn test_pushgoi_pop_basic() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x300);
    mmix.set_register(0, 42);
    mmix.set_register(1, 43);
    mmix.set_register(5, 0x310);
    mmix.set_special(SpecialReg::RL, 2);
    let ro = mmix.get_special(SpecialReg::RO);

    // PUSHGOI $2, $5, #0x10  → target = $5 + 0x10 = 0x320
    mmix.write_tetra(0x300, 0xBF020510);
    mmix.execute_instruction();

    // X=2: rO and rS both advance by X+1 = 3 octas.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro + 24);
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x304);
    assert_eq!(mmix.get_pc(), 0x320);

    assert_eq!(mmix.read_octa(ro), 42);
    assert_eq!(mmix.read_octa(ro + 8), 43);
    assert_eq!(mmix.read_octa(ro + 16), 2); // hole marker = X
    assert_eq!(mmix.call_depth(), 1);

    // POP 0,0 at target — restore caller's $0,$1.
    mmix.write_tetra(0x320, 0xF8000000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 42);
    assert_eq!(mmix.get_register(1), 43);
    assert_eq!(mmix.get_special(SpecialReg::RO), ro);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro);
    assert_eq!(mmix.get_special(SpecialReg::RL), 2);
    assert_eq!(mmix.get_pc(), 0x304);
    assert_eq!(mmix.call_depth(), 0);
}

#[test]
fn test_pushj_pop_nested() {
    // Two-level nested call exercises arg passing in both directions
    // and that the two frames stack contiguously in memory.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 10);
    mmix.set_register(1, 11);
    mmix.set_register(2, 0xCAFE); // arg for inner call: caller's $2
    mmix.set_special(SpecialReg::RL, 5);
    let ro = mmix.get_special(SpecialReg::RO);

    // Outer PUSHJ $2, +1 — saves $0,$1; slides $3,$4 to callee's $0,$1; arg at $2 → callee $0? No —
    // PUSHJ $X saves $0..$X-1 (here $0,$1) and the marginal at $X. Caller's $X+1, $X+2, ...
    // become callee's $0, $1, ... .  So $2 (the marginal) is *not* an arg; it becomes X.
    // To pass an arg via slide, stage at $X+1 = $3.
    mmix.set_register(3, 0xCAFE);
    mmix.write_tetra(0x100, 0xF2020001);
    mmix.execute_instruction();

    // Outer callee sees arg at $0.
    assert_eq!(mmix.get_register(0), 0xCAFE);
    assert_eq!(mmix.call_depth(), 1);
    // Outer frame: X+1 = 3 entries.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);

    // Outer callee stages an arg at $1 then nested PUSHJ $0, +1 (X=0: nothing saved, slide $1→$0).
    mmix.set_register(1, 0xBEEF);
    mmix.write_tetra(0x104, 0xF2000001);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 0xBEEF);
    assert_eq!(mmix.call_depth(), 2);
    // Inner frame starts exactly where the outer frame's entries end.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 32);

    // Inner POP 1, 0 — return 0xD00D at $0; lands at outer callee's $0 (hole=0).
    mmix.set_register(0, 0xD00D);
    mmix.write_tetra(0x108, 0xF8010000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(0), 0xD00D);
    assert_eq!(mmix.call_depth(), 1);
    // Popping the inner frame retracts rO to exactly where it started.
    assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);

    // Outer POP 1, 0 — return 0xD00D, lands at top-level caller's $2 (hole=2).
    mmix.write_tetra(0x10C, 0xF8010000);
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(2), 0xD00D);
    // Caller's $0, $1 restored.
    assert_eq!(mmix.get_register(0), 10);
    assert_eq!(mmix.get_register(1), 11);
    assert_eq!(mmix.get_special(SpecialReg::RO), ro);
    assert_eq!(mmix.call_depth(), 0);
}

/// `POP` never writes `rJ`: it only reads it for the branch target.
/// Set a distinct, nonzero `rJ` before the call so a restore, if `POP`
/// still did one, would be visible.
#[test]
fn test_pop_leaves_rj_as_pushj_set_it() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RJ, 0x999);
    mmix.write_tetra(0x100, 0xF2000002); // PUSHJ $0,2 -> 0x108, rJ := 0x104
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);

    mmix.write_tetra(0x108, 0xF8000000); // POP 0,0
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RJ),
        0x104,
        "POP must leave rJ exactly as PUSHJ set it, not restore 0x999"
    );
    assert_eq!(mmix.get_pc(), 0x104);
}

/// A callee that makes a nested call without saving `rJ` first has its
/// own `POP` branch to the address after the *nested* `PUSHJ`, not back
/// to its own caller: a subroutine that calls another must save and
/// restore `rJ` itself.
#[test]
fn test_pop_without_saving_rj_returns_to_the_nested_call_site() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    // Top-level caller: PUSHJ $0,2 -> callee at 0x108; rJ := 0x104.
    mmix.write_tetra(0x100, 0xF2000002);
    assert!(mmix.execute_instruction());

    // Callee makes a nested call without saving rJ: PUSHJ $0,4 ->
    // nested callee at 0x118; rJ := 0x10C.
    mmix.write_tetra(0x108, 0xF2000004);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x10C);

    // Nested callee returns immediately: POP 0,0 branches to rJ + 0,
    // landing back at 0x10C.
    mmix.write_tetra(0x118, 0xF8000000);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x10C);

    // The callee's own POP, resumed right there, reads the SAME stale
    // rJ (0x10C) and branches to it again -- the address after its
    // nested PUSHJ, not the address after the top-level PUSHJ (0x104).
    mmix.write_tetra(0x10C, 0xF8000000);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x10C);
    assert_eq!(mmix.call_depth(), 0);
}

/// A callee that saves `rJ` before its nested call and restores it
/// after, before its own `POP`, returns correctly to its own caller.
#[test]
fn test_pop_returns_correctly_when_rj_is_saved_and_restored() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    // Top-level caller: PUSHJ $0,2 -> callee at 0x108; rJ := 0x104.
    mmix.write_tetra(0x100, 0xF2000002);
    assert!(mmix.execute_instruction());

    // Callee saves rJ: GET $1,rJ.
    mmix.write_tetra(0x108, 0xFE010004);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x104);

    // Callee makes a nested call: PUSHJ $2,3 -- the hole must clear
    // rJ's stash at $1, or the slide swallows it into the nested
    // callee's own $0 -- nested callee at 0x118; rJ := 0x110.
    mmix.write_tetra(0x10C, 0xF2020003);
    assert!(mmix.execute_instruction());

    // Nested callee returns immediately: POP 0,0 branches to rJ + 0,
    // landing back at 0x110.
    mmix.write_tetra(0x118, 0xF8000000);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x110);

    // Callee restores rJ: PUT rJ,$1.
    mmix.write_tetra(0x110, 0xF6040001);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);

    // Callee's own POP now branches to the address after the
    // top-level PUSHJ, not the nested one.
    mmix.write_tetra(0x114, 0xF8010000);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x104);
    assert_eq!(mmix.call_depth(), 0);
}

#[test]
fn test_pushj() {
    let mut mmix = MMix::new();
    // PUSHJ $0, 0, 10 - Push and jump to relative offset 10
    mmix.write_tetra(0, 0xF200000A); // PUSHJ $0,0,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
    assert_eq!(mmix.get_special(SpecialReg::RJ), 4); // Return address saved
}

#[test]
fn test_pushjb() {
    let mut mmix = MMix::new();
    mmix.set_pc(100);
    // YZ = 0xFFFB is 0xFFFB - 65536 = -5 tetras.
    mmix.write_tetra(100, 0xF300FFFB); // PUSHJB $0,0xFFFB
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 80); // PC = 100 - 5*4 = 80
    assert_eq!(mmix.get_special(SpecialReg::RJ), 104); // Return address saved
}

#[test]
fn pop_returns_a_callee_computed_remainder_to_the_hole() {
    // Each case: dividend, and its Euclidean remainder mod 100.
    let cases = [
        (42, 42),
        (142, 42),
        (-58, 42),
        (-194, 6),
        (0, 0),
        (100, 0),
        (-100, 0),
        (-1, 99),
    ];
    for (dividend, expected) in cases {
        let source = format!(
            "\
\tLOC\t#100
Main\tSETI\t$1,{dividend}
\tSET\t$2,100
\tPUSHJ\t$0,RemEuclid
\tSET\t$255,$0
\tTRAP\t0,Halt,0
RemEuclid\tDIV\t$2,$0,$1
\tMUL\t$3,$2,$1
\tSUB\t$0,$0,$3
\tBNN\t$0,Done
\tADDU\t$0,$0,$1
Done\tPOP\t1,0
"
        );
        assert_eq!(run_to_halt(&source), expected, "{dividend} mod 100");
    }
}

#[test]
fn test_pop_frame_yz_above_half_the_field_resumes_forward() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.write_tetra(0x100, 0xF2000004); // PUSHJ $0,4 -> 0x110, rJ = 0x104
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);
    // POP resumes at rJ + 4*YZ, unsigned: 0x8000 is forward, not -32768.
    mmix.write_tetra(0x110, 0xF8008000); // POP 0,0x8000
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x104 + 32768 * 4);
}

#[test]
fn test_pop_without_a_frame_resumes_forward_from_rj() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RJ, 0x200);
    mmix.write_tetra(0x100, 0xF8008000); // POP 0,0x8000
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x200 + 32768 * 4);
}

/// `SAVE`'s own X < rG rejection, and proof that `writes_general_register_x`'s
/// pre-claim (real for every other destination-writing opcode) is
/// skipped for `SAVE`: were it not, claiming a marginal $45 here would
/// raise rL and zero $40 before this arm ever ran.
#[test]
fn test_save_rejects_a_destination_below_rg_leaving_the_machine_unchanged() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(40, 0xDEAD); // global while rG = 32
    mmix.set_special(SpecialReg::RG, 50);
    mmix.set_special(SpecialReg::RL, 3);

    // SAVE $45,0 - $45 < rG (50): a local, rejected.
    mmix.write_tetra(0, 0xFA2D0000);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_special(SpecialReg::RL), 3, "rL is untouched");
    assert_eq!(
        mmix.get_register(40),
        0xDEAD,
        "the destination-rise pre-claim never ran"
    );
    assert_eq!(mmix.get_register(45), 0, "the rejected SAVE wrote nothing");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("SAVE"));
    assert_eq!(mmix.get_exit_code(), 1);
}

/// VAL-1: `SAVE`'s Y must be zero.
#[test]
fn test_save_y_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(0, 0x1234); // a local, must survive

    // SAVE $60,1,0 -- Y=1 must be zero.
    mmix.write_tetra(0, 0xFA3C0100);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_register(0), 0x1234, "SAVE touched nothing");
    assert_eq!(mmix.get_register(60), 0, "SAVE touched nothing");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "SAVE Y=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000000"
    );
}

/// VAL-1: `SAVE`'s Z must be zero, checked after Y.
#[test]
fn test_save_z_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(0, 0x1234); // a local, must survive

    // SAVE $255,0,1 -- Z=1 must be zero.
    mmix.write_tetra(0, 0xFAFF0001);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_register(0), 0x1234, "SAVE touched nothing");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "SAVE Z=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000000"
    );
}

/// VAL-1: `UNSAVE`'s X must be zero.
#[test]
fn test_unsave_x_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(0, 0xFEED); // survives iff UNSAVE never runs

    // UNSAVE 1,0,$255 -- X=1 must be zero.
    mmix.write_tetra(0, 0xFB0100FF);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_register(0), 0xFEED, "UNSAVE touched nothing");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "UNSAVE X=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000000"
    );
}

#[test]
fn test_pop_rg_guard_preserves_a_global_set_after_a_mid_call_shrink() {
    // Caller has rG=40, rL=38 and calls PUSHJ $36; the callee shrinks
    // rG to 32 with PUT rG, then sets the newly-global $34 before
    // POP 0,0. Without the rG guard, POP would restore the caller's
    // stale $34 from the spilled frame and clobber the callee's global
    // write.
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RG, 40);
    mmix.set_special(SpecialReg::RL, 38);

    // PUSHJ $36, +1
    mmix.write_tetra(0x100, 0xF2240001);
    assert!(mmix.execute_instruction());

    // PUTI rG,32
    mmix.write_tetra(0x104, 0xF7130020);
    assert!(mmix.execute_instruction());

    // SETL $34,999
    mmix.write_tetra(0x108, 0xE32203E7);
    assert!(mmix.execute_instruction());

    // POP 0,0
    mmix.write_tetra(0x10C, 0xF8000000);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_register(34), 999);
    assert_eq!(mmix.get_special(SpecialReg::RG), 32);
    assert_eq!(mmix.get_special(SpecialReg::RL), 32);
}

#[test]
fn test_pop() {
    let mut mmix = MMix::new();
    // Set return address in rJ
    mmix.set_special(SpecialReg::RJ, 200);
    // POP 0, 0 - Return to address in rJ
    mmix.write_tetra(0, 0xF8000000); // POP 0,0,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 200); // PC = rJ value
}

#[test]
fn test_save() {
    let mut mmix = MMix::new();
    // SAVE $40,0 - $40 is global while rG = 32.
    mmix.write_tetra(0, 0xFA280000); // SAVE $40,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_unsave() {
    let mut mmix = MMix::new();
    // UNSAVE 0,$1 - $1 holds 0, so the packed octa read from address 0
    // is all zero: a packed rG of 0 is outside 32..=255 and rejects.
    mmix.write_tetra(0, 0xFB000001); // UNSAVE 0,$1
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0);
    assert_eq!(mmix.get_exit_code(), 1);
}

/// After `SAVE`, the register stack holds — lowest address to highest
/// — the locals, a marker octa of their count, the globals, the
/// twelve specials in `SAVE_SPECIALS` order, and a packed octa of
/// `rG << 56 | rA`. `$X`, `rO`, `rS` and `rL` land where §1 says.
/// Reverting `SAVE` to the old fixed-address format turns every
/// assertion here red.
#[test]
fn test_save_writes_the_documented_layout() {
    let mut mmix = MMix::new();
    mmix.set_register(0, 0x1111);
    mmix.set_register(1, 0x2222);
    mmix.set_register(2, 0x3333); // rL becomes 3
    mmix.set_special(SpecialReg::RG, 250); // six globals: $250..$255
    for i in 250u8..=255 {
        mmix.set_register(i, 0x9000 + i as u64);
    }
    for (i, reg) in SAVE_SPECIALS.iter().enumerate() {
        mmix.set_special(*reg, 0x7000 + i as u64);
    }
    mmix.set_special(SpecialReg::RA, 0x2A);
    let ro_before = mmix.get_special(SpecialReg::RO);

    // SAVE $250,0 - $250 is global (== rG).
    mmix.write_tetra(0, 0xFAFA0000);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.read_octa(ro_before), 0x1111);
    assert_eq!(mmix.read_octa(ro_before + 8), 0x2222);
    assert_eq!(mmix.read_octa(ro_before + 16), 0x3333);
    let marker_addr = ro_before + 24;
    assert_eq!(mmix.read_octa(marker_addr), 3, "the marker holds rL");

    let globals_base = marker_addr + 8;
    for i in 0u64..6 {
        assert_eq!(
            mmix.read_octa(globals_base + i * 8),
            0x9000 + 250 + i,
            "global {}",
            250 + i
        );
    }

    let specials_base = globals_base + 6 * 8;
    for i in 0..SAVE_SPECIALS.len() as u64 {
        assert_eq!(
            mmix.read_octa(specials_base + i * 8),
            0x7000 + i,
            "special at index {i}"
        );
    }

    let packed_addr = specials_base + (SAVE_SPECIALS.len() as u64) * 8;
    assert_eq!(mmix.read_octa(packed_addr), (250u64 << 56) | 0x2A);

    assert_eq!(
        mmix.get_register(250),
        packed_addr,
        "$X holds the packed octa's address"
    );
    assert_eq!(mmix.get_special(SpecialReg::RO), packed_addr + 8);
    assert_eq!(mmix.get_special(SpecialReg::RS), packed_addr + 8);
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);
}

/// `SAVE`, clobber every register class, `UNSAVE`: everything lands
/// back exactly where it was, and `rO = rS` returns to where `SAVE`
/// found them. Dropping any restoration in `UNSAVE`'s arm turns one of
/// these assertions red.
#[test]
fn test_save_unsave_round_trips_every_register_class() {
    let mut mmix = MMix::new();
    mmix.set_register(0, 0x1111); // local
    mmix.set_register(1, 0x2222); // local
    mmix.set_register(60, 0x3333); // global
    mmix.set_special(SpecialReg::RJ, 0x4444);
    mmix.set_special(SpecialReg::RM, 0x5555);
    mmix.set_special(SpecialReg::RA, 0x2A);
    let rl_before = mmix.get_special(SpecialReg::RL);
    let ro_before = mmix.get_special(SpecialReg::RO);

    // SAVE $70,0 - $70 is global.
    mmix.write_tetra(0, 0xFA460000);
    assert!(mmix.execute_instruction());

    // Clobber every register class SAVE just captured.
    mmix.set_register(0, 0);
    mmix.set_register(1, 0);
    mmix.set_register(60, 0);
    mmix.set_special(SpecialReg::RJ, 0);
    mmix.set_special(SpecialReg::RM, 0);
    mmix.set_special(SpecialReg::RA, 0);
    mmix.set_special(SpecialReg::RG, 200);
    mmix.set_special(SpecialReg::RL, 0);

    // UNSAVE 0,$70.
    mmix.write_tetra(4, 0xFB000046);
    assert!(mmix.execute_instruction());

    assert_eq!(mmix.get_register(0), 0x1111);
    assert_eq!(mmix.get_register(1), 0x2222);
    assert_eq!(mmix.get_register(60), 0x3333);
    assert_eq!(mmix.get_special(SpecialReg::RJ), 0x4444);
    assert_eq!(mmix.get_special(SpecialReg::RM), 0x5555);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0x2A);
    assert_eq!(mmix.get_special(SpecialReg::RG), 32);
    assert_eq!(mmix.get_special(SpecialReg::RL), rl_before);
    assert_eq!(mmix.get_special(SpecialReg::RO), ro_before);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro_before);
}

/// A `SAVE`/`UNSAVE` pair inside a call is transparent to the call
/// itself: the enclosing `PUSHJ`/`POP` retract exactly as if the pair
/// had never run.
#[test]
fn test_save_unsave_inside_a_call_leaves_the_caller_frame_intact() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(0, 888); // caller's own local, must survive the call
    mmix.set_special(SpecialReg::RL, 3); // $0..$2 real locals; $2 is PUSHJ's hole
    let ro_before_call = mmix.get_special(SpecialReg::RO);

    // PUSHJ $2,+1 pushes $0,$1 and the hole ($2); the callee starts
    // with rL = 0.
    mmix.write_tetra(0x100, 0xF2020001);
    assert!(mmix.execute_instruction());
    let ro_in_callee = mmix.get_special(SpecialReg::RO);
    assert_eq!(mmix.get_special(SpecialReg::RL), 0);

    // Callee's own local, then SAVE $40,0 ($40 is global).
    mmix.set_register(0, 0xABC);
    mmix.write_tetra(0x104, 0xFA280000);
    assert!(mmix.execute_instruction());

    // Clobber everything SAVE just captured.
    mmix.set_register(0, 0xDEAD);
    mmix.set_special(SpecialReg::RM, 0xBAD);

    // UNSAVE 0,$40.
    mmix.write_tetra(0x108, 0xFB000028);
    assert!(mmix.execute_instruction());

    assert_eq!(
        mmix.get_register(0),
        0xABC,
        "the callee's own local round-trips"
    );
    assert_eq!(
        mmix.get_special(SpecialReg::RM),
        0,
        "rM round-trips to its pre-SAVE value"
    );
    assert_eq!(mmix.get_special(SpecialReg::RO), ro_in_callee);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro_in_callee);
    assert_eq!(
        mmix.get_special(SpecialReg::RL),
        1,
        "the callee's own single local"
    );

    // POP 0,0 returns to the caller.
    mmix.write_tetra(0x10C, 0xF8000000);
    assert!(mmix.execute_instruction());

    assert_eq!(
        mmix.get_register(0),
        888,
        "caller's $0, pushed by PUSHJ, survives the call"
    );
    assert_eq!(mmix.get_special(SpecialReg::RO), ro_before_call);
    assert_eq!(mmix.get_special(SpecialReg::RS), ro_before_call);
    assert_eq!(mmix.call_depth(), 0);
}

/// The register-stack review's reproduction: a `SAVE` before a call,
/// a nested `PUSHJ`, then an `UNSAVE` inside the callee that rewinds
/// `rO` clear past the frame the `PUSHJ` just opened. Reintroducing a
/// counter `POP` trusts over `rO` turns this test red: it would still
/// think the frame was open and read a hole from dead memory instead
/// of taking the fallback `rO` now calls for.
#[test]
fn test_call_depth_and_pop_follow_ro_through_save_pushj_unsave() {
    let mut mmix = MMix::new();
    let base = mmix.get_special(SpecialReg::RO);
    assert_eq!(mmix.call_depth(), 0);

    // SAVE $40,0 at top level -- a context, not a frame.
    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFA280000);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 0, "a SAVE context is not a frame");

    // PUSHJ $0,+1 opens a frame inside what becomes the callee.
    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF2000001);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 1, "PUSHJ opened one frame");

    // UNSAVE 0,$40 inside the callee rewinds rO past that frame, back
    // to where SAVE found it -- the frame PUSHJ opened is now dead
    // memory below rO.
    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFB000028);
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RO),
        base,
        "UNSAVE rewinds rO to where SAVE found it"
    );
    assert_eq!(
        mmix.call_depth(),
        0,
        "rO names the top level again; the dead frame does not count"
    );

    // POP 1,0 acts on what rO now names: the top level, so it takes
    // the no-frame fallback, branching via rJ -- restored by UNSAVE
    // to 0, its value when SAVE captured it. The fallback touches no
    // register or memory, so rO and rL, unlike a real pop, do not move.
    let rl_before_pop = mmix.get_special(SpecialReg::RL);
    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF8010000);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0, "POP took the no-frame fallback");
    assert_eq!(
        mmix.get_special(SpecialReg::RO),
        base,
        "the fallback leaves rO alone"
    );
    assert_eq!(
        mmix.get_special(SpecialReg::RL),
        rl_before_pop,
        "the fallback leaves rL alone"
    );
    assert_eq!(mmix.call_depth(), 0);
}

/// A `SAVE`/`UNSAVE` pair inside a call leaves `call_depth` at the
/// call's own count throughout: the context never counts as a second
/// frame. Making `call_depth` count a `SAVE` context as a frame turns
/// the middle assertion here red (2, not 1).
#[test]
fn test_call_depth_reads_one_across_a_save_inside_a_call() {
    let mut mmix = MMix::new();
    assert_eq!(mmix.call_depth(), 0);

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF2000001); // PUSHJ $0,+1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 1);

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFA280000); // SAVE $40,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 1, "the SAVE context does not count");

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFB000028); // UNSAVE 0,$40
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 1, "still just the one open call");
}

/// Three nested `PUSHJ`s deep and back, `call_depth` 0→3→0, with a
/// `SAVE`/`UNSAVE` pair at depth 2 in between.
#[test]
fn test_call_depth_nests_three_deep_with_a_save_unsave_pair_at_depth_two() {
    let mut mmix = MMix::new();
    assert_eq!(mmix.call_depth(), 0);

    for depth in 1..=2u64 {
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF2000001); // PUSHJ $0,+1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), depth as usize);
    }

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFA280000); // SAVE $40,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 2);

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFB000028); // UNSAVE 0,$40
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 2);

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF2000001); // PUSHJ $0,+1 -- depth 3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 3);

    for depth in (0..=2u64).rev() {
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF8000000); // POP 0,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), depth as usize);
    }
}

/// `POP` does not recognize a `SAVE` context: at top level after a
/// bare `SAVE`, it reads the packed octa's low byte as a real hole
/// count and retracts `rO` accordingly, rather than taking the
/// no-frame fallback (which would leave `rO` untouched).
#[test]
fn test_pop_after_a_bare_save_reads_the_packed_octas_low_byte_as_a_hole() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 5); // packed octa's low byte becomes 5

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xFA280000); // SAVE $40,0
    assert!(mmix.execute_instruction());
    let ro_after_save = mmix.get_special(SpecialReg::RO);
    assert!(ro_after_save > STACK_SEGMENT_START);

    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF8000000); // POP 0,0
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RO),
        ro_after_save - 48,
        "rO retracted by 8*(5+1), the packed octa's low byte read as a hole"
    );
}

/// `rO` at or below the stack base — the fresh-machine case, and a
/// forged one further below, only reachable through `set_special` —
/// both take `POP`'s no-frame fallback, and `call_depth` reads 0
/// without looping either way.
#[test]
fn test_pop_and_call_depth_at_or_below_the_base_take_the_fallback() {
    let mut mmix = MMix::new();
    assert_eq!(mmix.get_special(SpecialReg::RO), STACK_SEGMENT_START);
    assert_eq!(mmix.call_depth(), 0);

    mmix.set_special(SpecialReg::RJ, 0x200);
    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF8008000); // POP 0,0x8000
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_pc(),
        0x200 + 32768 * 4,
        "at the base: the fallback fired"
    );

    mmix.set_special(SpecialReg::RO, STACK_SEGMENT_START - 8);
    assert_eq!(mmix.call_depth(), 0);
    mmix.set_special(SpecialReg::RJ, 0x300);
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xF8000000); // POP 0,0
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_pc(),
        0x300,
        "below the base: the fallback fired too"
    );
}

/// A misaligned `rO` — only reachable through a forged `set_special`,
/// since every legal instruction keeps it a multiple of 8 — halts
/// `POP` with a diagnostic and leaves the machine unchanged, since the
/// reference would raise a protection fault checksmix has no vector
/// for. `call_depth` stops at 0 without looping.
#[test]
fn test_pop_halts_on_a_misaligned_ro() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RO, STACK_SEGMENT_START + 3);
    mmix.set_register(0, 0xFEED);

    assert_eq!(
        mmix.call_depth(),
        0,
        "the walk stops at once on a misaligned rO"
    );

    mmix.write_tetra(0, 0xF8000000); // POP 0,0
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0, "no PC advance on a halt");
    assert_eq!(mmix.get_register(0), 0xFEED, "no register change on a halt");
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rO"));
    assert_eq!(mmix.get_exit_code(), 1);
}

/// An `rO` above the register-stack segment — only reachable through a
/// forged `set_special` — halts `POP` the same way, and `call_depth`
/// again stops at once without looping.
#[test]
fn test_pop_halts_on_an_ro_above_the_stack_segment() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RO, 0x8000000000000000);
    mmix.set_register(0, 0xFEED);

    assert_eq!(
        mmix.call_depth(),
        0,
        "the walk stops at once outside the segment"
    );

    mmix.write_tetra(0, 0xF8000000); // POP 0,0
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0);
    assert_eq!(mmix.get_register(0), 0xFEED);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rO"));
    assert_eq!(mmix.get_exit_code(), 1);
}

/// The review's non-forged reproduction: a real top-level `SAVE $40,0`,
/// then an ordinary `write_octa` overwrites the marker octa `SAVE`
/// wrote with a local count chosen so the walk's own arithmetic maps
/// `locals_base` back to `ro` itself -- a fixed point, no forged `rO`
/// anywhere. Without the saved-local-count bound in
/// `save_context_layout`, `call_depth` recomputes this same address
/// forever and the test never returns; with it, the count (far above
/// the saved `rG`) is rejected and the walk stops at once.
#[test]
fn test_call_depth_does_not_loop_on_a_marker_mapping_back_to_itself() {
    let mut mmix = MMix::new();

    // SAVE $40,0 at top level.
    mmix.write_tetra(0, 0xFA280000);
    assert!(mmix.execute_instruction());
    let packed_addr = mmix.get_register(40);
    let ro = mmix.get_special(SpecialReg::RO);
    assert_eq!(ro, packed_addr + 8);

    let global_count = 256u64 - 32; // rG = 32 at SAVE time
    let marker_addr = packed_addr - (SAVE_SPECIALS.len() as u64) * 8 - global_count * 8 - 8;

    // Solve for a count with locals_base == marker_addr - count*8 == ro.
    let diff = marker_addr.wrapping_sub(ro);
    assert_eq!(diff % 8, 0, "sanity: layout offsets are all multiples of 8");
    mmix.write_octa(marker_addr, diff / 8);

    assert_eq!(
        mmix.call_depth(),
        0,
        "the corrupted local count exceeds the saved rG; the walk stops \
             instead of looping back to where it started"
    );
}

/// A `SAVE` executed with a nonzero `rL` and `rG != 32` -- the review's
/// coverage gap. `call_depth` must step over the whole context, globals
/// and locals alike, using the saved (not the machine's current) `rG`.
#[test]
fn test_call_depth_walks_a_save_context_with_nonzero_rl_and_rg_ne_32() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RG, 50);
    mmix.set_special(SpecialReg::RL, 3);
    mmix.set_register(0, 0x111);
    mmix.set_register(1, 0x222);
    mmix.set_register(2, 0x333);

    // SAVE $60,0 -- $60 is global (rG=50).
    mmix.write_tetra(0, 0xFA3C0000);
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.call_depth(),
        0,
        "the context alone, locals and all, is not a frame"
    );

    // PUSHJ $0,+1 opens a frame above the context.
    let pc = mmix.get_pc();
    mmix.write_tetra(pc, 0xF2000001);
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.call_depth(),
        1,
        "the walk steps over the saved rL=3, rG=50 context uncounted \
             and still finds the one PUSHJ frame beneath it"
    );
}

/// `SAVE`'s Y and Z, and `UNSAVE`'s X and Y, are must-be-zero fields
/// (VAL-1): a nonzero value there is an illegal-instruction interrupt,
/// the machine left unchanged.
#[test]
fn test_save_and_unsave_reject_their_must_be_zero_fields() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(0, 0x1234); // a local
    mmix.set_register(50, 0xABCD); // a global

    // SAVE $60,255,255 - Y and Z both nonzero; Y is named first.
    mmix.write_tetra(0, 0xFA3CFFFF);
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_register(0), 0x1234, "SAVE touched nothing");
    assert_eq!(mmix.get_register(50), 0xABCD, "SAVE touched nothing");
    assert_eq!(mmix.get_exit_code(), 1);

    // UNSAVE 255,255,$60 - X and Y both nonzero; X is named first.
    mmix.set_pc(4);
    mmix.write_tetra(4, 0xFBFFFF3C);
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_exit_code(), 1);

    assert_eq!(handle.diagnostics().len(), 2);
    assert!(handle.diagnostics()[0].contains("SAVE Y=255"));
    assert!(handle.diagnostics()[1].contains("UNSAVE X=255"));
}

/// Two independent machines run the same `SAVE`; `$X` lands on the same
/// address in both. The old fixed-address format shared one process-
/// global counter across every `MMix`, so two machines (or two tests
/// running in parallel) handed out interleaved addresses instead.
#[test]
fn test_save_is_deterministic_across_instances() {
    let mut a = MMix::new();
    let mut b = MMix::new();

    a.write_tetra(0, 0xFA280000); // SAVE $40,0
    b.write_tetra(0, 0xFA280000);
    assert!(a.execute_instruction());
    assert!(b.execute_instruction());

    assert_eq!(a.get_register(40), b.get_register(40));
}

/// `UNSAVE` rejects a packed rG below 32 (above 255 cannot occur — it
/// is one byte), before touching any register or memory.
#[test]
fn test_unsave_rejects_a_packed_rg_below_32() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(0, 0xFEED); // survives iff UNSAVE never runs

    let context = 0x2000u64;
    mmix.write_octa(context, 31u64 << 56); // packed rG = 31, rA = 0
    mmix.set_register(60, context);

    // UNSAVE 0,$60
    mmix.write_tetra(0, 0xFB00003C);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0);
    assert_eq!(mmix.get_register(0), 0xFEED);
    assert_eq!(mmix.get_special(SpecialReg::RG), 32);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rG"));
    assert_eq!(mmix.get_exit_code(), 1);
}

/// `UNSAVE` rejects a packed rA above `RA_MAX`.
#[test]
fn test_unsave_rejects_a_packed_ra_above_ra_max() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(0, 0xFEED);

    let context = 0x2000u64;
    mmix.write_octa(context, (32u64 << 56) | (RA_MAX + 1));
    mmix.set_register(60, context);

    // UNSAVE 0,$60
    mmix.write_tetra(0, 0xFB00003C);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0);
    assert_eq!(mmix.get_register(0), 0xFEED);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("rA"));
    assert_eq!(mmix.get_exit_code(), 1);
}

/// `UNSAVE` rejects a saved local count greater than the packed rG. A
/// real `SAVE` gives a well-formed context; only its marker is
/// corrupted.
#[test]
fn test_unsave_rejects_a_saved_local_count_above_the_packed_rg() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_special(SpecialReg::RL, 0);

    // SAVE $40,0 with no locals: the marker holds 0.
    mmix.write_tetra(0, 0xFA280000);
    assert!(mmix.execute_instruction());
    let context = mmix.get_register(40);

    let global_count = 256u64 - 32; // rg = 32
    let marker_addr = context - (SAVE_SPECIALS.len() as u64) * 8 - global_count * 8 - 8;
    assert_eq!(
        mmix.read_octa(marker_addr),
        0,
        "sanity: 0 locals were saved"
    );
    mmix.write_octa(marker_addr, 33); // 33 > rG (32)

    mmix.set_register(0, 0xFEED); // survives iff UNSAVE never runs

    // UNSAVE 0,$40
    mmix.write_tetra(4, 0xFB000028);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 4);
    assert_eq!(mmix.get_register(0), 0xFEED);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("local count"));
    assert_eq!(mmix.get_exit_code(), 1);
}
