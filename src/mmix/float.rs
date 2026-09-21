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

    /// `NaN(1/2)`, MMIX's invalid-operation result: fraction exactly `1/2`,
    /// signed as given.
    #[inline]
    pub(super) fn nan_half(negative: bool) -> f64 {
        let bits = 0x7FF8_0000_0000_0000u64 | if negative { 1u64 << 63 } else { 0 };
        f64::from_bits(bits)
    }

    /// MMIX's "standard conventions" NaN pick for a binary operation: `$Z`
    /// (`b`) when `$Z` is a NaN, `$Y` (`a`) otherwise. Quieted. `None` when
    /// neither operand is a NaN.
    #[inline]
    pub(super) fn select_nan(a: f64, b: f64) -> Option<f64> {
        if b.is_nan() {
            Some(Self::quiet_nan(b))
        } else if a.is_nan() {
            Some(Self::quiet_nan(a))
        } else {
            None
        }
    }

    /// I for a NaN-operand result: raised when either operand signals.
    #[inline]
    pub(super) fn nan_i_flag(a: f64, b: f64) -> u64 {
        if Self::is_signaling_nan(a) || Self::is_signaling_nan(b) {
            RA_I
        } else {
            0
        }
    }

    /// `FADD`'s NaN result and its one invalid case, ∞ + (−∞): `NaN(1/2)`
    /// signed as `$Z`'s. `None` when host arithmetic may proceed.
    pub(super) fn fadd_special(a: f64, b: f64) -> Option<(f64, u64)> {
        if let Some(nan) = Self::select_nan(a, b) {
            return Some((nan, Self::nan_i_flag(a, b)));
        }
        if a.is_infinite() && b.is_infinite() && a.is_sign_negative() != b.is_sign_negative() {
            return Some((Self::nan_half(b.is_sign_negative()), RA_I));
        }
        None
    }

    /// `FMUL`'s NaN result and its invalid case, `0 × ∞`: `NaN(1/2)` signed
    /// by the operands' sign product.
    pub(super) fn fmul_special(a: f64, b: f64) -> Option<(f64, u64)> {
        if let Some(nan) = Self::select_nan(a, b) {
            return Some((nan, Self::nan_i_flag(a, b)));
        }
        if (a == 0.0 && b.is_infinite()) || (a.is_infinite() && b == 0.0) {
            let negative = a.is_sign_negative() != b.is_sign_negative();
            return Some((Self::nan_half(negative), RA_I));
        }
        None
    }

    /// `FDIV`'s NaN result and its invalid cases, `0/0` and `∞/∞`:
    /// `NaN(1/2)` signed by the operands' sign product.
    pub(super) fn fdiv_special(a: f64, b: f64) -> Option<(f64, u64)> {
        if let Some(nan) = Self::select_nan(a, b) {
            return Some((nan, Self::nan_i_flag(a, b)));
        }
        if (a == 0.0 && b == 0.0) || (a.is_infinite() && b.is_infinite()) {
            let negative = a.is_sign_negative() != b.is_sign_negative();
            return Some((Self::nan_half(negative), RA_I));
        }
        None
    }

    /// `FREM`'s NaN result and its invalid cases, an infinite `$Y` or a
    /// zero `$Z`: `NaN(1/2)` signed as `$Y`'s.
    pub(super) fn frem_special(a: f64, b: f64) -> Option<(f64, u64)> {
        if let Some(nan) = Self::select_nan(a, b) {
            return Some((nan, Self::nan_i_flag(a, b)));
        }
        if a.is_infinite() || b == 0.0 {
            return Some((Self::nan_half(a.is_sign_negative()), RA_I));
        }
        None
    }

    /// `FSQRT`'s NaN passthrough and its invalid case, a negative operand:
    /// `NaN(1/2)`, always negative.
    pub(super) fn fsqrt_special(a: f64) -> Option<(f64, u64)> {
        if a.is_nan() {
            let flags = if Self::is_signaling_nan(a) { RA_I } else { 0 };
            return Some((Self::quiet_nan(a), flags));
        }
        if a < 0.0 {
            return Some((Self::nan_half(true), RA_I));
        }
        None
    }

    /// `FINT`'s NaN passthrough: quieted, I only for a signaling operand.
    /// `FINT` has no invalid case of its own.
    pub(super) fn fint_special(a: f64) -> Option<(f64, u64)> {
        if a.is_nan() {
            let flags = if Self::is_signaling_nan(a) { RA_I } else { 0 };
            return Some((Self::quiet_nan(a), flags));
        }
        None
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

    /// Signed zero for an exact `FADD`/`FSUB` result under ROUND_DOWN:
    /// `-0`, except `(+0) + (+0) = +0`. Callers apply this only in
    /// ROUND_DOWN — every other mode already gets the right sign from the
    /// host's round-to-nearest zero, which agrees with MMIX's rule
    /// everywhere but ROUND_DOWN.
    #[inline]
    fn round_down_zero(a: f64, b: f64) -> f64 {
        let both_positive_zero =
            a == 0.0 && a.is_sign_positive() && b == 0.0 && b.is_sign_positive();
        if both_positive_zero { 0.0 } else { -0.0 }
    }

    /// Shared body of `FADD` and `FSUB`: `FSUB` calls this with `b` already
    /// negated (unless `b` is a NaN — MMIX negates `$Z` only when it is
    /// not), so this is always literally `$Y + b`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fadd_compute(
        &mut self,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
        y_val: u64,
        z_val: u64,
        a: f64,
        b: f64,
    ) -> bool {
        if let Some((result, flags)) = Self::fadd_special(a, b) {
            self.set_register(x, Self::f64_to_u64(result));
            return self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val);
        }
        let r_near = a + b;
        let (_, err) = Self::two_sum(a, b);
        // `two_sum` is exact for finite operands, so a zero sum with a
        // zero residual is exact cancellation, not an underflow.
        let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, err == 0.0, true);
        self.set_register(x, Self::f64_to_u64(r));
        self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
    }

    /// Finalize a binary FP op given the round-to-nearest-even result
    /// `r_near` and an exact residual `err` (`sign(err) == sign(true -
    /// r_near)`, `err == 0` means exact). Returns `(adjusted_result,
    /// flags)`: overflow clamping per rA mode, X / O / U detection.
    /// Callers filter NaN operands and invalid operations first — `a`,
    /// `b` and `r_near` are never NaN here.
    ///
    /// `true_result_is_zero` resolves a zero delivered result: U means the
    /// result was too small to represent, so a true result of exactly zero is
    /// never an underflow. Only the caller knows which its zero was.
    ///
    /// `additive` selects `FADD`/`FSUB`'s ROUND_DOWN zero-sign rule; `FMUL`
    /// and `FDIV` pass `false` and keep the ordinary sign-of-product zero.
    pub(super) fn finalize_fp_binop(
        &self,
        a: f64,
        b: f64,
        r_near: f64,
        err: f64,
        true_result_is_zero: bool,
        additive: bool,
    ) -> (f64, u64) {
        let mode = (self.get_special(SpecialReg::RA) >> RA_ROUND_SHIFT) & 0x3;
        let operands_finite = a.is_finite() && b.is_finite();
        let mut flags = 0u64;
        let result = if r_near.is_infinite() && operands_finite {
            flags |= RA_O | RA_X;
            Self::clamp_overflow_for_mode(r_near, mode)
        } else if r_near.is_finite() {
            // A non-finite operand yields an exact result — ±0 or ±inf —
            // and its residual is meaningless: `(-0.0).mul_add(inf, 1.0)` is
            // NaN. Neither X nor directed rounding applies there.
            let err = if operands_finite { err } else { 0.0 };
            if err != 0.0 {
                flags |= RA_X;
            }
            let rounded = Self::apply_directed_rounding(r_near, err, mode);
            if additive && rounded == 0.0 && mode & 0x3 == 3 {
                Self::round_down_zero(a, b)
            } else {
                rounded
            }
        } else {
            r_near
        };
        if operands_finite
            && a != 0.0
            && b != 0.0
            && (result.is_subnormal() || (result == 0.0 && !true_result_is_zero))
        {
            let trip_enabled = (self.get_special(SpecialReg::RA) >> 8) & RA_U != 0;
            if trip_enabled || err != 0.0 {
                flags |= RA_U;
            }
        }
        (result, flags)
    }

    /// Finalize a unary FP op (`FSQRT`). Same shape as `finalize_fp_binop`.
    /// The caller filters a NaN or negative operand first, so `a` and
    /// `r_near` are never NaN here.
    ///
    /// No U: the only caller is `FSQRT`, and the square root of a nonzero
    /// finite operand is neither zero nor subnormal — the root of the smallest
    /// subnormal is about `2^-537`.
    pub(super) fn finalize_fp_unop(&self, a: f64, r_near: f64, err: f64, mode: u64) -> (f64, u64) {
        let mut flags = 0u64;
        let result = if r_near.is_infinite() && a.is_finite() {
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
    /// `Opcode::TRIP`'s halt-with-diagnostic precedent, and exits 1 like
    /// every other halt but the `Halt` trap.
    pub(super) fn illegal_round_mode(&mut self, mnemonic: &str, y: u8) -> bool {
        self.host.diagnostic(&format!(
            "{mnemonic}: illegal Y={y} at PC={:#018x} (Y must be 0-4)",
            self.pc
        ));
        self.exit_code = 1;
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

    /// `FIX`'s signed-range check: `true` when the rounded integer falls
    /// below `-2^63` or above `2^63 - 1`. Compares against `2^63` itself
    /// (exact in `f64`) rather than `i64::MAX as f64`, which rounds up to
    /// `2^63` and would silently exempt exactly `2^63` from the overflow
    /// it belongs to.
    #[inline]
    pub(super) fn out_of_signed_i64_range(rounded: f64) -> bool {
        const LIMIT: f64 = 9_223_372_036_854_775_808.0; // 2^63
        rounded < -LIMIT || rounded >= LIMIT
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
    ///
    /// `value` is never NaN: `SFLOT`'s family converts from an integer, and
    /// `STSF` quiets and truncates a NaN's bits itself, bypassing this.
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
        if value.is_finite() && narrowed.is_infinite() {
            flags |= RA_O;
        }
        let inexact = value.is_finite() && result != value;
        if value != 0.0 && (narrowed == 0.0 || narrowed.is_subnormal()) {
            let trip_enabled = (self.get_special(SpecialReg::RA) >> 8) & RA_U != 0;
            if trip_enabled || inexact {
                flags |= RA_U;
            }
        }
        if inexact {
            flags |= RA_X;
        }
        (result, flags)
    }

    /// `STSF`/`STSFI`'s narrowing: a NaN quiets and truncates its own bits
    /// (I on a signaling operand, per MMIX's short-float rule), and every
    /// other value narrows through `f64_to_f32_rounded` under rA's mode.
    pub(super) fn narrow_for_store(&self, value: f64) -> (u32, u64) {
        if value.is_nan() {
            let flags = if Self::is_signaling_nan(value) {
                RA_I
            } else {
                0
            };
            return (Self::narrow_short_float_nan_or_inf(value.to_bits()), flags);
        }
        let mode = (self.get_special(SpecialReg::RA) >> RA_ROUND_SHIFT) & 0x3;
        let (narrowed, flags) = self.f64_to_f32_rounded(value, mode);
        ((narrowed as f32).to_bits(), flags)
    }

    /// Widen a short float's bits to `f64`, exactly: any finite or
    /// infinite value converts losslessly, and a NaN's payload survives
    /// bit for bit — including a signaling NaN's signaling bit — which
    /// Rust's `as` cast does not promise.
    #[inline]
    pub(super) fn widen_short_float(bits: u32) -> u64 {
        let sign = (bits as u64 & 0x8000_0000) << 32;
        let exp = (bits >> 23) & 0xFF;
        let frac = (bits & 0x007F_FFFF) as u64;
        if exp == 0xFF {
            sign | (0x7FFu64 << 52) | (frac << 29)
        } else if exp == 0 {
            if frac == 0 {
                sign
            } else {
                // Subnormal short float: f64's much wider exponent range
                // holds it as a normal double, so normalize the fraction.
                let p = 31 - (frac as u32).leading_zeros();
                let mantissa = frac & !(1u64 << p);
                let new_frac = mantissa << (52 - p);
                let new_exp = (p as i64 - 149 + 1023) as u64;
                sign | (new_exp << 52) | new_frac
            }
        } else {
            let new_exp = exp as u64 + 896; // rebias: (exp - 127) + 1023
            sign | (new_exp << 52) | (frac << 29)
        }
    }

    /// Narrow an `f64`'s bits to a short float, for a NaN or infinity
    /// only: truncates the fraction to its top 23 bits and, for a
    /// signaling NaN, forces the quiet bit — MMIX's `STSF` rule — rather
    /// than trusting Rust's `as f32` cast with the payload.
    #[inline]
    pub(super) fn narrow_short_float_nan_or_inf(bits: u64) -> u32 {
        let sign = ((bits >> 32) & 0x8000_0000) as u32;
        let frac52 = bits & 0x000F_FFFF_FFFF_FFFF;
        let mut frac23 = (frac52 >> 29) as u32;
        if frac52 != 0 && frac52 & (1u64 << 51) == 0 {
            frac23 |= 1 << 22; // quiet a signaling NaN
        }
        sign | (0xFFu32 << 23) | frac23
    }

    /// A finite `f64`'s exact value as `(negative, mantissa, exponent)`:
    /// magnitude is `mantissa × 2^exponent`, mantissa fits 53 bits, and
    /// `mantissa == 0` is a signed zero.
    #[inline]
    fn dyadic(v: f64) -> (bool, u64, i32) {
        let bits = v.to_bits();
        let negative = bits >> 63 == 1;
        let biased_exp = ((bits >> 52) & 0x7FF) as i32;
        let frac = bits & 0x000F_FFFF_FFFF_FFFF;
        if biased_exp == 0 {
            (negative, frac, -1074)
        } else {
            (negative, frac | (1u64 << 52), biased_exp - 1075)
        }
    }

    /// Left-shift `m` by `shift`, saturating to `u128::MAX` if a set bit
    /// would be pushed past bit 127. `dyadic_sub_sign`'s comparison only
    /// needs to know that side dominates, never by how much.
    #[inline]
    fn shl_saturating(m: u128, shift: u32) -> u128 {
        if m == 0 {
            0
        } else if shift >= 128 || m.leading_zeros() < shift {
            u128::MAX
        } else {
            m << shift
        }
    }

    /// Sign of `p − q` for two dyadic magnitudes, `(negative, mantissa,
    /// exponent)` each, compared exactly by aligning to a common exponent
    /// and comparing integers. `0` when equal; never rounds.
    fn dyadic_sub_sign(neg_p: bool, mp: u128, ep: i32, neg_q: bool, mq: u128, eq: i32) -> i32 {
        if mp == 0 && mq == 0 {
            return 0;
        }
        if mq == 0 {
            return if neg_p { -1 } else { 1 };
        }
        if mp == 0 {
            return if neg_q { 1 } else { -1 };
        }
        let common = ep.min(eq);
        let big_p = Self::shl_saturating(mp, (ep - common) as u32);
        let big_q = Self::shl_saturating(mq, (eq - common) as u32);
        let sign_p = if neg_p { -1 } else { 1 };
        let sign_q = if neg_q { -1 } else { 1 };
        if sign_p == sign_q {
            sign_p
                * match big_p.cmp(&big_q) {
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                }
        } else {
            sign_p
        }
    }

    /// Sign of the exact `a×b − r_near` as `-1.0`, `0.0` or `1.0` — callers
    /// use only its sign and zero-ness. `r_near` must already be `a*b`
    /// correctly rounded. A *nonzero* FMA result is trustworthy: correct
    /// rounding to nearest cannot cross zero, so its sign matches the true
    /// residual's. Only a *zero* FMA result is ambiguous — it may be
    /// exact, or it may be the FMA's own rounding underflowing a
    /// genuinely nonzero residual and losing its sign along with its
    /// magnitude — so that case alone falls back to an exact integer
    /// comparison of the operands' bits.
    pub(super) fn fmul_error_sign(a: f64, b: f64, r_near: f64) -> f64 {
        let fma = a.mul_add(b, -r_near);
        if fma != 0.0 {
            return fma;
        }
        let (neg_a, ma, ea) = Self::dyadic(a);
        let (neg_b, mb, eb) = Self::dyadic(b);
        let (neg_r, mr, er) = Self::dyadic(r_near);
        let sign = Self::dyadic_sub_sign(
            neg_a != neg_b,
            ma as u128 * mb as u128,
            ea + eb,
            neg_r,
            mr as u128,
            er,
        );
        sign as f64
    }

    /// Sign of the exact `a/b − r_near`, same shape as `fmul_error_sign`.
    /// Its exact fallback reduces to comparing `a` against the
    /// exactly-representable `r_near × b`, avoiding ever computing the
    /// (generally irrational) exact quotient: `sign(a/b − r) = sign(a −
    /// r·b) × sign(b)`.
    pub(super) fn fdiv_error_sign(a: f64, b: f64, r_near: f64) -> f64 {
        let residual = (-r_near).mul_add(b, a);
        let fma = if b.is_sign_negative() {
            -residual
        } else {
            residual
        };
        if fma != 0.0 {
            return fma;
        }
        let (neg_a, ma, ea) = Self::dyadic(a);
        let (neg_r, mr, er) = Self::dyadic(r_near);
        let (neg_b, mb, eb) = Self::dyadic(b);
        let sign = Self::dyadic_sub_sign(
            neg_a,
            ma as u128,
            ea,
            neg_r != neg_b,
            mr as u128 * mb as u128,
            er + eb,
        );
        if b.is_sign_negative() {
            -(sign as f64)
        } else {
            sign as f64
        }
    }

    /// Sign of the exact `sqrt(a) − r_near`, same shape as
    /// `fmul_error_sign`. Its exact fallback reduces to comparing `a`
    /// against the exactly-representable `r_near²`, avoiding the
    /// (generally irrational) exact root: `sign(sqrt(a) − r) = sign(a −
    /// r²)` for `a, r ≥ 0`.
    pub(super) fn fsqrt_error_sign(a: f64, r_near: f64) -> f64 {
        let fma = (-r_near).mul_add(r_near, a);
        if fma != 0.0 {
            return fma;
        }
        let (_, ma, ea) = Self::dyadic(a);
        let (_, mr, er) = Self::dyadic(r_near);
        let sign = Self::dyadic_sub_sign(
            false,
            ma as u128,
            ea,
            false,
            mr as u128 * mr as u128,
            er + er,
        );
        sign as f64
    }
}
