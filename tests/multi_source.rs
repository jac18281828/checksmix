use checksmix::{MMixAssembler, MmoDecoder};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn assembled_bytes(asm: &MMixAssembler) -> BTreeMap<u64, u8> {
    let mut memory = BTreeMap::new();
    MmoDecoder::new(asm.generate_object_code())
        .decode(|addr, byte| *memory.entry(addr).or_insert(0) ^= byte)
        .expect("object code decodes");
    memory
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

    assert_eq!(asm.labels().get("Main").copied(), Some(0x100));
    // `:Lib` names the same root symbol as `Lib`, keyed without the colon.
    assert_eq!(asm.labels().get("Lib").copied(), Some(0x200));

    // Decode the PUSHJ at 0x100's YZ field as an unsigned forward tetra
    // count from the PUSHJ's own address. With Main at 0x100 and Lib at
    // 0x200, the field is (0x200 - 0x100) / 4 = 0x40.
    let memory = assembled_bytes(&asm);
    let bytes: Vec<u8> = (0x100..0x104)
        .map(|addr| memory.get(&addr).copied().unwrap_or(0))
        .collect();
    assert_eq!(bytes[0], 0xF2, "PUSHJ opcode = 0xF2");
    let yz = ((bytes[2] as u16) << 8) | bytes[3] as u16;
    let target = 0x100 + (yz as u64) * 4;
    assert_eq!(target, 0x200, "PUSHJ target should be Lib at 0x200");
    assert_eq!(
        asm.addr_for_line(main_path.to_str().unwrap(), 4),
        Some(0x104),
        "PUSHJ encodes to one tetra"
    );
}
