//! Instruction dispatch: `execute_instruction`, the opcode `match`, and the small branch/conditional-set helpers it shares with no one else.

use super::{MMix, RA_D, RA_V, SpecialReg, TrapCode};
use tracing::{debug, instrument};

mod control;
mod floating_point;
mod load_store;

impl MMix {
    // ========== Internal Helpers ==========

    /// Conditional set: if cond($Y), $X = $Z, else do nothing
    #[inline]
    fn cond_set_rr(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        if cond {
            let val_z = self.get_register(z);
            self.set_register(x, val_z);
        }
        self.advance_pc();
    }

    /// Conditional set with immediate: if cond($Y), $X = Z, else do nothing
    #[inline]
    fn cond_set_ri(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        if cond {
            self.set_register(x, z as u64);
        }
        self.advance_pc();
    }

    /// Zero or set with register: if cond($Y), $X = $Z, else $X = 0
    #[inline]
    fn zero_set_rr(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        let result = if cond { self.get_register(z) } else { 0 };
        self.set_register(x, result);
        self.advance_pc();
    }

    /// Zero or set with immediate: if cond($Y), $X = Z, else $X = 0
    #[inline]
    fn zero_set_ri(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        let result = if cond { z as u64 } else { 0 };
        self.set_register(x, result);
        self.advance_pc();
    }

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
            // Opcodes 0xE0-0xE3. Each places YZ in its own wyde and zeroes the
            // other 48 bits; the INC/OR/ANDN families below preserve them.
            Opcode::SETH => {
                // SETH $X, YZ - u($X) <- YZ * 2^48
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz << 48);
                self.advance_pc();
                true
            }
            Opcode::SETMH => {
                // SETMH $X, YZ - u($X) <- YZ * 2^32
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz << 32);
                self.advance_pc();
                true
            }
            Opcode::SETML => {
                // SETML $X, YZ - u($X) <- YZ * 2^16
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz << 16);
                self.advance_pc();
                true
            }
            Opcode::SETL => {
                // SETL $X, YZ - u($X) <- YZ
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz);
                self.advance_pc();
                true
            }
            Opcode::INCH => {
                // INCH $X, YZ - Increase by high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            Opcode::INCMH => {
                // INCMH $X, YZ - Increase by medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            Opcode::INCML => {
                // INCML $X, YZ - Increase by medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            Opcode::INCL => {
                // INCL $X, YZ - Increase by low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(yz));
                self.advance_pc();
                true
            }
            Opcode::ORH => {
                // ORH $X, YZ - OR with high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            Opcode::ORMH => {
                // ORMH $X, YZ - OR with medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            Opcode::ORML => {
                // ORML $X, YZ - OR with medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            Opcode::ORL => {
                // ORL $X, YZ - OR with low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current | yz);
                self.advance_pc();
                true
            }
            Opcode::ANDNH => {
                // ANDNH $X, YZ - AND-NOT with high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 48);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            Opcode::ANDNMH => {
                // ANDNMH $X, YZ - AND-NOT with medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 32);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            Opcode::ANDNML => {
                // ANDNML $X, YZ - AND-NOT with medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 16);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            Opcode::ANDNL => {
                // ANDNL $X, YZ - AND-NOT with low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !yz;
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }

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

            // Arithmetic instructions - MUL/DIV opcodes 0x18-0x1F
            Opcode::MUL => {
                // MUL $X, $Y, $Z - Multiply signed with overflow
                mul_rr!(self, op_byte, x, y, z)
            }
            Opcode::MULI => {
                // MULI $X, $Y, Z - Multiply signed immediate with overflow
                mul_ri!(self, op_byte, x, y, z)
            }
            Opcode::MULU => {
                // MULU $X, $Y, $Z - Multiply unsigned
                mulu_rr!(self, x, y, z)
            }
            Opcode::MULUI => {
                // MULUI $X, $Y, Z - Multiply unsigned immediate
                mulu_ri!(self, x, y, z)
            }
            Opcode::DIV => {
                // DIV $X, $Y, $Z - Divide signed
                div_rr!(self, op_byte, x, y, z)
            }
            Opcode::DIVI => {
                // DIVI $X, $Y, Z - Divide signed immediate
                div_ri!(self, op_byte, x, y, z)
            }
            Opcode::DIVU => {
                // DIVU $X, $Y, $Z - Divide unsigned
                divu_rr!(self, x, y, z)
            }
            Opcode::DIVUI => {
                // DIVUI $X, $Y, Z - Divide unsigned immediate
                divu_ri!(self, x, y, z)
            }
            // ADD/SUB and variants - opcodes 0x20-0x2F
            Opcode::ADD => {
                // ADD $X, $Y, $Z - Add signed with overflow check
                add_rr!(self, op_byte, x, y, z)
            }
            Opcode::ADDI => {
                // ADDI $X, $Y, Z - Add signed immediate with overflow check
                add_ri!(self, op_byte, x, y, z)
            }
            Opcode::ADDU => {
                // ADDU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_add)
            }
            Opcode::ADDUI => {
                // ADDUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_add)
            }
            Opcode::SUB => {
                // SUB $X, $Y, $Z - Subtract signed with overflow check
                sub_rr!(self, op_byte, x, y, z)
            }
            Opcode::SUBI => {
                // SUBI $X, $Y, Z - Subtract signed immediate with overflow check
                sub_ri!(self, op_byte, x, y, z)
            }
            Opcode::SUBU => {
                // SUBU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_sub)
            }
            Opcode::SUBUI => {
                // SUBUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_sub)
            }
            Opcode::ADDU2 => {
                // 2ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 2)
            }
            Opcode::ADDU2I => {
                // 2ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 2)
            }
            Opcode::ADDU4 => {
                // 4ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 4)
            }
            Opcode::ADDU4I => {
                // 4ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 4)
            }
            Opcode::ADDU8 => {
                // 8ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 8)
            }
            Opcode::ADDU8I => {
                // 8ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 8)
            }
            Opcode::ADDU16 => {
                // 16ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 16)
            }
            Opcode::ADDU16I => {
                // 16ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 16)
            }
            // CMP and NEG instructions - opcodes 0x30-0x37
            Opcode::CMP => {
                // CMP $X, $Y, $Z
                cmp_rr!(self, x, y, z, |v| v as i64)
            }
            Opcode::CMPI => {
                // CMPI $X, $Y, Z
                cmp_ri!(self, x, y, z, |v| v as i64, |v| v as i64)
            }
            Opcode::CMPU => {
                // CMPU $X, $Y, $Z
                cmp_rr!(self, x, y, z, |v| v)
            }
            Opcode::CMPUI => {
                // CMPUI $X, $Y, Z
                cmp_ri!(self, x, y, z, |v| v, |v| v as u64)
            }
            Opcode::NEG => {
                // NEG $X, Y, $Z - Negate with overflow check
                // Y is immediate constant, $Z is register
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let a = y as i64;
                let b = z_val as i64;
                let flags = match a.checked_sub(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                        0
                    }
                    None => {
                        self.set_register(x, a.wrapping_sub(b) as u64);
                        RA_V
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::NEGI => {
                // NEG $X, Y, Z - Negate immediate with overflow check
                // Both Y and Z are immediate constants
                let y_val = y as u64;
                let z_val = z as u64;
                let a = y as i64;
                let b = z as i64;
                let flags = match a.checked_sub(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                        0
                    }
                    // Y and Z are bytes, so a - b lies in [-255, 255] and this
                    // arm is unreachable from the immediate encoding. It mirrors
                    // NEG, where s($Z) can carry the difference out of range.
                    None => {
                        self.set_register(x, a.wrapping_sub(b) as u64);
                        RA_V
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::NEGU => {
                // NEGU $X, Y, $Z - Negate unsigned
                let a = y as u64;
                let b = self.get_register(z);
                self.set_register(x, a.wrapping_sub(b));
                self.advance_pc();
                true
            }
            Opcode::NEGUI => {
                // NEGU $X, Y, Z - Negate unsigned immediate
                let a = y as u64;
                let b = z as u64;
                self.set_register(x, a.wrapping_sub(b));
                self.advance_pc();
                true
            }
            // Shift instructions - opcodes 0x38-0x3F
            Opcode::SL => {
                // SL $X, $Y, $Z - Shift left with overflow check
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let val_y = y_val as i64;
                let shift = z_val;
                let flags = if shift >= 64 {
                    // Shift by 64 or more: result is 0, overflow unless Y was 0
                    self.set_register(x, 0);
                    if val_y != 0 { RA_V } else { 0 }
                } else {
                    let result = (val_y as u64) << shift;
                    self.set_register(x, result);
                    // Overflow exactly when s($Y)·2^u($Z) leaves the signed
                    // range, which is when shifting back does not restore $Y.
                    if ((result as i64) >> shift) != val_y {
                        RA_V
                    } else {
                        0
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SLI => {
                // SLI $X, $Y, Z - Shift left immediate with overflow check
                let y_val = self.get_register(y);
                // Z is the literal shift amount, not a register.
                let z_val = z as u64;
                let val_y = y_val as i64;
                let shift = z as u64;
                let flags = if shift >= 64 {
                    self.set_register(x, 0);
                    if val_y != 0 { RA_V } else { 0 }
                } else {
                    let result = (val_y as u64) << shift;
                    self.set_register(x, result);
                    if ((result as i64) >> shift) != val_y {
                        RA_V
                    } else {
                        0
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SLU => {
                // SLU $X, $Y, $Z - Shift left unsigned (no overflow check)
                let val_y = self.get_register(y);
                let shift = self.get_register(z);
                let result = if shift >= 64 { 0 } else { val_y << shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SLUI => {
                // SLUI $X, $Y, Z - Shift left unsigned immediate (no overflow check)
                let val_y = self.get_register(y);
                let shift = z as u64;
                let result = if shift >= 64 { 0 } else { val_y << shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SR => {
                // SR $X, $Y, $Z - Shift right (arithmetic)
                let val_y = self.get_register(y) as i64;
                let shift = self.get_register(z);
                let result = if shift >= 64 {
                    if val_y < 0 { -1i64 as u64 } else { 0 }
                } else {
                    (val_y >> shift) as u64
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SRI => {
                // SRI $X, $Y, Z - Shift right immediate (arithmetic)
                let val_y = self.get_register(y) as i64;
                let shift = z as u64;
                let result = if shift >= 64 {
                    if val_y < 0 { -1i64 as u64 } else { 0 }
                } else {
                    (val_y >> shift) as u64
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SRU => {
                // SRU $X, $Y, $Z - Shift right unsigned (logical)
                let val_y = self.get_register(y);
                let shift = self.get_register(z);
                let result = if shift >= 64 { 0 } else { val_y >> shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SRUI => {
                // SRUI $X, $Y, Z - Shift right unsigned immediate (logical)
                let val_y = self.get_register(y);
                let shift = z as u64;
                let result = if shift >= 64 { 0 } else { val_y >> shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            // Conditional Set instructions - opcodes 0x60-0x6F
            Opcode::CSN => {
                // CSN $X, $Y, $Z - Conditional Set if Negative (checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNI => {
                // CSNI $X, $Y, Z - Conditional Set if Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSZ => {
                // CSZ $X, $Y, $Z - Conditional Set if Zero (checks $Y)
                let cond = self.get_register(y) == 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSZI => {
                // CSZI $X, $Y, Z - Conditional Set if Zero (immediate, checks $Y)
                let cond = self.get_register(y) == 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSP => {
                // CSP $X, $Y, $Z - Conditional Set if Positive (checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSPI => {
                // CSPI $X, $Y, Z - Conditional Set if Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSOD => {
                // CSOD $X, $Y, $Z - Conditional Set if Odd (checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSODI => {
                // CSODI $X, $Y, Z - Conditional Set if Odd (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSNN => {
                // CSNN $X, $Y, $Z - Conditional Set if Non-Negative (checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNNI => {
                // CSNNI $X, $Y, Z - Conditional Set if Non-Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSNZ => {
                // CSNZ $X, $Y, $Z - Conditional Set if Non-Zero (checks $Y)
                let cond = self.get_register(y) != 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNZI => {
                // CSNZI $X, $Y, Z - Conditional Set if Non-Zero (immediate, checks $Y)
                let cond = self.get_register(y) != 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSNP => {
                // CSNP $X, $Y, $Z - Conditional Set if Non-Positive (checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNPI => {
                // CSNPI $X, $Y, Z - Conditional Set if Non-Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSEV => {
                // CSEV $X, $Y, $Z - Conditional Set if Even (checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSEVI => {
                // CSEVI $X, $Y, Z - Conditional Set if Even (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }

            // Zero or Set instructions (0x70-0x7F)
            Opcode::ZSN => {
                // ZSN $X, $Y, $Z - Zero or Set if Negative (checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNI => {
                // ZSNI $X, $Y, Z - Zero or Set if Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSZ => {
                // ZSZ $X, $Y, $Z - Zero or Set if Zero (checks $Y)
                let cond = self.get_register(y) == 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSZI => {
                // ZSZI $X, $Y, Z - Zero or Set if Zero (immediate, checks $Y)
                let cond = self.get_register(y) == 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSP => {
                // ZSP $X, $Y, $Z - Zero or Set if Positive (checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSPI => {
                // ZSPI $X, $Y, Z - Zero or Set if Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSOD => {
                // ZSOD $X, $Y, $Z - Zero or Set if Odd (checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSODI => {
                // ZSODI $X, $Y, Z - Zero or Set if Odd (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSNN => {
                // ZSNN $X, $Y, $Z - Zero or Set if Non-Negative (checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNNI => {
                // ZSNNI $X, $Y, Z - Zero or Set if Non-Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSNZ => {
                // ZSNZ $X, $Y, $Z - Zero or Set if Non-Zero (checks $Y)
                let cond = self.get_register(y) != 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNZI => {
                // ZSNZI $X, $Y, Z - Zero or Set if Non-Zero (immediate, checks $Y)
                let cond = self.get_register(y) != 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSNP => {
                // ZSNP $X, $Y, $Z - Zero or Set if Non-Positive (checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNPI => {
                // ZSNPI $X, $Y, Z - Zero or Set if Non-Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSEV => {
                // ZSEV $X, $Y, $Z - Zero or Set if Even (checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSEVI => {
                // ZSEVI $X, $Y, Z - Zero or Set if Even (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }

            // Bitwise operations - opcodes 0xC0-0xCF
            Opcode::OR => {
                // OR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a | b)
            }
            Opcode::ORI => {
                // ORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a | b)
            }
            Opcode::ORN => {
                // ORN $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| a | !b)
            }
            Opcode::ORNI => {
                // ORNI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| a | !b)
            }
            Opcode::NOR => {
                // NOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a | b))
            }
            Opcode::NORI => {
                // NORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a | b))
            }
            Opcode::XOR => {
                // XOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a ^ b)
            }
            Opcode::XORI => {
                // XORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a ^ b)
            }
            Opcode::AND => {
                // AND $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a & b)
            }
            Opcode::ANDI => {
                // ANDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a & b)
            }
            Opcode::ANDN => {
                // ANDN $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| a & !b)
            }
            Opcode::ANDNI => {
                // ANDNI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| a & !b)
            }
            Opcode::NAND => {
                // NAND $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a & b))
            }
            Opcode::NANDI => {
                // NANDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a & b))
            }
            Opcode::NXOR => {
                // NXOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a ^ b))
            }
            Opcode::NXORI => {
                // NXORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a ^ b))
            }
            // Bit fiddling operations - opcodes 0xD0-0xDF
            Opcode::BDIF => {
                // BDIF $X, $Y, $Z - Byte difference
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..8 {
                    let byte_y = ((val_y >> (i * 8)) & 0xFF) as u8;
                    let byte_z = ((val_z >> (i * 8)) & 0xFF) as u8;
                    let diff = byte_y.saturating_sub(byte_z);
                    result |= (diff as u64) << (i * 8);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::BDIFI => {
                // BDIFI $X, $Y, Z - Byte difference immediate. Z is the
                // octabyte #00...0Z (bdif.html): lane 0 subtracts, every
                // higher lane is $Y's lane unchanged.
                let val_y = self.get_register(y);
                let byte0 = (val_y & 0xFF) as u8;
                let diff = byte0.saturating_sub(z);
                let result = (val_y & !0xFFu64) | diff as u64;
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::WDIF => {
                // WDIF $X, $Y, $Z - Wyde difference
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..4 {
                    let wyde_y = ((val_y >> (i * 16)) & 0xFFFF) as u16;
                    let wyde_z = ((val_z >> (i * 16)) & 0xFFFF) as u16;
                    let diff = wyde_y.saturating_sub(wyde_z);
                    result |= (diff as u64) << (i * 16);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::WDIFI => {
                // WDIFI $X, $Y, Z - Wyde difference immediate. Z is the
                // octabyte #00...0Z: lane 0 subtracts, every higher lane
                // is $Y's lane unchanged.
                let val_y = self.get_register(y);
                let wyde0 = (val_y & 0xFFFF) as u16;
                let diff = wyde0.saturating_sub(z as u16);
                let result = (val_y & !0xFFFFu64) | diff as u64;
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::TDIF => {
                // TDIF $X, $Y, $Z - Tetra difference
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..2 {
                    let tetra_y = ((val_y >> (i * 32)) & 0xFFFFFFFF) as u32;
                    let tetra_z = ((val_z >> (i * 32)) & 0xFFFFFFFF) as u32;
                    let diff = tetra_y.saturating_sub(tetra_z);
                    result |= (diff as u64) << (i * 32);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::TDIFI => {
                // TDIFI $X, $Y, Z - Tetra difference immediate. Z is the
                // octabyte #00...0Z: lane 0 subtracts, every higher lane
                // is $Y's lane unchanged.
                let val_y = self.get_register(y);
                let tetra0 = (val_y & 0xFFFF_FFFF) as u32;
                let diff = tetra0.saturating_sub(z as u32);
                let result = (val_y & !0xFFFF_FFFFu64) | diff as u64;
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::ODIF => {
                // ODIF $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::saturating_sub)
            }
            Opcode::ODIFI => {
                // ODIFI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::saturating_sub)
            }
            Opcode::SADD => {
                // SADD $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| (a & !b).count_ones() as u64)
            }
            Opcode::SADDI => {
                // SADDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| (a & !b).count_ones() as u64)
            }
            Opcode::MOR => {
                // MOR $X, $Y, $Z - Multiple or (Boolean matrix multiplication)
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..8 {
                    for j in 0..8 {
                        let mut bit = false;
                        for k in 0..8 {
                            let y_bit = (val_y >> (k * 8 + j)) & 1;
                            let z_bit = (val_z >> (i * 8 + k)) & 1;
                            if y_bit != 0 && z_bit != 0 {
                                bit = true;
                                break;
                            }
                        }
                        if bit {
                            result |= 1 << (i * 8 + j);
                        }
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MORI => {
                // MORI $X, $Y, Z - Multiple or immediate
                let val_y = self.get_register(y);
                let mut result: u64 = 0;
                // For immediate form, only bottom byte of result is non-zero
                for j in 0..8 {
                    let mut bit = false;
                    for k in 0..8 {
                        let y_bit = (val_y >> (k * 8 + j)) & 1;
                        let z_bit = (z >> k) & 1;
                        if y_bit != 0 && z_bit != 0 {
                            bit = true;
                            break;
                        }
                    }
                    if bit {
                        result |= 1 << j;
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MXOR => {
                // MXOR $X, $Y, $Z - Multiple exclusive-or (matrix product over GF(2))
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..8 {
                    for j in 0..8 {
                        let mut bit = false;
                        for k in 0..8 {
                            let y_bit = (val_y >> (k * 8 + j)) & 1;
                            let z_bit = (val_z >> (i * 8 + k)) & 1;
                            if y_bit != 0 && z_bit != 0 {
                                bit = !bit;
                            }
                        }
                        if bit {
                            result |= 1 << (i * 8 + j);
                        }
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MXORI => {
                // MXORI $X, $Y, Z - Multiple exclusive-or immediate
                let val_y = self.get_register(y);
                let mut result: u64 = 0;
                for j in 0..8 {
                    let mut bit = false;
                    for k in 0..8 {
                        let y_bit = (val_y >> (k * 8 + j)) & 1;
                        let z_bit = (z >> k) & 1;
                        if y_bit != 0 && z_bit != 0 {
                            bit = !bit;
                        }
                    }
                    if bit {
                        result |= 1 << j;
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MUX => {
                // MUX $X, $Y, $Z - Bitwise multiplex
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mask = self.special_regs[SpecialReg::RM as usize];
                self.set_register(x, (val_y & mask) | (val_z & !mask));
                self.advance_pc();
                true
            }
            Opcode::MUXI => {
                // MUXI $X, $Y, Z - Bitwise multiplex immediate
                let val_y = self.get_register(y);
                let mask = self.special_regs[SpecialReg::RM as usize];
                self.set_register(x, (val_y & mask) | ((z as u64) & !mask));
                self.advance_pc();
                true
            }
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
