//! IEEE 754 float<->fix conversions, rounding modes, and the shared flag/epsilon helpers behind the FP opcodes.

use super::{MMix, RA_I, RA_O, RA_ROUND_SHIFT, RA_U, RA_X, SpecialReg};

impl MMix {
    /// Convert u64 to f64 (reinterpret bits)
    #[inline]
    pub(super) fn u64_to_f64(value: u64) -> f64 {
        f64::from_bits(value)
    }

    /// Convert f64 to u64 (reinterpret bits)
    #[inline]
    pub(super) fn f64_to_u64(value: f64) -> u64 {
        value.to_bits()
    }

    /// Floating point comparison: `$X <- [f($Y) > f($Z)] - [f($Y) < f($Z)]`, so
    /// -1, 0 or +1 and nothing else. Both predicates are false on an unordered
    /// pair, which therefore answers 0 — `FCMP` cannot distinguish unordered
    /// from equal, and `FUN` is what does.
    #[inline]
    pub(super) fn fcmp(y: f64, z: f64) -> u64 {
        if y < z {
            (-1i64) as u64
        } else if y > z {
            1
        } else {
            0
        }
    }

    /// `u ∈ Nε(v)`: `|u − v| ≤ 2^(e−1022)·ε`, where `e` is `v`'s biased
    /// exponent field for a normal `v` and is taken as `1` for a
    /// subnormal `v` (radius `2^−1021·ε`). `Nε(0) = {0}`. `Nε(+∞)` is
    /// `{+∞}` when `ε < 1`, everything except `−∞` when `1 ≤ ε < 2`, and
    /// everything when `ε ≥ 2`; `Nε(−∞)` mirrors it. Example: `v = 1.0`
    /// has `e = 1023`, so with `ε = 0.5` the radius is `1.0` and `u =
    /// 1.9 ∈ Nε(1.0)`. Callers exclude NaN operands and a NaN or
    /// negative `ε` beforehand (the shared exception condition,
    /// `epsilon_exceptional`) — `v` and `ε` are never NaN here.
    #[inline]
    pub(super) fn in_epsilon_neighborhood(u: f64, v: f64, epsilon: f64) -> bool {
        if v.is_infinite() {
            if epsilon < 1.0 {
                u == v
            } else if epsilon < 2.0 {
                u != -v
            } else {
                true
            }
        } else if v == 0.0 {
            u == 0.0
        } else {
            let e = (v.to_bits() >> 52) & 0x7FF;
            let scale = if e == 0 { -1021 } else { e as i32 - 1022 };
            // `2f64.powi(scale)` overflows to `inf` at the top binade
            // (scale == 1024), turning a finite radius into "everything" or,
            // with epsilon == 0.0, into NaN. Scale the difference down by
            // `2^-scale` instead of scaling epsilon up by `2^scale` — but a
            // single `2f64.powi(-scale)` has its own top-binade failure:
            // at `-scale == -1024`, `f64::powi`'s squaring chain overflows
            // an intermediate power to `inf`, and the final reciprocal turns
            // that `inf` into `0.0` — a total loss, not a rounding error.
            // Splitting `-scale` into two halves keeps each `powi` call's
            // result within the normal exponent range, so multiplying
            // `diff` by each half in turn never hits that collapse.
            let half = -scale / 2;
            let rest = -scale - half;
            (u - v).abs() * 2f64.powi(half) * 2f64.powi(rest) <= epsilon
        }
    }

    /// The shared exceptional condition for `FCMPE`/`FEQLE`/`FUNE`: either
    /// compared value is NaN, or `rE` is NaN or negative. `-0.0 < 0.0` is
    /// `false`, so `rE = -0.0` is not negative.
    #[inline]
    pub(super) fn epsilon_exceptional(y_val: f64, z_val: f64, epsilon: f64) -> bool {
        y_val.is_nan() || z_val.is_nan() || epsilon.is_nan() || epsilon < 0.0
    }

    /// True iff `x` is a signaling NaN per IEEE 754 binary64 (exponent all 1s,
    /// mantissa nonzero, high mantissa bit clear).
    #[inline]
    pub(super) fn is_signaling_nan(x: f64) -> bool {
        let bits = x.to_bits();
        let exp = (bits >> 52) & 0x7FF;
        let mant = bits & 0x000F_FFFF_FFFF_FFFF;
        exp == 0x7FF && mant != 0 && (mant & (1u64 << 51)) == 0
    }

    /// Force any NaN to its quiet form by setting the high mantissa bit. Leaves
    /// non-NaN values unchanged. Used to suppress sNaN propagation per IEEE 754.
    #[inline]
    pub(super) fn quiet_nan(x: f64) -> f64 {
        if x.is_nan() {
            f64::from_bits(x.to_bits() | (1u64 << 51))
        } else {
            x
        }
    }

    /// Compute rA event flags for a binary IEEE 754 operation. The X flag is
    /// reported by callers that supply an exact residual (`finalize_fp_binop`).
    /// No U: the only caller is `FREM`, and the IEEE remainder is exact by
    /// definition, so it can neither round nor underflow.
    #[inline]
    pub(super) fn fp_arith_flags(a: f64, b: f64, result: f64) -> u64 {
        let mut flags = 0u64;
        if Self::is_signaling_nan(a) || Self::is_signaling_nan(b) {
            flags |= RA_I;
        }
        if !a.is_nan() && !b.is_nan() && result.is_nan() {
            flags |= RA_I;
        } else if a.is_finite() && b.is_finite() && result.is_infinite() {
            flags |= RA_O;
        }
        flags
    }

    /// Veltkamp-Knuth 2Sum: returns `(s, err)` such that `s + err == a + b`
    /// exactly, where `s = a + b` rounded to nearest-even. Valid for all
    /// finite operands (no overflow case here — caller checks `s.is_finite()`).
    #[inline]
    pub(super) fn two_sum(a: f64, b: f64) -> (f64, f64) {
        let s = a + b;
        let bb = s - a;
        let err = (a - (s - bb)) + (b - bb);
        (s, err)
    }

    /// Apply MMIX rA rounding mode to `r` (the round-to-nearest-even result of
    /// some op) given an exact residual `err` whose sign equals
    /// `sign(true_result - r)`. `err == 0` means the result is exact.
    #[inline]
    fn apply_directed_rounding(r: f64, err: f64, mode: u64) -> f64 {
        if err == 0.0 || !r.is_finite() {
            return r;
        }
        match mode & 0x3 {
            0 => r,
            1 => {
                if r > 0.0 && err < 0.0 {
                    r.next_down()
                } else if r < 0.0 && err > 0.0 {
                    r.next_up()
                } else {
                    r
                }
            }
            2 => {
                if err > 0.0 {
                    r.next_up()
                } else {
                    r
                }
            }
            3 => {
                if err < 0.0 {
                    r.next_down()
                } else {
                    r
                }
            }
            _ => r,
        }
    }

    /// On overflow (finite operands → infinite hardware result), directed
    /// rounding modes ROUND_OFF and the "wrong-sign" infinity for ±∞ modes
    /// must clamp to ±MAX instead of leaving ±Inf.
    #[inline]
    fn clamp_overflow_for_mode(r: f64, mode: u64) -> f64 {
        if !r.is_infinite() {
            return r;
        }
        match (r.is_sign_positive(), mode & 0x3) {
            (true, 1) | (true, 3) => f64::MAX,
            (false, 1) | (false, 2) => -f64::MAX,
            _ => r,
        }
    }

    /// Finalize a binary FP op given the round-to-nearest-even result `r_near`
    /// and an exact residual `err` (`sign(err) == sign(true - r_near)`, `err==0`
    /// means exact). Returns `(adjusted_result, flags)`. Centralizes sNaN
    /// quieting, overflow clamping per rA mode, X / I / O / U detection.
    ///
    /// `true_result_is_zero` resolves a zero delivered result: U means the
    /// result was too small to represent, so a true result of exactly zero is
    /// never an underflow. Only the caller knows which its zero was.
    pub(super) fn finalize_fp_binop(
        &self,
        a: f64,
        b: f64,
        r_near: f64,
        err: f64,
        true_result_is_zero: bool,
    ) -> (f64, u64) {
        let mode = (self.get_special(SpecialReg::RA) >> RA_ROUND_SHIFT) & 0x3;
        let operands_finite = a.is_finite() && b.is_finite();
        let mut flags = 0u64;
        if Self::is_signaling_nan(a) || Self::is_signaling_nan(b) {
            flags |= RA_I;
        }
        let result = if r_near.is_nan() {
            if !a.is_nan() && !b.is_nan() {
                flags |= RA_I;
            }
            Self::quiet_nan(r_near)
        } else if r_near.is_infinite() && operands_finite {
            flags |= RA_O | RA_X;
            Self::clamp_overflow_for_mode(r_near, mode)
        } else if r_near.is_finite() {
            // A non-finite operand yields an exact result — ±0, ±inf or NaN —
            // and its residual is meaningless: `(-0.0).mul_add(inf, 1.0)` is
            // NaN. Neither X nor directed rounding applies there.
            let err = if operands_finite { err } else { 0.0 };
            if err != 0.0 {
                flags |= RA_X;
            }
            Self::apply_directed_rounding(r_near, err, mode)
        } else {
            r_near
        };
        if operands_finite
            && a != 0.0
            && b != 0.0
            && (result.is_subnormal() || (result == 0.0 && !true_result_is_zero))
        {
            flags |= RA_U;
        }
        (result, flags)
    }

    /// Finalize a unary FP op (FSQRT). Same shape as `finalize_fp_binop`.
    ///
    /// No U: the only caller is `FSQRT`, and the square root of a nonzero
    /// finite operand is neither zero nor subnormal — the root of the smallest
    /// subnormal is about `2^-537`.
    pub(super) fn finalize_fp_unop(&self, a: f64, r_near: f64, err: f64, mode: u64) -> (f64, u64) {
        let mut flags = 0u64;
        if Self::is_signaling_nan(a) {
            flags |= RA_I;
        }
        let result = if r_near.is_nan() {
            if !a.is_nan() {
                flags |= RA_I;
            }
            Self::quiet_nan(r_near)
        } else if r_near.is_infinite() && a.is_finite() {
            flags |= RA_O | RA_X;
            Self::clamp_overflow_for_mode(r_near, mode)
        } else if r_near.is_finite() {
            if err != 0.0 {
                flags |= RA_X;
            }
            Self::apply_directed_rounding(r_near, err, mode)
        } else {
            r_near
        };
        (result, flags)
    }

    /// IEEE 754 floating-point remainder: `r = a − round-half-to-even(a/b) · b`.
    /// Rust's `%` operator is truncated remainder; this is the rounded remainder
    /// required by the MMIX FREM spec.
    ///
    /// A zero remainder takes the sign of the dividend, per IEEE 754. The
    /// subtraction cannot produce it: `x - x` is `+0.0` under every rounding
    /// mode but ROUND_DOWN.
    #[inline]
    pub(super) fn ieee_remainder(a: f64, b: f64) -> f64 {
        if a.is_nan() || b.is_nan() || a.is_infinite() || b == 0.0 {
            return f64::NAN;
        }
        if b.is_infinite() {
            return a;
        }
        let n = (a / b).round_ties_even();
        let r = a - n * b;
        if r == 0.0 { 0.0f64.copysign(a) } else { r }
    }

    /// Resolve the `Y` rounding-mode override that `FIX`, `FIXU`, `FSQRT`,
    /// `FINT`, and the `FLOT`/`SFLOT` families carry.
    /// `Y == 0` defers to rA's own persistent mode (`RA_ROUND_SHIFT`); `Y`
    /// in `1..=4` forces a mode via `Y & 3` — this maps `Y=4` (`ROUND_NEAR`)
    /// onto rA's mode `0`, since the two numberings are not related by a
    /// simple offset (`MMIX.md`'s rounding-mode table). `Y > 4` is the
    /// illegal-instruction condition; `Err` and the caller halts.
    #[inline]
    pub(super) fn resolved_round_mode(&self, y: u8) -> Result<u64, ()> {
        match y {
            0 => Ok((self.get_special(SpecialReg::RA) >> RA_ROUND_SHIFT) & 0x3),
            1..=4 => Ok((y & 3) as u64),
            _ => Err(()),
        }
    }

    /// `Y > 4` on an instruction that takes a rounding-mode override is an
    /// illegal-instruction interrupt this VM has no vector for. Mirrors
    /// `Opcode::TRIP`'s halt-with-diagnostic precedent.
    pub(super) fn illegal_round_mode(&mut self, mnemonic: &str, y: u8) -> bool {
        self.host.diagnostic(&format!(
            "{mnemonic}: illegal Y={y} at PC={:#018x} (Y must be 0-4)",
            self.pc
        ));
        false
    }

    /// MMIX rounding mode (rA bits 17-16): 0=NEAR (default), 1=OFF (trunc), 2=UP
    /// (ceil toward +∞), 3=DOWN (floor toward −∞). Applies to FINT and to the
    /// f64→f32 conversion in SFLOT/STSF.
    #[inline]
    pub(super) fn round_with_mode(value: f64, mode: u64) -> f64 {
        match mode & 0x3 {
            0 => value.round_ties_even(),
            1 => value.trunc(),
            2 => value.ceil(),
            3 => value.floor(),
            _ => unreachable!(),
        }
    }

    /// Reduce an integral finite `f64` to the low 64 bits of its exact value,
    /// as `FIXU` requires: `u($X) <- int(f($Z)) mod 2^64`. A magnitude whose
    /// ulp reaches `2^64` therefore yields zero.
    ///
    /// Read out of the bit pattern because neither Rust cast will do it: `as
    /// u64` saturates at the range ends instead of wrapping, and the wrapping
    /// `to_int_unchecked` is `unsafe`.
    #[inline]
    pub(super) fn wrap_to_u64(value: f64) -> u64 {
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7FF) as i32 - 1023;
        // A zero or subnormal encoding has no integral part; the caller has
        // already rounded, so any |value| < 1 is ±0.
        if exponent < 0 {
            return 0;
        }
        let significand = (bits & 0x000F_FFFF_FFFF_FFFF) | (1u64 << 52);
        let magnitude = if exponent < 52 {
            significand >> (52 - exponent)
        } else if exponent < 116 {
            // Bits above 2^64 fall off the left, which is the reduction itself.
            significand << (exponent - 52)
        } else {
            0
        };
        if value.is_sign_negative() {
            magnitude.wrapping_neg()
        } else {
            magnitude
        }
    }

    /// Exact residual of rounding `magnitude` to `f64`, that is
    /// `magnitude - (magnitude as f64)` — an integer below `2^11` in magnitude,
    /// since at most eleven bits fall off a 64-bit value.
    ///
    /// A round trip through the float cannot recover it: `u64::MAX as f64` is
    /// `2^64`, whose cast back saturates to `u64::MAX` and so reports a
    /// conversion that lost eleven bits as exact.
    #[inline]
    fn u64_to_f64_residual(magnitude: u64) -> i64 {
        let significant = 64 - magnitude.leading_zeros();
        if significant <= 53 {
            return 0;
        }
        let dropped = significant - 53;
        let low = magnitude & ((1u64 << dropped) - 1);
        let half = 1u64 << (dropped - 1);
        // Round-to-nearest-even, matching Rust's integer-to-float cast.
        let up = low > half || (low == half && (magnitude >> dropped) & 1 == 1);
        if up {
            low as i64 - (1i64 << dropped)
        } else {
            low as i64
        }
    }

    /// X for the integer-to-`f64` step of the `SFLOT` family, whose narrowing to
    /// `f32` reports its own flags through `f64_to_f32_rounded`. That first step
    /// can lose bits above `2^53` and said nothing about it.
    #[inline]
    pub(super) fn int_to_f64_inexact(magnitude: u64) -> u64 {
        if Self::u64_to_f64_residual(magnitude) != 0 {
            RA_X
        } else {
            0
        }
    }

    /// Convert an exact integer to `f64` under the given rounding mode,
    /// reporting X when the conversion loses bits. The value is given as
    /// sign and magnitude so the residual stays in integer arithmetic.
    #[inline]
    pub(super) fn int_to_f64_rounded(
        &self,
        negative: bool,
        magnitude: u64,
        mode: u64,
    ) -> (f64, u64) {
        let residual = Self::u64_to_f64_residual(magnitude) as f64;
        let (r_near, err) = if negative {
            (-(magnitude as f64), -residual)
        } else {
            (magnitude as f64, residual)
        };
        let flags = if err != 0.0 { RA_X } else { 0 };
        (Self::apply_directed_rounding(r_near, err, mode), flags)
    }

    /// Convert f64 → f32 under the given rounding mode, reporting flags.
    /// Returns `(narrowed_as_f64, flags)`. `STSF`/`STSFI` always pass rA's
    /// own mode; `SFLOT`'s family passes its resolved `Y` override.
    #[inline]
    pub(super) fn f64_to_f32_rounded(&self, value: f64, mode: u64) -> (f64, u64) {
        let near = value as f32; // hardware default: round-to-nearest-even
        let narrowed = if !value.is_finite() || (near as f64) == value {
            near
        } else {
            let near_high = (near as f64) > value;
            match mode {
                0 => near,
                1 => {
                    // ROUND_OFF: toward zero
                    if (near as f64).abs() > value.abs() {
                        if near > 0.0 {
                            near.next_down()
                        } else {
                            near.next_up()
                        }
                    } else {
                        near
                    }
                }
                2 => {
                    // ROUND_UP: toward +∞
                    if near_high { near } else { near.next_up() }
                }
                3 => {
                    // ROUND_DOWN: toward -∞
                    if near_high { near.next_down() } else { near }
                }
                _ => near,
            }
        };
        let result = narrowed as f64;
        let mut flags = 0u64;
        if !value.is_nan() && narrowed.is_nan() {
            flags |= RA_I;
        }
        if value.is_finite() && narrowed.is_infinite() {
            flags |= RA_O;
        }
        if !value.is_nan() && value != 0.0 && (narrowed == 0.0 || narrowed.is_subnormal()) {
            flags |= RA_U;
        }
        if !value.is_nan() && !value.is_infinite() && result != value {
            flags |= RA_X;
        }
        (result, flags)
    }
}
