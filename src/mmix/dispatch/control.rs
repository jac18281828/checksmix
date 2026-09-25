//! The branch-and-call family: B*/PB*, GO/GOI, PUSHGO/PUSHGOI, JMP/JMPB,
//! PUSHJ/PUSHJB, GETA/GETAB and POP.

use super::super::{MMix, PopFrame, SpecialReg};
use crate::mmixal::Opcode;

impl MMix {
    /// Conditional branch forward: if cond, PC = PC + 4*YZ.
    #[inline]
    fn branch_forward(&mut self, cond: bool, y: u8, z: u8) {
        if cond {
            let yz = ((y as u16) << 8) | (z as u16);
            self.pc = self.pc.wrapping_add((yz as u64) * 4);
        } else {
            self.advance_pc();
        }
    }

    /// Conditional branch backward: if cond, PC = PC + 4*(YZ - 65536).
    #[inline]
    fn branch_backward(&mut self, cond: bool, y: u8, z: u8) {
        if cond {
            // YZ is unsigned in both directions; the backward opcode carries the
            // sign, so YZ = 0 is -65536 tetras and YZ = 65535 is -1.
            let yz = ((y as u16) << 8) | (z as u16);
            let offset = yz as i64 - 65536;
            self.pc = self.pc.wrapping_add((offset * 4) as u64);
        } else {
            self.advance_pc();
        }
    }

    /// Dispatches the branch-and-call family; `dispatch` routes only these
    /// opcodes here.
    pub(super) fn dispatch_control(
        &mut self,
        opcode: Opcode,
        _op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        match opcode {
            // Special Load/Store instructions (0x9E-0x9F): GO, GOI
            Opcode::GO => {
                // GO $X, $Y, $Z - Go to location. u($X) <- @+4 mod 2^64,
                // per the Instruction Reference's GO page.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.set_register(x, self.pc.wrapping_add(4));
                self.pc = addr;
                true
            }
            Opcode::GOI => {
                // GOI $X, $Y, Z - Go to location immediate. Same rule as GO.
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.set_register(x, self.pc.wrapping_add(4));
                self.pc = addr;
                true
            }
            // Store-range call instructions (0xBE-0xBF): PUSHGO, PUSHGOI
            Opcode::PUSHGO => {
                // PUSHGO $X, $Y, $Z - Push registers and go (absolute target $Y+$Z)
                let target = self.get_register(y).wrapping_add(self.get_register(z));
                self.push_frame(x);
                self.set_pc(target);
                true
            }
            Opcode::PUSHGOI => {
                // PUSHGOI $X, $Y, Z - Push registers and go (absolute target $Y+Z)
                let target = self.get_register(y).wrapping_add(z as u64);
                self.push_frame(x);
                self.set_pc(target);
                true
            }
            // Branch instructions - opcodes 0x40-0x5F
            Opcode::BN => {
                // BN $X, $Y, Z - Branch if negative
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNB => {
                // BNB $X, $Y, Z - Branch if negative (backward)
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BZ => {
                // BZ $X, $Y, Z - Branch if zero
                let cond = self.get_register(x) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BZB => {
                // BZB $X, $Y, Z - Branch if zero (backward)
                let cond = self.get_register(x) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BP => {
                // BP $X, $Y, Z - Branch if positive
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BPB => {
                // BPB $X, $Y, Z - Branch if positive (backward)
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BOD => {
                // BOD $X, $Y, Z - Branch if odd
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BODB => {
                // BODB $X, $Y, Z - Branch if odd (backward)
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BNN => {
                // BNN $X, $Y, Z - Branch if non-negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNNB => {
                // BNNB $X, $Y, Z - Branch if non-negative (backward)
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BNZ => {
                // BNZ $X, $Y, Z - Branch if non-zero
                let cond = self.get_register(x) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNZB => {
                // BNZB $X, $Y, Z - Branch if non-zero (backward)
                let cond = self.get_register(x) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BNP => {
                // BNP $X, $Y, Z - Branch if non-positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNPB => {
                // BNPB $X, $Y, Z - Branch if non-positive (backward)
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BEV => {
                // BEV $X, $Y, Z - Branch if even
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BEVB => {
                // BEVB $X, $Y, Z - Branch if even (backward)
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBN => {
                // PBN $X, $Y, Z - Probable branch if negative
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNB => {
                // PBNB $X, $Y, Z - Probable branch if negative (backward)
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBZ => {
                // PBZ $X, $Y, Z - Probable branch if zero
                let cond = self.get_register(x) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBZB => {
                // PBZB $X, $Y, Z - Probable branch if zero (backward)
                let cond = self.get_register(x) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBP => {
                // PBP $X, $Y, Z - Probable branch if positive
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBPB => {
                // PBPB $X, $Y, Z - Probable branch if positive (backward)
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBOD => {
                // PBOD $X, $Y, Z - Probable branch if odd
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBODB => {
                // PBODB $X, $Y, Z - Probable branch if odd (backward)
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBNN => {
                // PBNN $X, $Y, Z - Probable branch if non-negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNNB => {
                // PBNNB $X, $Y, Z - Probable branch if non-negative (backward)
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBNZ => {
                // PBNZ $X, $Y, Z - Probable branch if non-zero
                let cond = self.get_register(x) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNZB => {
                // PBNZB $X, $Y, Z - Probable branch if non-zero (backward)
                let cond = self.get_register(x) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBNP => {
                // PBNP $X, $Y, Z - Probable branch if non-positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNPB => {
                // PBNPB $X, $Y, Z - Probable branch if non-positive (backward)
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBEV => {
                // PBEV $X, $Y, Z - Probable branch if even
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBEVB => {
                // PBEVB $X, $Y, Z - Probable branch if even (backward)
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            // Jump/Stack instructions - opcodes 0xF0-0xF5, 0xF8
            Opcode::JMP => {
                // JMP XYZ - Jump to PC + 4*XYZ.
                let xyz = ((x as u32) << 16) | ((y as u32) << 8) | (z as u32);
                self.pc = self.pc.wrapping_add((xyz as u64) * 4);
                true
            }
            Opcode::JMPB => {
                // JMPB XYZ - Jump to PC + 4*(XYZ - 2^24).
                let xyz = ((x as u32) << 16) | ((y as u32) << 8) | (z as u32);
                let offset = xyz as i64 - (1 << 24);
                self.pc = self.pc.wrapping_add((offset * 4) as u64);
                true
            }
            Opcode::PUSHJ => {
                // PUSHJ $X, YZ - Push registers and jump to PC + 4*YZ.
                let pc_at_inst = self.pc;
                self.push_frame(x);
                let yz = ((y as u16) << 8) | z as u16;
                self.pc = pc_at_inst.wrapping_add((yz as u64) * 4);
                true
            }
            Opcode::PUSHJB => {
                // PUSHJB $X, YZ - Push registers and jump to PC + 4*(YZ - 65536).
                let pc_at_inst = self.pc;
                self.push_frame(x);
                let yz = ((y as u16) << 8) | z as u16;
                let offset = yz as i64 - 65536;
                self.pc = pc_at_inst.wrapping_add((offset * 4) as u64);
                true
            }
            Opcode::GETA => {
                // GETA $X, YZ - Address PC + 4*YZ.
                let yz = ((y as u16) << 8) | z as u16;
                let addr = self.pc.wrapping_add((yz as u64) * 4);
                self.set_register(x, addr);
                self.advance_pc();
                true
            }
            Opcode::GETAB => {
                // GETAB $X, YZ - Address PC + 4*(YZ - 65536).
                let yz = ((y as u16) << 8) | z as u16;
                let offset = yz as i64 - 65536;
                let addr = self.pc.wrapping_add((offset * 4) as u64);
                self.set_register(x, addr);
                self.advance_pc();
                true
            }
            Opcode::POP => {
                // POP X, YZ - Pop frame and return.
                // Puts the callee's last output in the hole and the rest
                // above it in order, restores caller's locals below the
                // hole, and branches to rJ + 4·YZ. rJ itself is untouched;
                // a subroutine that calls another must save and restore it.
                let yz = ((y as u16) << 8) | z as u16;
                match self.pop_frame(x, yz) {
                    PopFrame::Frame(target) => {
                        self.pc = target;
                        true
                    }
                    PopFrame::NoFrame => {
                        // No frame to pop: branch via current rJ as a defensive fallback.
                        self.pc = self
                            .get_special(SpecialReg::RJ)
                            .wrapping_add((yz as u64) * 4);
                        true
                    }
                    PopFrame::Rejected => false,
                }
            }
            _ => unreachable!("dispatch routes only branch-and-call opcodes to dispatch_control"),
        }
    }
}
