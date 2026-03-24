// lumindd — Arbitrary Precision Arithmetic (APA) for exact minterm counting
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Arbitrary Precision Integer type for exact minterm counting.
//!
//! [`ApInt`] stores a non-negative integer as a `Vec<u32>` of digits in
//! base 2^32, little-endian order (least significant digit first). This
//! allows exact minterm counts for BDDs with arbitrarily many variables
//! — no rounding, no overflow.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Shl, Sub};

/// Arbitrary precision non-negative integer.
///
/// Digits are stored little-endian in base 2^32. The representation is
/// always canonical: no trailing zero digits (except for the value zero
/// itself, which is stored as an empty vector).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApInt {
    /// Little-endian base-2^32 digits.
    pub digits: Vec<u32>,
}

impl ApInt {
    /// The value zero.
    pub fn zero() -> Self {
        ApInt { digits: Vec::new() }
    }

    /// The value one.
    pub fn one() -> Self {
        ApInt { digits: vec![1] }
    }

    /// Create from a `u64`.
    pub fn from_u64(v: u64) -> Self {
        if v == 0 {
            return Self::zero();
        }
        let lo = v as u32;
        let hi = (v >> 32) as u32;
        let mut digits = vec![lo];
        if hi != 0 {
            digits.push(hi);
        }
        ApInt { digits }
    }

    /// Exact representation of 2^n.
    pub fn two_power(n: u32) -> Self {
        let word_idx = (n / 32) as usize;
        let bit_idx = n % 32;
        let mut digits = vec![0u32; word_idx + 1];
        digits[word_idx] = 1u32 << bit_idx;
        ApInt { digits }
    }

    /// Returns `true` if the value is zero.
    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// Remove trailing zero digits to keep canonical form.
    fn trim(&mut self) {
        while self.digits.last() == Some(&0) {
            self.digits.pop();
        }
    }

    /// Number of significant bits (0 for zero).
    pub fn bit_length(&self) -> u32 {
        if self.digits.is_empty() {
            return 0;
        }
        let top = *self.digits.last().unwrap();
        let top_bits = 32 - top.leading_zeros();
        (self.digits.len() as u32 - 1) * 32 + top_bits
    }
}

// =====================================================================
// Addition
// =====================================================================

impl Add for ApInt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        add_ref(&self, &rhs)
    }
}

impl Add for &ApInt {
    type Output = ApInt;

    fn add(self, rhs: Self) -> ApInt {
        add_ref(self, rhs)
    }
}

fn add_ref(a: &ApInt, b: &ApInt) -> ApInt {
    let len = a.digits.len().max(b.digits.len());
    let mut result = Vec::with_capacity(len + 1);
    let mut carry: u64 = 0;
    for i in 0..len {
        let da = if i < a.digits.len() {
            a.digits[i] as u64
        } else {
            0
        };
        let db = if i < b.digits.len() {
            b.digits[i] as u64
        } else {
            0
        };
        let sum = da + db + carry;
        result.push(sum as u32);
        carry = sum >> 32;
    }
    if carry != 0 {
        result.push(carry as u32);
    }
    let mut r = ApInt { digits: result };
    r.trim();
    r
}

// =====================================================================
// Subtraction (panics if result would be negative)
// =====================================================================

impl Sub for ApInt {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        sub_ref(&self, &rhs)
    }
}

impl Sub for &ApInt {
    type Output = ApInt;

    fn sub(self, rhs: Self) -> ApInt {
        sub_ref(self, rhs)
    }
}

fn sub_ref(a: &ApInt, b: &ApInt) -> ApInt {
    assert!(
        a >= b,
        "ApInt subtraction underflow: cannot subtract larger from smaller"
    );
    let mut result = Vec::with_capacity(a.digits.len());
    let mut borrow: i64 = 0;
    for i in 0..a.digits.len() {
        let da = a.digits[i] as i64;
        let db = if i < b.digits.len() {
            b.digits[i] as i64
        } else {
            0
        };
        let diff = da - db - borrow;
        if diff < 0 {
            result.push((diff + (1i64 << 32)) as u32);
            borrow = 1;
        } else {
            result.push(diff as u32);
            borrow = 0;
        }
    }
    debug_assert_eq!(borrow, 0);
    let mut r = ApInt { digits: result };
    r.trim();
    r
}

// =====================================================================
// Scalar multiplication (ApInt * u32)
// =====================================================================

impl Mul<u32> for ApInt {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self {
        scalar_mul(&self, rhs)
    }
}

impl Mul<u32> for &ApInt {
    type Output = ApInt;

    fn mul(self, rhs: u32) -> ApInt {
        scalar_mul(self, rhs)
    }
}

fn scalar_mul(a: &ApInt, b: u32) -> ApInt {
    if b == 0 || a.is_zero() {
        return ApInt::zero();
    }
    let b = b as u64;
    let mut result = Vec::with_capacity(a.digits.len() + 1);
    let mut carry: u64 = 0;
    for &d in &a.digits {
        let prod = d as u64 * b + carry;
        result.push(prod as u32);
        carry = prod >> 32;
    }
    if carry != 0 {
        result.push(carry as u32);
    }
    let mut r = ApInt { digits: result };
    r.trim();
    r
}

// =====================================================================
// Left shift (multiply by 2^n)
// =====================================================================

impl Shl<u32> for ApInt {
    type Output = Self;

    fn shl(self, n: u32) -> Self {
        shl_ref(&self, n)
    }
}

impl Shl<u32> for &ApInt {
    type Output = ApInt;

    fn shl(self, n: u32) -> ApInt {
        shl_ref(self, n)
    }
}

fn shl_ref(a: &ApInt, n: u32) -> ApInt {
    if a.is_zero() || n == 0 {
        return a.clone();
    }
    let word_shift = (n / 32) as usize;
    let bit_shift = n % 32;

    let new_len = a.digits.len() + word_shift + if bit_shift > 0 { 1 } else { 0 };
    let mut result = vec![0u32; new_len];

    if bit_shift == 0 {
        for (i, &d) in a.digits.iter().enumerate() {
            result[i + word_shift] = d;
        }
    } else {
        let mut carry = 0u32;
        for (i, &d) in a.digits.iter().enumerate() {
            let shifted = ((d as u64) << bit_shift) | carry as u64;
            result[i + word_shift] = shifted as u32;
            carry = (shifted >> 32) as u32;
        }
        if carry != 0 {
            result[a.digits.len() + word_shift] = carry;
        }
    }

    let mut r = ApInt { digits: result };
    r.trim();
    r
}

// =====================================================================
// Ordering
// =====================================================================

impl PartialOrd for ApInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ApInt {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare lengths first.
        let a_len = self.digits.len();
        let b_len = other.digits.len();
        match a_len.cmp(&b_len) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => {
                // Same length: compare from most significant digit.
                for i in (0..a_len).rev() {
                    match self.digits[i].cmp(&other.digits[i]) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                Ordering::Equal
            }
        }
    }
}

// =====================================================================
// Display — decimal string
// =====================================================================

impl fmt::Display for ApInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }

        // Repeatedly divide by 10^9 to extract groups of 9 decimal digits.
        const DIVISOR: u64 = 1_000_000_000;
        let mut remaining = self.digits.clone();
        let mut groups: Vec<u32> = Vec::new();

        while !remaining.is_empty() {
            let mut rem: u64 = 0;
            for i in (0..remaining.len()).rev() {
                let cur = rem << 32 | remaining[i] as u64;
                remaining[i] = (cur / DIVISOR) as u32;
                rem = cur % DIVISOR;
            }
            groups.push(rem as u32);
            // Trim leading zeros.
            while remaining.last() == Some(&0) {
                remaining.pop();
            }
        }

        // Print the most significant group without leading zeros.
        let last = groups.len() - 1;
        write!(f, "{}", groups[last])?;
        // Print the rest with zero-padding to 9 digits.
        for i in (0..last).rev() {
            write!(f, "{:09}", groups[i])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert_eq!(ApInt::zero().to_string(), "0");
        assert_eq!(ApInt::one().to_string(), "1");
        assert_eq!(ApInt::from_u64(123456789).to_string(), "123456789");
    }

    #[test]
    fn test_two_power() {
        assert_eq!(ApInt::two_power(0).to_string(), "1");
        assert_eq!(ApInt::two_power(10).to_string(), "1024");
        assert_eq!(ApInt::two_power(32).to_string(), "4294967296");
        assert_eq!(ApInt::two_power(64).to_string(), "18446744073709551616");
    }

    #[test]
    fn test_add_sub() {
        let a = ApInt::from_u64(u64::MAX);
        let b = ApInt::one();
        let c = &a + &b;
        assert_eq!(c.to_string(), "18446744073709551616"); // 2^64
        let d = &c - &b;
        assert_eq!(d, a);
    }

    #[test]
    fn test_shift() {
        let one = ApInt::one();
        let shifted = &one << 100;
        assert_eq!(shifted, ApInt::two_power(100));
    }

    #[test]
    fn test_scalar_mul() {
        let a = ApInt::from_u64(1_000_000_000);
        let b = &a * 1_000_000_000u32;
        assert_eq!(b.to_string(), "1000000000000000000");
    }

    #[test]
    fn test_ord() {
        assert!(ApInt::zero() < ApInt::one());
        assert!(ApInt::two_power(100) > ApInt::two_power(99));
    }
}
