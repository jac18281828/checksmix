//! The MMix Display impl.

use super::*;

/// The dump labels each slot with the name of the register that slot
/// holds, so a value never appears under a neighbour's name.
#[test]
fn display_labels_each_special_register_with_its_own_name() {
    let mut mmix = MMix::new();
    let expected = [
        (SpecialReg::RN, "rN", 0x1111_u64),
        (SpecialReg::RO, "rO", 0x2222),
        (SpecialReg::RG, "rG", 0x3333),
        (SpecialReg::RL, "rL", 0x4444),
    ];
    for (reg, _, value) in expected {
        mmix.set_special(reg, value);
    }

    let dump = mmix.to_string();
    for (_, name, value) in expected {
        let line = format!("{:<4} = {:#018x}", name, value);
        assert!(
            dump.contains(&line),
            "expected `{line}` in the dump, got:\n{dump}"
        );
    }
}
