//! The floating-point family: FCMP through FINT.

use super::super::float::FpOp;
use super::super::{MMix, RA_I, RA_W, RA_X, RA_Z, SpecialReg};
use crate::mmixal::Opcode;

impl MMix {
    /// Dispatches the floating-point family; `dispatch` routes only these
    /// opcodes here.
    pub(super) fn dispatch_floating_point(
        &mut self,
        opcode: Opcode,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        match opcode {
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
                // SFLOT $X, Y, $Z - convert a signed integer straight to
                // short precision (in an f64 register), one rounding.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let v = z_val as i64;
                let (result, flags) = Self::int_to_f32_rounded(v < 0, v.unsigned_abs(), mode);
                self.set_register(x, Self::f64_to_u64(result));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SFLOTI => {
                // SFLOTI $X, Y, Z - Z is an unsigned byte, like every
                // immediate operand, so always exact.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTI", y),
                };
                // Y is the rounding-mode field and Z the literal operand,
                // neither a register.
                let y_val = y as u64;
                let z_val = z as u64;
                let (result, flags) = Self::int_to_f32_rounded(false, z_val, mode);
                self.set_register(x, Self::f64_to_u64(result));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SFLOTU => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTU", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let v = self.get_register(z);
                let (result, flags) = Self::int_to_f32_rounded(false, v, mode);
                self.set_register(x, Self::f64_to_u64(result));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, v)
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
                let (result, flags) = Self::int_to_f32_rounded(false, z_val, mode);
                self.set_register(x, Self::f64_to_u64(result));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
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
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, false, FpOp::Mul);
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
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, false, FpOp::Div);
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
            _ => unreachable!(
                "dispatch routes only floating-point opcodes to dispatch_floating_point"
            ),
        }
    }
}
