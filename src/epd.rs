// lumindd — Extended Precision Double (EPD) for large floating-point values
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Extended Precision Double representation.
//!
//! An `EpDouble` stores a value as `mantissa * 2^exponent`, where the mantissa
//! is a normal `f64` and the exponent is an `i32`. This avoids overflow when
//! counting minterms for BDDs with many variables (plain `f64` overflows
//! around 2^1024).

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// Extended precision double: value = mantissa * 2^exponent.
///
/// After normalization the mantissa is kept in the range [0.5, 1.0)
/// (or is exactly zero).
#[derive(Clone, Copy, Debug)]
pub struct EpDouble {
    /// The significand (fraction part).
    pub mantissa: f64,
    /// The binary exponent.
    pub exponent: i32,
}

impl EpDouble {
    /// Create an `EpDouble` from a plain `f64` value.
    pub fn new(value: f64) -> Self {
        if value == 0.0 {
            return Self::zero();
        }
        let mut ep = EpDouble {
            mantissa: value,
            exponent: 0,
        };
        ep.normalize();
        ep
    }

    /// Create an `EpDouble` from explicit mantissa and exponent.
    pub fn from_parts(mantissa: f64, exponent: i32) -> Self {
        let mut ep = EpDouble { mantissa, exponent };
        if mantissa != 0.0 {
            ep.normalize();
        }
        ep
    }

    /// The value zero.
    pub fn zero() -> Self {
        EpDouble {
            mantissa: 0.0,
            exponent: 0,
        }
    }

    /// The value one.
    pub fn one() -> Self {
        EpDouble {
            mantissa: 0.5,
            exponent: 1,
        }
    }

    /// Represent 2^n exactly.
    pub fn two_power(n: i32) -> Self {
        EpDouble {
            mantissa: 0.5,
            exponent: n + 1,
        }
    }

    /// Normalize so that the mantissa is in [0.5, 1.0) (or zero).
    ///
    /// Uses `f64::frexp`-style decomposition via `log2` + bit manipulation.
    pub fn normalize(&mut self) {
        if self.mantissa == 0.0 {
            self.exponent = 0;
            return;
        }

        // Handle negative mantissa: work with absolute value, restore sign.
        let negative = self.mantissa < 0.0;
        let mut m = if negative {
            -self.mantissa
        } else {
            self.mantissa
        };

        // frexp: decompose m = frac * 2^exp where frac in [0.5, 1.0)
        // We extract the exponent from the IEEE 754 representation.
        let bits = m.to_bits();
        let biased_exp = ((bits >> 52) & 0x7FF) as i32;

        if biased_exp == 0 {
            // Subnormal — scale up, decompose, then adjust.
            m *= f64::from_bits(0x4350_0000_0000_0000); // 2^54
            let bits2 = m.to_bits();
            let biased2 = ((bits2 >> 52) & 0x7FF) as i32;
            let exp = biased2 - 1023 + 1 - 54;
            let frac_bits = (bits2 & 0x000F_FFFF_FFFF_FFFF) | 0x3FE0_0000_0000_0000;
            m = f64::from_bits(frac_bits);
            self.exponent = self.exponent.saturating_add(exp);
        } else if biased_exp == 0x7FF {
            // Inf or NaN — leave as-is.
            return;
        } else {
            // IEEE 754: value = 1.frac * 2^(biased_exp - 1023)
            // We want: mantissa in [0.5, 1.0), so mantissa = 0.5 * (1.frac)
            // which means: value = mantissa * 2^(biased_exp - 1023 + 1)
            let exp = biased_exp - 1023 + 1;
            // Set exponent field to 0x3FE (biased -1) so value is in [0.5, 1.0)
            let frac_bits = (bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FE0_0000_0000_0000;
            m = f64::from_bits(frac_bits);
            self.exponent = self.exponent.saturating_add(exp);
        }

        self.mantissa = if negative { -m } else { m };
    }

    /// Try to convert back to a plain `f64`. Returns `None` on overflow.
    pub fn to_f64(self) -> Option<f64> {
        if self.mantissa == 0.0 {
            return Some(0.0);
        }
        // f64 can represent exponents roughly in -1074 .. 1023.
        // mantissa is in [0.5, 1.0), so the effective exponent is self.exponent - 1.
        // ldexp(mantissa, exponent) = mantissa * 2^exponent
        if self.exponent > 1024 {
            return None; // would overflow
        }
        if self.exponent < -1074 {
            return Some(0.0); // underflow to zero
        }
        // Use multiplication by power-of-two to perform ldexp.
        // Split into two steps to avoid intermediate overflow.
        let half = self.exponent / 2;
        let other = self.exponent - half;
        let result = self.mantissa * exp2_i32(half) * exp2_i32(other);
        if result.is_infinite() {
            None
        } else {
            Some(result)
        }
    }

    /// Returns true if the value is zero.
    pub fn is_zero(self) -> bool {
        self.mantissa == 0.0
    }

    /// Negate the value.
    pub fn negate(self) -> Self {
        EpDouble {
            mantissa: -self.mantissa,
            exponent: self.exponent,
        }
    }
}

/// Compute 2^n as f64, clamped to avoid overflow/underflow of the intermediate.
fn exp2_i32(n: i32) -> f64 {
    if n > 1023 {
        f64::from_bits(0x7FE0_0000_0000_0000) // largest finite power of 2
    } else if n < -1074 {
        0.0
    } else {
        // For n in [-1022, 1023] use direct IEEE construction.
        if n >= -1022 {
            f64::from_bits(((n + 1023) as u64) << 52)
        } else {
            // Subnormal territory
            2.0f64.powi(n)
        }
    }
}

/// Align two EpDoubles to the same exponent (the larger one).
/// Returns (m1_adjusted, m2_adjusted, common_exponent).
fn align(a: EpDouble, b: EpDouble) -> (f64, f64, i32) {
    if a.mantissa == 0.0 {
        return (0.0, b.mantissa, b.exponent);
    }
    if b.mantissa == 0.0 {
        return (a.mantissa, 0.0, a.exponent);
    }
    let diff = a.exponent as i64 - b.exponent as i64;
    if diff > 0 {
        // a has the larger exponent; shift b's mantissa down.
        if diff > 1074 {
            (a.mantissa, 0.0, a.exponent)
        } else {
            (a.mantissa, b.mantissa * exp2_i32(-diff as i32), a.exponent)
        }
    } else if diff < 0 {
        let neg = -diff;
        if neg > 1074 {
            (0.0, b.mantissa, b.exponent)
        } else {
            (a.mantissa * exp2_i32(diff as i32), b.mantissa, b.exponent)
        }
    } else {
        (a.mantissa, b.mantissa, a.exponent)
    }
}

impl Add for EpDouble {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let (m1, m2, exp) = align(self, rhs);
        EpDouble::from_parts(m1 + m2, exp)
    }
}

impl Sub for EpDouble {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let (m1, m2, exp) = align(self, rhs);
        EpDouble::from_parts(m1 - m2, exp)
    }
}

impl Mul for EpDouble {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        EpDouble::from_parts(
            self.mantissa * rhs.mantissa,
            self.exponent.saturating_add(rhs.exponent),
        )
    }
}

impl Div for EpDouble {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        assert!(rhs.mantissa != 0.0, "EpDouble division by zero");
        EpDouble::from_parts(
            self.mantissa / rhs.mantissa,
            self.exponent.saturating_sub(rhs.exponent),
        )
    }
}

impl PartialEq for EpDouble {
    fn eq(&self, other: &Self) -> bool {
        if self.mantissa == 0.0 && other.mantissa == 0.0 {
            return true;
        }
        self.mantissa == other.mantissa && self.exponent == other.exponent
    }
}

impl PartialOrd for EpDouble {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Handle signs.
        let a_neg = self.mantissa < 0.0;
        let b_neg = other.mantissa < 0.0;
        if self.mantissa == 0.0 && other.mantissa == 0.0 {
            return Some(Ordering::Equal);
        }
        if a_neg && !b_neg {
            return Some(Ordering::Less);
        }
        if !a_neg && b_neg {
            return Some(Ordering::Greater);
        }
        if self.mantissa == 0.0 {
            return if b_neg {
                Some(Ordering::Greater)
            } else {
                Some(Ordering::Less)
            };
        }
        if other.mantissa == 0.0 {
            return if a_neg {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Greater)
            };
        }
        // Both same sign, both non-zero.
        // Compare by exponent first (larger exponent = larger magnitude).
        let mag_cmp = self.exponent.cmp(&other.exponent).then_with(|| {
            self.mantissa
                .abs()
                .partial_cmp(&other.mantissa.abs())
                .unwrap_or(Ordering::Equal)
        });
        if a_neg {
            // Both negative: larger magnitude = smaller value.
            Some(mag_cmp.reverse())
        } else {
            Some(mag_cmp)
        }
    }
}

impl fmt::Display for EpDouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mantissa == 0.0 {
            return write!(f, "0");
        }
        // Convert to base-10 scientific notation.
        // value = mantissa * 2^exponent
        // log10(value) = log10(|mantissa|) + exponent * log10(2)
        let log10_val =
            self.mantissa.abs().log10() + (self.exponent as f64) * std::f64::consts::LOG10_2;
        let exp10 = log10_val.floor() as i32;
        // significand in [1, 10)
        let sig = if exp10.abs() < 300 {
            let pow = 10.0f64.powi(-exp10);
            self.mantissa.abs() * exp2_i32(self.exponent) * pow
        } else {
            // Avoid overflow: compute via logs.
            let log_sig = log10_val - exp10 as f64;
            10.0f64.powf(log_sig)
        };
        let sign = if self.mantissa < 0.0 { "-" } else { "" };
        write!(f, "{}{}e{}", sign, sig.abs(), exp10)
    }
}
