# Extended Precision Counting

The standard minterm counting method `bdd_count_minterm` returns an `f64`. While this is fine for BDDs with up to about 1000 variables, `f64` overflows at approximately 2^1024 and loses precision much earlier due to its 53-bit mantissa. For large BDDs, lumindd provides two extended precision alternatives.

## The Overflow Problem

A BDD with `n` variables can have up to 2^n satisfying assignments. For `n > 1023`, this value exceeds the range of `f64`. Even for smaller `n`, the mantissa precision of `f64` (53 bits) means that counts above 2^53 are only approximate.

```rust
// Standard counting -- works for small n
let count_f64 = mgr.bdd_count_minterm(f, 20);  // fine: 2^20 = ~1M

// But for large n:
let count_big = mgr.bdd_count_minterm(f, 2000); // returns infinity!
```

## EpDouble: Extended Precision Double

`EpDouble` represents a value as `mantissa * 2^exponent`, where the mantissa is a normalized `f64` in `[0.5, 1.0)` and the exponent is an `i32`. This extends the range to approximately 2^(2^31) while preserving full `f64` mantissa precision.

### Basic Usage

```rust
use lumindd::epd::EpDouble;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
// ... build BDD f ...

let count = mgr.bdd_count_minterm_epd(f, 2000);

// Convert to f64 if the value fits
if let Some(val) = count.to_f64() {
    println!("Count: {}", val);
} else {
    println!("Count too large for f64: {}", count);
}
```

### EpDouble API

```rust
// Construction
let zero = EpDouble::zero();
let one = EpDouble::one();
let val = EpDouble::new(3.14);
let power = EpDouble::two_power(1000);  // exactly 2^1000
let parts = EpDouble::from_parts(0.75, 10);  // 0.75 * 2^10 = 768

// Arithmetic (all return normalized EpDouble)
let sum = a + b;
let diff = a - b;
let prod = a * b;
let quot = a / b;

// Comparison
if a > b { /* ... */ }
if a == EpDouble::zero() { /* ... */ }

// Conversion
let f64_val: Option<f64> = val.to_f64();  // None if overflow
let is_zero: bool = val.is_zero();
let negated = val.negate();

// Display (base-10 scientific notation)
println!("{}", count); // e.g., "1.5e300"
```

### When to Use EpDouble

- BDDs with 1000+ variables where `f64` would overflow
- Applications requiring the full dynamic range but where mantissa precision of ~15 decimal digits is acceptable
- Performance-sensitive code (EpDouble arithmetic is only slightly slower than `f64`)

## ApInt: Arbitrary Precision Integer

`ApInt` is a non-negative arbitrary precision integer stored as a vector of base-2^32 digits (little-endian). It provides exact counts with no rounding and no overflow, at the cost of slower arithmetic for very large values.

### Basic Usage

```rust
use lumindd::apa::ApInt;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();
// ... build BDD f ...

let count = mgr.bdd_count_minterm_apa(f, 100);
println!("Exact count: {}", count);  // prints full decimal integer
```

### ApInt API

```rust
// Construction
let zero = ApInt::zero();
let one = ApInt::one();
let val = ApInt::from_u64(123456789);
let power = ApInt::two_power(256);  // exactly 2^256

// Arithmetic
let sum = &a + &b;     // or: a + b (consumes operands)
let diff = &a - &b;    // panics if a < b
let scaled = &a * 42u32;  // scalar multiplication
let shifted = &a << 10;   // left shift (multiply by 2^10)

// Comparison
assert!(ApInt::two_power(100) > ApInt::two_power(99));

// Properties
let bits = val.bit_length();  // number of significant bits
let is_zero = val.is_zero();

// Display (full decimal string)
println!("{}", ApInt::two_power(100));
// prints: 1267650600228229401496703205376
```

### When to Use ApInt

- When you need the exact integer count with no rounding
- Formal verification where approximation is unacceptable
- Computing ratios or probabilities from exact counts
- BDDs where the count exceeds 2^1024 and `EpDouble` precision is insufficient

## Comparison

| Feature | `f64` | `EpDouble` | `ApInt` |
|---|---|---|---|
| Max value | ~2^1024 | ~2^(2^31) | Unlimited |
| Precision | 53 bits | 53 bits mantissa | Exact |
| Speed | Fastest | Fast | Slower for large values |
| Memory | 8 bytes | 12 bytes | Proportional to value |
| Method | `bdd_count_minterm` | `bdd_count_minterm_epd` | `bdd_count_minterm_apa` |

## Example: Exact Counting for Large BDDs

```rust
use lumindd::Manager;
use lumindd::apa::ApInt;

let mut mgr = Manager::new();
let x = mgr.bdd_new_var();

// Count minterms of just x with 100 total variables
// Answer should be exactly 2^99
let count = mgr.bdd_count_minterm_apa(x, 100);
assert_eq!(count, ApInt::two_power(99));
println!("x has {} minterms over 100 variables", count);
```

## API Reference

| Method | Return Type | Description |
|---|---|---|
| `bdd_count_minterm(f, n)` | `f64` | Standard minterm count |
| `bdd_count_minterm_epd(f, n)` | `EpDouble` | Extended precision count |
| `bdd_count_minterm_apa(f, n)` | `ApInt` | Arbitrary precision exact count |
