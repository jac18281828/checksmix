use checksmix::MMixAssembler;
use std::fs;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn cross_file_pushj_resolves_to_global_address() {
    let main_path = fixture_path("multi_main.mms");
    let lib_path = fixture_path("multi_lib.mms");
    let main_src = fs::read_to_string(&main_path).expect("read main.mms");
    let lib_src = fs::read_to_string(&lib_path).expect("read lib.mms");

    let mut asm = MMixAssembler::new(&main_src, main_path.to_str().unwrap());
    asm.add_source(&lib_src, lib_path.to_str().unwrap());
    asm.parse().expect("multi-source assemble");

    assert_eq!(asm.labels.get("Main").copied(), Some(0x100));
    assert_eq!(asm.labels.get(":Lib").copied(), Some(0x200));

    // Find the PUSHJ instruction at 0x100 and decode its YZ field as a
    // signed-tetra offset from the PUSHJ's own address. With Main at 0x100
    // and :Lib at 0x200, the offset is (0x200 - 0x100) / 4 = 0x40.
    let (addr, inst) = asm
        .instructions
        .iter()
        .find(|(a, _)| *a == 0x100)
        .expect("instruction at Main address");
    let bytes = asm.encode_instruction_bytes(inst);
    assert_eq!(bytes.len(), 4, "PUSHJ encodes to one tetra");
    assert_eq!(bytes[0], 0xF2, "PUSHJ opcode = 0xF2");
    let yz = ((bytes[2] as u16) << 8) | bytes[3] as u16;
    let offset_tetras = yz as i16 as i64;
    let target = (*addr as i64 + offset_tetras * 4) as u64;
    assert_eq!(target, 0x200, "PUSHJ target should be :Lib at 0x200");
}
