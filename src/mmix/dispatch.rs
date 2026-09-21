//! Instruction dispatch: `execute_instruction`, the opcode `match`, and the small branch/conditional-set helpers it shares with no one else.

use super::{MMix, PopFrame, RA_D, RA_I, RA_V, RA_W, RA_X, RA_Z, SpecialReg, TrapCode};
use tracing::{debug, instrument};

impl MMix {
    // ========== Internal Helpers ==========

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
            // Floating Point instructions
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
            Opcode::FCMP => {
                // FCMP $X, $Y, $Z - Floating compare. Raises I when an operand is NaN.
                let y_raw = self.get_register(y);
                let z_raw = self.get_register(z);
                let y_val = Self::u64_to_f64(y_raw);
                let z_val = Self::u64_to_f64(z_raw);
                let flags = if y_val.is_nan() || z_val.is_nan() {
                    RA_I
                } else {
                    0
                };
                self.set_register(x, Self::fcmp(y_val, z_val));
                self.raise_exceptions(flags, op_byte, x, y, z, y_raw, z_raw)
            }
            Opcode::FUN => {
                // FUN $X, $Y, $Z - Floating unordered (no exception flag)
                fcmp_rr!(
                    self,
                    x,
                    y,
                    z,
                    |y: f64, z: f64| if y.is_nan() || z.is_nan() { 1 } else { 0 }
                )
            }
            Opcode::FEQL => {
                // FEQL $X, $Y, $Z - Floating equal to (no exception flag)
                fcmp_rr!(self, x, y, z, |y: f64, z: f64| if y == z { 1 } else { 0 })
            }
            Opcode::FADD => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                self.fadd_compute(op_byte, x, y, z, y_val, z_val, a, b)
            }
            Opcode::FIX => {
                // FIX $X, Y, $Z - Convert floating to fixed (signed). Raises
                // X on inexact and W when the rounded value falls outside
                // the signed 64-bit range; an infinite or NaN operand
                // copies through unchanged with I alone.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FIX", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let f = Self::u64_to_f64(z_val);
                let mut flags = 0u64;
                let value = if !f.is_finite() {
                    flags |= RA_I;
                    z_val
                } else {
                    let rounded = Self::round_with_mode(f, mode);
                    if rounded != f {
                        flags |= RA_X;
                    }
                    if Self::out_of_signed_i64_range(rounded) {
                        flags |= RA_W;
                    }
                    Self::wrap_to_u64(rounded)
                };
                self.set_register(x, value);
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FSUB => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let z_raw = Self::u64_to_f64(z_val);
                // FSUB is FADD of -$Z; a NaN $Z passes through unnegated so
                // its own sign selects the result, per MMIX's standard
                // conventions.
                let b = if z_raw.is_nan() { z_raw } else { -z_raw };
                self.fadd_compute(op_byte, x, y, z, y_val, z_val, a, b)
            }
            Opcode::FIXU => {
                // FIXU $X, Y, $Z - Convert floating to fixed unsigned. Never
                // raises W; an infinite or NaN operand copies through
                // unchanged with I alone.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FIXU", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let f = Self::u64_to_f64(z_val);
                let mut flags = 0u64;
                let value = if !f.is_finite() {
                    flags |= RA_I;
                    z_val
                } else {
                    let rounded = Self::round_with_mode(f, mode);
                    if rounded != f {
                        flags |= RA_X;
                    }
                    Self::wrap_to_u64(rounded)
                };
                self.set_register(x, value);
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FLOT => {
                // FLOT $X, Y, $Z - Convert fixed to floating (signed)
                i2f_conv_rr!(self, op_byte, x, y, z, true, "FLOT")
            }
            Opcode::FLOTI => {
                // FLOTI $X, Y, Z - Convert fixed to floating immediate. Z is
                // an unsigned byte, like every immediate operand.
                i2f_conv_ri!(self, op_byte, x, y, z, "FLOTI")
            }
            Opcode::FLOTU => {
                // FLOTU $X, Y, $Z - Convert fixed unsigned to floating
                i2f_conv_rr!(self, op_byte, x, y, z, false, "FLOTU")
            }
            Opcode::FLOTUI => {
                // FLOTUI $X, Y, Z - Convert fixed unsigned to floating immediate
                i2f_conv_ri!(self, op_byte, x, y, z, "FLOTUI")
            }
            Opcode::SFLOT => {
                // SFLOT $X, Y, $Z - Convert signed integer to f32 (in f64 register)
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let v = z_val as i64;
                let flags = Self::int_to_f64_inexact(v.unsigned_abs());
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(v as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SFLOTI => {
                // SFLOTI $X, Y, Z - Z is an unsigned byte, like every
                // immediate operand.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTI", y),
                };
                // Y is the rounding-mode field and Z the literal operand,
                // neither a register.
                let y_val = y as u64;
                let z_val = z as u64;
                let flags = Self::int_to_f64_inexact(z_val);
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(z_val as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SFLOTU => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTU", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let v = self.get_register(z);
                let flags = Self::int_to_f64_inexact(v);
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(v as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, v)
            }
            Opcode::SFLOTUI => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTUI", y),
                };
                // Y is the rounding-mode field and Z the literal operand,
                // neither a register.
                let y_val = y as u64;
                let z_val = z as u64;
                let flags = Self::int_to_f64_inexact(z as u64);
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(z as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FMUL => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                if let Some((result, flags)) = Self::fmul_special(a, b) {
                    self.set_register(x, Self::f64_to_u64(result));
                    return self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val);
                }
                let r_near = a * b;
                let err = Self::fmul_error_sign(a, b, r_near);
                // A product of nonzero finite operands is never
                // mathematically zero.
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, false, false);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FCMPE => {
                // FCMPE $X, $Y, $Z - the ε-relation: −1 (≺), 0 (∼, an
                // epsilon-close pair), or +1 (≻). Forces 0 and raises I on
                // an exceptional input; never both -1/+1 and I.
                let y_raw = self.get_register(y);
                let z_raw = self.get_register(z);
                let y_val = Self::u64_to_f64(y_raw);
                let z_val = Self::u64_to_f64(z_raw);
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let exceptional = Self::epsilon_exceptional(y_val, z_val, epsilon);
                let result = if exceptional
                    || Self::in_epsilon_neighborhood(y_val, z_val, epsilon)
                    || Self::in_epsilon_neighborhood(z_val, y_val, epsilon)
                {
                    0
                } else if y_val < z_val {
                    (-1i64) as u64
                } else {
                    1
                };
                self.set_register(x, result);
                let flags = if exceptional { RA_I } else { 0 };
                self.raise_exceptions(flags, op_byte, x, y, z, y_raw, z_raw)
            }
            Opcode::FUNE => {
                // FUNE $X, $Y, $Z - reports only whether $Y, $Z, or rE is
                // exceptional (NaN operand, or rE NaN/negative); says
                // nothing about proximity, unlike FCMPE/FEQLE's ∼. Exempt
                // from the invalid exception FCMPE/FEQLE raise on that same
                // condition — raises no flag either way.
                let y_val = Self::u64_to_f64(self.get_register(y));
                let z_val = Self::u64_to_f64(self.get_register(z));
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let result = if Self::epsilon_exceptional(y_val, z_val, epsilon) {
                    1
                } else {
                    0
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::FEQLE => {
                // FEQLE $X, $Y, $Z - ≈: both directions of Nε membership
                // must hold, stronger than FCMPE's ∼.
                let y_raw = self.get_register(y);
                let z_raw = self.get_register(z);
                let y_val = Self::u64_to_f64(y_raw);
                let z_val = Self::u64_to_f64(z_raw);
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let exceptional = Self::epsilon_exceptional(y_val, z_val, epsilon);
                let result = if exceptional {
                    0
                } else if Self::in_epsilon_neighborhood(y_val, z_val, epsilon)
                    && Self::in_epsilon_neighborhood(z_val, y_val, epsilon)
                {
                    1
                } else {
                    0
                };
                self.set_register(x, result);
                let flags = if exceptional { RA_I } else { 0 };
                self.raise_exceptions(flags, op_byte, x, y, z, y_raw, z_raw)
            }
            Opcode::FDIV => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                if let Some((result, flags)) = Self::fdiv_special(a, b) {
                    self.set_register(x, Self::f64_to_u64(result));
                    return self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val);
                }
                if b == 0.0 && a.is_finite() && a != 0.0 {
                    // A finite nonzero dividend over zero is an exact
                    // infinity in every rounding mode: Z alone, no overflow
                    // clamp.
                    let result = a / b;
                    self.set_register(x, Self::f64_to_u64(result));
                    return self.raise_exceptions(RA_Z, op_byte, x, y, z, y_val, z_val);
                }
                let r_near = a / b;
                let operands_finite = a.is_finite() && b.is_finite();
                let err = if operands_finite {
                    Self::fdiv_error_sign(a, b, r_near)
                } else {
                    // An infinite operand's quotient (0, or an infinity
                    // from a finite-over-infinite or infinite-over-finite
                    // division) is exact; there is no residual to report.
                    0.0
                };
                // A quotient of nonzero finite operands is never mathematically
                // zero, so a zero result from such operands underflowed.
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, false, false);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FSQRT => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FSQRT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(z_val);
                if let Some((result, flags)) = Self::fsqrt_special(a) {
                    self.set_register(x, Self::f64_to_u64(result));
                    return self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val);
                }
                let r_near = a.sqrt();
                let err = Self::fsqrt_error_sign(a, r_near);
                let (r, flags) = self.finalize_fp_unop(a, r_near, err, mode);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FREM => {
                // FREM $X, $Y, $Z - IEEE 754 floating remainder.
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                if let Some((result, flags)) = Self::frem_special(a, b) {
                    self.set_register(x, Self::f64_to_u64(result));
                    return self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val);
                }
                let r = Self::ieee_remainder(a, b);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(0, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FINT => {
                // FINT $X, Y, $Z — Integerize under Y's rounding-mode
                // override, or rA's own mode when Y is 0.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FINT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let v = Self::u64_to_f64(z_val);
                if let Some((result, flags)) = Self::fint_special(v) {
                    self.set_register(x, Self::f64_to_u64(result));
                    return self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val);
                }
                let r = Self::round_with_mode(v, mode);
                let flags = if v.is_finite() && r != v { RA_X } else { 0 };
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }

            // Load instructions
            Opcode::LDB => {
                // LDB $X, $Y, $Z - Load byte signed
                // s($X) <- s(M[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let byte = self.read_byte(addr);
                let value = (byte as i8) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDBI => {
                // LDB $X, $Y, Z - Load byte signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let byte = self.read_byte(addr);
                let value = (byte as i8) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDBU => {
                // LDBU $X, $Y, $Z - Load byte unsigned
                // u($X) <- M[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let byte = self.read_byte(addr);
                self.set_register(x, byte as u64);
                self.advance_pc();
                true
            }
            Opcode::LDBUI => {
                // LDBU $X, $Y, Z - Load byte unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let byte = self.read_byte(addr);
                self.set_register(x, byte as u64);
                self.advance_pc();
                true
            }
            Opcode::LDW => {
                // LDW $X, $Y, $Z - Load wyde signed
                // s($X) <- s(M2[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let wyde = self.read_wyde(addr);
                let value = (wyde as i16) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDWI => {
                // LDW $X, $Y, Z - Load wyde signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let wyde = self.read_wyde(addr);
                let value = (wyde as i16) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDWU => {
                // LDWU $X, $Y, $Z - Load wyde unsigned
                // u($X) <- M2[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let wyde = self.read_wyde(addr);
                self.set_register(x, wyde as u64);
                self.advance_pc();
                true
            }
            Opcode::LDWUI => {
                // LDWU $X, $Y, Z - Load wyde unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let wyde = self.read_wyde(addr);
                self.set_register(x, wyde as u64);
                self.advance_pc();
                true
            }
            Opcode::LDT => {
                // LDT $X, $Y, $Z - Load tetra signed
                // s($X) <- s(M4[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as i32) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDTI => {
                // LDT $X, $Y, Z - Load tetra signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as i32) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDTU => {
                // LDTU $X, $Y, $Z - Load tetra unsigned
                // u($X) <- M4[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                self.set_register(x, tetra as u64);
                self.advance_pc();
                true
            }
            Opcode::LDTUI => {
                // LDTU $X, $Y, Z - Load tetra unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                self.set_register(x, tetra as u64);
                self.advance_pc();
                true
            }
            Opcode::LDO => {
                // LDO $X, $Y, $Z - Load octa
                // u($X) <- M8[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDOI => {
                // LDO $X, $Y, Z - Load octa (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDOU => {
                // LDOU $X, $Y, $Z - Load octa unsigned (same as LDO)
                // u($X) <- M8[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDOUI => {
                // LDOU $X, $Y, Z - Load octa unsigned (immediate, same as LDO)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDSF => {
                // LDSF $X, $Y, $Z - Load short float (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                self.set_register(x, Self::widen_short_float(tetra));
                self.advance_pc();
                true
            }
            Opcode::LDSFI => {
                // LDSFI $X, $Y, Z - Load short float immediate (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                self.set_register(x, Self::widen_short_float(tetra));
                self.advance_pc();
                true
            }
            Opcode::ADDU => {
                // ADDU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_add)
            }
            Opcode::ADDUI => {
                // ADDUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_add)
            }
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

            // Special Load/Store instructions (0x92-0x9F)
            Opcode::LDHT => {
                // LDHT $X, $Y, $Z - Load high tetra
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDHTI => {
                // LDHTI $X, $Y, Z - Load high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::CSWAP => {
                // CSWAP $X, $Y, $Z - Compare and swap octabytes
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let mem_value = self.read_octa(addr);
                let compare_value = self.get_special(SpecialReg::RP);
                if mem_value == compare_value {
                    // Values match, perform swap
                    self.write_octa(addr, self.get_register(x));
                    self.set_register(x, 1); // Success
                } else {
                    // Values don't match: on failure rP <- M8[$Y+$Z], giving the
                    // caller the current value to retry with.
                    self.set_special(SpecialReg::RP, mem_value);
                    self.set_register(x, 0); // Failure
                }
                self.advance_pc();
                true
            }
            Opcode::CSWAPI => {
                // CSWAPI $X, $Y, Z - Compare and swap octabytes immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let mem_value = self.read_octa(addr);
                let compare_value = self.get_special(SpecialReg::RP);
                if mem_value == compare_value {
                    // Values match, perform swap
                    self.write_octa(addr, self.get_register(x));
                    self.set_register(x, 1); // Success
                } else {
                    // Values don't match: on failure rP <- M8[$Y+Z], giving the
                    // caller the current value to retry with.
                    self.set_special(SpecialReg::RP, mem_value);
                    self.set_register(x, 0); // Failure
                }
                self.advance_pc();
                true
            }
            Opcode::LDUNC => {
                // LDUNC $X, $Y, $Z - Load uncached (treat as normal load)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.read_octa(addr);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDUNCI => {
                // LDUNCI $X, $Y, Z - Load uncached immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.read_octa(addr);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDVTS => {
                // LDVTS $X, $Y, $Z - Load virtual translation status (simplified)
                // In a full implementation, this would interact with virtual memory
                // For now, return 0 (no translation)
                self.set_register(x, 0);
                self.advance_pc();
                true
            }
            Opcode::LDVTSI => {
                // LDVTSI $X, $Y, Z - Load virtual translation status immediate
                self.set_register(x, 0);
                self.advance_pc();
                true
            }
            Opcode::PRELD => {
                // PRELD $X, $Y, $Z - Preload data (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::PRELDI => {
                // PRELDI $X, $Y, Z - Preload data immediate (hint, no-op)
                self.advance_pc();
                true
            }
            Opcode::PREGO => {
                // PREGO $X, $Y, $Z - Preload to go (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::PREGOI => {
                // PREGOI $X, $Y, Z - Preload to go immediate (hint, no-op)
                self.advance_pc();
                true
            }
            Opcode::GO => {
                // GO $X, $Y, $Z - Go to location
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.set_register(x, self.pc + 4); // Save return address
                self.pc = addr;
                true
            }
            Opcode::GOI => {
                // GOI $X, $Y, Z - Go to location immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.set_register(x, self.pc + 4); // Save return address
                self.pc = addr;
                true
            }

            // Store instructions
            Opcode::STB => {
                // STB $X, $Y, $Z - Store byte (with overflow check). A trip
                // sets rY to the address and rZ to the merged octabyte after
                // the store.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i8::MIN as i64, i8::MAX as i64);
                self.write_byte(addr, value as u8);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STBI => {
                // STB $X, $Y, Z - Store byte immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i8::MIN as i64, i8::MAX as i64);
                self.write_byte(addr, value as u8);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STBU => {
                // STBU $X, $Y, $Z - Store byte unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            Opcode::STBUI => {
                // STBU $X, $Y, Z - Store byte unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            Opcode::STW => {
                // STW $X, $Y, $Z - Store wyde (with overflow check). A trip
                // sets rY to the address and rZ to the merged octabyte after
                // the store.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i16::MIN as i64, i16::MAX as i64);
                self.write_wyde(addr, value as u16);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STWI => {
                // STW $X, $Y, Z - Store wyde immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i16::MIN as i64, i16::MAX as i64);
                self.write_wyde(addr, value as u16);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STWU => {
                // STWU $X, $Y, $Z - Store wyde unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            Opcode::STWUI => {
                // STWU $X, $Y, Z - Store wyde unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            Opcode::STT => {
                // STT $X, $Y, $Z - Store tetra (with overflow check). A trip
                // sets rY to the address and rZ to the merged octabyte after
                // the store.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i32::MIN as i64, i32::MAX as i64);
                self.write_tetra(addr, value as u32);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STTI => {
                // STT $X, $Y, Z - Store tetra immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i32::MIN as i64, i32::MAX as i64);
                self.write_tetra(addr, value as u32);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STTU => {
                // STTU $X, $Y, $Z - Store tetra unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            Opcode::STTUI => {
                // STTU $X, $Y, Z - Store tetra unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            Opcode::STO => {
                // STO $X, $Y, $Z - Store octa
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STOI => {
                // STO $X, $Y, Z - Store octa immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STOU => {
                // STOU $X, $Y, $Z - Store octa unsigned (same as STO)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STOUI => {
                // STOU $X, $Y, Z - Store octa unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STSF => {
                // STSF $X, $Y, $Z - Narrow $X to f32 using rA mode and store at $Y+$Z.
                // No Y-operand override: STSF takes no rounding-mode field.
                // A store trip, so a trip sets rY to the address and rZ to
                // the merged octabyte after the store, per §1 rule 3.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = Self::u64_to_f64(self.get_register(x));
                let (bits, flags) = self.narrow_for_store(value);
                self.write_tetra(addr, bits);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STSFI => {
                // A store trip: rY takes the address, rZ the merged octabyte
                // after the store, per §1 rule 3.
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = Self::u64_to_f64(self.get_register(x));
                let (bits, flags) = self.narrow_for_store(value);
                self.write_tetra(addr, bits);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STHT => {
                // STHT $X, $Y, $Z - Store high tetra
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            Opcode::STHTI => {
                // STHTI $X, $Y, Z - Store high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            Opcode::STCO => {
                // STCO X, $Y, $Z - Store constant octabyte (X is immediate value)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            Opcode::STCOI => {
                // STCOI X, $Y, Z - Store constant octabyte immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            Opcode::STUNC => {
                // STUNC $X, $Y, $Z - Store uncached (treat as normal store)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STUNCI => {
                // STUNCI $X, $Y, Z - Store uncached immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::SYNCD => {
                // SYNCD X, $Y, $Z - Synchronize data (no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::SYNCDI => {
                // SYNCDI X, $Y, Z - Synchronize data immediate (no-op)
                self.advance_pc();
                true
            }
            Opcode::PREST => {
                // PREST X, $Y, $Z - Prestore (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::PRESTI => {
                // PRESTI X, $Y, Z - Prestore immediate (hint, no-op)
                self.advance_pc();
                true
            }
            Opcode::SYNCID => {
                // SYNCID X, $Y, $Z - Synchronize instruction data (no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::SYNCIDI => {
                // SYNCIDI X, $Y, Z - Synchronize instruction data immediate (no-op)
                self.advance_pc();
                true
            }
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
            // 0x22 and 0x23 are ADDU/ADDUI, already implemented above
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
            // CMP instructions - opcodes 0x30-0x33
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

            // Bitwise operations - opcodes 0xC0-0xCF, 0xD8-0xD9
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
            // Jump/Stack/System instructions - opcodes 0xF0-0xFF
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
