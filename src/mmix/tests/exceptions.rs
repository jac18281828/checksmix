//! rA exception routing, TRIP, and overflow/divide-check flags.

use super::*;

#[test]
fn test_trip_to_unloaded_vector_halts_with_diagnostic_and_nonzero_exit() {
    // #00 reads zero, which decodes as TRAP 0,0,0 — the case the
    // unloaded-vector halt exists to catch instead of hiding.
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_pc(0x100);
    mmix.write_tetra(0x100, 0xFF000000); // TRIP 0,0,0
    let result = mmix.execute_instruction();
    assert!(!result);
    assert_eq!(mmix.get_pc(), 0x100, "PC stays on the tripping instruction");
    assert_eq!(mmix.get_exit_code(), 1);
    // Registers are set before the halt is detected, so a debugger sees why.
    assert_eq!(mmix.get_special(SpecialReg::RW), 0x104);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("TRIP"));
}

#[test]
fn test_trip() {
    // TRIP X,Y,Z sets rX/rY/rZ/rB/$255/rW (§1 rule 3) and lands at #00.
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x00, 0xFD000000); // vector loaded: SWYM
    mmix.set_pc(0x100);
    mmix.set_register(255, 0xAAAA);
    mmix.set_register(2, 0x2222);
    mmix.set_register(3, 0x3333);
    mmix.set_special(SpecialReg::RJ, 0xBEEF);
    mmix.write_tetra(0x100, 0xFF110203); // TRIP X=0x11, Y=2, Z=3

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x00);
    assert_eq!(mmix.get_special(SpecialReg::RW), 0x104);
    assert_eq!(mmix.get_special(SpecialReg::RX), 0x80000000FF110203);
    assert_eq!(mmix.get_special(SpecialReg::RY), 0x2222);
    assert_eq!(mmix.get_special(SpecialReg::RZ), 0x3333);
    assert_eq!(mmix.get_special(SpecialReg::RB), 0xAAAA);
    assert_eq!(mmix.get_register(255), 0xBEEF);
}

#[test]
fn test_divide_check_trips_when_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x10, 0xFD000000); // D's vector loaded
    mmix.set_special(SpecialReg::RA, RA_D << 8); // enable D only
    mmix.set_pc(0x100);
    mmix.set_register(2, 7);
    mmix.set_register(3, 0);
    mmix.write_tetra(0x100, 0x1C010203); // DIV $1,$2,$3

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x10, "trips to D's vector");
    assert_eq!(
        mmix.get_register(1),
        0,
        "DIV's own zero result is written first"
    );
    assert_eq!(mmix.get_special(SpecialReg::RY), 7);
    assert_eq!(
        mmix.get_special(SpecialReg::RA) & RA_D,
        0,
        "a tripped exception's own event bit stays clear"
    );
}

#[test]
fn test_integer_overflow_trips_and_the_handler_sees_the_pre_write_operand() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x20, 0xFD000000); // V's vector loaded
    mmix.set_special(SpecialReg::RA, RA_V << 8); // enable V only
    mmix.set_pc(0x100);
    mmix.set_register(5, i64::MAX as u64);
    mmix.set_register(3, 1);
    mmix.write_tetra(0x100, 0x20050503); // ADD $5,$5,$3 - destination aliases a source

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x20);
    assert_eq!(
        mmix.get_register(5),
        i64::MIN as u64,
        "the wrapped result is written before the trip"
    );
    assert_eq!(
        mmix.get_special(SpecialReg::RY),
        i64::MAX as u64,
        "rY holds $5's value from before ADD overwrote it"
    );
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_V, 0);
}

#[test]
fn test_immediate_overflow_trip_reports_the_literal_z() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x20, 0xFD000000); // V's vector loaded
    mmix.set_special(SpecialReg::RA, RA_V << 8); // enable V only
    mmix.set_pc(0x100);
    mmix.set_register(2, i64::MAX as u64);
    // $5 holds a value distinct from the literal Z=5: a revert that reads
    // get_register(5) instead of the literal would report this instead.
    mmix.set_register(5, 0xDEAD_BEEF);
    mmix.write_tetra(0x100, 0x21030205); // ADDI $3,$2,5

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x20);
    assert_eq!(
        mmix.get_special(SpecialReg::RZ),
        5,
        "rZ holds the literal Z, not $5's contents"
    );
}

#[test]
fn test_store_overflow_trip_reports_address_and_stored_value() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x20, 0xFD000000); // V's vector loaded
    mmix.set_special(SpecialReg::RA, RA_V << 8); // enable V only
    mmix.set_pc(0x100);
    mmix.set_register(1, 200); // out of signed byte range: overflows
    mmix.set_register(2, 0x4000);
    mmix.set_register(3, 8);
    // A nonzero neighbouring byte in the target octabyte: rZ must be the
    // merged octabyte memory now holds, not the raw stored byte alone.
    mmix.write_byte(0x4009, 0xAB);
    mmix.write_tetra(0x100, 0xA0010203); // STB $1,$2,$3

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x20);
    assert_eq!(
        mmix.get_special(SpecialReg::RY),
        0x4008,
        "rY holds the computed address, not raw $Y"
    );
    assert_eq!(
        mmix.get_special(SpecialReg::RZ),
        0xC8AB_0000_0000_0000,
        "rZ holds the aligned octabyte after the store: the stored byte \
             (200 = 0xC8) in place, the neighbouring byte and the rest of \
             memory unchanged"
    );
}

#[test]
fn test_stsf_overflow_trip_reports_address_and_merged_octa() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x50, 0xFD000000); // O's vector loaded
    mmix.set_special(SpecialReg::RA, RA_O << 8); // enable O only
    mmix.set_pc(0x100);
    mmix.set_register(1, f64::MAX.to_bits()); // narrows to +inf: overflow
    mmix.set_register(2, 0x4000);
    mmix.set_register(3, 8);
    // A nonzero byte past the stored tetra, in the same octabyte: rZ
    // must be the merged octabyte, not the plain $Y/$Z operands.
    mmix.write_byte(0x400C, 0xAB);
    mmix.write_tetra(0x100, 0xB0010203); // STSF $1,$2,$3

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x50);
    assert!(f32::from_bits(mmix.read_tetra(0x4008)).is_infinite());
    assert_eq!(
        mmix.get_special(SpecialReg::RY),
        0x4008,
        "rY holds the computed address, not raw $Y — catches a revert \
             to the plain $Y/$Z operands"
    );
    assert_eq!(
        mmix.get_special(SpecialReg::RZ),
        0x7F80_0000_AB00_0000,
        "rZ holds the aligned octabyte after the store: the stored \
             +inf tetra in place, the neighbouring byte unchanged — catches \
             a revert to the plain $Y/$Z operands"
    );
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_O, 0);
}

#[test]
fn test_invalid_operation_trips_when_enabled() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x40, 0xFD000000); // I's vector loaded
    mmix.set_special(SpecialReg::RA, RA_I << 8); // enable I only
    mmix.set_pc(0x100);
    mmix.set_register(2, f64::NAN.to_bits());
    mmix.set_register(3, 0);
    mmix.write_tetra(0x100, 0x01010203); // FCMP $1,$2,$3 - $2 is NaN

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x40);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
}

#[test]
fn test_two_enabled_exceptions_trip_to_the_leftmost_and_drop_the_other_silently() {
    let mut mmix = MMix::new();
    load_tetra(&mut mmix, 0x50, 0xFD000000); // O's vector loaded
    mmix.set_special(SpecialReg::RA, (RA_O << 8) | (RA_X << 8)); // both enabled
    mmix.set_pc(0x100);
    mmix.set_register(2, f64::MAX.to_bits());
    mmix.set_register(3, f64::MAX.to_bits());
    mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3 overflows (raises O and X)

    assert!(mmix.execute_instruction());
    assert_eq!(
        mmix.get_pc(),
        0x50,
        "trips to O, the leftmost enabled exception"
    );
    assert_eq!(
        mmix.get_special(SpecialReg::RA) & 0xFF,
        0,
        "O tripped and X was enabled but not leftmost: neither sets an event bit"
    );
}

#[test]
fn test_signed_stores_raise_v_when_the_value_does_not_fit() {
    // Register form: ST* $1,$2,$3 with $2 = $3 = 0. Immediate form: Z = 0.
    for (word, wide, narrow) in [
        (0xA0010203u32, 200u64, 127u64),   // STB
        (0xA1010200, 200, 127),            // STBI
        (0xA4010203, 40000, 32767),        // STW
        (0xA5010200, 40000, 32767),        // STWI
        (0xA8010203, 1 << 32, 0x7FFFFFFF), // STT
        (0xA9010200, 1 << 32, 0x7FFFFFFF), // STTI
    ] {
        assert_eq!(run_store(word, wide) & RA_V, RA_V, "{word:#010X} overflows");
        assert_eq!(run_store(word, narrow), 0, "{word:#010X} in range");
    }
}
