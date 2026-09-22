//! Floating-point opcodes, rounding modes, and NaN handling.

use super::*;

#[test]
fn test_float_to_fix_overflow_trips_when_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x30, 0xFD000000); // W's vector loaded
    mmix.set_special(SpecialReg::RA, RA_W << 8); // enable W only
    mmix.set_pc(0x100);
    // A finite overflow, not +∞: FIX copies an infinite operand through
    // with I alone and never raises W for it.
    mmix.set_register(2, 1e20_f64.to_bits());
    mmix.write_tetra(0x100, 0x05010002); // FIX $1,0,$2

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x30);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_W, 0);
}

#[test]
fn test_floating_overflow_trips_and_the_unenabled_inexact_still_sets_its_event_bit() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x50, 0xFD000000); // O's vector loaded
    mmix.set_special(SpecialReg::RA, RA_O << 8); // enable O only, not X
    mmix.set_pc(0x100);
    mmix.set_register(2, f64::MAX.to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3 overflows (raises O and X)

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x50);
    let ra = mmix.get_special(SpecialReg::RA);
    assert_eq!(ra & RA_O, 0, "O tripped: its own event bit stays clear");
    assert_eq!(
        ra & RA_X,
        RA_X,
        "X was raised but not enabled: it still sets its event bit"
    );
}

#[test]
fn test_floating_underflow_trips_when_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x60, 0xFD000000); // U's vector loaded
    mmix.set_special(SpecialReg::RA, RA_U << 8); // enable U only
    mmix.set_pc(0x100);
    mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
    mmix.set_register(3, f64::MIN_POSITIVE.to_bits());
    mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3 underflows to zero

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x60);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_U, 0);
}

#[test]
fn test_floating_divide_by_zero_trips_when_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x70, 0xFD000000); // Z's vector loaded
    mmix.set_special(SpecialReg::RA, RA_Z << 8); // enable Z only
    mmix.set_pc(0x100);
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0x100, 0x14010203); // FDIV $1,$2,$3

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x70);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_Z, 0);
}

#[test]
fn test_floating_inexact_trips_when_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x80, 0xFD000000); // X's vector loaded
    mmix.set_special(SpecialReg::RA, RA_X << 8); // enable X only
    mmix.set_pc(0x100);
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 1e-30f64.to_bits());
    mmix.write_tetra(0x100, 0x04010203); // FADD $1,$2,$3 rounds 1e-30 away

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x80);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
}

// ========== Floating Point Tests ==========

#[test]
fn test_fcmp_less_than() {
    let mut mmix = MMix::new();
    // FCMP $1, $2, $3 - Compare 2.5 < 5.0
    mmix.set_register(2, 2.5f64.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1); // Less than
}

#[test]
fn test_fcmp_greater_than() {
    let mut mmix = MMix::new();
    // FCMP $1, $2, $3 - Compare 10.0 > 3.0
    mmix.set_register(2, 10.0f64.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1); // Greater than
}

#[test]
fn test_fcmp_equal() {
    let mut mmix = MMix::new();
    // FCMP $1, $2, $3 - Compare 7.5 == 7.5
    mmix.set_register(2, 7.5f64.to_bits());
    mmix.set_register(3, 7.5f64.to_bits());
    mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Equal
}

#[test]
fn test_fcmp_unordered() {
    let mut mmix = MMix::new();
    // FCMP computes $X = [$Y > $Z] − [$Y < $Z]; with a NaN operand
    // both brackets are 0, so $X = 0, and I reports the NaN.
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_cmp_cmpu_signed_unsigned_divergence() {
    let mut mmix = MMix::new();
    // $2 has its top bit set: negative as i64, huge as u64. $3 is a
    // small positive value. Signed and unsigned 3-way compare disagree.
    mmix.set_register(2, 0x8000000000000000);
    mmix.set_register(3, 1);
    mmix.write_tetra(0, 0x30010203); // CMP $1,$2,$3
    mmix.write_tetra(4, 0x32040203); // CMPU $4,$2,$3
    assert!(mmix.execute_instruction());
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1); // signed: $2 < $3
    assert_eq!(mmix.get_register(4) as i64, 1); // unsigned: $2 > $3
}

#[test]
fn test_feql() {
    let mut mmix = MMix::new();
    // FEQL $1, $2, $3 - Test 4.0 == 4.0
    mmix.set_register(2, 4.0f64.to_bits());
    mmix.set_register(3, 4.0f64.to_bits());
    mmix.write_tetra(0, 0x03010203); // FEQL $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1); // Equal
}

#[test]
fn test_feql_not_equal() {
    let mut mmix = MMix::new();
    // FEQL $1, $2, $3 - Test 4.0 != 5.0
    mmix.set_register(2, 4.0f64.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x03010203); // FEQL $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Not equal
}

#[test]
fn test_fun() {
    let mut mmix = MMix::new();
    // FUN $1, $2, $3 - Test if unordered
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x02010203); // FUN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1); // Unordered
}

#[test]
fn test_fun_ordered() {
    let mut mmix = MMix::new();
    // FUN $1, $2, $3 - Test if unordered (both normal)
    mmix.set_register(2, 2.0f64.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    mmix.write_tetra(0, 0x02010203); // FUN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Ordered
}

#[test]
fn test_fcmpe() {
    let mut mmix = MMix::new();
    // FCMPE $1, $2, $3 - Compare 5.0 and 5.001 with epsilon 0.01
    mmix.set_special(SpecialReg::RE, 0.01f64.to_bits());
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, 5.001f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Equal within epsilon
}

#[test]
fn test_feqle() {
    let mut mmix = MMix::new();
    // FEQLE $1, $2, $3 - Test equivalence with epsilon
    mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
    mmix.set_register(2, 10.0f64.to_bits());
    mmix.set_register(3, 10.05f64.to_bits());
    mmix.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1); // Equivalent
}

#[test]
fn test_fune() {
    let mut mmix = MMix::new();
    // FUNE $1, $2, $3 - neither operand nor rE is exceptional (no NaN,
    // rE not negative), so FUNE reports 0: it says nothing about
    // proximity, only whether the inputs are exceptional.
    mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
    mmix.set_register(2, 7.0f64.to_bits());
    mmix.set_register(3, 7.3f64.to_bits());
    mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_fadd() {
    let mut mmix = MMix::new();
    // FADD $1, $2, $3 - Add 2.5 + 3.7
    mmix.set_register(2, 2.5f64.to_bits());
    mmix.set_register(3, 3.7f64.to_bits());
    mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 6.2).abs() < 1e-10);
}

#[test]
fn test_fsub() {
    let mut mmix = MMix::new();
    // FSUB $1, $2, $3 - Subtract 10.0 - 3.5
    mmix.set_register(2, 10.0f64.to_bits());
    mmix.set_register(3, 3.5f64.to_bits());
    mmix.write_tetra(0, 0x06010203); // FSUB $1,$2,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 6.5).abs() < 1e-10);
}

#[test]
fn test_fmul() {
    let mut mmix = MMix::new();
    // FMUL $1, $2, $3 - Multiply 4.0 * 2.5
    mmix.set_register(2, 4.0f64.to_bits());
    mmix.set_register(3, 2.5f64.to_bits());
    mmix.write_tetra(0, 0x10010203); // FMUL $1,$2,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 10.0).abs() < 1e-10);
}

#[test]
fn test_fdiv() {
    let mut mmix = MMix::new();
    // FDIV $1, $2, $3 - Divide 15.0 / 3.0
    mmix.set_register(2, 15.0f64.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203); // FDIV $1,$2,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 5.0).abs() < 1e-10);
}

#[test]
fn test_frem() {
    let mut mmix = MMix::new();
    // IEEE 754 remainder of 7.5 by 2.0: 7.5/2 = 3.75 → round-half-even = 4
    // → r = 7.5 - 4·2 = -0.5. (Rust's `%` would give 1.5.)
    mmix.set_register(2, 7.5f64.to_bits());
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.write_tetra(0, 0x16010203);
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result + 0.5).abs() < 1e-10, "got {}", result);
}

#[test]
fn test_frem_zero_divisor_nan() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x16010203);
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_nan());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fsqrt() {
    let mut mmix = MMix::new();
    // FSQRT $1, $3 - Square root of 16.0
    mmix.set_register(3, 16.0f64.to_bits());
    mmix.write_tetra(0, 0x15010003); // FSQRT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 4.0).abs() < 1e-10);
}

#[test]
fn test_fint() {
    let mut mmix = MMix::new();
    // FINT $1, $3 - Round 3.7 to nearest integer
    mmix.set_register(3, 3.7f64.to_bits());
    mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 4.0).abs() < 1e-10);
}

#[test]
fn test_fix() {
    let mut mmix = MMix::new();
    // Default rA mode 0 = ROUND_NEAR: 42.9 → 43.
    mmix.set_register(3, 42.9f64.to_bits());
    mmix.write_tetra(0, 0x05010003); // FIX $1,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 43);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_fix_trunc_mode() {
    let mut mmix = MMix::new();
    // rA mode 1 = ROUND_OFF (toward zero): 42.9 → 42.
    mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT);
    mmix.set_register(3, 42.9f64.to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 42);
}

#[test]
fn test_fix_negative() {
    let mut mmix = MMix::new();
    // Mode 0 = NEAR rounds -17.8 → -18.
    mmix.set_register(3, (-17.8f64).to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -18);
}

#[test]
fn test_fix_nan_raises_i_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(3, f64::NAN.to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    let ra = mmix.get_special(SpecialReg::RA);
    assert_eq!(ra & RA_W, 0);
    assert_ne!(ra & RA_I, 0);
    assert_eq!(mmix.get_register(1), f64::NAN.to_bits());
}

#[test]
fn test_fixu() {
    let mut mmix = MMix::new();
    // Mode 0 = NEAR with round-half-to-even: 99.5 → 100 (100 is even).
    mmix.set_register(3, 99.5f64.to_bits());
    mmix.write_tetra(0, 0x07010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 100);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_flot() {
    let mut mmix = MMix::new();
    // FLOT $1, $3 - Convert signed integer 42 to float
    mmix.set_register(3, 42);
    mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 42.0).abs() < 1e-10);
}

#[test]
fn test_flot_negative() {
    let mut mmix = MMix::new();
    // FLOT $1, $3 - Convert signed integer -100 to float
    mmix.set_register(3, (-100i64) as u64);
    mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - (-100.0)).abs() < 1e-10);
}

#[test]
fn test_floti() {
    let mut mmix = MMix::new();
    // FLOTI $1, $0, 100 - Convert immediate signed 100 to float (Y=$0 is rounding mode, Z=100 is value)
    mmix.write_tetra(0, 0x09010064); // FLOTI $1,$0,100 (X=01, Y=00, Z=64)
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 100.0).abs() < 1e-10);
}

#[test]
fn test_floti_immediate_is_unsigned() {
    let mut mmix = MMix::new();
    // FLOTI $1,$0,255 - Z is an unsigned byte, like every immediate
    // operand, so 0xFF converts to 255.0, not -1.0.
    mmix.write_tetra(0, 0x090100FF);
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 255.0).abs() < 1e-10);
}

#[test]
fn test_flotu() {
    let mut mmix = MMix::new();
    // FLOTU $1, $3 - Convert unsigned integer to float
    mmix.set_register(3, 1000);
    mmix.write_tetra(0, 0x0A010003); // FLOTU $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 1000.0).abs() < 1e-10);
}

#[test]
fn test_flotui() {
    let mut mmix = MMix::new();
    // FLOTUI $1, $0, 244 - Convert immediate unsigned 244 to float (Y=$0, Z=244)
    mmix.write_tetra(0, 0x0B0100F4); // FLOTUI $1,$0,244 (X=01, Y=00, Z=F4=244)
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 244.0).abs() < 1e-10);
}

#[test]
fn test_sflot() {
    let mut mmix = MMix::new();
    // SFLOT $1, $3 - Convert signed to short float (f32 precision)
    mmix.set_register(3, 123);
    mmix.write_tetra(0, 0x0C010003); // SFLOT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 123.0).abs() < 1e-5);
}

#[test]
fn test_sfloti() {
    let mut mmix = MMix::new();
    // SFLOTI $1, 64 - Convert immediate signed to short float
    mmix.write_tetra(0, 0x0D010040); // SFLOTI $1,64 (YZ=0x0040)
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 64.0).abs() < 1e-5);
}

#[test]
fn test_sflotu() {
    let mut mmix = MMix::new();
    // SFLOTU $1, $3 - Convert unsigned to short float
    mmix.set_register(3, 777);
    mmix.write_tetra(0, 0x0E010003); // SFLOTU $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 777.0).abs() < 1e-5);
}

#[test]
fn test_sflotui() {
    let mut mmix = MMix::new();
    // SFLOTUI $1, 255 - Convert immediate unsigned to short float
    mmix.write_tetra(0, 0x0F0100FF); // SFLOTUI $1,255 (YZ=0x00FF)
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 255.0).abs() < 1e-5);
}

#[test]
fn test_fint_round_near() {
    let mut mmix = MMix::new();
    // FINT $1, $0, $3 - Integerize with ROUND_NEAR mode
    mmix.set_special(SpecialReg::RA, 0 << RA_ROUND_SHIFT); // Round mode 0 = ROUND_NEAR
    mmix.set_register(3, 3.7f64.to_bits());
    mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 4.0).abs() < 1e-10);
}

#[test]
fn test_fint_round_off() {
    let mut mmix = MMix::new();
    // Mode 1 = ROUND_OFF (toward zero / truncate).
    mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT);
    mmix.set_register(3, 3.7f64.to_bits());
    mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 3.0).abs() < 1e-10);
}

#[test]
fn test_fint_round_up() {
    let mut mmix = MMix::new();
    // Mode 2 = ROUND_UP (toward +∞).
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT);
    mmix.set_register(3, 3.2f64.to_bits());
    mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 4.0).abs() < 1e-10);
}

#[test]
fn test_fint_round_down() {
    let mut mmix = MMix::new();
    // Mode 3 = ROUND_DOWN (toward -∞).
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT);
    mmix.set_register(3, 3.9f64.to_bits());
    mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result - 3.0).abs() < 1e-10);
}

#[test]
fn test_fint_round_down_negative() {
    let mut mmix = MMix::new();
    // Mode 3 floors -3.2 → -4.0 (toward -∞).
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT);
    mmix.set_register(3, (-3.2f64).to_bits());
    mmix.write_tetra(0, 0x17010003);
    assert!(mmix.execute_instruction());
    let result = f64::from_bits(mmix.get_register(1));
    assert!((result + 4.0).abs() < 1e-10);
}

#[test]
fn test_fint_inexact_flag() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 0 << RA_ROUND_SHIFT);
    mmix.set_register(3, 3.5f64.to_bits());
    mmix.write_tetra(0, 0x17010003);
    assert!(mmix.execute_instruction());
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

// ========== Stack/Sync/Store Tests ==========

#[test]
fn test_stsf() {
    let mut mmix = MMix::new();
    // STSF $1, $2, $3 - Store short float
    let f64_value = std::f64::consts::PI;
    mmix.set_register(1, f64_value.to_bits());
    mmix.set_register(2, 1000);
    mmix.set_register(3, 8);
    mmix.write_tetra(0, 0xB0010203); // STSF $1,$2,$3
    assert!(mmix.execute_instruction());

    let stored_tetra = mmix.read_tetra(1008);
    let f32_value = f32::from_bits(stored_tetra);
    assert!((f32_value - std::f32::consts::PI).abs() < 1e-5);
}

#[test]
fn test_stsfi() {
    let mut mmix = MMix::new();
    // STSFI $1, $2, 16 - Store short float immediate
    let f64_value = std::f64::consts::E;
    mmix.set_register(1, f64_value.to_bits());
    mmix.set_register(2, 2000);
    mmix.write_tetra(0, 0xB1010210); // STSFI $1,$2,16
    assert!(mmix.execute_instruction());

    let stored_tetra = mmix.read_tetra(2016);
    let f32_value = f32::from_bits(stored_tetra);
    assert!((f32_value - std::f32::consts::E).abs() < 1e-5);
}

// ==================== Floating-point: rA flag coverage ====================

#[test]
fn test_fadd_overflow_sets_o() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::MAX.to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_infinite());
    assert!((mmix.get_special(SpecialReg::RA) & RA_O) != 0);
}

#[test]
fn test_fsub_inf_minus_inf_sets_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::INFINITY.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x06010203); // FSUB $1,$2,$3
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_nan());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fmul_zero_times_inf_sets_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0.0f64.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x10010203); // FMUL $1,$2,$3
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_nan());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fdiv_by_zero_sets_z() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203); // FDIV $1,$2,$3
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_infinite());
    let ra = mmix.get_special(SpecialReg::RA);
    assert!((ra & RA_Z) != 0, "rA={:#x}", ra);
}

#[test]
fn test_fdiv_zero_by_zero_sets_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_nan());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fdiv_underflow_sets_u() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
    mmix.set_register(3, 1e16f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    let r = f64::from_bits(mmix.get_register(1));
    assert_eq!(r, 0.0, "expected complete underflow to zero, got {:?}", r);
    assert!((mmix.get_special(SpecialReg::RA) & RA_U) != 0);
}

#[test]
fn test_fsqrt_negative_sets_i() {
    let mut mmix = MMix::new();
    mmix.set_register(3, (-4.0f64).to_bits());
    mmix.write_tetra(0, 0x15010003); // FSQRT $1,$0,$3
    assert!(mmix.execute_instruction());
    assert!(f64::from_bits(mmix.get_register(1)).is_nan());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fsqrt_normal_no_flag() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 9.0f64.to_bits());
    mmix.write_tetra(0, 0x15010003);
    assert!(mmix.execute_instruction());
    assert!((f64::from_bits(mmix.get_register(1)) - 3.0).abs() < 1e-10);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
}

#[test]
fn test_fcmp_nan_sets_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x01010203); // FCMP
    assert!(mmix.execute_instruction());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_feql_nan_no_flag() {
    let mut mmix = MMix::new();
    // FEQL is "quiet" for NaN: returns 0, no I flag.
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x03010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
}

#[test]
fn test_fune_includes_nan() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.001f64.to_bits());
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1);
}

#[test]
fn test_fcmpe_outside_epsilon() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.001f64.to_bits());
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_fcmpe_binade_scaled_radius() {
    // A flat |y-z|<=epsilon test gives -1 (16 > 0.25). 1024's raw
    // exponent field is 1033, so Nε's radius is 0.25 * 2^11 = 512,
    // which covers 1040 (diff 16) and FCMPE reports 0.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.25f64.to_bits());
    mmix.set_register(2, 1024.0f64.to_bits());
    mmix.set_register(3, 1040.0f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_fcmpe_radius_uses_e_minus_1022_not_1023() {
    // 1.0's raw exponent field is 1023, so the radius is
    // 0.5 * 2^(1023-1022) = 1.0, covering the 0.75 gap to 1.75. An
    // off-by-one-binade radius (2^(e-1023) = 0.5) would not.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 1.75f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_fcmpe_denormal_neighborhood_uses_fixed_radius() {
    // $3 is subnormal (raw exponent field 0): Nε's denormal case uses
    // the fixed radius 2^-1021 * ε, not a per-value binade scale, and
    // that radius is far smaller than the gap to $2. A flat
    // |y-z|<=epsilon check would call this pair close (1e-300 <=
    // 1e-10) and report 0; the correct radius does not, and $2 > $3
    // gives +1.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 1e-10f64.to_bits());
    mmix.set_register(2, 1e-300f64.to_bits());
    mmix.set_register(3, f64::from_bits(1).to_bits()); // smallest subnormal
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, 1);
}

#[test]
fn test_fcmpe_infinite_neighborhood_epsilon_at_least_two() {
    // Nε(+∞) is everything when ε≥2, so 5.0 ∈ Nε(+∞) and FCMPE
    // reports 0. A flat |y-z|<=epsilon check sees an infinite
    // difference, never within any finite epsilon, and falls back to
    // the sign compare (5.0 < ∞ ⇒ -1).
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 3.0f64.to_bits());
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_fcmpe_nan_operand_forces_zero_and_raises_invalid() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fcmpe_negative_epsilon_forces_zero_and_raises_invalid() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, (-1.0f64).to_bits());
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, 6.0f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fune_negative_epsilon_is_exceptional() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, (-1.0f64).to_bits());
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0); // FUNE raises nothing
}

#[test]
fn test_fix_y_override_forces_mode_regardless_of_ra() {
    // rA's persistent mode is ROUND_UP (2); Y=1 (ROUND_OFF) must
    // override it: trunc(2.5)=2, not rA's ceil(2.5)=3.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT);
    mmix.set_register(2, 2.5f64.to_bits());
    mmix.write_tetra(0, 0x05010102); // FIX $1,1,$2 (Y=ROUND_OFF)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, 2);
}

#[test]
fn test_fix_two_operand_form_still_honors_ra_mode() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
    mmix.set_register(2, 2.5f64.to_bits());
    mmix.write_tetra(0, 0x05010002); // FIX $1,0,$2 (Y=0, no override)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, 3); // ceil(2.5) per rA's mode
}

#[test]
fn test_y_greater_than_four_halts_with_diagnostic() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.write_tetra(0, 0x0901050A); // FLOTI $1,5,10 (Y=5, illegal)
    let should_continue = mmix.execute_instruction();
    assert!(!should_continue);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("FLOTI"));
    assert!(handle.diagnostics()[0].contains("Y=5"));
}

#[test]
fn test_fsqrt_y_greater_than_four_halts_with_diagnostic() {
    // The Y>4 halt is wired at every rounding-mode read site; FLOTI
    // above pins the FLOT/i2f_conv_ri! site, this pins FSQRT's separate
    // finalize_fp_unop site.
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.write_tetra(0, 0x15010502); // FSQRT $1,5,$2 (Y=5, illegal)
    let should_continue = mmix.execute_instruction();
    assert!(!should_continue);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("FSQRT"));
    assert!(handle.diagnostics()[0].contains("Y=5"));
}

#[test]
fn test_fix_y_two_selects_round_up_not_round_off() {
    // Y=2 (ROUND_UP) must map to rA mode 2, not `Y-1`'s mode 1
    // (ROUND_OFF). rA holds ROUND_OFF already, so the two mappings
    // coincide unless Y's own value (2) is honored: ceil(2.5)=3 under
    // the correct mapping's ROUND_UP, trunc(2.5)=2 under the
    // forbidden Y-1 mapping (indistinguishable from rA's own
    // ROUND_OFF, i.e. Y ignored).
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT); // ROUND_OFF
    mmix.set_register(2, 2.5f64.to_bits());
    mmix.write_tetra(0, 0x05010202); // FIX $1,2,$2 (Y=ROUND_UP)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, 3);
}

#[test]
fn test_fcmpe_denormal_radius_pins_exact_constant() {
    // Both operands are denormal (raw exponent field 0), placing the
    // gap strictly between the off-by-one radius 2^-1022*ε and the
    // correct radius 2^-1021*ε — only the correct constant reports 0.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
    mmix.set_register(2, 1.5e-308f64.to_bits());
    mmix.set_register(3, 1.83e-308f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_fcmpe_denormal_radius_pins_exact_constant_upper_side() {
    // The sibling test above pins the -1021 -> -1022 boundary; this
    // pins the other side. The gap sits strictly between the correct
    // radius 2^-1021*ε and the off-by-one radius 2^-1020*ε, so only
    // the correct constant reports non-zero (not close).
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
    mmix.set_register(2, 1.5e-308f64.to_bits());
    mmix.set_register(3, 2.16752215755216e-308f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_fcmpe_infinite_neighborhood_epsilon_below_one() {
    // Nε(+∞) = {+∞} only when ε < 1: a finite value is never close to
    // +∞ and the ordinary sign compare applies.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_fcmpe_infinite_neighborhood_epsilon_below_one_opposite_infinities() {
    // Nε(+∞) = {+∞} when ε < 1: the entry condition is `u == v`, exact
    // equality, not `u.is_infinite()` — opposite infinities are each
    // infinite but never equal, so they must compare -1, not 0. A
    // mutant widening the entry test to `u.is_infinite()` would wrongly
    // place -∞ in Nε(+∞) here and report 0.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
    mmix.set_register(2, f64::NEG_INFINITY.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_fcmpe_infinite_neighborhood_epsilon_one_to_two() {
    // Nε(+∞) = everything except -∞ when 1 ≤ ε < 2: a finite value is
    // close to +∞, but -∞ itself is not — the "except" half, which a
    // mutation collapsing this branch to unconditional "everything"
    // (ε ≥ 2's behavior) would not catch on the finite vector alone.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 1.5f64.to_bits());
    mmix.set_register(2, 5.0f64.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);

    let mut opposite = MMix::new();
    opposite.set_special(SpecialReg::RE, 1.5f64.to_bits());
    opposite.set_register(2, f64::NEG_INFINITY.to_bits());
    opposite.set_register(3, f64::INFINITY.to_bits());
    opposite.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(opposite.execute_instruction());
    assert_eq!(opposite.get_register(1) as i64, -1);
}

#[test]
fn test_feqle_stronger_than_fcmpe_asymmetric_binade() {
    // 4.0 sits at the start of a binade twice as wide as 3.99's; the
    // gap fits inside 4.0's radius but not inside 3.99's, so
    // 3.99 ∈ Nε(4.0) while 4.0 ∉ Nε(3.99) — FCMPE's OR is satisfied
    // (∼ holds) but FEQLE's AND (≈) is not. An &&-to-|| mutation in
    // FEQLE's arm would survive without this test.
    let mut fcmpe = MMix::new();
    fcmpe.set_special(SpecialReg::RE, 0.002f64.to_bits());
    fcmpe.set_register(2, 3.99f64.to_bits());
    fcmpe.set_register(3, 4.0f64.to_bits());
    fcmpe.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(fcmpe.execute_instruction());
    assert_eq!(fcmpe.get_register(1), 0);

    let mut feqle = MMix::new();
    feqle.set_special(SpecialReg::RE, 0.002f64.to_bits());
    feqle.set_register(2, 3.99f64.to_bits());
    feqle.set_register(3, 4.0f64.to_bits());
    feqle.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
    assert!(feqle.execute_instruction());
    assert_eq!(feqle.get_register(1), 0);
}

#[test]
fn test_feqle_nan_operand_forces_zero_and_raises_invalid() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 5.0f64.to_bits());
    mmix.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fcmpe_reflexive_at_top_binade_zero_epsilon() {
    // f64::MAX's raw exponent field is 2046, the top binade, where
    // `2^(e-1022)` is `2^1024` — unrepresentable, and `0.0 * inf` is
    // NaN. Reflexivity must still hold: a value is always in its own
    // Nε-neighborhood, so FCMPE(v, v) is 0 even with ε = 0.0.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.0f64.to_bits());
    mmix.set_register(2, f64::MAX.to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, 0);
}

#[test]
fn test_feqle_reflexive_at_top_binade_zero_epsilon() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.0f64.to_bits());
    mmix.set_register(2, f64::MAX.to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1);
}

#[test]
fn test_fcmpe_zero_neighborhood_reflexive() {
    // Nε(0) = {0}: zero is always in its own neighborhood, so
    // FCMPE(0.0, 0.0) is 0 for any ε >= 0.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.0f64.to_bits());
    mmix.set_register(2, 0.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
}

#[test]
fn test_fcmpe_zero_neighborhood_excludes_nonzero() {
    // Nε(0) = {0}, the single point, not "anything within ε of 0" —
    // a nonzero value is never in it, however small, and no ε widens
    // that set.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
    mmix.set_register(2, 0.0f64.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_fcmpe_top_binade_radius_stays_finite_not_infinite() {
    // Both operands are huge and finite (top binade), but far enough
    // apart that the correct, finite radius (ε * 2^1024, computed
    // without materializing the unrepresentable literal `2^1024`)
    // does not cover the gap — an `inf`-radius implementation would
    // wrongly call this pair close (0) for any ε > 0.
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RE, 1e-300f64.to_bits());
    mmix.set_register(2, (f64::MAX / 2.0).to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_stsf_rounding_default_near() {
    let mut mmix = MMix::new();
    let v: f64 = 1.0f64 + f32::EPSILON as f64 / 2.0; // exactly half-ULP above 1.0 in f32
    mmix.set_register(1, v.to_bits()); // value to store
    mmix.set_register(2, 0x100); // base address
    mmix.set_register(3, 0); // offset
    // STSF $1,$2,$3
    mmix.write_tetra(0, 0xB0010203);
    assert!(mmix.execute_instruction());
    let stored_bits = mmix.read_tetra(0x100);
    let round_trip = f32::from_bits(stored_bits) as f64;
    // Default round-to-nearest-even rounds to 1.0 exactly (1.0 is even).
    assert_eq!(round_trip, 1.0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_stsf_rounding_up_mode() {
    let mut mmix = MMix::new();
    // Pick a value strictly between two adjacent f32 values.
    let v: f64 = 1.0f64 + (f32::EPSILON as f64) * 0.25;
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
    mmix.set_register(1, v.to_bits());
    mmix.set_register(2, 0x100);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0xB0010203);
    assert!(mmix.execute_instruction());
    let stored = f32::from_bits(mmix.read_tetra(0x100));
    // Result must be >= input.
    assert!((stored as f64) >= v, "{} >= {}", stored, v);
    // And strictly above 1.0 since exact value > 1.0.
    assert!(stored > 1.0);
}

#[test]
fn test_stsf_rounding_down_mode() {
    let mut mmix = MMix::new();
    let v: f64 = 1.0f64 + (f32::EPSILON as f64) * 0.25;
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
    mmix.set_register(1, v.to_bits());
    mmix.set_register(2, 0x100);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0xB0010203);
    assert!(mmix.execute_instruction());
    let stored = f32::from_bits(mmix.read_tetra(0x100));
    // Result must be <= input.
    assert!((stored as f64) <= v);
    // And exactly 1.0 (largest f32 ≤ v).
    assert_eq!(stored, 1.0);
}

#[test]
fn test_stsf_overflow_to_inf_sets_o() {
    let mut mmix = MMix::new();
    // 1e40 overflows f32.
    mmix.set_register(1, 1e40f64.to_bits());
    mmix.set_register(2, 0x100);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0xB0010203);
    assert!(mmix.execute_instruction());
    let stored = f32::from_bits(mmix.read_tetra(0x100));
    assert!(stored.is_infinite() && stored.is_sign_positive());
    assert!((mmix.get_special(SpecialReg::RA) & RA_O) != 0);
}

#[test]
fn test_ieee_remainder_helper() {
    // Direct check of the helper for clarity.
    assert_eq!(MMix::ieee_remainder(7.5, 2.0), -0.5);
    assert_eq!(MMix::ieee_remainder(10.0, 3.0), 1.0);
    assert!(MMix::ieee_remainder(1.0, 0.0).is_nan());
    assert!(MMix::ieee_remainder(f64::INFINITY, 1.0).is_nan());
    assert_eq!(MMix::ieee_remainder(3.0, f64::INFINITY), 3.0);
}

// ==================== Assembler integration: FCMPE/FUNE/FEQLE ====================

#[test]
fn test_assembler_emits_fcmpe() {
    use crate::encode::encode_instruction_bytes;
    use crate::mmixal::MMixInstruction;
    let bytes = encode_instruction_bytes(&MMixInstruction::FCMPE(1, 2, 3));
    assert_eq!(bytes, vec![0x11, 1, 2, 3]);
}

#[test]
fn test_assembler_emits_fune() {
    use crate::encode::encode_instruction_bytes;
    use crate::mmixal::MMixInstruction;
    let bytes = encode_instruction_bytes(&MMixInstruction::FUNE(1, 2, 3));
    assert_eq!(bytes, vec![0x12, 1, 2, 3]);
}

#[test]
fn test_assembler_emits_feqle() {
    use crate::encode::encode_instruction_bytes;
    use crate::mmixal::MMixInstruction;
    let bytes = encode_instruction_bytes(&MMixInstruction::FEQLE(1, 2, 3));
    assert_eq!(bytes, vec![0x13, 1, 2, 3]);
}

// ==================== sNaN handling ====================

/// IEEE 754 binary64 sNaN: exponent all 1s, mantissa nonzero, bit 51 clear.
const SNAN_BITS: u64 = 0x7FF0_0000_0000_0001;

const QNAN_BITS: u64 = 0x7FF8_0000_0000_0001;

#[test]
fn test_is_signaling_nan_classifies_correctly() {
    assert!(MMix::is_signaling_nan(f64::from_bits(SNAN_BITS)));
    assert!(!MMix::is_signaling_nan(f64::from_bits(QNAN_BITS)));
    assert!(!MMix::is_signaling_nan(f64::NAN));
    assert!(!MMix::is_signaling_nan(1.0));
    assert!(!MMix::is_signaling_nan(f64::INFINITY));
}

#[test]
fn test_quiet_nan_sets_high_mantissa_bit() {
    let q = MMix::quiet_nan(f64::from_bits(SNAN_BITS));
    assert!(q.is_nan());
    assert_eq!(q.to_bits() & (1 << 51), 1 << 51);
    // Non-NaN passes through unchanged.
    assert_eq!(MMix::quiet_nan(1.5).to_bits(), 1.5f64.to_bits());
}

#[test]
fn test_fadd_snan_raises_i_and_quiets() {
    let mut mmix = MMix::new();
    mmix.set_register(2, SNAN_BITS);
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    let result = mmix.get_register(1);
    // Result must be a NaN, and must be quiet.
    assert!(f64::from_bits(result).is_nan());
    assert_eq!(result & (1 << 51), 1 << 51);
}

#[test]
fn test_fadd_qnan_does_not_raise_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, QNAN_BITS);
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    // qNaN propagation is silent — I must not be raised.
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
    assert!(f64::from_bits(mmix.get_register(1)).is_nan());
}

#[test]
fn test_fmul_snan_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 2.0f64.to_bits());
    mmix.set_register(3, SNAN_BITS);
    mmix.write_tetra(0, 0x10010203); // FMUL
    assert!(mmix.execute_instruction());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

#[test]
fn test_fsqrt_snan_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(3, SNAN_BITS);
    mmix.write_tetra(0, 0x15010003); // FSQRT
    assert!(mmix.execute_instruction());
    assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
}

// ==================== Directed rounding for FP arithmetic ====================

/// Build an operand pair whose true sum lies strictly between two adjacent
/// f64 values: 1.0 + 2^-53. Round-to-nearest-even gives 1.0 (mantissa LSB
/// of 1.0 is even); ROUND_UP must produce 1.0 + 2^-52.
fn one_plus_half_ulp() -> (f64, f64) {
    let a = 1.0f64;
    let half_ulp = f64::from_bits(0x3CA0_0000_0000_0000); // 2^-53
    (a, half_ulp)
}

#[test]
fn test_fadd_round_near_default() {
    let mut mmix = MMix::new();
    let (a, b) = one_plus_half_ulp();
    mmix.set_register(2, a.to_bits());
    mmix.set_register(3, b.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 1.0);
    // Inexact must be raised because the sum was rounded.
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_fadd_round_up_bumps_to_next_f64() {
    let mut mmix = MMix::new();
    let (a, b) = one_plus_half_ulp();
    mmix.set_register(2, a.to_bits());
    mmix.set_register(3, b.to_bits());
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    let r = f64::from_bits(mmix.get_register(1));
    assert!(r > 1.0, "expected r > 1.0, got {}", r);
    assert_eq!(r, 1.0f64.next_up());
}

#[test]
fn test_fadd_round_down_keeps_below() {
    let mut mmix = MMix::new();
    let (a, b) = one_plus_half_ulp();
    mmix.set_register(2, a.to_bits());
    mmix.set_register(3, b.to_bits());
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 1.0);
}

#[test]
fn test_fadd_round_off_toward_zero_negative() {
    let mut mmix = MMix::new();
    // -(1 + 2^-53) under round-up should keep -1.0 (less negative).
    // Under ROUND_OFF (toward 0), also -1.0.
    let (a, b) = one_plus_half_ulp();
    mmix.set_register(2, (-a).to_bits());
    mmix.set_register(3, (-b).to_bits());
    mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT); // ROUND_OFF
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), -1.0);
}

#[test]
fn test_fmul_directed_rounding() {
    let mut mmix = MMix::new();
    // 1/3 in f64 is inexact; (1/3) * 3 ≠ 1 exactly, gives a nearby f64.
    // The exact product 0.333…·3 = 0.999… so result is just below 1.
    let third = 1.0f64 / 3.0;
    mmix.set_register(2, third.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    // Default: round-to-nearest gives 1.0.
    mmix.write_tetra(0, 0x10010203); // FMUL
    assert!(mmix.execute_instruction());
    let near_result = f64::from_bits(mmix.get_register(1));

    // Reset and try ROUND_DOWN (toward -∞): result must be ≤ near_result
    // and strictly less than 1.
    let mut mmix = MMix::new();
    mmix.set_register(2, third.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT);
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    let down_result = f64::from_bits(mmix.get_register(1));
    assert!(down_result < 1.0, "ROUND_DOWN gave {}", down_result);
    assert!(down_result <= near_result);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_fdiv_directed_rounding() {
    let mut mmix = MMix::new();
    // 1.0 / 3.0 is inexact. Default → nearest. ROUND_UP must give a value
    // strictly greater than the nearest result.
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203); // FDIV
    assert!(mmix.execute_instruction());
    let near = f64::from_bits(mmix.get_register(1));

    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 3.0f64.to_bits());
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    let up = f64::from_bits(mmix.get_register(1));
    assert!(up > near, "ROUND_UP={} should exceed nearest={}", up, near);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_fdiv_directed_rounding_negative_divisor() {
    let mut mmix = MMix::new();
    // 1.0 / -3.0 — verify sign-of-divisor handling in residual.
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, (-3.0f64).to_bits());
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    let r = f64::from_bits(mmix.get_register(1));
    // True 1/-3 ≈ -0.333…; ROUND_DOWN must give a value ≤ true (more negative).
    assert!(r < -0.333, "ROUND_DOWN of 1/-3 was {}", r);
}

#[test]
fn test_fsqrt_directed_rounding() {
    let mut mmix = MMix::new();
    // sqrt(2) is irrational. ROUND_UP and ROUND_DOWN must straddle the
    // nearest result.
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.write_tetra(0, 0x15010003); // FSQRT
    assert!(mmix.execute_instruction());
    let near = f64::from_bits(mmix.get_register(1));

    let mut mmix = MMix::new();
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // UP
    mmix.write_tetra(0, 0x15010003);
    assert!(mmix.execute_instruction());
    let up = f64::from_bits(mmix.get_register(1));

    let mut mmix = MMix::new();
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // DOWN
    mmix.write_tetra(0, 0x15010003);
    assert!(mmix.execute_instruction());
    let down = f64::from_bits(mmix.get_register(1));

    assert!(down <= near && near <= up);
    assert!(down < up);
}

// ==================== Inexact (X) flag for arithmetic ====================

#[test]
fn test_fadd_exact_no_x_flag() {
    let mut mmix = MMix::new();
    // 1.0 + 2.0 is exact; X must not be raised.
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
}

#[test]
fn test_fmul_exact_no_x_flag() {
    let mut mmix = MMix::new();
    // 1.5 × 4.0 = 6.0 exactly.
    mmix.set_register(2, 1.5f64.to_bits());
    mmix.set_register(3, 4.0f64.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
}

#[test]
fn test_fdiv_exact_no_x_flag() {
    let mut mmix = MMix::new();
    // 12.0 / 4.0 = 3.0 exactly.
    mmix.set_register(2, 12.0f64.to_bits());
    mmix.set_register(3, 4.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
}

#[test]
fn test_exact_cancellation_fadd_raises_no_underflow() {
    let mut mmix = MMix::new();
    // 1.0 + -1.0 is exactly zero: nothing was too small to represent.
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, (-1.0f64).to_bits());
    mmix.write_tetra(0, 0x04010203); // FADD
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
    assert_eq!(
        mmix.get_special(SpecialReg::RA) & RA_U,
        0,
        "exact cancellation is not an underflow"
    );
}

#[test]
fn test_exact_cancellation_fsub_raises_no_underflow() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x06010203); // FSUB
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
    assert_eq!(
        mmix.get_special(SpecialReg::RA) & RA_U,
        0,
        "exact cancellation is not an underflow"
    );
}

#[test]
fn test_frem_exact_zero_raises_no_underflow() {
    let mut mmix = MMix::new();
    // The IEEE remainder is exact by definition, so FREM cannot underflow.
    mmix.set_register(2, (-3.0f64).to_bits());
    mmix.set_register(3, 1.5f64.to_bits());
    mmix.write_tetra(0, 0x16010203); // FREM
    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_special(SpecialReg::RA) & RA_U,
        0,
        "an exact remainder is not an underflow"
    );
}

#[test]
fn test_fdiv_to_zero_raises_underflow() {
    let mut mmix = MMix::new();
    // The true quotient is nonzero but far below the subnormal range.
    mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
    mmix.set_register(3, 1e16f64.to_bits());
    mmix.write_tetra(0, 0x14010203); // FDIV
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_U) != 0);
}

#[test]
fn test_fmul_to_zero_raises_underflow() {
    let mut mmix = MMix::new();
    // MIN_POSITIVE^2 is 2^-2044: nonzero, and below the subnormal range.
    // The FMA residual rounds to zero here, so it cannot witness the
    // underflow — only the operands can.
    mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
    mmix.set_register(3, f64::MIN_POSITIVE.to_bits());
    mmix.write_tetra(0, 0x10010203); // FMUL
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_U) != 0);
}

#[test]
fn test_fdiv_by_infinity_is_exact_zero() {
    let mut mmix = MMix::new();
    // finite / inf is exactly +0.0: neither inexact nor an underflow. The
    // residual is NaN here, so an unguarded `err != 0.0` would raise X.
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x14010203); // FDIV
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0.0f64.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ==================== FREM / FIXU / FCMP / conversions ====================

#[test]
fn test_frem_zero_takes_dividend_sign() {
    let mut mmix = MMix::new();
    // IEEE 754 gives a zero remainder the sign of the dividend; the
    // divisor's sign does not enter. Compare bit patterns — -0.0 == 0.0.
    mmix.set_register(2, (-3.0f64).to_bits());
    mmix.set_register(3, 1.5f64.to_bits());
    mmix.write_tetra(0, 0x16010203); // FREM $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), (-0.0f64).to_bits());

    let mut mmix = MMix::new();
    mmix.set_register(2, 3.0f64.to_bits());
    mmix.set_register(3, (-1.5f64).to_bits());
    mmix.write_tetra(0, 0x16010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0.0f64.to_bits());
}

#[test]
fn test_fixu_wraps_mod_two_to_the_64() {
    // u($X) <- int(f($Z)) mod 2^64. A value whose ulp reaches 2^64
    // therefore yields zero rather than saturating.
    let cases: [(f64, u64); 10] = [
        (-1.0, 0xFFFF_FFFF_FFFF_FFFF),
        (-0.5, 0), // rounds to -0.0 under NEAR
        (3.7, 4),
        (9223372036854775808.0, 0x8000_0000_0000_0000), // 2^63
        (-9223372036854775808.0, 0x8000_0000_0000_0000), // -2^63
        (18446744073709551616.0, 0),                    // 2^64
        (1e300, 0),
        (18446744073709549568.0, 0xFFFF_FFFF_FFFF_F800),
        // The exponent at which every low bit has shifted out. An odd
        // significand distinguishes the two sides: at 2^115 the value is
        // 2^115 + 2^63, whose low octabyte is 2^63; one exponent higher
        // every surviving bit sits above the octabyte.
        (
            f64::from_bits(((115 + 1023) << 52) | 1),
            0x8000_0000_0000_0000,
        ),
        (f64::from_bits(((116 + 1023) << 52) | 1), 0),
    ];
    for (operand, expected) in cases {
        let mut mmix = MMix::new();
        mmix.set_register(3, operand.to_bits());
        mmix.write_tetra(0, 0x07010003); // FIXU $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_register(1),
            expected,
            "FIXU {operand} should be {expected:#X}"
        );
    }
}

#[test]
fn test_fix_negative_is_still_signed() {
    let mut mmix = MMix::new();
    // FIX keeps its signed conversion; only FIXU wraps.
    mmix.set_register(3, (-1.0f64).to_bits());
    mmix.write_tetra(0, 0x05010003); // FIX $1,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -1);
}

#[test]
fn test_fcmp_nan_answers_zero_in_either_position() {
    for (y, z) in [(f64::NAN, 1.0f64), (1.0f64, f64::NAN), (f64::NAN, f64::NAN)] {
        let mut mmix = MMix::new();
        mmix.set_register(2, y.to_bits());
        mmix.set_register(3, z.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0, "FCMP {y},{z}");
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }
}

#[test]
fn test_fcmp_ordered_still_three_way() {
    for (y, z, expected) in [(1.0f64, 2.0f64, -1i64), (2.0, 2.0, 0), (3.0, 2.0, 1)] {
        let mut mmix = MMix::new();
        mmix.set_register(2, y.to_bits());
        mmix.set_register(3, z.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, expected, "FCMP {y},{z}");
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
    }
}

#[test]
fn test_flot_inexact_conversion_raises_x() {
    let mut mmix = MMix::new();
    // 2^53+1 has no f64 image; it rounds to 2^53 and X reports the loss.
    mmix.set_register(3, (1u64 << 53) + 1);
    mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 9007199254740992.0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_flot_exact_conversion_raises_nothing() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 42);
    mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 42.0);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_flot_honors_directed_rounding_mode() {
    let mut up = MMix::new();
    up.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
    up.set_register(3, (1u64 << 53) + 1);
    up.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
    assert!(up.execute_instruction());
    let up_result = f64::from_bits(up.get_register(1));

    let mut down = MMix::new();
    down.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
    down.set_register(3, (1u64 << 53) + 1);
    down.write_tetra(0, 0x08010003);
    assert!(down.execute_instruction());
    let down_result = f64::from_bits(down.get_register(1));

    assert_eq!(down_result, 9007199254740992.0);
    assert_eq!(up_result, 9007199254740994.0);
    assert!(up_result > down_result);

    // The negative arm negates both the magnitude and the residual, so a
    // directed mode must still round toward its own infinity.
    let mut neg_up = MMix::new();
    neg_up.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
    neg_up.set_register(3, (-((1i64 << 53) + 1)) as u64);
    neg_up.write_tetra(0, 0x08010003);
    assert!(neg_up.execute_instruction());
    assert_eq!(f64::from_bits(neg_up.get_register(1)), -9007199254740992.0);

    let mut neg_down = MMix::new();
    neg_down.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
    neg_down.set_register(3, (-((1i64 << 53) + 1)) as u64);
    neg_down.write_tetra(0, 0x08010003);
    assert!(neg_down.execute_instruction());
    assert_eq!(
        f64::from_bits(neg_down.get_register(1)),
        -9007199254740994.0
    );
}

#[test]
fn test_flotu_max_raises_x() {
    let mut mmix = MMix::new();
    // u64::MAX rounds up to 2^64, losing eleven bits. Recovering the
    // residual by casting back saturates to u64::MAX and reports zero, so
    // this vector is the one that catches that mistake.
    mmix.set_register(3, u64::MAX);
    mmix.write_tetra(0, 0x0A010003); // FLOTU $1,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 18446744073709551616.0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

#[test]
fn test_sflotu_inexact_wide_step_raises_x() {
    let mut mmix = MMix::new();
    // 2^53+1 rounds to 2^53 on the way to f64; 2^53 then narrows to f32
    // exactly, so only the integer-to-f64 step can raise X here.
    mmix.set_register(3, (1u64 << 53) + 1);
    mmix.write_tetra(0, 0x0E010003); // SFLOTU $1,$0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(f64::from_bits(mmix.get_register(1)), 9007199254740992.0);
    assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
}

// ==================== Overflow with directed rounding ====================

#[test]
fn test_overflow_round_off_clamps_to_max() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::MAX.to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT); // ROUND_OFF (toward 0)
    mmix.write_tetra(0, 0x04010203); // FADD
    assert!(mmix.execute_instruction());
    let r = f64::from_bits(mmix.get_register(1));
    assert_eq!(r, f64::MAX);
    assert!((mmix.get_special(SpecialReg::RA) & RA_O) != 0);
}

#[test]
fn test_overflow_round_down_negative_keeps_neg_inf() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::MIN.to_bits()); // most-negative finite
    mmix.set_register(3, f64::MIN.to_bits());
    mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN (toward -∞)
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    let r = f64::from_bits(mmix.get_register(1));
    assert!(r.is_infinite() && r.is_sign_negative());
}

// ==================== FLOT/FIX/NaN/flag behaviour ====================
//
// One block per rule a floating-point instruction follows: how it reads
// an immediate operand, FIX's wraparound and range, NaN propagation and
// quieting, and which flag bits an operation raises.

// ---- immediate FLOT/SFLOT read Z as an unsigned byte ----

#[test]
fn test_floti_immediate_200() {
    let mut mmix = MMix::new();
    mmix.write_tetra(0, 0x090100C8); // FLOTI $1,0,200
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x4069000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_sfloti_immediate_200() {
    let mut mmix = MMix::new();
    mmix.write_tetra(0, 0x0D0100C8); // SFLOTI $1,0,200
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x4069000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_floti_round_up_255() {
    let mut mmix = MMix::new();
    mmix.write_tetra(0, 0x090102FF); // FLOTI $1,ROUND_UP,255
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x406F_E000_0000_0000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ---- FIX wraps mod 2^64 and raises W outside the signed range ----

#[test]
fn test_fix_1e20_wraps_and_raises_w() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 1e20_f64.to_bits());
    mmix.write_tetra(0, 0x05010003); // FIX $1,0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x6BC7_5E2D_6310_0000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_W);
}

#[test]
fn test_fix_two_to_the_64_wraps_to_zero() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 2f64.powi(64).to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_W);
}

#[test]
fn test_fix_1e300_wraps_to_zero() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 1e300_f64.to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_W);
}

// ---- the signed range's own boundary ----

#[test]
fn test_fix_two_to_the_63_is_out_of_range() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 2f64.powi(63).to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x8000000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_W);
}

#[test]
fn test_fix_negative_two_to_the_63_is_in_range() {
    let mut mmix = MMix::new();
    mmix.set_register(3, (-(2f64.powi(63))).to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x8000000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ---- an infinite or NaN operand copies through with I alone ----

#[test]
fn test_fix_infinity_copies_through_with_i_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), f64::INFINITY.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fix_quiet_nan_copies_through_with_i_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 0x7FF8000000000005);
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000005);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fixu_quiet_nan_copies_through_with_i_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 0x7FF8000000000005);
    mmix.write_tetra(0, 0x07010003); // FIXU $1,0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000005);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fix_signaling_nan_copies_through_unquieted() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 0x7FF0000000000001); // sNaN#1
    mmix.write_tetra(0, 0x05010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF0000000000001);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

// ---- FADD/FMUL/FDIV/FSUB's standard-conventions NaN pick ----

#[test]
fn test_fadd_two_quiet_nans_picks_z() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF8000000000005); // qNaN#5, $Y
    mmix.set_register(3, 0x7FF8000000000007); // qNaN#7, $Z
    mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000007);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fadd_signaling_y_picks_z_and_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF0000000000001); // sNaN#1
    mmix.set_register(3, 0x7FF8000000000007); // qNaN#7
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000007);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fadd_signaling_z_quiets_and_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF8000000000005);
    mmix.set_register(3, 0x7FF0000000000002); // sNaN#2
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000002);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fadd_both_signaling_picks_z_and_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF0000000000001);
    mmix.set_register(3, 0x7FF0000000000002);
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000002);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fadd_signaling_y_nonnan_z_picks_y() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF0000000000001);
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000001);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fmul_two_quiet_nans_picks_z() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF8000000000005);
    mmix.set_register(3, 0x7FF8000000000007);
    mmix.write_tetra(0, 0x10010203); // FMUL $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000007);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fdiv_signaling_y_picks_z_and_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF0000000000001);
    mmix.set_register(3, 0x7FF8000000000007);
    mmix.write_tetra(0, 0x14010203); // FDIV $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000007);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fsub_negative_nan_z_stays_unnegated() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0xFFF8000000000005);
    mmix.write_tetra(0, 0x06010203); // FSUB $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000005);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ---- FREM's standard-conventions NaN pick ----

#[test]
fn test_frem_quiet_nan_y_picks_y() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x7FF8000000000005);
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x16010203); // FREM $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000005);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_frem_signaling_z_quiets_and_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0x7FF0000000000002);
    mmix.write_tetra(0, 0x16010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000002);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

// ---- the invalid operations and their signs ----

#[test]
fn test_fadd_opposite_infinities_is_invalid_signed_by_z() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::INFINITY.to_bits());
    mmix.set_register(3, f64::NEG_INFINITY.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fadd_opposite_infinities_reversed_is_invalid_signed_by_z() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::NEG_INFINITY.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fsub_same_sign_infinities_is_invalid() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::INFINITY.to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x06010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fmul_zero_times_infinity_is_invalid() {
    let mut mmix = MMix::new();
    mmix.set_register(2, (-0.0f64).to_bits());
    mmix.set_register(3, f64::INFINITY.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fdiv_zero_over_zero_is_invalid() {
    let mut mmix = MMix::new();
    mmix.set_register(2, (-0.0f64).to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_frem_infinite_y_is_invalid() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::NEG_INFINITY.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x16010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_frem_zero_z_is_invalid() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x16010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_fsqrt_of_negative_is_invalid() {
    let mut mmix = MMix::new();
    mmix.set_register(3, (-1.0f64).to_bits());
    mmix.write_tetra(0, 0x15010003); // FSQRT $1,0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFFF8000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

// ---- FINT's NaN passthrough ----

#[test]
fn test_fint_quiet_nan_passes_through() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 0x7FF8000000000005);
    mmix.write_tetra(0, 0x17010003); // FINT $1,0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000005);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fint_signaling_nan_quiets_and_raises_i() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 0x7FF0000000000001);
    mmix.write_tetra(0, 0x17010003);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF8000000000001);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

// ---- LDSF widens a short float's bits exactly ----

#[test]
fn test_ldsf_signaling_nan_widens_bit_for_bit() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 100);
    mmix.set_register(3, 0);
    mmix.write_tetra(100, 0x7F800001);
    mmix.write_tetra(0, 0x90010203); // LDSF $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF0000020000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_ldsf_quiet_nan_widens_bit_for_bit() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 100);
    mmix.set_register(3, 0);
    mmix.write_tetra(100, 0x7FC00005);
    mmix.write_tetra(0, 0x90010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x7FF80000A0000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ---- STSF quiets a signaling NaN and narrows a subnormal on store ----

#[test]
fn test_stsf_signaling_nan_quiets_on_store() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 0x7FF0000000000001); // sNaN#1
    mmix.set_register(2, 100);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0xB0010203); // STSF $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(100), 0x7FC00000);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_stsf_signaling_nan_truncates_and_quiets() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 0x7FF0000020000000);
    mmix.set_register(2, 100);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0xB0010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(100), 0x7FC00001);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_I);
}

#[test]
fn test_stsf_exact_short_subnormal_raises_nothing() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 0x37D0000000000000); // 2^-130
    mmix.set_register(2, 100);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0xB0010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(100), 0x00080000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ---- an exact subnormal result raises nothing ----

#[test]
fn test_fadd_smallest_subnormals_sum_exactly() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1);
    mmix.set_register(3, 1);
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 2);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fsub_at_the_subnormal_boundary_is_exact() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x0010000000000000);
    mmix.set_register(3, 1);
    mmix.write_tetra(0, 0x06010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x000FFFFFFFFFFFFF);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fmul_exact_subnormal_product_raises_nothing() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 2f64.powi(-1000).to_bits());
    mmix.set_register(3, 2f64.powi(-30).to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x0000100000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fmul_boundary_normal_times_half_is_exact_subnormal() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x0010000000000000);
    mmix.set_register(3, 0.5f64.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x0008000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_fmul_exact_subnormal_still_trips_when_u_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x60, 0xFD000000); // U's vector loaded
    mmix.set_special(SpecialReg::RA, RA_U << 8); // enable U only
    mmix.set_pc(0x100);
    mmix.set_register(2, 0x0010000000000000);
    mmix.set_register(3, 0.5f64.to_bits());
    mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x60);
}

// ---- an inexact underflow raises U and X together ----

#[test]
fn test_fmul_inexact_underflow_raises_u_and_x() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x0008000000000000);
    mmix.set_register(3, 0.1f64.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x0000CCCCCCCCCCCD);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_U | RA_X);
}

#[test]
fn test_fmul_underflow_to_zero_raises_u_and_x() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x0010000000000000);
    mmix.set_register(3, 0x0010000000000000);
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_U | RA_X);
}

#[test]
fn test_fmul_smallest_subnormal_times_half_underflows_to_zero() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1);
    mmix.set_register(3, 0.5f64.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_U | RA_X);
}

#[test]
fn test_fmul_subnormal_times_half_rounds_and_underflows() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 3);
    mmix.set_register(3, 0.5f64.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 2);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_U | RA_X);
}

#[test]
fn test_fdiv_boundary_normal_over_one_point_five_underflows() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 0x0010000000000000);
    mmix.set_register(3, 1.5f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x000AAAAAAAAAAAAB);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_U | RA_X);
}

// ---- the exact error's sign, decided even where a hardware FMA underflows ----

#[test]
fn test_fmul_round_up_pushes_a_tiny_product_up_from_zero() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 0x20000); // ROUND_UP, no trips
    mmix.set_register(2, 1);
    mmix.set_register(3, 0.5f64.to_bits());
    mmix.write_tetra(0, 0x10010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0x20005);
}

#[test]
fn test_fsqrt_of_a_subnormal_rounds_to_a_normal_result() {
    let mut mmix = MMix::new();
    mmix.set_register(3, 2);
    mmix.write_tetra(0, 0x15010003); // FSQRT $1,0,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x1E66A09E667F3BCD);
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_X);
}

// ---- ±∞/±0 raises nothing ----

#[test]
fn test_fdiv_infinity_over_zero_raises_nothing() {
    let mut mmix = MMix::new();
    mmix.set_register(2, f64::INFINITY.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), f64::INFINITY.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

// ---- a finite nonzero dividend over zero raises Z alone ----

#[test]
fn test_fdiv_by_zero_raises_z_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), f64::INFINITY.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_Z);
}

#[test]
fn test_fdiv_negative_by_zero_raises_z_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(2, (-1.0f64).to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), f64::NEG_INFINITY.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_Z);
}

#[test]
fn test_fdiv_subnormal_by_zero_raises_z_alone() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1);
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), f64::INFINITY.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), RA_Z);
}

#[test]
fn test_fdiv_by_zero_is_exact_in_round_off_too() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 0x10000); // ROUND_OFF, no trips
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), f64::INFINITY.to_bits());
    assert_eq!(mmix.get_special(SpecialReg::RA), 0x10002);
}

// ---- ROUND_DOWN's exact-zero sign ----

#[test]
fn test_fadd_round_down_mixed_zero_is_negative() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 0x30000); // ROUND_DOWN
    mmix.set_register(2, 0.0f64.to_bits());
    mmix.set_register(3, (-0.0f64).to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x8000000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0x30000);
}

#[test]
fn test_fsub_round_down_cancellation_is_negative() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 0x30000);
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x06010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x8000000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0x30000);
}

#[test]
fn test_fadd_round_down_both_positive_zero_stays_positive() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RA, 0x30000);
    mmix.set_register(2, 0.0f64.to_bits());
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0x30000);
}

#[test]
fn test_fsub_cancellation_is_positive_by_default() {
    let mut mmix = MMix::new();
    mmix.set_register(2, 1.0f64.to_bits());
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x06010203);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

/// A rounding-mode override above 4 halts with a diagnostic and
/// exits 1, like every other halt but the `Halt` trap.
#[test]
fn test_fix_with_illegal_round_mode_exits_1() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(3, 42.0f64.to_bits());
    mmix.write_tetra(0, 0x05020503); // FIX $2,5,$3 -- Y=5 is out of range
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
    assert_eq!(mmix.get_register(2), 0, "the write did not land");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("FIX"));
}
