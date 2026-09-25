//! Instruction dispatch: `execute_instruction`, the opcode `match`, and the small branch/conditional-set helpers it shares with no one else.

use super::{MMix, SpecialReg, TrapCode};
use tracing::{debug, instrument};

mod control;
mod floating_point;
mod integer;
mod load_store;

impl MMix {
    // ========== Internal Helpers ==========

    /// The first must-be-zero field found nonzero, in `X, Y, Z` order, for
    /// the five opcodes the MMIX instruction reference names (get.html,
    /// put.html, save.html, gitraptrip.html "UNSAVE"/"RESUME"). `None` for
    /// every legal encoding and every other opcode.
    fn must_be_zero_violation(
        opcode: crate::mmixal::Opcode,
        x: u8,
        y: u8,
        z: u8,
    ) -> Option<(&'static str, char, u8)> {
        use crate::mmixal::Opcode;
        match opcode {
            Opcode::GET if y != 0 => Some(("GET", 'Y', y)),
            Opcode::PUT if y != 0 => Some(("PUT", 'Y', y)),
            Opcode::PUTI if y != 0 => Some(("PUTI", 'Y', y)),
            Opcode::SAVE if y != 0 => Some(("SAVE", 'Y', y)),
            Opcode::SAVE if z != 0 => Some(("SAVE", 'Z', z)),
            Opcode::UNSAVE if x != 0 => Some(("UNSAVE", 'X', x)),
            Opcode::UNSAVE if y != 0 => Some(("UNSAVE", 'Y', y)),
            Opcode::RESUME if x != 0 => Some(("RESUME", 'X', x)),
            Opcode::RESUME if y != 0 => Some(("RESUME", 'Y', y)),
            _ => None,
        }
    }

    // ========== Instruction Execution ==========

    /// Execute a single instruction at the current program counter.
    /// Returns true if execution should continue, false if halted.
    #[instrument(skip(self), fields(pc = format!("0x{:X}", self.pc)))]
    pub fn execute_instruction(&mut self) -> bool {
        let (op_byte, x, y, z) = self.fetch_instruction();
        debug!(
            op = format!("0x{:02X}", op_byte),
            x, y, z, "Executing instruction"
        );

        use crate::mmixal::Opcode;
        let opcode = Opcode::try_from(op_byte).unwrap_or_else(|_| {
            panic!("Invalid opcode {:#04x} at PC {:#018x}", op_byte, self.pc);
        });

        self.dispatch(opcode, op_byte, x, y, z)
    }

    /// Decode and run one instruction given its already-resolved opcode and
    /// fields. Split out of [`MMix::execute_instruction`] so `RESUME` can run
    /// the instruction carried in `rX` (§1 rule 4) as if fetched at a chosen
    /// address, with no real memory read.
    fn dispatch(
        &mut self,
        opcode: crate::mmixal::Opcode,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        use crate::mmixal::Opcode;

        // A nonzero must-be-zero field (get.html, put.html, save.html,
        // gitraptrip.html "UNSAVE"/"RESUME") is an illegal-instruction
        // interrupt, named in X, Y, Z order. This runs before every other
        // check the instruction makes and before this claims $X as a
        // local, so a rejected instruction leaves rL untouched.
        if let Some((mnemonic, field, value)) = Self::must_be_zero_violation(opcode, x, y, z) {
            return self.reject(&format!(
                "{mnemonic} {field}={value}: must be zero; illegal-instruction \
                 interrupt at PC={:#018x}",
                self.pc
            ));
        }

        // Operands are read before the destination raises rL. A marginal $Y
        // or $Z still reads as zero when the instruction executes.
        //
        // SAVE is excluded even though its X is a genuine destination: X
        // must already be global, so a legal SAVE never needs this claim,
        // and claiming a local X here would raise rL before SAVE's own
        // rejection runs, breaking its promise to leave a rejected machine
        // unchanged. SAVE's arm validates X itself.
        //
        // GET's Z must name a register before this claim runs, so a
        // rejected GET leaves rL untouched; a legal GET claims $X here like
        // every other destination opcode, so GET $X,rL sees the value rL
        // rises to.
        if opcode == Opcode::GET && SpecialReg::from_u8(z).is_none() {
            return self.reject(&format!(
                "GET X={x},Z={z}: no special register above 31 at PC={:#018x}",
                self.pc
            ));
        }

        if Self::writes_general_register_x(op_byte) && opcode != Opcode::SAVE {
            self.claim_local(x);
        }

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
            Opcode::FCMP
            | Opcode::FUN
            | Opcode::FEQL
            | Opcode::FADD
            | Opcode::FIX
            | Opcode::FSUB
            | Opcode::FIXU
            | Opcode::FLOT
            | Opcode::FLOTI
            | Opcode::FLOTU
            | Opcode::FLOTUI
            | Opcode::SFLOT
            | Opcode::SFLOTI
            | Opcode::SFLOTU
            | Opcode::SFLOTUI
            | Opcode::FMUL
            | Opcode::FCMPE
            | Opcode::FUNE
            | Opcode::FEQLE
            | Opcode::FDIV
            | Opcode::FSQRT
            | Opcode::FREM
            | Opcode::FINT => self.dispatch_floating_point(opcode, op_byte, x, y, z),

            Opcode::LDB
            | Opcode::LDBI
            | Opcode::LDBU
            | Opcode::LDBUI
            | Opcode::LDW
            | Opcode::LDWI
            | Opcode::LDWU
            | Opcode::LDWUI
            | Opcode::LDT
            | Opcode::LDTI
            | Opcode::LDTU
            | Opcode::LDTUI
            | Opcode::LDO
            | Opcode::LDOI
            | Opcode::LDOU
            | Opcode::LDOUI
            | Opcode::LDSF
            | Opcode::LDSFI
            | Opcode::LDHT
            | Opcode::LDHTI
            | Opcode::CSWAP
            | Opcode::CSWAPI
            | Opcode::LDUNC
            | Opcode::LDUNCI
            | Opcode::LDVTS
            | Opcode::LDVTSI
            | Opcode::PRELD
            | Opcode::PRELDI
            | Opcode::PREGO
            | Opcode::PREGOI
            | Opcode::STB
            | Opcode::STBI
            | Opcode::STBU
            | Opcode::STBUI
            | Opcode::STW
            | Opcode::STWI
            | Opcode::STWU
            | Opcode::STWUI
            | Opcode::STT
            | Opcode::STTI
            | Opcode::STTU
            | Opcode::STTUI
            | Opcode::STO
            | Opcode::STOI
            | Opcode::STOU
            | Opcode::STOUI
            | Opcode::STSF
            | Opcode::STSFI
            | Opcode::STHT
            | Opcode::STHTI
            | Opcode::STCO
            | Opcode::STCOI
            | Opcode::STUNC
            | Opcode::STUNCI
            | Opcode::SYNCD
            | Opcode::SYNCDI
            | Opcode::PREST
            | Opcode::PRESTI
            | Opcode::SYNCID
            | Opcode::SYNCIDI => self.dispatch_load_store(opcode, op_byte, x, y, z),
            Opcode::SETH
            | Opcode::SETMH
            | Opcode::SETML
            | Opcode::SETL
            | Opcode::INCH
            | Opcode::INCMH
            | Opcode::INCML
            | Opcode::INCL
            | Opcode::ORH
            | Opcode::ORMH
            | Opcode::ORML
            | Opcode::ORL
            | Opcode::ANDNH
            | Opcode::ANDNMH
            | Opcode::ANDNML
            | Opcode::ANDNL
            | Opcode::MUL
            | Opcode::MULI
            | Opcode::MULU
            | Opcode::MULUI
            | Opcode::DIV
            | Opcode::DIVI
            | Opcode::DIVU
            | Opcode::DIVUI
            | Opcode::ADD
            | Opcode::ADDI
            | Opcode::ADDU
            | Opcode::ADDUI
            | Opcode::SUB
            | Opcode::SUBI
            | Opcode::SUBU
            | Opcode::SUBUI
            | Opcode::ADDU2
            | Opcode::ADDU2I
            | Opcode::ADDU4
            | Opcode::ADDU4I
            | Opcode::ADDU8
            | Opcode::ADDU8I
            | Opcode::ADDU16
            | Opcode::ADDU16I
            | Opcode::CMP
            | Opcode::CMPI
            | Opcode::CMPU
            | Opcode::CMPUI
            | Opcode::NEG
            | Opcode::NEGI
            | Opcode::NEGU
            | Opcode::NEGUI
            | Opcode::SL
            | Opcode::SLI
            | Opcode::SLU
            | Opcode::SLUI
            | Opcode::SR
            | Opcode::SRI
            | Opcode::SRU
            | Opcode::SRUI
            | Opcode::CSN
            | Opcode::CSNI
            | Opcode::CSZ
            | Opcode::CSZI
            | Opcode::CSP
            | Opcode::CSPI
            | Opcode::CSOD
            | Opcode::CSODI
            | Opcode::CSNN
            | Opcode::CSNNI
            | Opcode::CSNZ
            | Opcode::CSNZI
            | Opcode::CSNP
            | Opcode::CSNPI
            | Opcode::CSEV
            | Opcode::CSEVI
            | Opcode::ZSN
            | Opcode::ZSNI
            | Opcode::ZSZ
            | Opcode::ZSZI
            | Opcode::ZSP
            | Opcode::ZSPI
            | Opcode::ZSOD
            | Opcode::ZSODI
            | Opcode::ZSNN
            | Opcode::ZSNNI
            | Opcode::ZSNZ
            | Opcode::ZSNZI
            | Opcode::ZSNP
            | Opcode::ZSNPI
            | Opcode::ZSEV
            | Opcode::ZSEVI
            | Opcode::OR
            | Opcode::ORI
            | Opcode::ORN
            | Opcode::ORNI
            | Opcode::NOR
            | Opcode::NORI
            | Opcode::XOR
            | Opcode::XORI
            | Opcode::AND
            | Opcode::ANDI
            | Opcode::ANDN
            | Opcode::ANDNI
            | Opcode::NAND
            | Opcode::NANDI
            | Opcode::NXOR
            | Opcode::NXORI
            | Opcode::BDIF
            | Opcode::BDIFI
            | Opcode::WDIF
            | Opcode::WDIFI
            | Opcode::TDIF
            | Opcode::TDIFI
            | Opcode::ODIF
            | Opcode::ODIFI
            | Opcode::SADD
            | Opcode::SADDI
            | Opcode::MOR
            | Opcode::MORI
            | Opcode::MXOR
            | Opcode::MXORI
            | Opcode::MUX
            | Opcode::MUXI => self.dispatch_integer(opcode, op_byte, x, y, z),

            Opcode::GO
            | Opcode::GOI
            | Opcode::PUSHGO
            | Opcode::PUSHGOI
            | Opcode::BN
            | Opcode::BNB
            | Opcode::BZ
            | Opcode::BZB
            | Opcode::BP
            | Opcode::BPB
            | Opcode::BOD
            | Opcode::BODB
            | Opcode::BNN
            | Opcode::BNNB
            | Opcode::BNZ
            | Opcode::BNZB
            | Opcode::BNP
            | Opcode::BNPB
            | Opcode::BEV
            | Opcode::BEVB
            | Opcode::PBN
            | Opcode::PBNB
            | Opcode::PBZ
            | Opcode::PBZB
            | Opcode::PBP
            | Opcode::PBPB
            | Opcode::PBOD
            | Opcode::PBODB
            | Opcode::PBNN
            | Opcode::PBNNB
            | Opcode::PBNZ
            | Opcode::PBNZB
            | Opcode::PBNP
            | Opcode::PBNPB
            | Opcode::PBEV
            | Opcode::PBEVB
            | Opcode::JMP
            | Opcode::JMPB
            | Opcode::PUSHJ
            | Opcode::PUSHJB
            | Opcode::GETA
            | Opcode::GETAB
            | Opcode::POP => self.dispatch_control(opcode, op_byte, x, y, z),

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
                // bits. Y must be zero; the must-be-zero check above rejects
                // a nonzero Y before this arm runs.
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
                // GET $X, $Z - Get from special register. Z is validated
                // ahead of the pre-dispatch $X claim, above.
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
        }
    }
}
