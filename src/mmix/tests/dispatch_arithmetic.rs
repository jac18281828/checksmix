//! ADD/SUB/NEG/MUL/DIV, bitwise ops, and shifts.

use super::*;

#[test]
fn test_incl_instruction() {
    let mut mmix = MMix::new();
    // INCL $1, YZ=0x0203 - opcode 0xE7, X=1, YZ=0x0203
    mmix.write_tetra(0, 0xE7010203);
    mmix.set_register(1, 50);

    let result = mmix.execute_instruction();
    assert!(result); // Should continue
    assert_eq!(mmix.get_register(1), 50 + 0x0203); // 50 + YZ value
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_incl_with_zero() {
    let mut mmix = MMix::new();
    // INCL $2, YZ=0
    mmix.write_tetra(0, 0xE7020000);
    mmix.set_register(2, 42);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(2), 42); // No change
}

#[test]
fn test_incl_overflow() {
    let mut mmix = MMix::new();
    // INCL $3, YZ=0x0405
    mmix.write_tetra(0, 0xE7030405);
    mmix.set_register(3, u64::MAX - 5);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(3), (u64::MAX - 5).wrapping_add(0x0405)); // Wraps around
}

#[test]
fn test_incl_large_values() {
    let mut mmix = MMix::new();
    // INCL $1, YZ=0x0203
    mmix.write_tetra(0, 0xE7010203);
    mmix.set_register(1, 100);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 100 + 0x0203);
}

#[test]
fn test_incl_register_255() {
    let mut mmix = MMix::new();
    // INCL $255, YZ=0x0102 - should modify $255 like any other register
    mmix.write_tetra(0, 0xE7FF0102);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(255), 0x0102); // Should have the immediate value
}

#[test]
fn test_incl_using_255() {
    let mut mmix = MMix::new();
    // INCL $1, YZ=0xFF02
    mmix.write_tetra(0, 0xE701FF02);
    mmix.set_register(1, 100);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 100 + 0xFF02);
}

#[test]
fn test_run_multiple_incl() {
    let mut mmix = MMix::new();
    // Program: 3 INCL instructions followed by TRAP (halt)
    mmix.write_tetra(0, 0xE7010000); // INCL $1, YZ=0 (no change)
    mmix.write_tetra(4, 0xE7010203); // INCL $1, YZ=0x0203
    mmix.write_tetra(8, 0xE7010203); // INCL $1, YZ=0x0203
    mmix.write_tetra(12, 0xFF000000); // TRIP (halt)

    let count = mmix.run();
    assert_eq!(count, 3);
    assert_eq!(mmix.get_register(1), 0x0203 * 2); // 0 + 0x0203 + 0x0203
    assert_eq!(mmix.get_pc(), 12);
}

#[test]
fn test_incl_has_multiple_tests() {
    // INCL is already tested in:
    // - test_incl_instruction
    // - test_incl_with_zero
    // - test_incl_overflow
    // - test_incl_large_values
    // - test_incl_register_255
    // - test_incl_using_255
    // - test_run_multiple_incl
    // This test just confirms coverage
    let mut mmix = MMix::new();
    mmix.write_tetra(0, 0xE7010203);
    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x0203);
}

// Arithmetic instruction tests - Add and Subtract

#[test]
fn test_add_positive_numbers() {
    let mut mmix = MMix::new();
    // ADD $1, $2, $3
    mmix.write_tetra(0, 0x20010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 150);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_add_immediate() {
    let mut mmix = MMix::new();
    // ADD $1, $2, 75
    mmix.write_tetra(0, 0x2101024B);
    mmix.set_register(2, 25);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 100);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_add_negative_numbers() {
    let mut mmix = MMix::new();
    // ADD $1, $2, $3
    mmix.write_tetra(0, 0x20010203);
    mmix.set_register(2, (-50i64) as u64);
    mmix.set_register(3, (-30i64) as u64);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -80);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_addu_wrapping() {
    let mut mmix = MMix::new();
    // ADDU $1, $2, $3 (already tested as LDA, but verify here)
    mmix.write_tetra(0, 0x22010203);
    mmix.set_register(2, u64::MAX);
    mmix.set_register(3, 1);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0); // Wraps around
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_addu_immediate() {
    let mut mmix = MMix::new();
    // ADDU $1, $2, 100
    mmix.write_tetra(0, 0x23010264);
    mmix.set_register(2, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 150);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_2addu_register() {
    let mut mmix = MMix::new();
    // 2ADDU $1, $2, $3
    mmix.write_tetra(0, 0x28010203);
    mmix.set_register(2, 10);
    mmix.set_register(3, 5);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 25); // 2*10 + 5 = 25
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_2addu_immediate() {
    let mut mmix = MMix::new();
    // 2ADDU $1, $2, 7
    mmix.write_tetra(0, 0x29010207);
    mmix.set_register(2, 12);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 31); // 2*12 + 7 = 31
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_4addu_register() {
    let mut mmix = MMix::new();
    // 4ADDU $1, $2, $3
    mmix.write_tetra(0, 0x2A010203);
    mmix.set_register(2, 10);
    mmix.set_register(3, 5);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 45); // 4*10 + 5 = 45
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_4addu_immediate() {
    let mut mmix = MMix::new();
    // 4ADDU $1, $2, 8
    mmix.write_tetra(0, 0x2B010208);
    mmix.set_register(2, 10);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 48); // 4*10 + 8 = 48
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_8addu_register() {
    let mut mmix = MMix::new();
    // 8ADDU $1, $2, $3
    mmix.write_tetra(0, 0x2C010203);
    mmix.set_register(2, 10);
    mmix.set_register(3, 5);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 85); // 8*10 + 5 = 85
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_8addu_immediate() {
    let mut mmix = MMix::new();
    // 8ADDU $1, $2, 15
    mmix.write_tetra(0, 0x2D01020F);
    mmix.set_register(2, 10);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 95); // 8*10 + 15 = 95
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_16addu_register() {
    let mut mmix = MMix::new();
    // 16ADDU $1, $2, $3
    mmix.write_tetra(0, 0x2E010203);
    mmix.set_register(2, 10);
    mmix.set_register(3, 5);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 165); // 16*10 + 5 = 165
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_16addu_immediate() {
    let mut mmix = MMix::new();
    // 16ADDU $1, $2, 20
    mmix.write_tetra(0, 0x2F010214);
    mmix.set_register(2, 10);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 180); // 16*10 + 20 = 180
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sub_positive_result() {
    let mut mmix = MMix::new();
    // SUB $1, $2, $3
    mmix.write_tetra(0, 0x24010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 30);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 70);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sub_negative_result() {
    let mut mmix = MMix::new();
    // SUB $1, $2, $3
    mmix.write_tetra(0, 0x24010203);
    mmix.set_register(2, 30);
    mmix.set_register(3, 100);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -70);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sub_immediate() {
    let mut mmix = MMix::new();
    // SUB $1, $2, 25
    mmix.write_tetra(0, 0x25010219);
    mmix.set_register(2, 100);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 75);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_subu_wrapping() {
    let mut mmix = MMix::new();
    // SUBU $1, $2, $3
    mmix.write_tetra(0, 0x26010203);
    mmix.set_register(2, 10);
    mmix.set_register(3, 20);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), u64::MAX - 9); // 10 - 20 wraps
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_subu_immediate() {
    let mut mmix = MMix::new();
    // SUBU $1, $2, 30
    mmix.write_tetra(0, 0x2701021E);
    mmix.set_register(2, 100);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 70);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_neg_zero_minus_value() {
    let mut mmix = MMix::new();
    // NEG $1, 0, $3 - effectively 0 - $3
    mmix.write_tetra(0, 0x34010003);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -50);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_neg_immediate_both() {
    let mut mmix = MMix::new();
    // NEG $1, 10, 3 - effectively 10 - 3
    mmix.write_tetra(0, 0x35010A03);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 7);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_neg_one_minus_two() {
    let mut mmix = MMix::new();
    // NEG $1, 1, 2 - effectively 1 - 2 = -1
    mmix.write_tetra(0, 0x35010102);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -1);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_negu_register() {
    let mut mmix = MMix::new();
    // NEGU $1, 0, $3
    mmix.write_tetra(0, 0x36010003);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), u64::MAX - 49); // 0 - 50 wraps
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_negu_immediate() {
    let mut mmix = MMix::new();
    // NEGU $1, 100, 30
    mmix.write_tetra(0, 0x3701641E);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 70); // 100 - 30 = 70
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_multiply_add_for_array_indexing() {
    let mut mmix = MMix::new();
    // Common pattern: 8ADDU for array of 64-bit values
    // base_addr + index * 8
    mmix.write_tetra(0, 0x2C010203);
    mmix.set_register(2, 5); // index
    mmix.set_register(3, 1000); // base address

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 1040); // 1000 + 5*8
}

#[test]
fn test_all_arithmetic_instructions_have_tests() {
    let mut mmix = MMix::new();

    // ADD $1, $2, $3
    mmix.write_tetra(0, 0x20010203);
    mmix.set_register(2, 200);
    mmix.set_register(3, 44);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 244);

    // ADDU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x22010203);
    mmix.set_register(2, 900);
    mmix.set_register(3, 33);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 933);

    // 2ADDU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x28010203);
    mmix.set_register(2, 7);
    mmix.set_register(3, 9);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 23); // 2*7 + 9 = 23

    // 4ADDU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x2A010203);
    mmix.set_register(2, 7);
    mmix.set_register(3, 9);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 37); // 4*7 + 9 = 37

    // 8ADDU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x2C010203);
    mmix.set_register(2, 7);
    mmix.set_register(3, 9);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 65); // 8*7 + 9 = 65

    // 16ADDU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x2E010203);
    mmix.set_register(2, 7);
    mmix.set_register(3, 9);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 121); // 16*7 + 9 = 121

    // SUB $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x24010203);
    mmix.set_register(2, 500);
    mmix.set_register(3, 120);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 380);

    // SUBU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x26010203);
    mmix.set_register(2, 5);
    mmix.set_register(3, 8);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), u64::MAX - 2); // 5 - 8 wraps

    // NEG $1, 0, $3 - effectively 0 - $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x34010003);
    mmix.set_register(3, 77);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1) as i64, -77);

    // NEGU $1, 0, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0x36010003);
    mmix.set_register(3, 77);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), u64::MAX - 76); // 0 - 77 wraps
}

#[test]
fn test_bitwise_operations() {
    let mut mmix = MMix::new();

    // AND: 0xFF & 0x0F = 0x0F
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0x0F);
    mmix.write_tetra(0, 0xC8030102); // AND $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x0F);

    // ANDI: 0xFF & 0x0F = 0x0F
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.write_tetra(0, 0xC903010F); // ANDI $3,$1,0x0F
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x0F);

    // OR: 0xF0 | 0x0F = 0xFF
    mmix.set_pc(0);
    mmix.set_register(1, 0xF0);
    mmix.set_register(2, 0x0F);
    mmix.write_tetra(0, 0xC0030102); // OR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFF);

    // ORI: 0xF0 | 0x0F = 0xFF
    mmix.set_pc(0);
    mmix.set_register(1, 0xF0);
    mmix.write_tetra(0, 0xC103010F); // ORI $3,$1,0x0F
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFF);

    // XOR: 0xFF ^ 0xAA = 0x55
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0xAA);
    mmix.write_tetra(0, 0xC6030102); // XOR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x55);

    // XORI: 0xFF ^ 0xAA = 0x55
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.write_tetra(0, 0xC70301AA); // XORI $3,$1,0xAA
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x55);

    // ANDN: 0xFF & !0x0F = 0xF0
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0x0F);
    mmix.write_tetra(0, 0xCA030102); // ANDN $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xF0);

    // ANDNI: 0xFF & !0x0F = 0xF0
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.write_tetra(0, 0xCB03010F); // ANDNI $3,$1,0x0F
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xF0);

    // ORN: 0x00 | !0x0F = 0xFFFFFFFFFFFFFFF0
    mmix.set_pc(0);
    mmix.set_register(1, 0x00);
    mmix.set_register(2, 0x0F);
    mmix.write_tetra(0, 0xC2030102); // ORN $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFF0);

    // ORNI: 0x00 | !0x0F = 0xFFFFFFFFFFFFFFF0
    mmix.set_pc(0);
    mmix.set_register(1, 0x00);
    mmix.write_tetra(0, 0xC303010F); // ORNI $3,$1,0x0F
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFF0);

    // NAND: !(0xFF & 0xFF) = 0xFFFFFFFFFFFFFF00
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0xFF);
    mmix.write_tetra(0, 0xCC030102); // NAND $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFF00);

    // NANDI: !(0xFF & 0xFF) = 0xFFFFFFFFFFFFFF00
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.write_tetra(0, 0xCD0301FF); // NANDI $3,$1,0xFF
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFF00);

    // NOR: !(0x00 | 0x00) = 0xFFFFFFFFFFFFFFFF
    mmix.set_pc(0);
    mmix.set_register(1, 0x00);
    mmix.set_register(2, 0x00);
    mmix.write_tetra(0, 0xC4030102); // NOR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

    // NORI: !(0x00 | 0x00) = 0xFFFFFFFFFFFFFFFF
    mmix.set_pc(0);
    mmix.set_register(1, 0x00);
    mmix.write_tetra(0, 0xC5030100); // NORI $3,$1,0x00
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

    // NXOR: !(0xFF ^ 0xFF) = 0xFFFFFFFFFFFFFFFF
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0xFF);
    mmix.write_tetra(0, 0xCE030102); // NXOR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

    // NXORI: !(0xFF ^ 0xFF) = 0xFFFFFFFFFFFFFFFF
    mmix.set_pc(0);
    mmix.set_register(1, 0xFF);
    mmix.write_tetra(0, 0xCF0301FF); // NXORI $3,$1,0xFF
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

    // MUX: mask=0xF0, Y=0xFF, Z=0x00 -> (0xFF & 0xF0) | (0x00 & !0xF0) = 0xF0
    mmix.set_pc(0);
    mmix.set_special(SpecialReg::RM, 0xF0);
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0x00);
    mmix.write_tetra(0, 0xD8030102); // MUX $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xF0);

    // MUXI: mask=0xAA, Y=0xFF, Z=0x55 -> (0xFF & 0xAA) | (0x55 & !0xAA) = 0xFF
    mmix.set_pc(0);
    mmix.set_special(SpecialReg::RM, 0xAA);
    mmix.set_register(1, 0xFF);
    mmix.write_tetra(0, 0xD9030155); // MUXI $3,$1,0x55
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFF);
}

#[test]
fn test_bdif() {
    let mut mmix = MMix::new();
    // BDIF: byte difference - each byte independently
    mmix.set_register(1, 0xFF20_3040_5060_7080);
    mmix.set_register(2, 0x1010_1010_1010_1010);
    mmix.write_tetra(0, 0xD0030102); // BDIF $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xEF10_2030_4050_6070);
}

#[test]
fn test_bdifi() {
    let mut mmix = MMix::new();
    // BDIFI: byte difference immediate
    mmix.set_register(1, 0x2020_2020_2020_2020);
    mmix.write_tetra(0, 0xD1030110); // BDIFI $3,$1,0x10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x1010_1010_1010_1010);
}

#[test]
fn test_wdif() {
    let mut mmix = MMix::new();
    // WDIF: wyde difference
    mmix.set_register(1, 0xFFFF_2000_3000_4000);
    mmix.set_register(2, 0x1000_1000_1000_1000);
    mmix.write_tetra(0, 0xD2030102); // WDIF $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xEFFF_1000_2000_3000);
}

#[test]
fn test_wdifi() {
    let mut mmix = MMix::new();
    // WDIFI: wyde difference immediate
    mmix.set_register(1, 0x1000_2000_3000_4000);
    mmix.write_tetra(0, 0xD3030105); // WDIFI $3,$1,5
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x0FFB_1FFB_2FFB_3FFB);
}

#[test]
fn test_tdif() {
    let mut mmix = MMix::new();
    // TDIF: tetra difference
    mmix.set_register(1, 0xFFFFFFFF_20000000);
    mmix.set_register(2, 0x10000000_10000000);
    mmix.write_tetra(0, 0xD4030102); // TDIF $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xEFFFFFFF_10000000);
}

#[test]
fn test_tdifi() {
    let mut mmix = MMix::new();
    // TDIFI: tetra difference immediate
    mmix.set_register(1, 0x10000000_20000000);
    mmix.write_tetra(0, 0xD503010A); // TDIFI $3,$1,10
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x0FFFFFF6_1FFFFFF6);
}

#[test]
fn test_odif() {
    let mut mmix = MMix::new();
    // ODIF: octa difference (unsigned)
    mmix.set_register(1, 1000);
    mmix.set_register(2, 300);
    mmix.write_tetra(0, 0xD6030102); // ODIF $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 700);

    // Test clipping to zero
    mmix.set_pc(0);
    mmix.set_register(1, 100);
    mmix.set_register(2, 500);
    mmix.write_tetra(0, 0xD6030102); // ODIF $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

#[test]
fn test_odifi() {
    let mut mmix = MMix::new();
    // ODIFI: octa difference immediate
    mmix.set_register(1, 255);
    mmix.write_tetra(0, 0xD70301FF); // ODIFI $3,$1,255
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

#[test]
fn test_sadd() {
    let mut mmix = MMix::new();
    // SADD: sideways add (population count of Y \ Z)
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 0x0F);
    mmix.write_tetra(0, 0xDA030102); // SADD $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 4); // 0xFF & !0x0F = 0xF0 has 4 ones
}

#[test]
fn test_saddi_population_count() {
    let mut mmix = MMix::new();
    // SADDI with Z=0 gives population count
    mmix.set_register(1, 0b10101010);
    mmix.write_tetra(0, 0xDB030100); // SADDI $3,$1,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 4); // 4 ones in 10101010
}

#[test]
fn test_mor() {
    let mut mmix = MMix::new();
    // MOR: multiple or (Boolean matrix multiplication)
    // Example: byte reversal with Z = 0x0102040810204080
    mmix.set_register(1, 0x0123456789ABCDEF);
    mmix.set_register(2, 0x0102040810204080);
    mmix.write_tetra(0, 0xDC030102); // MOR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xEFCDAB8967452301); // byte-reversed
}

#[test]
fn test_mori() {
    let mut mmix = MMix::new();
    // MORI: multiple or immediate
    mmix.set_register(1, 0xFF00FF00FF00FF00);
    mmix.write_tetra(0, 0xDD0301FF); // MORI $3,$1,255
    assert!(mmix.execute_instruction());
    // Result should be in bottom byte
    assert_eq!(mmix.get_register(3) & 0xFF, 0xFF);
}

#[test]
fn test_mxor() {
    let mut mmix = MMix::new();
    // MXOR: multiple exclusive-or (matrix product over GF(2))
    // Simple test: identity matrix behavior
    mmix.set_register(1, 0x00);
    mmix.set_register(2, 0x00);
    mmix.write_tetra(0, 0xDE030102); // MXOR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

#[test]
fn test_mxori() {
    let mut mmix = MMix::new();
    // MXORI: multiple exclusive-or immediate
    mmix.set_register(1, 0x00);
    mmix.write_tetra(0, 0xDF030100); // MXORI $3,$1,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

// Shift instruction tests
#[test]
fn test_sl() {
    let mut mmix = MMix::new();
    // SL: shift left - 0xFF << 4 = 0xFF0
    mmix.set_register(1, 0xFF);
    mmix.set_register(2, 4);
    mmix.write_tetra(0, 0x38030102); // SL $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFF0);
}

#[test]
fn test_sli() {
    let mut mmix = MMix::new();
    // SLI: shift left immediate - 0x123 << 8 = 0x12300
    mmix.set_register(1, 0x123);
    mmix.write_tetra(0, 0x39030108); // SLI $3,$1,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x12300);
}

#[test]
fn test_sl_overflow() {
    let mut mmix = MMix::new();
    // SL with overflow: shifting out non-sign bits sets overflow
    mmix.set_register(1, 0x8000_0000_0000_0000);
    mmix.set_register(2, 1);
    mmix.write_tetra(0, 0x38030102); // SL $3,$1,$2
    assert!(mmix.execute_instruction());
    // Check that overflow bit is set in rA
    assert!((mmix.get_special(SpecialReg::RA) & RA_V) != 0);
}

#[test]
fn test_sl_large_shift() {
    let mut mmix = MMix::new();
    // SL with shift >= 64 results in 0
    mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
    mmix.set_register(2, 64);
    mmix.write_tetra(0, 0x38030102); // SL $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

#[test]
fn test_slu() {
    let mut mmix = MMix::new();
    // SLU: shift left unsigned - no overflow check
    mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
    mmix.set_register(2, 8);
    mmix.write_tetra(0, 0x3A030102); // SLU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFF_FFFF_FFFF_FF00);
}

#[test]
fn test_slui() {
    let mut mmix = MMix::new();
    // SLUI: shift left unsigned immediate
    mmix.set_register(1, 0x1);
    mmix.write_tetra(0, 0x3B030110); // SLUI $3,$1,16
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x10000);
}

#[test]
fn test_sr() {
    let mut mmix = MMix::new();
    // SR: arithmetic shift right - negative number stays negative
    mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFF0u64); // -16 as u64
    mmix.set_register(2, 4);
    mmix.write_tetra(0, 0x3C030102); // SR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFF_FFFF_FFFF_FFFFu64); // -1 as u64
}

#[test]
fn test_sri() {
    let mut mmix = MMix::new();
    // SRI: arithmetic shift right immediate - positive number
    mmix.set_register(1, 0x1000);
    mmix.write_tetra(0, 0x3D030104); // SRI $3,$1,4
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x100);
}

#[test]
fn test_sr_large_shift_negative() {
    let mut mmix = MMix::new();
    // SR with large shift on negative number results in -1
    mmix.set_register(1, 0x8000_0000_0000_0000);
    mmix.set_register(2, 100);
    mmix.write_tetra(0, 0x3C030102); // SR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0xFFFF_FFFF_FFFF_FFFF);
}

#[test]
fn test_sr_large_shift_positive() {
    let mut mmix = MMix::new();
    // SR with large shift on positive number results in 0
    mmix.set_register(1, 0x7FFF_FFFF_FFFF_FFFF);
    mmix.set_register(2, 100);
    mmix.write_tetra(0, 0x3C030102); // SR $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

#[test]
fn test_sru() {
    let mut mmix = MMix::new();
    // SRU: logical shift right - fills with zeros
    mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
    mmix.set_register(2, 4);
    mmix.write_tetra(0, 0x3E030102); // SRU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x0FFF_FFFF_FFFF_FFFF);
}

#[test]
fn test_srui() {
    let mut mmix = MMix::new();
    // SRUI: logical shift right immediate
    mmix.set_register(1, 0x8000_0000_0000_0000);
    mmix.write_tetra(0, 0x3F030101); // SRUI $3,$1,1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x4000_0000_0000_0000);
}

#[test]
fn test_sru_large_shift() {
    let mut mmix = MMix::new();
    // SRU with shift >= 64 results in 0
    mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
    mmix.set_register(2, 64);
    mmix.write_tetra(0, 0x3E030102); // SRU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
}

#[test]
fn incl_adds_its_immediate() {
    let source = "\
\tLOC\t#100
Main\tSETI\t$1,100
\tINCL\t$1,#203
\tSET\t$255,$1
\tTRAP\t0,Halt,0
";
    assert_eq!(run_to_halt(source), 100 + 0x203);
}

// ============== Subnormal operands and results ==============

#[test]
fn test_subnormal_operand_raises_no_divide_check() {
    let mut mmix = MMix::new();
    // MMIX has no denormalized-operand event; D is the integer divide check.
    mmix.set_register(2, f64::MIN_POSITIVE.to_bits() >> 4); // subnormal
    mmix.set_register(3, 1.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203); // FADD
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_D, 0);
}

#[test]
fn test_exact_subnormal_result_raises_no_underflow() {
    let mut mmix = MMix::new();
    // MIN_POSITIVE / 2.0 is an exact subnormal: halving a power of two
    // loses nothing, so U must not fire.
    mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
    mmix.set_register(3, 2.0f64.to_bits());
    mmix.write_tetra(0, 0x14010203); // FDIV
    assert!(mmix.execute_instruction());
    let ra = mmix.get_special(SpecialReg::RA);
    let r = f64::from_bits(mmix.get_register(1));
    assert!(r.is_subnormal(), "expected subnormal result, got {}", r);
    assert_eq!(ra & RA_U, 0, "an exact subnormal result raises no U");
}

#[test]
fn test_subnormal_plus_zero_raises_no_underflow() {
    let mut mmix = MMix::new();
    // A subnormal result that merely reproduces a subnormal operand lost
    // nothing; the nonzero-operand guard keeps U off it.
    mmix.set_register(2, 1u64); // smallest positive subnormal
    mmix.set_register(3, 0.0f64.to_bits());
    mmix.write_tetra(0, 0x04010203); // FADD
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1u64);
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_U, 0);
}

/// Run the signed division `word` encodes over $1 and the divisor,
/// returning quotient, remainder and rA. The register form reads the
/// divisor from $2, the immediate form from the instruction's Z field.
fn run_signed_div(word: u32, dividend: i64, divisor: i64) -> (i64, i64, u64) {
    let mut mmix = MMix::new();
    mmix.set_register(1, dividend as u64);
    mmix.set_register(2, divisor as u64);
    mmix.write_tetra(0, word);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4, "division must advance the PC");
    (
        mmix.get_register(3) as i64,
        mmix.get_special(SpecialReg::RR) as i64,
        mmix.get_special(SpecialReg::RA),
    )
}

/// Run `DIV $3,$1,$2` on the given operands.
fn run_div(dividend: i64, divisor: i64) -> (i64, i64, u64) {
    run_signed_div(0x1C030102, dividend, divisor)
}

/// Run `DIVI $3,$1,Z`; Z is a byte, so the divisor is in `0..=255`.
fn run_divi(dividend: i64, divisor: u8) -> (i64, i64, u64) {
    run_signed_div(0x1D030100 | divisor as u32, dividend, 0)
}

#[test]
fn test_div_floors_toward_negative_infinity() {
    // Truncation would give (-3, -1), (-3, 1) and (3, 1).
    assert_eq!(run_div(-7, 2), (-4, 1, 0));
    assert_eq!(run_div(7, -2), (-4, -1, 0));
    assert_eq!(run_div(-7, -2), (3, -1, 0));
    // Exact division is unaffected by the floor adjustment.
    assert_eq!(run_div(-8, 2), (-4, 0, 0));
    assert_eq!(run_div(7, 2), (3, 1, 0));
}

#[test]
fn test_div_min_by_minus_one_wraps_and_raises_v() {
    let (quotient, remainder, ra) = run_div(i64::MIN, -1);
    assert_eq!(quotient as u64, 0x8000000000000000);
    assert_eq!(remainder, 0);
    assert_eq!(ra & RA_V, RA_V);
}

#[test]
fn test_divi_floors_toward_negative_infinity() {
    // Only positive divisors are encodable; truncation would give (-3, -1),
    // (-2, -1) and (3, 1).
    assert_eq!(run_divi(-7, 2), (-4, 1, 0));
    assert_eq!(run_divi(-7, 3), (-3, 2, 0));
    // Exact division is unaffected by the floor adjustment.
    assert_eq!(run_divi(-8, 2), (-4, 0, 0));
    assert_eq!(run_divi(7, 2), (3, 1, 0));
}

#[test]
fn test_divi_by_zero_raises_divide_check() {
    let (quotient, remainder, ra) = run_divi(42, 0);
    assert_eq!(quotient, 0);
    assert_eq!(remainder, 42);
    assert_eq!(ra & RA_D, RA_D, "D is the integer divide check");
    assert_eq!(ra & RA_V, 0, "divide by zero is not an overflow");
}

#[test]
fn test_signed_divide_by_zero_raises_divide_check() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 42);
    mmix.set_register(2, 0);
    mmix.write_tetra(0, 0x1C030102); // DIV $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
    assert_eq!(mmix.get_special(SpecialReg::RR), 42);
    let ra = mmix.get_special(SpecialReg::RA);
    assert_eq!(ra & RA_D, RA_D, "D is the integer divide check");
    assert_eq!(ra & RA_V, 0, "divide by zero is not an overflow");
}

#[test]
fn test_divu_uses_rd_when_divisor_is_not_greater() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RD, 5);
    mmix.set_register(1, 0x1234);
    mmix.set_register(2, 3);
    mmix.write_tetra(0, 0x1E030102); // DIVU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 5);
    assert_eq!(mmix.get_special(SpecialReg::RR), 0x1234);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_divu_by_zero_is_the_rd_rule_and_raises_nothing() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RD, 7);
    mmix.set_register(1, 0xABCD);
    mmix.set_register(2, 0);
    mmix.write_tetra(0, 0x1E030102); // DIVU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 7);
    assert_eq!(mmix.get_special(SpecialReg::RR), 0xABCD);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0, "DIVU has no D");
}

#[test]
fn test_divu_divides_the_full_128_bit_dividend() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RD, 1);
    mmix.set_register(1, 0);
    mmix.set_register(2, 2);
    mmix.write_tetra(0, 0x1E030102); // DIVU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x8000000000000000); // 2^64 / 2
    assert_eq!(mmix.get_special(SpecialReg::RR), 0);
}

#[test]
fn test_divui_uses_rd_when_divisor_is_not_greater() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RD, 5);
    mmix.set_register(1, 0x1234);
    mmix.write_tetra(0, 0x1F030103); // DIVUI $3,$1,3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 5);
    assert_eq!(mmix.get_special(SpecialReg::RR), 0x1234);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_divui_by_zero_is_the_rd_rule_and_raises_nothing() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RD, 7);
    mmix.set_register(1, 0xABCD);
    mmix.write_tetra(0, 0x1F030100); // DIVUI $3,$1,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 7);
    assert_eq!(mmix.get_special(SpecialReg::RR), 0xABCD);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0, "DIVUI has no D");
}

#[test]
fn test_divui_divides_the_full_128_bit_dividend() {
    let mut mmix = MMix::new();
    mmix.set_special(SpecialReg::RD, 1);
    mmix.set_register(1, 0);
    mmix.write_tetra(0, 0x1F030102); // DIVUI $3,$1,2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x8000000000000000); // 2^64 / 2
    assert_eq!(mmix.get_special(SpecialReg::RR), 0);
}

#[test]
fn test_mul_leaves_rh_to_mulu() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 1 << 32);
    mmix.set_register(2, 1 << 32);
    mmix.write_tetra(0, 0x1A030102); // MULU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0);
    assert_eq!(mmix.get_special(SpecialReg::RH), 1);

    mmix.set_register(5, 6);
    mmix.set_register(6, 7);
    mmix.write_tetra(4, 0x18040506); // MUL $4,$5,$6
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(4), 42);
    assert_eq!(mmix.get_special(SpecialReg::RH), 1, "rH is MULU's output");
}

#[test]
fn test_mul_overflow_raises_v() {
    let mut mmix = MMix::new();
    mmix.set_register(1, i64::MAX as u64);
    mmix.set_register(2, 2);
    mmix.write_tetra(0, 0x18030102); // MUL $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_special(SpecialReg::RA) & RA_V, RA_V);
    assert_eq!(mmix.get_special(SpecialReg::RH), 0);
}

#[test]
fn test_add_overflow_raises_v_not_u() {
    let mut mmix = MMix::new();
    mmix.set_register(1, i64::MAX as u64);
    mmix.set_register(2, 1);
    mmix.write_tetra(0, 0x20030102); // ADD $3,$1,$2
    assert!(mmix.execute_instruction());
    let ra = mmix.get_special(SpecialReg::RA);
    assert_eq!(ra & RA_V, RA_V, "integer overflow is V");
    assert_eq!(ra & RA_U, 0, "U is floating underflow");
}

/// Run the signed left shift `word` encodes over $1, returning the result
/// and rA. The register form reads the count from $2, the immediate form
/// from the instruction's Z field.
fn run_shift_left(word: u32, value: u64, shift: u64) -> (u64, u64) {
    let mut mmix = MMix::new();
    mmix.set_register(1, value);
    mmix.set_register(2, shift);
    mmix.write_tetra(0, word);
    assert!(mmix.execute_instruction());
    (mmix.get_register(3), mmix.get_special(SpecialReg::RA))
}

/// Run `SL $3,$1,$2`.
fn run_sl(value: u64, shift: u64) -> (u64, u64) {
    run_shift_left(0x38030102, value, shift)
}

/// Run `SLI $3,$1,Z`.
fn run_sli(value: u64, shift: u8) -> (u64, u64) {
    run_shift_left(0x39030100 | shift as u32, value, 0)
}

#[test]
fn test_sl_overflows_when_the_product_leaves_the_signed_range() {
    // A set bit leaves the top: 2^62 · 4 = 2^64.
    let (result, ra) = run_sl(1 << 62, 2);
    assert_eq!(result, 0);
    assert_eq!(ra & RA_V, RA_V);

    // Nothing is shifted out, yet 2^62 · 2 = 2^63 exceeds the signed range.
    let (result, ra) = run_sl(0x4000000000000000, 1);
    assert_eq!(result, 0x8000000000000000);
    assert_eq!(ra & RA_V, RA_V, "the sign flip is an overflow");

    // 2^60 · 4 = 2^62 fits, so nothing is raised.
    let (result, ra) = run_sl(1 << 60, 2);
    assert_eq!(result, 1 << 62);
    assert_eq!(ra, 0, "a representable product raises nothing");

    // -1 · 2^63 = -2^63 is the most negative octabyte, and fits.
    let (result, ra) = run_sl(u64::MAX, 63);
    assert_eq!(result, 0x8000000000000000);
    assert_eq!(ra, 0);
}

#[test]
fn test_sli_overflows_when_the_product_leaves_the_signed_range() {
    // A set bit leaves the top: 2^62 · 4 = 2^64.
    let (result, ra) = run_sli(1 << 62, 2);
    assert_eq!(result, 0);
    assert_eq!(ra & RA_V, RA_V);

    // Nothing is shifted out, yet 2^62 · 2 = 2^63 exceeds the signed range.
    let (result, ra) = run_sli(0x4000000000000000, 1);
    assert_eq!(result, 0x8000000000000000);
    assert_eq!(ra & RA_V, RA_V, "the sign flip is an overflow");

    // 2^60 · 4 = 2^62 fits, so nothing is raised.
    let (result, ra) = run_sli(1 << 60, 2);
    assert_eq!(result, 1 << 62);
    assert_eq!(ra, 0, "a representable product raises nothing");

    // A shift of zero is the identity, whatever the sign bits hold.
    let (result, ra) = run_sli(0x8000000000000000, 0);
    assert_eq!(result, 0x8000000000000000);
    assert_eq!(ra, 0);
}

#[test]
fn test_slu_never_overflows() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 0x4000000000000000);
    mmix.set_register(2, 1);
    mmix.write_tetra(0, 0x3A030102); // SLU $3,$1,$2
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x8000000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}

#[test]
fn test_neg_overflow_raises_v() {
    // NEG $3,Y,$1 computes Y - s($1); every such overflow reports V.
    for y in [0u32, 5, 255] {
        let mut mmix = MMix::new();
        mmix.set_register(1, i64::MIN as u64);
        mmix.write_tetra(0, 0x34030001 | (y << 8)); // NEG $3,y,$1
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RA) & RA_V,
            RA_V,
            "NEG $3,{y},$1 overflows"
        );
        assert_eq!(
            mmix.get_register(3),
            (y as i64).wrapping_sub(i64::MIN) as u64
        );
    }
}

#[test]
fn test_negu_has_no_overflow() {
    let mut mmix = MMix::new();
    mmix.set_register(1, i64::MIN as u64);
    mmix.write_tetra(0, 0x36030001); // NEGU $3,0,$1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(3), 0x8000000000000000);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
}
