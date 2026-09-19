//! Sparse memory, the journal, and the LOAD/STORE opcode family.

use super::super::*;
use super::*;

#[test]
fn test_memory_byte() {
    let mut mmix = MMix::new();
    mmix.write_byte(0x1000, 0x42);
    assert_eq!(mmix.read_byte(0x1000), 0x42);
    assert_eq!(mmix.read_byte(0x1001), 0);
}

#[test]
fn test_memory_wyde() {
    let mut mmix = MMix::new();
    mmix.write_wyde(0x1000, 0x1234);
    assert_eq!(mmix.read_wyde(0x1000), 0x1234);
    assert_eq!(mmix.read_byte(0x1000), 0x12);
    assert_eq!(mmix.read_byte(0x1001), 0x34);
}

#[test]
fn test_memory_tetra() {
    let mut mmix = MMix::new();
    mmix.write_tetra(0x1000, 0x12345678);
    assert_eq!(mmix.read_tetra(0x1000), 0x12345678);
}

#[test]
fn test_memory_octa() {
    let mut mmix = MMix::new();
    mmix.write_octa(0x1000, 0x123456789ABCDEF0);
    assert_eq!(mmix.read_octa(0x1000), 0x123456789ABCDEF0);
}

#[test]
fn test_fetch_instruction() {
    let mut mmix = MMix::new();
    // Store instruction #20010203 (ADD $1, $2, $3)
    mmix.write_tetra(0, 0x20010203);
    let (op, x, y, z) = mmix.fetch_instruction();
    assert_eq!(op, 0x20);
    assert_eq!(x, 0x01);
    assert_eq!(y, 0x02);
    assert_eq!(z, 0x03);
}

#[test]
fn test_sparse_memory() {
    let mut mmix = MMix::new();
    mmix.write_byte(0x1000, 0x42);
    mmix.write_byte(0x1000, 0); // Writing zero should remove it
    assert_eq!(mmix.memory.len(), 0);
}

// Load instruction tests

#[test]
fn test_ldb_signed_positive() {
    let mut mmix = MMix::new();
    // LDB $1, $2, $3 - Load signed byte (positive)
    mmix.write_tetra(0, 0x80010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_byte(150, 127); // Max positive signed byte

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 127);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldb_signed_negative() {
    let mut mmix = MMix::new();
    // LDB $1, $2, $3 - Load signed byte (negative)
    mmix.write_tetra(0, 0x80010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_byte(150, 0xFF); // -1 in signed byte

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -1);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldb_immediate() {
    let mut mmix = MMix::new();
    // LDB $1, $2, 10 - Load signed byte with immediate offset
    mmix.write_tetra(0, 0x8101020A);
    mmix.set_register(2, 100);
    mmix.write_byte(110, 0x80); // -128 in signed byte

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -128);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldbu_unsigned() {
    let mut mmix = MMix::new();
    // LDBU $1, $2, $3 - Load unsigned byte
    mmix.write_tetra(0, 0x82010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_byte(150, 0xFF); // 255 unsigned

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 255);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldbu_immediate() {
    let mut mmix = MMix::new();
    // LDBU $1, $2, 20 - Load unsigned byte with immediate
    mmix.write_tetra(0, 0x83010214);
    mmix.set_register(2, 100);
    mmix.write_byte(120, 200);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 200);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldw_signed_positive() {
    let mut mmix = MMix::new();
    // LDW $1, $2, $3 - Load signed wyde (positive)
    mmix.write_tetra(0, 0x84010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_wyde(150, 32767); // Max positive signed wyde

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 32767);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldw_signed_negative() {
    let mut mmix = MMix::new();
    // LDW $1, $2, $3 - Load signed wyde (negative)
    mmix.write_tetra(0, 0x84010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_wyde(150, 0xFFFF); // -1 in signed wyde

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -1);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldw_immediate() {
    let mut mmix = MMix::new();
    // LDW $1, $2, 10 - Load signed wyde with immediate
    mmix.write_tetra(0, 0x8501020A);
    mmix.set_register(2, 100);
    mmix.write_wyde(110, 0x8000); // -32768 in signed wyde

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -32768);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldwu_unsigned() {
    let mut mmix = MMix::new();
    // LDWU $1, $2, $3 - Load unsigned wyde
    mmix.write_tetra(0, 0x86010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_wyde(150, 0xFFFF); // 65535 unsigned

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 65535);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldwu_immediate() {
    let mut mmix = MMix::new();
    // LDWU $1, $2, 30 - Load unsigned wyde with immediate
    mmix.write_tetra(0, 0x8701021E);
    mmix.set_register(2, 100);
    mmix.write_wyde(130, 50000);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 50000);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldt_signed_positive() {
    let mut mmix = MMix::new();
    // LDT $1, $2, $3 - Load signed tetra (positive)
    mmix.write_tetra(0, 0x88010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_tetra(150, 2_147_483_647); // Max positive signed tetra

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 2_147_483_647);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldt_signed_negative() {
    let mut mmix = MMix::new();
    // LDT $1, $2, $3 - Load signed tetra (negative)
    mmix.write_tetra(0, 0x88010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_tetra(150, 0xFFFFFFFF); // -1 in signed tetra

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -1);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldt_immediate() {
    let mut mmix = MMix::new();
    // LDT $1, $2, 20 - Load signed tetra with immediate
    mmix.write_tetra(0, 0x89010214);
    mmix.set_register(2, 100);
    mmix.write_tetra(120, 0x80000000); // -2147483648 in signed tetra

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1) as i64, -2147483648);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldtu_unsigned() {
    let mut mmix = MMix::new();
    // LDTU $1, $2, $3 - Load unsigned tetra
    mmix.write_tetra(0, 0x8A010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_tetra(150, 0xFFFFFFFF); // 4294967295 unsigned

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 4294967295);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldtu_immediate() {
    let mut mmix = MMix::new();
    // LDTU $1, $2, 40 - Load unsigned tetra with immediate
    mmix.write_tetra(0, 0x8B010228);
    mmix.set_register(2, 100);
    mmix.write_tetra(140, 3_000_000_000);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 3_000_000_000);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldo_load_octa() {
    let mut mmix = MMix::new();
    // LDO $1, $2, $3 - Load octa
    mmix.write_tetra(0, 0x8C010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_octa(150, 0x123456789ABCDEF0);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x123456789ABCDEF0);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldo_immediate() {
    let mut mmix = MMix::new();
    // LDO $1, $2, 16 - Load octa with immediate
    mmix.write_tetra(0, 0x8D010210);
    mmix.set_register(2, 100);
    mmix.write_octa(116, 0xFEDCBA9876543210);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0xFEDCBA9876543210);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldou_same_as_ldo() {
    let mut mmix = MMix::new();
    // LDOU $1, $2, $3 - Load octa unsigned (same as LDO)
    mmix.write_tetra(0, 0x8E010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_octa(150, 0x123456789ABCDEF0);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x123456789ABCDEF0);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldou_immediate() {
    let mut mmix = MMix::new();
    // LDOU $1, $2, 8 - Load octa unsigned with immediate
    mmix.write_tetra(0, 0x8F010208);
    mmix.set_register(2, 1000);
    mmix.write_octa(1008, 0xFFFFFFFFFFFFFFFF);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0xFFFFFFFFFFFFFFFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldsf_short_float() {
    let mut mmix = MMix::new();
    // LDSF $1, $2, $3 - Load short float (32-bit to 64-bit)
    mmix.write_tetra(0, 0x90010203);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    // Write a 32-bit float (e.g., 3.14159 in IEEE 754 single precision)
    let float_val = std::f32::consts::PI;
    mmix.write_tetra(150, float_val.to_bits());

    mmix.execute_instruction();
    // Should be converted to 64-bit float
    let result_f64 = MMix::u64_to_f64(mmix.get_register(1));
    assert!((result_f64 - std::f64::consts::PI).abs() < 0.0001);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_ldsfi_immediate() {
    let mut mmix = MMix::new();
    // LDSFI $1, $2, 12 - Load short float with immediate offset
    mmix.write_tetra(0, 0x9101020C);
    mmix.set_register(2, 200);
    // Write a 32-bit float (e.g., -2.5 in IEEE 754 single precision)
    let float_val = -2.5f32;
    mmix.write_tetra(212, float_val.to_bits());

    mmix.execute_instruction();
    let result_f64 = MMix::u64_to_f64(mmix.get_register(1));
    assert_eq!(result_f64, -2.5);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_lda_load_address() {
    let mut mmix = MMix::new();
    // LDA $1, $2, $3 - Load address (same as ADDU)
    mmix.write_tetra(0, 0x22010203);
    mmix.set_register(2, 0x1000);
    mmix.set_register(3, 0x500);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x1500);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_lda_immediate() {
    let mut mmix = MMix::new();
    // LDA $1, $2, 64 - Load address with immediate (same as ADDU)
    mmix.write_tetra(0, 0x23010240);
    mmix.set_register(2, 0x2000);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x2040);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_load_from_uninitialized_memory() {
    let mut mmix = MMix::new();
    // LDBU $1, $0, 100 - Load from uninitialized memory (should be 0)
    mmix.write_tetra(0, 0x83010064);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_load_address_wraparound() {
    let mut mmix = MMix::new();
    // LDA $1, $2, $3 - Test address wraparound
    mmix.write_tetra(0, 0x22010203);
    mmix.set_register(2, u64::MAX - 100);
    mmix.set_register(3, 200);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 99); // Wraps around
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_wyde_set_replaces_whole_register() {
    // SETH, SETMH, SETML, SETL each deposit YZ in one wyde and zero the
    // other 48 bits. The all-ones preload is what distinguishes a set
    // from a merge.
    for (opcode, shift) in [(0xE0_u32, 48_u32), (0xE1, 32), (0xE2, 16), (0xE3, 0)] {
        let mut mmix = MMix::new();
        mmix.set_register(1, u64::MAX);
        mmix.write_tetra(0, (opcode << 24) | (1 << 16) | 0xABCD);

        mmix.execute_instruction();

        let reg = mmix.get_register(1);
        assert_eq!((reg >> shift) & 0xFFFF, 0xABCD, "opcode {opcode:#04X} wyde");
        assert_eq!(
            reg & !(0xFFFF_u64 << shift),
            0,
            "opcode {opcode:#04X} residue"
        );
        assert_eq!(mmix.get_pc(), 4);
    }
}

#[test]
fn test_setl_zero_clears_whole_register() {
    let mut mmix = MMix::new();
    mmix.set_register(1, u64::MAX);
    // SETL $1, 0 - a one-instruction register clear
    mmix.write_tetra(0, 0xE3010000);

    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0);
}

/// Assemble `source`, load its image and execute `steps` instructions
/// from the first assembled address.
fn assemble_and_run(source: &str, steps: usize) -> MMix {
    use crate::debugger::write_image;
    use crate::mmixal::MMixAssembler;

    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().expect("test source must assemble");
    let start = asm.instructions[0].0;

    let mut mmix = MMix::new();
    write_image(&mut mmix, &asm);
    mmix.set_pc(start);
    for _ in 0..steps {
        mmix.execute_instruction();
    }
    mmix
}

#[test]
fn test_seti_lands_a_wide_constant() {
    let mmix = assemble_and_run("SETI $1,#0123456789ABCDEF", 4);
    assert_eq!(mmix.get_register(1), 0x0123456789ABCDEF);
}

#[test]
fn test_lda_large_address_lands_whole_address() {
    // LDA $X,Label above #FF expands to the same four tetras SETI uses;
    // examples/hello_world.mms gets its string pointer this way.
    let mmix = assemble_and_run("\tLOC\t#2000000000001234\nMain\tLDA\t$255,Main\n", 4);
    assert_eq!(mmix.get_register(255), 0x2000_0000_0000_1234);
}

#[test]
fn test_inc_and_or_wydes_preserve_the_others() {
    let mut mmix = MMix::new();
    mmix.set_register(1, 0x1111_2222_3333_4444);

    // INCML $1, 0x0001
    mmix.write_tetra(0, 0xE6010001);
    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x1111_2222_3334_4444);

    // ORL $1, 0x000F
    mmix.set_pc(4);
    mmix.write_tetra(4, 0xEB01000F);
    mmix.execute_instruction();
    assert_eq!(mmix.get_register(1), 0x1111_2222_3334_444F);
}

// Store instruction tests

#[test]
fn test_stb_store_byte() {
    let mut mmix = MMix::new();
    // STB $1, $2, $3 - Store byte
    mmix.write_tetra(0, 0xA0010203);
    mmix.set_register(1, 0x42); // Value to store
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_byte(150), 0x42);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stb_immediate() {
    let mut mmix = MMix::new();
    // STB $1, $2, 10 - Store byte immediate
    mmix.write_tetra(0, 0xA101020A);
    mmix.set_register(1, 0x7F); // Max positive signed byte
    mmix.set_register(2, 200);

    mmix.execute_instruction();
    assert_eq!(mmix.read_byte(210), 0x7F);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stbu_store_byte_unsigned() {
    let mut mmix = MMix::new();
    // STBU $1, $2, $3 - Store byte unsigned
    mmix.write_tetra(0, 0xA2010203);
    mmix.set_register(1, 0xFF); // 255 unsigned
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_byte(150), 0xFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stbu_immediate() {
    let mut mmix = MMix::new();
    // STBU $1, $2, 20 - Store byte unsigned immediate
    mmix.write_tetra(0, 0xA3010214);
    mmix.set_register(1, 0xAB);
    mmix.set_register(2, 1000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_byte(1020), 0xAB);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stw_store_wyde() {
    let mut mmix = MMix::new();
    // STW $1, $2, $3 - Store wyde
    mmix.write_tetra(0, 0xA4010203);
    mmix.set_register(1, 0x1234);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_wyde(150), 0x1234);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stw_immediate() {
    let mut mmix = MMix::new();
    // STW $1, $2, 30 - Store wyde immediate
    mmix.write_tetra(0, 0xA501021E);
    mmix.set_register(1, 0x7FFF); // Max positive signed wyde
    mmix.set_register(2, 2000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_wyde(2030), 0x7FFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stwu_store_wyde_unsigned() {
    let mut mmix = MMix::new();
    // STWU $1, $2, $3 - Store wyde unsigned
    mmix.write_tetra(0, 0xA6010203);
    mmix.set_register(1, 0xFFFF); // 65535 unsigned
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_wyde(150), 0xFFFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stwu_immediate() {
    let mut mmix = MMix::new();
    // STWU $1, $2, 40 - Store wyde unsigned immediate
    mmix.write_tetra(0, 0xA7010228);
    mmix.set_register(1, 0xABCD);
    mmix.set_register(2, 5000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_wyde(5040), 0xABCD);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stt_store_tetra() {
    let mut mmix = MMix::new();
    // STT $1, $2, $3 - Store tetra
    mmix.write_tetra(0, 0xA8010203);
    mmix.set_register(1, 0x12345678);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_tetra(150), 0x12345678);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stt_immediate() {
    let mut mmix = MMix::new();
    // STT $1, $2, 50 - Store tetra immediate
    mmix.write_tetra(0, 0xA9010232);
    mmix.set_register(1, 0x7FFFFFFF); // Max positive signed tetra
    mmix.set_register(2, 10000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_tetra(10050), 0x7FFFFFFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sttu_store_tetra_unsigned() {
    let mut mmix = MMix::new();
    // STTU $1, $2, $3 - Store tetra unsigned
    mmix.write_tetra(0, 0xAA010203);
    mmix.set_register(1, 0xFFFFFFFF); // 4294967295 unsigned
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_tetra(150), 0xFFFFFFFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sttu_immediate() {
    let mut mmix = MMix::new();
    // STTU $1, $2, 60 - Store tetra unsigned immediate
    mmix.write_tetra(0, 0xAB01023C);
    mmix.set_register(1, 0xDEADBEEF);
    mmix.set_register(2, 20000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_tetra(20060), 0xDEADBEEF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sto_store_octa() {
    let mut mmix = MMix::new();
    // STO $1, $2, $3 - Store octa
    mmix.write_tetra(0, 0xAC010203);
    mmix.set_register(1, 0x123456789ABCDEF0);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_octa(150), 0x123456789ABCDEF0);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_sto_immediate() {
    let mut mmix = MMix::new();
    // STO $1, $2, 70 - Store octa immediate
    mmix.write_tetra(0, 0xAD010246);
    mmix.set_register(1, 0xFEDCBA9876543210);
    mmix.set_register(2, 30000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_octa(30070), 0xFEDCBA9876543210);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stou_same_as_sto() {
    let mut mmix = MMix::new();
    // STOU $1, $2, $3 - Store octa unsigned (same as STO)
    mmix.write_tetra(0, 0xAE010203);
    mmix.set_register(1, 0xFFFFFFFFFFFFFFFF);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_octa(150), 0xFFFFFFFFFFFFFFFF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stou_immediate() {
    let mut mmix = MMix::new();
    // STOU $1, $2, 80 - Store octa unsigned immediate
    mmix.write_tetra(0, 0xAF010250);
    mmix.set_register(1, 0x0123456789ABCDEF);
    mmix.set_register(2, 40000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_octa(40080), 0x0123456789ABCDEF);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stco_store_constant() {
    let mut mmix = MMix::new();
    // STCO 42, $2, $3 - Store constant octabyte
    mmix.write_tetra(0, 0xB42A0203); // X=42 (0x2A)
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_octa(150), 42);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stco_immediate() {
    let mut mmix = MMix::new();
    // STCO 255, $2, 90 - Store constant octabyte immediate
    mmix.write_tetra(0, 0xB5FF025A); // X=255 (0xFF)
    mmix.set_register(2, 50000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_octa(50090), 255);
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stht_store_high_tetra() {
    let mut mmix = MMix::new();
    // STHT $1, $2, $3 - Store high tetra
    mmix.write_tetra(0, 0xB2010203);
    mmix.set_register(1, 0xDEADBEEF12345678);
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);

    mmix.execute_instruction();
    assert_eq!(mmix.read_tetra(150), 0xDEADBEEF); // High 32 bits
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_stht_immediate() {
    let mut mmix = MMix::new();
    // STHT $1, $2, 100 - Store high tetra immediate
    mmix.write_tetra(0, 0xB3010264);
    mmix.set_register(1, 0xABCD123456789ABC);
    mmix.set_register(2, 60000);

    mmix.execute_instruction();
    assert_eq!(mmix.read_tetra(60100), 0xABCD1234); // High 32 bits
    assert_eq!(mmix.get_pc(), 4);
}

#[test]
fn test_store_and_load_roundtrip() {
    let mut mmix = MMix::new();
    let test_addr = 5000u64;

    // Store a value
    mmix.set_register(1, 0x123456789ABCDEF0);
    mmix.set_register(2, test_addr);
    mmix.write_tetra(0, 0xAD010200); // STO $1, $2, 0
    mmix.execute_instruction();

    // Load it back
    mmix.set_pc(4);
    mmix.write_tetra(4, 0x8D030200); // LDO $3, $2, 0
    mmix.execute_instruction();

    assert_eq!(mmix.get_register(3), 0x123456789ABCDEF0);
}

#[test]
fn test_all_store_instructions_have_tests() {
    // Verify all store instructions are covered
    let mut mmix = MMix::new();

    // STB $1, $2, $3
    mmix.write_tetra(0, 0xA0010203);
    mmix.set_register(1, 0x5A);
    mmix.set_register(2, 300);
    mmix.set_register(3, 12);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_byte(312), 0x5A);

    // STBU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xA2010203);
    mmix.set_register(1, 0xE1);
    mmix.set_register(2, 300);
    mmix.set_register(3, 13);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_byte(313), 0xE1);

    // STW $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xA4010203);
    mmix.set_register(1, 0x4321);
    mmix.set_register(2, 400);
    mmix.set_register(3, 14);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_wyde(414), 0x4321);

    // STWU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xA6010203);
    mmix.set_register(1, 0xBEEF);
    mmix.set_register(2, 400);
    mmix.set_register(3, 16);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_wyde(416), 0xBEEF);

    // STT $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xA8010203);
    mmix.set_register(1, 0x87654321);
    mmix.set_register(2, 500);
    mmix.set_register(3, 20);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(520), 0x87654321);

    // STTU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xAA010203);
    mmix.set_register(1, 0xCAFEBABE);
    mmix.set_register(2, 500);
    mmix.set_register(3, 24);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(524), 0xCAFEBABE);

    // STO $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xAC010203);
    mmix.set_register(1, 0x0F1E2D3C4B5A6978);
    mmix.set_register(2, 600);
    mmix.set_register(3, 30);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(630), 0x0F1E2D3C4B5A6978);

    // STOU $1, $2, $3
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xAE010203);
    mmix.set_register(1, 0x1122334455667788);
    mmix.set_register(2, 600);
    mmix.set_register(3, 40);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(640), 0x1122334455667788);

    // STCO 17, $2, $3 - Store constant octabyte
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xB4110203); // X=17 (0x11)
    mmix.set_register(2, 700);
    mmix.set_register(3, 5);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(705), 17);

    // STHT $1, $2, $3 - Store high tetra
    mmix.set_pc(0);
    mmix.write_tetra(0, 0xB2010203);
    mmix.set_register(1, 0x1357924600000000);
    mmix.set_register(2, 800);
    mmix.set_register(3, 8);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(808), 0x13579246); // High 32 bits
}

// ========== Special Load/Store Tests ==========

#[test]
fn test_ldht() {
    let mut mmix = MMix::new();
    // LDHT $1, $2, $3 - Load high tetra
    mmix.set_register(2, 100);
    mmix.set_register(3, 4);
    mmix.write_tetra(104, 0x12345678);
    mmix.write_tetra(0, 0x92010203); // LDHT $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x1234567800000000);
}

#[test]
fn test_ldhti() {
    let mut mmix = MMix::new();
    // LDHTI $1, $2, 8 - Load high tetra immediate
    mmix.set_register(2, 100);
    mmix.write_tetra(108, 0xABCDEF01);
    mmix.write_tetra(0, 0x93010208); // LDHTI $1,$2,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xABCDEF0100000000);
}

#[test]
fn test_cswap_success() {
    let mut mmix = MMix::new();
    // CSWAP $1, $2, $3 - Compare and swap (successful)
    let addr = 1000u64;
    let old_value = 0x123456789ABCDEF0u64;
    let new_value = 0xFEDCBA9876543210u64;

    mmix.write_octa(addr, old_value);
    mmix.set_special(SpecialReg::RP, old_value); // Set compare value
    mmix.set_register(1, new_value); // New value to write
    mmix.set_register(2, addr);
    mmix.set_register(3, 0);

    mmix.write_tetra(0, 0x94010203); // CSWAP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1); // Success
    assert_eq!(mmix.read_octa(addr), new_value); // Memory updated
}

#[test]
fn test_cswap_failure() {
    let mut mmix = MMix::new();
    // CSWAP $1, $2, $3 - Compare and swap (failed)
    let addr = 1000u64;
    let mem_value = 0x123456789ABCDEF0u64;
    let compare_value = 0x1111111111111111u64;
    let new_value = 0xFEDCBA9876543210u64;

    mmix.write_octa(addr, mem_value);
    mmix.set_special(SpecialReg::RP, compare_value); // Different compare value
    mmix.set_register(1, new_value);
    mmix.set_register(2, addr);
    mmix.set_register(3, 0);

    mmix.write_tetra(0, 0x94010203); // CSWAP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Failure
    assert_eq!(mmix.read_octa(addr), mem_value); // Memory unchanged
    assert_eq!(mmix.get_special(SpecialReg::RP), mem_value); // rP <- M8[$Y+$Z]
}

#[test]
fn test_cswapi() {
    let mut mmix = MMix::new();
    // CSWAPI $1, $2, 16 - Compare and swap immediate
    let addr = 2000u64;
    let old_value = 0xAAAAAAAAAAAAAAAAu64;
    let new_value = 0xBBBBBBBBBBBBBBBBu64;

    mmix.write_octa(addr + 16, old_value);
    mmix.set_special(SpecialReg::RP, old_value);
    mmix.set_register(1, new_value);
    mmix.set_register(2, addr);

    mmix.write_tetra(0, 0x95010210); // CSWAPI $1,$2,16
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 1); // Success
    assert_eq!(mmix.read_octa(addr + 16), new_value);
}

#[test]
fn test_cswapi_failure() {
    let mut mmix = MMix::new();
    // CSWAPI $1, $2, 16 - Compare and swap immediate (failed)
    let addr = 2000u64;
    let mem_value = 0x123456789ABCDEF0u64;
    let compare_value = 0x1111111111111111u64;
    let new_value = 0xFEDCBA9876543210u64;

    mmix.write_octa(addr + 16, mem_value);
    mmix.set_special(SpecialReg::RP, compare_value); // Different compare value
    mmix.set_register(1, new_value);
    mmix.set_register(2, addr);

    mmix.write_tetra(0, 0x95010210); // CSWAPI $1,$2,16
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Failure
    assert_eq!(mmix.read_octa(addr + 16), mem_value); // Memory unchanged
    assert_eq!(mmix.get_special(SpecialReg::RP), mem_value); // rP <- M8[$Y+Z]
}

#[test]
fn octa_load_reads_the_aligned_base_from_any_address_in_the_block() {
    // LDO $1,$2,$3 with $3 sweeping the octabyte's own aligned block:
    // every one of the 8 addresses must resolve to the same value:
    // M8[A] = M8[8*floor(A/8)].
    let base = 800u64;
    let value = 0x1122334455667788u64;
    for offset in 0u64..8 {
        let mut mmix = MMix::new();
        mmix.write_octa(base, value);
        mmix.set_register(2, base);
        mmix.set_register(3, offset);
        mmix.write_tetra(0, 0x8C010203); // LDO $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), value, "offset {offset}");
    }
}

#[test]
fn octa_store_at_misaligned_address_lands_at_aligned_base() {
    // STO $1,$2,$3 at base+5: the write must land at the aligned base,
    // not straddle base and base+8. Reverting write_octa's mask alone
    // fails this, since the value would then land 5 bytes high.
    let mut mmix = MMix::new();
    let base = 800u64;
    let value = 0x99AABBCCDDEEFF00u64;
    mmix.set_register(1, value);
    mmix.set_register(2, base);
    mmix.set_register(3, 5);
    mmix.write_tetra(0, 0xAC010203); // STO $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(base), value);
}

#[test]
fn tetra_load_reads_the_aligned_base_from_any_address_in_the_block() {
    // LDTU $1,$2,$3 with $3 sweeping the tetra's own aligned block.
    let base = 400u64;
    let value = 0x11223344u32;
    for offset in 0u64..4 {
        let mut mmix = MMix::new();
        mmix.write_tetra(base, value);
        mmix.set_register(2, base);
        mmix.set_register(3, offset);
        mmix.write_tetra(0, 0x8A010203); // LDTU $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), value as u64, "offset {offset}");
    }
}

#[test]
fn tetra_store_at_misaligned_address_lands_at_aligned_base() {
    // STT $1,$2,$3 at base+3: reverting write_tetra's mask alone fails
    // this.
    let mut mmix = MMix::new();
    let base = 400u64;
    let value = 0xAABBCCDDu32;
    mmix.set_register(1, value as u64);
    mmix.set_register(2, base);
    mmix.set_register(3, 3);
    mmix.write_tetra(0, 0xA8010203); // STT $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(base), value);
}

#[test]
fn wyde_load_reads_the_aligned_base_from_any_address_in_the_block() {
    // LDWU $1,$2,$3 with $3 sweeping the wyde's own aligned block.
    let base = 200u64;
    let value = 0xBEEFu16;
    for offset in 0u64..2 {
        let mut mmix = MMix::new();
        mmix.write_wyde(base, value);
        mmix.set_register(2, base);
        mmix.set_register(3, offset);
        mmix.write_tetra(0, 0x86010203); // LDWU $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), value as u64, "offset {offset}");
    }
}

#[test]
fn wyde_store_at_misaligned_address_lands_at_aligned_base() {
    // STW $1,$2,$3 at base+1: reverting write_wyde's mask alone fails
    // this.
    let mut mmix = MMix::new();
    let base = 200u64;
    let value = 0xCAFEu16;
    mmix.set_register(1, value as u64);
    mmix.set_register(2, base);
    mmix.set_register(3, 1);
    mmix.write_tetra(0, 0xA4010203); // STW $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_wyde(base), value);
}

#[test]
fn byte_access_at_an_odd_address_still_reads_that_byte() {
    // A byte is its own alignment: read_byte/write_byte take no mask,
    // unlike the wider accessors above. There is no fix to revert here —
    // this guards against someone later "helpfully" masking read_byte to
    // match its wider siblings.
    let mut mmix = MMix::new();
    mmix.write_byte(801, 0x42);
    assert_eq!(mmix.read_byte(801), 0x42);
    assert_eq!(mmix.read_byte(800), 0);
}

#[test]
fn test_ldunc() {
    let mut mmix = MMix::new();
    // LDUNC $1, $2, $3 - Load uncached
    mmix.set_register(2, 500);
    mmix.set_register(3, 24);
    mmix.write_octa(524, 0x0123456789ABCDEFu64);
    mmix.write_tetra(0, 0x96010203); // LDUNC $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0x0123456789ABCDEFu64);
}

#[test]
fn test_ldunci() {
    let mut mmix = MMix::new();
    // LDUNCI $1, $2, 32 - Load uncached immediate
    mmix.set_register(2, 600);
    mmix.write_octa(632, 0xFEDCBA9876543210u64);
    mmix.write_tetra(0, 0x97010220); // LDUNCI $1,$2,32
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0xFEDCBA9876543210u64);
}

#[test]
fn test_ldvts() {
    let mut mmix = MMix::new();
    // LDVTS $1, $2, $3 - Load virtual translation status
    mmix.set_register(2, 0x1000);
    mmix.set_register(3, 0);
    mmix.write_tetra(0, 0x98010203); // LDVTS $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Returns 0 in simulation
}

#[test]
fn test_ldvtsi() {
    let mut mmix = MMix::new();
    // LDVTSI $1, $2, 0 - Load virtual translation status immediate
    mmix.set_register(2, 0x2000);
    mmix.write_tetra(0, 0x99010200); // LDVTSI $1,$2,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Returns 0 in simulation
}

#[test]
fn test_preld() {
    let mut mmix = MMix::new();
    // PRELD $1, $2, $3 - Preload data (no-op)
    mmix.write_tetra(0, 0x9A010203); // PRELD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_preldi() {
    let mut mmix = MMix::new();
    // PRELDI $1, $2, 64 - Preload data immediate (no-op)
    mmix.write_tetra(0, 0x9B010240); // PRELDI $1,$2,64
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_prego() {
    let mut mmix = MMix::new();
    // PREGO $1, $2, $3 - Preload to go (no-op)
    mmix.write_tetra(0, 0x9C010203); // PREGO $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_pregoi() {
    let mut mmix = MMix::new();
    // PREGOI $1, $2, 128 - Preload to go immediate (no-op)
    mmix.write_tetra(0, 0x9D010280); // PREGOI $1,$2,128
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_stht() {
    let mut mmix = MMix::new();
    // STHT $1, $2, $3 - Store high tetra
    mmix.set_register(1, 0x1234567890ABCDEFu64);
    mmix.set_register(2, 500);
    mmix.set_register(3, 12);
    mmix.write_tetra(0, 0xB2010203); // STHT $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(512), 0x12345678);
}

#[test]
fn test_sthti() {
    let mut mmix = MMix::new();
    // STHTI $1, $2, 24 - Store high tetra immediate
    mmix.set_register(1, 0xFEDCBA9876543210u64);
    mmix.set_register(2, 600);
    mmix.write_tetra(0, 0xB7010218); // STHTI $1,$2,24
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_tetra(624), 0xFEDCBA98);
}

#[test]
fn test_stco() {
    let mut mmix = MMix::new();
    // STCO $X, $Y, $Z - Store constant octabyte (X=42)
    mmix.set_register(2, 1500);
    mmix.set_register(3, 8);
    mmix.write_tetra(0, 0xB42A0203); // STCO $42,$2,$3 (X=0x2A=42)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(1508), 42);
}

#[test]
fn test_stcoi() {
    let mut mmix = MMix::new();
    // STCOI $X, $Y, Z - Store constant octabyte immediate (X=100)
    mmix.set_register(2, 2500);
    mmix.write_tetra(0, 0xB5640220); // STCOI $100,$2,32 (X=0x64=100)
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(2532), 100);
}

#[test]
fn test_stunc() {
    let mut mmix = MMix::new();
    // STUNC $1, $2, $3 - Store uncached
    mmix.set_register(1, 0xABCDEF0123456789u64);
    mmix.set_register(2, 3000);
    mmix.set_register(3, 16);
    mmix.write_tetra(0, 0xB6010203); // STUNC $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(3016), 0xABCDEF0123456789u64);
}

#[test]
fn test_stunci() {
    let mut mmix = MMix::new();
    // STUNCI $1, $2, 40 - Store uncached immediate
    mmix.set_register(1, 0x123456789ABCDEFu64);
    mmix.set_register(2, 4000);
    mmix.write_tetra(0, 0xB7010228); // STUNCI $1,$2,40
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.read_octa(4040), 0x123456789ABCDEFu64);
}

#[test]
fn test_syncd() {
    let mut mmix = MMix::new();
    // SYNCD $1, $2, $3 - Synchronize data (no-op)
    mmix.write_tetra(0, 0xB8010203); // SYNCD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_syncdi() {
    let mut mmix = MMix::new();
    // SYNCDI $1, $2, 64 - Synchronize data immediate (no-op)
    mmix.write_tetra(0, 0xB9010203); // SYNCDI $1,$2,64
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_prest() {
    let mut mmix = MMix::new();
    // PREST $1, $2, $3 - Prestore (no-op)
    mmix.write_tetra(0, 0xBA010203); // PREST $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn test_presti() {
    let mut mmix = MMix::new();
    // PRESTI $1, $2, 128 - Prestore immediate (no-op)
    mmix.write_tetra(0, 0xBB010280); // PRESTI $1,$2,128
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advanced
}

#[test]
fn occupied_yields_nonzero_bytes_ascending_after_a_zero_write() {
    // Six surviving addresses written out of ascending order: with only two
    // survivors, HashMap iteration lands in ascending order by chance often
    // enough that deleting the sort in `occupied` doesn't reliably fail this
    // test. Six addresses drops that accidental-pass rate to roughly 1/720.
    let mut mmix = MMix::new();
    mmix.write_byte(500, 5);
    mmix.write_byte(100, 1);
    mmix.write_byte(700, 7);
    mmix.write_byte(300, 3);
    mmix.write_byte(900, 9);
    mmix.write_byte(400, 4);
    mmix.write_byte(200, 6);
    mmix.write_byte(200, 0); // zero-write: removed, must not appear

    let items: Vec<(u64, u8)> = mmix.occupied().collect();
    assert_eq!(
        items,
        vec![(100, 1), (300, 3), (400, 4), (500, 5), (700, 7), (900, 9)]
    );
}

#[test]
fn loaded_extent_includes_the_hello_world_nul_terminator_that_occupied_omits() {
    // Reproduces examples/hello_world.mms's Text BYTE directive: its
    // trailing NUL is a real loaded byte that write_byte's zero-removal
    // hides from `occupied`, checked by address rather than by a total
    // count (write_image also writes every instruction, so the totals
    // include far more than this one directive).
    use crate::debugger::write_image;
    use crate::mmixal::MMixAssembler;

    const HELLO_WORLD: &str = "\
\tLOC\tData_Segment
\tGREG\t@
Text\tBYTE\t\"Hello world!\",'\\n',0

\tLOC\t#100

Main\tLDA\t$255,Text
\tTRAP\t0,Fputs,StdOut
\tTRAP\t0,Halt,0
";
    let mut asm = MMixAssembler::new(HELLO_WORLD, "hello_world.mms");
    asm.parse().expect("hello_world.mms must assemble");
    let text_addr = *asm.labels.get("Text").expect("Text label");
    // "Hello world!",'\n',0 is 14 bytes; the NUL terminator is the last.
    let nul_addr = text_addr + 13;

    let mut mmix = MMix::new();
    write_image(&mut mmix, &asm);

    let loaded: Vec<(u64, u8)> = mmix.loaded_extent().collect();
    assert!(loaded.contains(&(nul_addr, 0)));

    let occupied: Vec<(u64, u8)> = mmix.occupied().collect();
    assert!(!occupied.iter().any(|&(addr, _)| addr == nul_addr));
}

#[test]
fn loaded_extent_is_unchanged_by_runtime_writes_during_execution() {
    // Mirrors debugger.rs's CALL_PROGRAM fixture: PUSHJ spills the
    // caller's frame to the register stack (addresses at
    // 0x6000000000000000+), a real runtime write that goes through
    // `write_byte`, not `write_loaded_byte`. `loaded_extent` tracks only
    // what `write_image` loaded, so a run must leave it unchanged.
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;

    const CALL_PROGRAM: &str = "\
\tLOC\t#100
Main\tPUSHJ\t$0,Sub
\tSETI\t$1,7
\tTRAP\t0,Halt,0
Sub\tSETI\t$0,3
\tPOP\t0,0
";
    let mut asm = MMixAssembler::new(CALL_PROGRAM, "call.mms");
    asm.parse().expect("call.mms must assemble");

    let mut mmix = MMix::new();
    write_image(&mut mmix, &asm);
    let before: Vec<(u64, u8)> = mmix.loaded_extent().collect();
    assert!(!before.is_empty(), "write_image must have loaded something");

    mmix.set_pc(entry_point(&asm));
    mmix.run(); // PUSHJ spills a frame; POP restores it; TRAP halts.

    let after: Vec<(u64, u8)> = mmix.loaded_extent().collect();
    assert_eq!(
        before, after,
        "a runtime write (PUSHJ's register-stack spill) must not appear \
             in loaded_extent"
    );
}

#[test]
fn journal_records_writes_only_while_enabled_including_a_zero_write() {
    let mut mmix = MMix::new();
    mmix.write_byte(10, 1); // before enabling: not recorded

    mmix.set_journal(true);
    mmix.write_byte(20, 2);
    // A zero-write to a fresh address (never written before) is still a
    // recorded state change, distinct from the nonzero write above.
    mmix.write_byte(25, 0);
    mmix.write_byte(30, 3);
    mmix.set_journal(false);
    mmix.write_byte(40, 4); // journal off again: not recorded

    assert_eq!(mmix.take_journal(), vec![20, 25, 30]);
    // Drained: the next call is empty without another write.
    assert!(mmix.take_journal().is_empty());
}

#[test]
fn journal_enabled_flag_survives_reset_but_the_buffer_does_not() {
    let mut mmix = MMix::new();
    mmix.set_journal(true);
    mmix.write_byte(50, 7);
    assert_eq!(mmix.take_journal(), vec![50]);

    mmix.reset();
    assert!(mmix.take_journal().is_empty());

    // The flag itself survived the reset.
    mmix.write_byte(60, 8);
    assert_eq!(mmix.take_journal(), vec![60]);
}
