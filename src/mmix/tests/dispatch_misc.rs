//! Conditional-set/zero-set families, SWYM, SYNC, and RESUME.

use super::*;

#[test]
fn test_swym() {
    let mut mmix = MMix::new();
    // SWYM - no-op
    mmix.write_tetra(0, 0xFD000000); // SWYM 0,0,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4); // PC advances normally
}

#[test]
fn test_sync() {
    let mut mmix = MMix::new();
    // SYNC - memory barrier (no-op in simulator)
    mmix.write_tetra(0, 0xFC000000); // SYNC 0,0,0
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

/// `SYNC` 0-3 is a no-op; 3 is the top of that range.
#[test]
fn test_sync_3_is_a_no_op() {
    let mut mmix = MMix::new();
    mmix.write_tetra(0, 0xFC000003); // SYNC 3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 4);
}

/// `SYNC` 4-7 is a privileged-operation interrupt.
#[test]
fn test_sync_4_to_7_is_privileged() {
    for xyz in [4u32, 7] {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.write_tetra(0, 0xFC000000 | xyz); // SYNC xyz
        assert!(!mmix.execute_instruction(), "SYNC {xyz}");
        assert_eq!(mmix.get_pc(), 0, "PC unmoved for SYNC {xyz}");
        assert_eq!(mmix.get_exit_code(), 1, "SYNC {xyz}");
        assert_eq!(handle.diagnostics().len(), 1);
        assert_eq!(
            handle.diagnostics()[0],
            format!("SYNC {xyz}: privileged-operation interrupt at PC=0x0000000000000000")
        );
    }
}

/// `SYNC` above 7 is an illegal-instruction interrupt.
#[test]
fn test_sync_above_7_is_illegal() {
    for xyz in [8u32, 1000] {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.write_tetra(0, 0xFC000000 | xyz); // SYNC xyz
        assert!(!mmix.execute_instruction(), "SYNC {xyz}");
        assert_eq!(mmix.get_pc(), 0, "PC unmoved for SYNC {xyz}");
        assert_eq!(mmix.get_exit_code(), 1, "SYNC {xyz}");
        assert_eq!(handle.diagnostics().len(), 1);
        assert_eq!(
            handle.diagnostics()[0],
            format!("SYNC {xyz}: illegal-instruction interrupt at PC=0x0000000000000000")
        );
    }
}

#[test]
fn test_resume_with_negative_rx_continues_at_rw() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_special(SpecialReg::RX, 0x8000000000000000); // negative
    mmix.set_special(SpecialReg::RW, 0x200);
    mmix.write_tetra(0x100, 0xF9000000); // RESUME 0

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x200);
}

#[test]
fn test_resume_ropcode_0_runs_rxs_instruction_and_continues_at_rw() {
    let mut mmix = MMix::new();
    mmix.set_pc(0x100);
    mmix.set_register(2, 10);
    mmix.set_register(3, 20);
    // rX (nonnegative, ropcode 0): ADD $1,$2,$3.
    mmix.set_special(SpecialReg::RX, 0x20010203);
    mmix.set_special(SpecialReg::RW, 0x300);
    mmix.write_tetra(0x100, 0xF9000000); // RESUME 0

    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 30);
    assert_eq!(mmix.get_pc(), 0x300);
}

#[test]
fn test_resume_ropcodes_1_to_3_halt() {
    for ropcode in 1u64..=3 {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RX, ropcode << 56);
        mmix.set_special(SpecialReg::RW, 0x300);
        mmix.write_tetra(0x100, 0xF9000000); // RESUME 0

        assert!(!mmix.execute_instruction(), "ropcode {ropcode}");
        assert_eq!(mmix.get_pc(), 0x100, "PC unmoved for ropcode {ropcode}");
        assert_eq!(handle.diagnostics().len(), 1);
        assert_eq!(mmix.get_exit_code(), 1, "ropcode {ropcode}");
    }
}

#[test]
fn test_resume_with_nonzero_z_halts() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_pc(0x100);
    mmix.write_tetra(0x100, 0xF9000001); // RESUME 1 - privileged

    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_pc(), 0x100, "PC stays on the rejected instruction");
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(mmix.get_exit_code(), 1);
}

/// `RESUME`'s X must be zero, checked before the existing Z check.
#[test]
fn test_resume_x_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_pc(0x100);

    // RESUME 1,0,0 -- X=1 must be zero.
    mmix.write_tetra(0x100, 0xF9010000);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0x100, "PC stays on the rejected instruction");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "RESUME X=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000100"
    );
}

/// `RESUME`'s Y must be zero, checked after X.
#[test]
fn test_resume_y_nonzero_is_rejected() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_pc(0x100);

    // RESUME 0,1,0 -- Y=1 must be zero.
    mmix.write_tetra(0x100, 0xF9000100);
    assert!(!mmix.execute_instruction());

    assert_eq!(mmix.get_pc(), 0x100, "PC stays on the rejected instruction");
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert_eq!(
        handle.diagnostics()[0],
        "RESUME Y=1: must be zero; illegal-instruction interrupt at PC=0x0000000000000100"
    );
}

#[test]
fn test_csn_condition_true() {
    let mut mmix = MMix::new();
    // CSN $1, $2, $3 - If $2 < 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, (-10i64) as u64);
    mmix.set_register(3, 50);
    mmix.write_tetra(0, 0x60010203); // CSN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 50); // Condition true: $3
}

#[test]
fn test_csn_condition_false() {
    let mut mmix = MMix::new();
    // CSN $1, $2, $3 - If $2 >= 0, do nothing
    mmix.set_register(1, 99); // Initial value
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_tetra(0, 0x60010203); // CSN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 99); // Condition false: unchanged
}

#[test]
fn test_csni() {
    let mut mmix = MMix::new();
    // CSNI $1, $2, 50 - If $2 < 0, set $1 = 50, else $1 = $2
    mmix.set_register(2, (-1i64) as u64);
    mmix.write_tetra(0, 0x61010232); // CSNI $1,$2,50
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 50); // Condition true: 50
}

#[test]
fn test_csz_condition_true() {
    let mut mmix = MMix::new();
    // CSZ $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, 0);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x62010203); // CSZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 20); // Condition true: $3
}

#[test]
fn test_csz_condition_false() {
    let mut mmix = MMix::new();
    // CSZ $1, $2, $3 - If $2 != 0, do nothing
    mmix.set_register(1, 88); // Initial value
    mmix.set_register(2, 10);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x62010203); // CSZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 88); // Condition false: unchanged
}

#[test]
fn test_cszi() {
    let mut mmix = MMix::new();
    // CSZI $1, $2, 15 - If $2 == 0, set $1 = 15, else $1 = $2
    mmix.set_register(2, 0);
    mmix.write_tetra(0, 0x6301020F); // CSZI $1,$2,15
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 15); // Condition true: 15
}

#[test]
fn test_csp_condition_true() {
    let mut mmix = MMix::new();
    // CSP $1, $2, $3 - If $2 > 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, 5);
    mmix.set_register(3, 7);
    mmix.write_tetra(0, 0x64010203); // CSP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 7); // Condition true: $3
}

#[test]
fn test_csp_condition_false_zero() {
    let mut mmix = MMix::new();
    // CSP $1, $2, $3 - If $2 <= 0, do nothing
    mmix.set_register(1, 77); // Initial value
    mmix.set_register(2, 0);
    mmix.set_register(3, 7);
    mmix.write_tetra(0, 0x64010203); // CSP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 77); // Condition false: unchanged
}

#[test]
fn test_cspi() {
    let mut mmix = MMix::new();
    // CSPI $1, $2, 25 - If $2 > 0, set $1 = 25, else $1 = $2
    mmix.set_register(2, 50);
    mmix.write_tetra(0, 0x65010219); // CSPI $1,$2,25
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 25); // Condition true: 25
}

#[test]
fn test_csod_condition_true() {
    let mut mmix = MMix::new();
    // CSOD $1, $2, $3 - If $2 is odd, set $1 = $3, else $1 = $2
    mmix.set_register(2, 7);
    mmix.set_register(3, 15);
    mmix.write_tetra(0, 0x66010203); // CSOD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 15); // Condition true: $3
}

#[test]
fn test_csod_condition_false() {
    let mut mmix = MMix::new();
    // CSOD $1, $2, $3 - If $2 is even, do nothing
    mmix.set_register(1, 66); // Initial value
    mmix.set_register(2, 8);
    mmix.set_register(3, 15);
    mmix.write_tetra(0, 0x66010203); // CSOD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 66); // Condition false: unchanged
}

#[test]
fn test_csodi() {
    let mut mmix = MMix::new();
    // CSODI $1, $2, 11 - If $2 is odd, set $1 = 11, else $1 = $2
    mmix.set_register(2, 99);
    mmix.write_tetra(0, 0x6701020B); // CSODI $1,$2,11
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 11); // Condition true: 11
}

#[test]
fn test_csnn_condition_true_positive() {
    let mut mmix = MMix::new();
    // CSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, 30);
    mmix.set_register(3, 40);
    mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 40); // Condition true: $3
}

#[test]
fn test_csnn_condition_true_zero() {
    let mut mmix = MMix::new();
    // CSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, 0);
    mmix.set_register(3, 40);
    mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 40); // Condition true: $3
}

#[test]
fn test_csnn_condition_false() {
    let mut mmix = MMix::new();
    // CSNN $1, $2, $3 - If $2 < 0, do nothing
    mmix.set_register(1, 55); // Initial value
    mmix.set_register(2, (-5i64) as u64);
    mmix.set_register(3, 40);
    mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 55); // Condition false: unchanged
}

#[test]
fn test_csnni() {
    let mut mmix = MMix::new();
    // CSNNI $1, $2, 8 - If $2 >= 0, set $1 = 8, else $1 = $2
    mmix.set_register(2, 92);
    mmix.write_tetra(0, 0x69010208); // CSNNI $1,$2,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 8); // Condition true: 8
}

#[test]
fn test_csnz_condition_true() {
    let mut mmix = MMix::new();
    // CSNZ $1, $2, $3 - If $2 != 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, 100);
    mmix.set_register(3, 200);
    mmix.write_tetra(0, 0x6A010203); // CSNZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 200); // Condition true: $3
}

#[test]
fn test_csnz_condition_false() {
    let mut mmix = MMix::new();
    // CSNZ $1, $2, $3 - If $2 == 0, do nothing
    mmix.set_register(1, 44); // Initial value
    mmix.set_register(2, 0);
    mmix.set_register(3, 200);
    mmix.write_tetra(0, 0x6A010203); // CSNZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 44); // Condition false: unchanged
}

#[test]
fn test_csnzi() {
    let mut mmix = MMix::new();
    // CSNZI $1, $2, 33 - If $2 != 0, set $1 = 33, else $1 = $2
    mmix.set_register(2, 67);
    mmix.write_tetra(0, 0x6B010221); // CSNZI $1,$2,33
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 33); // Condition true: 33
}

#[test]
fn test_csnp_condition_true_negative() {
    let mut mmix = MMix::new();
    // CSNP $1, $2, $3 - If $2 <= 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, (-100i64) as u64);
    mmix.set_register(3, 25);
    mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 25); // Condition true: $3
}

#[test]
fn test_csnp_condition_true_zero() {
    let mut mmix = MMix::new();
    // CSNP $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = $2
    mmix.set_register(2, 0);
    mmix.set_register(3, 25);
    mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 25); // Condition true: $3
}

#[test]
fn test_csnp_condition_false() {
    let mut mmix = MMix::new();
    // CSNP $1, $2, $3 - If $2 > 0, do nothing
    mmix.set_register(1, 33); // Initial value
    mmix.set_register(2, 50);
    mmix.set_register(3, 25);
    mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 33); // Condition false: unchanged
}

#[test]
fn test_csnpi() {
    let mut mmix = MMix::new();
    // CSNPI $1, $2, 44 - If $2 <= 0, set $1 = 44, else $1 = $2
    mmix.set_register(2, 0);
    mmix.write_tetra(0, 0x6D01022C); // CSNPI $1,$2,44
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 44); // Condition true: 44
}

#[test]
fn test_csev_condition_true() {
    let mut mmix = MMix::new();
    // CSEV $1, $2, $3 - If $2 is even, set $1 = $3, else $1 = $2
    mmix.set_register(2, 80);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x6E010203); // CSEV $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 20); // Condition true: $3
}

#[test]
fn test_csev_condition_false() {
    let mut mmix = MMix::new();
    // CSEV $1, $2, $3 - If $2 is odd, do nothing
    mmix.set_register(1, 22); // Initial value
    mmix.set_register(2, 7);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x6E010203); // CSEV $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 22); // Condition false: unchanged
}

#[test]
fn test_csevi() {
    let mut mmix = MMix::new();
    // CSEVI $1, $2, 12 - If $2 is even, set $1 = 12, else $1 = $2
    mmix.set_register(2, 88);
    mmix.write_tetra(0, 0x6F01020C); // CSEVI $1,$2,12
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 12); // Condition true: 12
}

// ========== Zero or Set Tests ==========

#[test]
fn test_zsn_condition_true() {
    let mut mmix = MMix::new();
    // ZSN $1, $2, $3 - If $2 < 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, (-10i64) as u64);
    mmix.set_register(3, 50);
    mmix.write_tetra(0, 0x70010203); // ZSN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 50); // Condition true: $3
}

#[test]
fn test_zsn_condition_false() {
    let mut mmix = MMix::new();
    // ZSN $1, $2, $3 - If $2 >= 0, set $1 = 0
    mmix.set_register(2, 100);
    mmix.set_register(3, 50);
    mmix.write_tetra(0, 0x70010203); // ZSN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zsni() {
    let mut mmix = MMix::new();
    // ZSNI $1, $2, 50 - If $2 < 0, set $1 = 50, else $1 = 0
    mmix.set_register(2, (-1i64) as u64);
    mmix.write_tetra(0, 0x71010232); // ZSNI $1,$2,50
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 50); // Condition true: 50
}

#[test]
fn test_zsz_condition_true() {
    let mut mmix = MMix::new();
    // ZSZ $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, 0);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x72010203); // ZSZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 20); // Condition true: $3
}

#[test]
fn test_zsz_condition_false() {
    let mut mmix = MMix::new();
    // ZSZ $1, $2, $3 - Set $1 = 0 if $1 is not zero
    mmix.set_register(1, 1);
    mmix.set_register(2, 10);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x72010203); // ZSZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zszi() {
    let mut mmix = MMix::new();
    // ZSZI $1, $2, 15 - If $2 == 0, set $1 = 15, else $1 = 0
    mmix.set_register(2, 0);
    mmix.write_tetra(0, 0x7301020F); // ZSZI $1,$2,15
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 15); // Condition true: 15
}

#[test]
fn test_zsp_condition_true() {
    let mut mmix = MMix::new();
    // ZSP $1, $2, $3 - If $2 > 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, 5);
    mmix.set_register(3, 7);
    mmix.write_tetra(0, 0x74010203); // ZSP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 7); // Condition true: $3
}

#[test]
fn test_zsp_condition_false_zero() {
    let mut mmix = MMix::new();
    // ZSP $1, $2, $3 - If $2 <= 0, set $1 = 0
    mmix.set_register(2, 0);
    mmix.set_register(3, 7);
    mmix.write_tetra(0, 0x74010203); // ZSP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zspi() {
    let mut mmix = MMix::new();
    // ZSPI $1, $2, 25 - If $2 > 0, set $1 = 25, else $1 = 0
    mmix.set_register(2, 50);
    mmix.write_tetra(0, 0x75010219); // ZSPI $1,$2,25
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 25); // Condition true: 25
}

#[test]
fn test_zsod_condition_true() {
    let mut mmix = MMix::new();
    // ZSOD $1, $2, $3 - If $2 is odd, set $1 = $3, else $1 = 0
    mmix.set_register(2, 7);
    mmix.set_register(3, 15);
    mmix.write_tetra(0, 0x76010203); // ZSOD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 15); // Condition true: $3
}

#[test]
fn test_zsod_condition_false() {
    let mut mmix = MMix::new();
    // ZSOD $1, $2, $3 - Set $1 = 0 if $1 is even
    mmix.set_register(1, 8);
    mmix.set_register(2, 10);
    mmix.set_register(3, 15);
    mmix.write_tetra(0, 0x76010203); // ZSOD $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zsodi() {
    let mut mmix = MMix::new();
    // ZSODI $1, $2, 11 - If $2 is odd, set $1 = 11, else $1 = 0
    mmix.set_register(2, 99);
    mmix.write_tetra(0, 0x7701020B); // ZSODI $1,$2,11
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 11); // Condition true: 11
}

#[test]
fn test_zsnn_condition_true_positive() {
    let mut mmix = MMix::new();
    // ZSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, 30);
    mmix.set_register(3, 40);
    mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 40); // Condition true: $3
}

#[test]
fn test_zsnn_condition_true_zero() {
    let mut mmix = MMix::new();
    // ZSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, 0);
    mmix.set_register(3, 40);
    mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 40); // Condition true: $3
}

#[test]
fn test_zsnn_condition_false() {
    let mut mmix = MMix::new();
    // ZSNN $1, $2, $3 - If $2 < 0, set $1 = 0
    mmix.set_register(2, (-5i64) as u64);
    mmix.set_register(3, 40);
    mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zsnni() {
    let mut mmix = MMix::new();
    // ZSNNI $1, $2, 8 - If $2 >= 0, set $1 = 8, else $1 = 0
    mmix.set_register(2, 92);
    mmix.write_tetra(0, 0x79010208); // ZSNNI $1,$2,8
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 8); // Condition true: 8
}

#[test]
fn test_zsnz_condition_true() {
    let mut mmix = MMix::new();
    // ZSNZ $1, $2, $3 - If $2 != 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, 100);
    mmix.set_register(3, 200);
    mmix.write_tetra(0, 0x7A010203); // ZSNZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 200); // Condition true: $3
}

#[test]
fn test_zsnz_condition_false() {
    let mut mmix = MMix::new();
    // ZSNZ $1, $2, $3 - If $2 == 0, set $1 = 0
    mmix.set_register(2, 0);
    mmix.set_register(3, 200);
    mmix.write_tetra(0, 0x7A010203); // ZSNZ $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zsnzi() {
    let mut mmix = MMix::new();
    // ZSNZI $1, $2, 33 - If $2 != 0, set $1 = 33, else $1 = 0
    mmix.set_register(2, 67);
    mmix.write_tetra(0, 0x7B010221); // ZSNZI $1,$2,33
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 33); // Condition true: 33
}

#[test]
fn test_zsnp_condition_true_negative() {
    let mut mmix = MMix::new();
    // ZSNP $1, $2, $3 - If $2 <= 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, (-100i64) as u64);
    mmix.set_register(3, 25);
    mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 25); // Condition true: $3
}

#[test]
fn test_zsnp_condition_true_zero() {
    let mut mmix = MMix::new();
    // ZSNP $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = 0
    mmix.set_register(2, 0);
    mmix.set_register(3, 25);
    mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 25); // Condition true: $3
}

#[test]
fn test_zsnp_condition_false() {
    let mut mmix = MMix::new();
    // ZSNP $1, $2, $3 - Set $1 = 0 if $1 is positive
    mmix.set_register(1, 1);
    mmix.set_register(2, 50);
    mmix.set_register(3, 25);
    mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zsnpi() {
    let mut mmix = MMix::new();
    // ZSNPI $1, $2, 44 - If $2 <= 0, set $1 = 44, else $1 = 0
    mmix.set_register(2, 0);
    mmix.write_tetra(0, 0x7D01022C); // ZSNPI $1,$2,44
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 44); // Condition true: 44
}

#[test]
fn test_zsev_condition_true() {
    let mut mmix = MMix::new();
    // ZSEV $1, $2, $3 - If $2 is even, set $1 = $3, else $1 = 0
    mmix.set_register(2, 80);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x7E010203); // ZSEV $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 20); // Condition true: $3
}

#[test]
fn test_zsev_condition_false() {
    let mut mmix = MMix::new();
    // ZSEV $1, $2, $3 - If $2 is odd, set $1 = 0
    mmix.set_register(2, 7);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x7E010203); // ZSEV $1,$2,$3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 0); // Condition false: 0
}

#[test]
fn test_zsevi() {
    let mut mmix = MMix::new();
    // ZSEVI $1, $2, 12 - If $2 is even, set $1 = 12, else $1 = 0
    mmix.set_register(2, 88);
    mmix.write_tetra(0, 0x7F01020C); // ZSEVI $1,$2,12
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(1), 12); // Condition true: 12
}
