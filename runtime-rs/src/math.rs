//! Math Operations for Tarqeem Runtime
//!
//! This module implements mathematical functions for the Tarqeem language,
//! including basic math, trigonometry, logarithms, and random number generation.

use std::cell::RefCell;
use std::f64::consts::PI;

// ============================================================================
// Thread-local RNG state
// ============================================================================

thread_local! {
    static RNG_SEEDED: RefCell<bool> = RefCell::new(false);
    static RNG_STATE: RefCell<u64> = RefCell::new(0);
}

// Simple xorshift64 PRNG
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn ensure_rng_seeded() {
    RNG_SEEDED.with(|seeded| {
        if !*seeded.borrow() {
            // Use current time as seed
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(12345);
            RNG_STATE.with(|state| {
                *state.borrow_mut() = seed;
            });
            *seeded.borrow_mut() = true;
        }
    });
}

// ============================================================================
// Basic Math Functions
// ============================================================================

/// Integer power using binary exponentiation.
/// Returns 0 for negative exponents.
#[no_mangle]
pub extern "C" fn trq_pow_int(base: i64, exp: i64) -> i64 {
    if exp < 0 {
        return 0;
    }
    if exp == 0 {
        return 1;
    }

    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp;

    while e > 0 {
        if e & 1 == 1 {
            result = result.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1;
    }

    result
}

/// Float power.
#[no_mangle]
pub extern "C" fn trq_pow_float(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Absolute value of an integer.
#[no_mangle]
pub extern "C" fn trq_abs_int(value: i64) -> i64 {
    value.abs()
}

/// Absolute value of a float.
#[no_mangle]
pub extern "C" fn trq_abs_float(value: f64) -> f64 {
    value.abs()
}

/// Square root.
#[no_mangle]
pub extern "C" fn trq_sqrt(value: f64) -> f64 {
    value.sqrt()
}

/// Cube root.
#[no_mangle]
pub extern "C" fn trq_cbrt(value: f64) -> f64 {
    value.cbrt()
}

/// Nth root: value^(1/n).
#[no_mangle]
pub extern "C" fn trq_nroot(value: f64, n: i64) -> f64 {
    if n == 0 {
        return 0.0;
    }
    value.powf(1.0 / n as f64)
}

// ============================================================================
// Logarithmic Functions
// ============================================================================

/// Natural logarithm (ln).
#[no_mangle]
pub extern "C" fn trq_log(value: f64) -> f64 {
    value.ln()
}

/// Base-10 logarithm.
#[no_mangle]
pub extern "C" fn trq_log10(value: f64) -> f64 {
    value.log10()
}

/// Base-2 logarithm.
#[no_mangle]
pub extern "C" fn trq_log2(value: f64) -> f64 {
    value.log2()
}

/// Exponential function (e^x).
#[no_mangle]
pub extern "C" fn trq_exp(value: f64) -> f64 {
    value.exp()
}

// ============================================================================
// Rounding Functions
// ============================================================================

/// Floor: largest integer less than or equal to value.
#[no_mangle]
pub extern "C" fn trq_floor(value: f64) -> f64 {
    value.floor()
}

/// Ceiling: smallest integer greater than or equal to value.
#[no_mangle]
pub extern "C" fn trq_ceil(value: f64) -> f64 {
    value.ceil()
}

/// Round to nearest integer.
#[no_mangle]
pub extern "C" fn trq_round(value: f64) -> f64 {
    value.round()
}

/// Truncate toward zero.
#[no_mangle]
pub extern "C" fn trq_trunc(value: f64) -> f64 {
    value.trunc()
}

// ============================================================================
// Comparison and Clamping Functions
// ============================================================================

/// Minimum of two integers.
#[no_mangle]
pub extern "C" fn trq_min_int(a: i64, b: i64) -> i64 {
    a.min(b)
}

/// Maximum of two integers.
#[no_mangle]
pub extern "C" fn trq_max_int(a: i64, b: i64) -> i64 {
    a.max(b)
}

/// Minimum of two floats.
#[no_mangle]
pub extern "C" fn trq_min_float(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Maximum of two floats.
#[no_mangle]
pub extern "C" fn trq_max_float(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Clamp an integer to a range.
#[no_mangle]
pub extern "C" fn trq_clamp_int(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

/// Clamp a float to a range.
#[no_mangle]
pub extern "C" fn trq_clamp_float(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

/// Sign of an integer: -1, 0, or 1.
#[no_mangle]
pub extern "C" fn trq_sign(value: i64) -> i64 {
    value.signum()
}

// ============================================================================
// Number Theory Functions
// ============================================================================

/// Modulo operation that always returns a positive result.
#[no_mangle]
pub extern "C" fn trq_mod(a: i64, b: i64) -> i64 {
    if b == 0 {
        return 0;
    }
    let result = a % b;
    if result < 0 {
        result + b.abs()
    } else {
        result
    }
}

/// Greatest common divisor.
#[no_mangle]
pub extern "C" fn trq_gcd(a: i64, b: i64) -> i64 {
    let mut a = a.abs();
    let mut b = b.abs();
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Least common multiple.
#[no_mangle]
pub extern "C" fn trq_lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        return 0;
    }
    let g = trq_gcd(a, b);
    (a / g) * b.abs()
}

/// Factorial.
#[no_mangle]
pub extern "C" fn trq_factorial(n: i64) -> i64 {
    if n < 0 {
        return 0;
    }
    if n <= 1 {
        return 1;
    }

    let mut result: i64 = 1;
    for i in 2..=n {
        result = result.wrapping_mul(i);
    }
    result
}

// ============================================================================
// Trigonometric Functions
// ============================================================================

/// Sine.
#[no_mangle]
pub extern "C" fn trq_sin(value: f64) -> f64 {
    value.sin()
}

/// Cosine.
#[no_mangle]
pub extern "C" fn trq_cos(value: f64) -> f64 {
    value.cos()
}

/// Tangent.
#[no_mangle]
pub extern "C" fn trq_tan(value: f64) -> f64 {
    value.tan()
}

/// Cotangent (1/tan).
#[no_mangle]
pub extern "C" fn trq_cot(value: f64) -> f64 {
    1.0 / value.tan()
}

/// Secant (1/cos).
#[no_mangle]
pub extern "C" fn trq_sec(value: f64) -> f64 {
    1.0 / value.cos()
}

/// Cosecant (1/sin).
#[no_mangle]
pub extern "C" fn trq_csc(value: f64) -> f64 {
    1.0 / value.sin()
}

// ============================================================================
// Inverse Trigonometric Functions
// ============================================================================

/// Arcsine.
#[no_mangle]
pub extern "C" fn trq_asin(value: f64) -> f64 {
    value.asin()
}

/// Arccosine.
#[no_mangle]
pub extern "C" fn trq_acos(value: f64) -> f64 {
    value.acos()
}

/// Arctangent.
#[no_mangle]
pub extern "C" fn trq_atan(value: f64) -> f64 {
    value.atan()
}

/// Two-argument arctangent.
#[no_mangle]
pub extern "C" fn trq_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

// ============================================================================
// Hyperbolic Functions
// ============================================================================

/// Hyperbolic sine.
#[no_mangle]
pub extern "C" fn trq_sinh(value: f64) -> f64 {
    value.sinh()
}

/// Hyperbolic cosine.
#[no_mangle]
pub extern "C" fn trq_cosh(value: f64) -> f64 {
    value.cosh()
}

/// Hyperbolic tangent.
#[no_mangle]
pub extern "C" fn trq_tanh(value: f64) -> f64 {
    value.tanh()
}

// ============================================================================
// Angle Conversion
// ============================================================================

/// Convert degrees to radians.
#[no_mangle]
pub extern "C" fn trq_to_radians(degrees: f64) -> f64 {
    degrees * (PI / 180.0)
}

/// Convert radians to degrees.
#[no_mangle]
pub extern "C" fn trq_to_degrees(radians: f64) -> f64 {
    radians * (180.0 / PI)
}

// ============================================================================
// Random Number Generation
// ============================================================================

/// Seed the random number generator.
#[no_mangle]
pub extern "C" fn trq_random_seed(seed: i64) {
    RNG_STATE.with(|state| {
        *state.borrow_mut() = seed as u64;
    });
    RNG_SEEDED.with(|seeded| {
        *seeded.borrow_mut() = true;
    });
}

/// Generate a random 64-bit integer.
#[no_mangle]
pub extern "C" fn trq_random_int() -> i64 {
    ensure_rng_seeded();
    RNG_STATE.with(|state| xorshift64(&mut *state.borrow_mut()) as i64)
}

/// Generate a random integer in range [min, max].
#[no_mangle]
pub extern "C" fn trq_random_int_range(min: i64, max: i64) -> i64 {
    if min >= max {
        return min;
    }
    ensure_rng_seeded();
    let range = (max - min + 1) as u64;
    let random = RNG_STATE.with(|state| xorshift64(&mut *state.borrow_mut()));
    min + (random % range) as i64
}

/// Generate a random float in range [0, 1).
#[no_mangle]
pub extern "C" fn trq_random_float() -> f64 {
    ensure_rng_seeded();
    let random = RNG_STATE.with(|state| xorshift64(&mut *state.borrow_mut()));
    (random as f64) / (u64::MAX as f64 + 1.0)
}

/// Generate a random float in range [min, max).
#[no_mangle]
pub extern "C" fn trq_random_float_range(min: f64, max: f64) -> f64 {
    if min >= max {
        return min;
    }
    min + trq_random_float() * (max - min)
}

/// Generate a random boolean.
#[no_mangle]
pub extern "C" fn trq_random_bool() -> bool {
    ensure_rng_seeded();
    RNG_STATE.with(|state| xorshift64(&mut *state.borrow_mut()) & 1 == 0)
}

// ============================================================================
// Mathematical Constants
// ============================================================================

/// Return the mathematical constant π (pi).
#[no_mangle]
pub extern "C" fn trq_pi() -> f64 {
    PI
}

/// Return the mathematical constant e (Euler's number).
#[no_mangle]
pub extern "C" fn trq_e() -> f64 {
    std::f64::consts::E
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_pow_int() {
        assert_eq!(trq_pow_int(2, 0), 1);
        assert_eq!(trq_pow_int(2, 1), 2);
        assert_eq!(trq_pow_int(2, 10), 1024);
        assert_eq!(trq_pow_int(3, 4), 81);
        assert_eq!(trq_pow_int(5, 3), 125);
        assert_eq!(trq_pow_int(2, -1), 0); // Negative exponent
    }

    #[test]
    fn test_pow_float() {
        assert!(approx_eq(trq_pow_float(2.0, 3.0), 8.0));
        assert!(approx_eq(trq_pow_float(4.0, 0.5), 2.0));
        assert!(approx_eq(trq_pow_float(27.0, 1.0 / 3.0), 3.0));
    }

    #[test]
    fn test_abs() {
        assert_eq!(trq_abs_int(-5), 5);
        assert_eq!(trq_abs_int(5), 5);
        assert_eq!(trq_abs_int(0), 0);

        assert!(approx_eq(trq_abs_float(-3.14), 3.14));
        assert!(approx_eq(trq_abs_float(3.14), 3.14));
    }

    #[test]
    fn test_sqrt_cbrt() {
        assert!(approx_eq(trq_sqrt(16.0), 4.0));
        assert!(approx_eq(trq_sqrt(2.0), std::f64::consts::SQRT_2));
        assert!(approx_eq(trq_cbrt(27.0), 3.0));
        assert!(approx_eq(trq_cbrt(8.0), 2.0));
    }

    #[test]
    fn test_nroot() {
        assert!(approx_eq(trq_nroot(16.0, 4), 2.0));
        assert!(approx_eq(trq_nroot(81.0, 4), 3.0));
    }

    #[test]
    fn test_log_functions() {
        assert!(approx_eq(trq_log(std::f64::consts::E), 1.0));
        assert!(approx_eq(trq_log10(100.0), 2.0));
        assert!(approx_eq(trq_log2(8.0), 3.0));
        assert!(approx_eq(trq_exp(1.0), std::f64::consts::E));
    }

    #[test]
    fn test_rounding() {
        assert!(approx_eq(trq_floor(3.7), 3.0));
        assert!(approx_eq(trq_floor(-3.7), -4.0));
        assert!(approx_eq(trq_ceil(3.2), 4.0));
        assert!(approx_eq(trq_ceil(-3.2), -3.0));
        assert!(approx_eq(trq_round(3.5), 4.0));
        assert!(approx_eq(trq_round(3.4), 3.0));
        assert!(approx_eq(trq_trunc(3.9), 3.0));
        assert!(approx_eq(trq_trunc(-3.9), -3.0));
    }

    #[test]
    fn test_minmax() {
        assert_eq!(trq_min_int(5, 3), 3);
        assert_eq!(trq_max_int(5, 3), 5);
        assert!(approx_eq(trq_min_float(5.0, 3.0), 3.0));
        assert!(approx_eq(trq_max_float(5.0, 3.0), 5.0));
    }

    #[test]
    fn test_clamp() {
        assert_eq!(trq_clamp_int(5, 0, 10), 5);
        assert_eq!(trq_clamp_int(-5, 0, 10), 0);
        assert_eq!(trq_clamp_int(15, 0, 10), 10);

        assert!(approx_eq(trq_clamp_float(0.5, 0.0, 1.0), 0.5));
        assert!(approx_eq(trq_clamp_float(-0.5, 0.0, 1.0), 0.0));
        assert!(approx_eq(trq_clamp_float(1.5, 0.0, 1.0), 1.0));
    }

    #[test]
    fn test_sign() {
        assert_eq!(trq_sign(-5), -1);
        assert_eq!(trq_sign(0), 0);
        assert_eq!(trq_sign(5), 1);
    }

    #[test]
    fn test_mod() {
        assert_eq!(trq_mod(7, 3), 1);
        assert_eq!(trq_mod(-7, 3), 2); // Always positive
        assert_eq!(trq_mod(7, -3), 1);
    }

    #[test]
    fn test_gcd_lcm() {
        assert_eq!(trq_gcd(12, 8), 4);
        assert_eq!(trq_gcd(17, 13), 1);
        assert_eq!(trq_gcd(0, 5), 5);

        assert_eq!(trq_lcm(4, 6), 12);
        assert_eq!(trq_lcm(3, 5), 15);
        assert_eq!(trq_lcm(0, 5), 0);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(trq_factorial(0), 1);
        assert_eq!(trq_factorial(1), 1);
        assert_eq!(trq_factorial(5), 120);
        assert_eq!(trq_factorial(10), 3628800);
        assert_eq!(trq_factorial(-1), 0);
    }

    #[test]
    fn test_trig() {
        assert!(approx_eq(trq_sin(0.0), 0.0));
        assert!(approx_eq(trq_cos(0.0), 1.0));
        assert!(approx_eq(trq_tan(0.0), 0.0));

        // sin(π/2) = 1
        assert!(approx_eq(trq_sin(PI / 2.0), 1.0));

        // cos(π) = -1
        assert!(approx_eq(trq_cos(PI), -1.0));
    }

    #[test]
    fn test_inverse_trig() {
        assert!(approx_eq(trq_asin(0.0), 0.0));
        assert!(approx_eq(trq_acos(1.0), 0.0));
        assert!(approx_eq(trq_atan(0.0), 0.0));

        // asin(1) = π/2
        assert!(approx_eq(trq_asin(1.0), PI / 2.0));
    }

    #[test]
    fn test_hyperbolic() {
        assert!(approx_eq(trq_sinh(0.0), 0.0));
        assert!(approx_eq(trq_cosh(0.0), 1.0));
        assert!(approx_eq(trq_tanh(0.0), 0.0));
    }

    #[test]
    fn test_angle_conversion() {
        assert!(approx_eq(trq_to_radians(180.0), PI));
        assert!(approx_eq(trq_to_radians(90.0), PI / 2.0));
        assert!(approx_eq(trq_to_degrees(PI), 180.0));
        assert!(approx_eq(trq_to_degrees(PI / 2.0), 90.0));
    }

    #[test]
    fn test_random() {
        // Seed for reproducibility
        trq_random_seed(42);

        // Test random int
        let r1 = trq_random_int();
        let r2 = trq_random_int();
        assert_ne!(r1, r2); // Should be different

        // Test random float is in range
        trq_random_seed(42);
        for _ in 0..100 {
            let f = trq_random_float();
            assert!(f >= 0.0 && f < 1.0);
        }

        // Test random int range
        trq_random_seed(42);
        for _ in 0..100 {
            let r = trq_random_int_range(10, 20);
            assert!(r >= 10 && r <= 20);
        }

        // Test random float range
        trq_random_seed(42);
        for _ in 0..100 {
            let f = trq_random_float_range(5.0, 10.0);
            assert!(f >= 5.0 && f < 10.0);
        }
    }
}
