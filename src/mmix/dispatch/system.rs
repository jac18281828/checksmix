//! The system family: TRAP, PUT/PUTI, RESUME, SAVE, UNSAVE, SYNC, SWYM,
//! GET and TRIP.

use super::super::{MMix, SpecialReg, TrapCode};
use crate::mmixal::Opcode;
use tracing::debug;

impl MMix {
    /// Dispatches the system family; `dispatch` routes only these opcodes
    /// here.
    pub(super) fn dispatch_system(
        &mut self,
        opcode: Opcode,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        match opcode {
            Opcode::TRAP => {
                // TRAP X, YZ or TRAP X, Y, Z - Force trap interrupt
                // X = 0 for immediate (YZ), X > 0 for register ($Y, $Z)
                // For immediate form: Y is the trap number, Z is an argument
                if x == 0 {
                    // Immediate TRAP - handle system calls
                    match TrapCode::from_u8(y) {
                        Some(trap) => self.handle_trap(trap, z),
                        None => {
                            debug!(trap_code = y, arg = z, "TRAP: Unhandled trap code");
                            self.host.diagnostic(&format!(
                                "Unhandled TRAP code {} at PC={:#018x}",
                                y, self.pc
                            ));
                            self.advance_pc();
                            true
                        }
                    }
                } else {
                    // Register form - not commonly used for syscalls
                    let trap_val = {
                        let y_val = self.get_register(y);
                        let z_val = self.get_register(z);
                        (y_val << 32) | z_val
                    };
                    self.host.diagnostic(&format!(
                        "Register TRAP x={} y=$ {} z=$ {} val=0x{:016X} at PC={:#018x}",
                        x, y, z, trap_val, self.pc
                    ));
                    self.set_special(SpecialReg::RBB, trap_val);
                    self.advance_pc();
                    self.exit_code = 1;
                    false // Halt by default for unhandled register traps
                }
            }
            // Jump/Stack/System instructions - opcodes 0xF6-0xF7, 0xF9-0xFF
            Opcode::PUT => {
                // PUT X, $Z - Put $Z into special register X.
                let value = self.get_register(z);
                if !self.put_special("PUT", x, value) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::PUTI => {
                // PUT X, Z - Put immediate Z into special register X, eight
                // bits. Y must be zero; dispatch's prologue rejects a nonzero Y
                // before this arm runs.
                if !self.put_special("PUTI", x, z as u64) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::RESUME => {
                // RESUME Z (gitraptrip.html "RESUME"; §1 rule 4): X and Y
                // are always zero, so only Z varies. Z != 0 is RESUME 1,
                // which restores from the kernel's rWW/rXX/rYY/rZZ and is
                // privileged. rX < 0 resumes at rW. Otherwise rX's top byte
                // is the ropcode: 0 inserts rX's instruction as if it stood
                // at rW-4 and resumes at rW; 1 substitutes rY/rZ into an
                // interruptible instruction; 2 serves forced-trap emulation;
                // 3 inserts a page-table entry. checksmix has no
                // interruptible instruction, no emulated opcode and no page
                // table, so ropcodes 1-3 never arise legitimately. Each
                // unsupported form halts through the shared MMix::reject
                // path: diagnostic, false, PC unmoved.
                if z != 0 {
                    return self.reject(&format!(
                        "RESUME {z}: privileged form (RESUME 1) at PC={:#018x}",
                        self.pc
                    ));
                }
                let rx = self.get_special(SpecialReg::RX);
                if (rx as i64) < 0 {
                    self.pc = self.get_special(SpecialReg::RW);
                    return true;
                }
                let ropcode = rx >> 56;
                if ropcode != 0 {
                    return self.reject(&format!(
                        "RESUME: ropcode {ropcode} at PC={:#018x} \
                         (only ropcode 0 and rX < 0 are supported)",
                        self.pc
                    ));
                }
                let word = rx as u32;
                let ins_op = (word >> 24) as u8;
                let ins_x = (word >> 16) as u8;
                let ins_y = (word >> 8) as u8;
                let ins_z = word as u8;
                let ins_opcode = match Opcode::try_from(ins_op) {
                    Ok(op) => op,
                    Err(_) => {
                        return self.reject(&format!(
                            "RESUME: invalid opcode {ins_op:#04x} in rX at PC={:#018x}",
                            self.pc
                        ));
                    }
                };
                self.pc = self.get_special(SpecialReg::RW).wrapping_sub(4);
                self.dispatch(ins_opcode, ins_op, ins_x, ins_y, ins_z)
            }
            Opcode::SAVE => {
                // SAVE $X,0 - push the machine's context onto the register
                // stack. See MMix::save_context.
                if !self.save_context(x) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::UNSAVE => {
                // UNSAVE 0,$Z - restore the context $Z addresses. See
                // MMix::unsave_context.
                let packed_addr = self.get_register(z);
                if !self.unsave_context(packed_addr) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::SYNC => {
                // SYNC XYZ (sync.html): 0-3 is a no-op user programs may
                // issue; 4-7 is reserved for the kernel; above 7 names
                // nothing.
                let xyz = ((x as u32) << 16) | ((y as u32) << 8) | z as u32;
                match xyz {
                    0..=3 => {
                        self.advance_pc();
                        true
                    }
                    4..=7 => self.reject(&format!(
                        "SYNC {xyz}: privileged-operation interrupt at PC={:#018x}",
                        self.pc
                    )),
                    _ => self.reject(&format!(
                        "SYNC {xyz}: illegal-instruction interrupt at PC={:#018x}",
                        self.pc
                    )),
                }
            }
            Opcode::SWYM => {
                // SWYM XYZ - Sympathize with your machinery (no-op)
                self.advance_pc();
                true
            }
            Opcode::GET => {
                // GET $X, $Z - Get from special register. Z is validated in
                // dispatch's prologue, ahead of the $X claim.
                let special_reg = SpecialReg::from_u8(z)
                    .expect("dispatch rejected an unnamed Z before this arm ran");
                let value = self.get_special(special_reg);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::TRIP => {
                // TRIP X,Y,Z: user trip to the handler at #00 (trip.html;
                // §1 rule 3).
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                self.trip(0x00, "TRIP", op_byte, x, y, z, y_val, z_val)
            }
            _ => unreachable!("dispatch routes only system opcodes to dispatch_system"),
        }
    }
}
