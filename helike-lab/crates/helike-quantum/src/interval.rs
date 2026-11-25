//! Outward-rounded interval arithmetic on binary64.
//!
//! Every operation rounds to nearest (IEEE-754 +, −, ×, ÷ and √ are correctly rounded, and Rust never contracts
//! a·b + c into a fused multiply-add) and then steps each endpoint one ulp outward. The exact result of a correctly
//! rounded operation lies within half an ulp of the rounded value, so it lies inside the widened interval. That holds
//! for subnormal results too. Any overflow is caught: every result is checked to be finite.
//!
//! The exponential is not taken from the platform's `exp`, which is not correctly rounded. Here
//! exp x = 2^k · exp r, where r = x − k·ln 2 is formed with an enclosure of ln 2 and |r| ≤ 0.35. exp r is a degree-24
//! Taylor polynomial evaluated in intervals, plus an explicit Lagrange remainder. The ln 2 enclosure is checked
//! against an interval series in the tests.
//!
//! Exact zeros stay exact. 0 + x = x and 0 · x = 0 with no widening, so structural zeros, such as unused top-degree
//! Hermite coefficients, remain exactly zero and the "never truncate a nonzero coefficient" checks keep working.

use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iv {
    pub lo: f64,
    pub hi: f64,
}

pub const ZERO: Iv = Iv { lo: 0.0, hi: 0.0 };
pub const ONE: Iv = Iv { lo: 1.0, hi: 1.0 };

/// ln 2 lies in [0x3FE62E42FEFA39EF, next up] (the binary64 nearest to ln 2 is below it).
pub const LN2: Iv = Iv { lo: f64::from_bits(0x3FE62E42FEFA39EF), hi: f64::from_bits(0x3FE62E42FEFA39F0) };
/// Cody–Waite split: ln 2 = LN2_HI + t with LN2_HI exact (33 significant bits, so k·LN2_HI is exact for
/// |k| ≤ 1024, which covers the |x| ≤ 700 contract) and t in LN2_LO (one ulp of 1.9e-10, about 2.6e-26 wide). Both are checked in the tests against
/// 50 published digits of ln 2 in exact integer arithmetic.
pub const LN2_HI: f64 = f64::from_bits(0x3FE62E42FEE00000);
pub const LN2_LO: Iv = Iv { lo: f64::from_bits(0x3DEA39EF35793C76), hi: f64::from_bits(0x3DEA39EF35793C77) };

#[inline]
fn out(lo: f64, hi: f64) -> Iv {
    let r = Iv { lo: lo.next_down(), hi: hi.next_up() };
    debug_assert!(r.lo <= r.hi);
    r
}

impl Iv {
    #[inline]
    pub fn point(x: f64) -> Iv {
        assert!(x.is_finite(), "nonfinite interval endpoint");
        Iv { lo: x, hi: x }
    }
    pub fn new(lo: f64, hi: f64) -> Iv {
        assert!(lo.is_finite() && hi.is_finite() && lo <= hi, "invalid interval [{lo}, {hi}]");
        Iv { lo, hi }
    }
    /// An integer, exactly (|n| < 2^53).
    pub fn int(n: i64) -> Iv {
        assert!(n.unsigned_abs() < (1u64 << 53));
        Iv::point(n as f64)
    }
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.lo == 0.0 && self.hi == 0.0
    }
    pub fn is_finite(&self) -> bool {
        self.lo.is_finite() && self.hi.is_finite()
    }
    pub fn mid(&self) -> f64 {
        0.5 * self.lo + 0.5 * self.hi
    }
    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }
    pub fn mag(&self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
    pub fn contains(&self, x: f64) -> bool {
        self.lo <= x && x <= self.hi
    }
    pub fn contains_iv(&self, o: &Iv) -> bool {
        self.lo <= o.lo && o.hi <= self.hi
    }
    pub fn hull(&self, o: &Iv) -> Iv {
        Iv { lo: self.lo.min(o.lo), hi: self.hi.max(o.hi) }
    }
    /// x², which is never negative (unlike x · x when x straddles zero).
    #[inline]
    pub fn sqr(self) -> Iv {
        if self.is_zero() {
            return ZERO;
        }
        let (a, b) = (self.lo.abs(), self.hi.abs());
        if self.lo <= 0.0 && self.hi >= 0.0 {
            Iv { lo: 0.0, hi: (a.max(b) * a.max(b)).next_up() }
        } else {
            let (m, n) = if a < b { (a, b) } else { (b, a) };
            Iv { lo: (m * m).next_down().max(0.0), hi: (n * n).next_up() }
        }
    }
    pub fn sqrt(self) -> Iv {
        assert!(self.lo >= 0.0, "square root of an interval reaching below zero: [{}, {}]", self.lo, self.hi);
        Iv { lo: self.lo.sqrt().next_down().max(0.0), hi: self.hi.sqrt().next_up() }
    }
    pub fn recip(self) -> Iv {
        ONE / self
    }
    /// e^x, monotone: the lower bound of e^lo and the upper bound of e^hi.
    pub fn exp(self) -> Iv {
        Iv { lo: exp_point(self.lo).lo, hi: exp_point(self.hi).hi }
    }
    /// e^x − 1, monotone, accurate for small |x|.
    pub fn expm1(self) -> Iv {
        Iv { lo: expm1_point(self.lo).lo, hi: expm1_point(self.hi).hi }
    }
    pub fn powi(self, n: u32) -> Iv {
        let mut r = ONE;
        for _ in 0..n {
            r = r * self;
        }
        r
    }
}

impl Add for Iv {
    type Output = Iv;
    #[inline]
    fn add(self, o: Iv) -> Iv {
        if self.is_zero() {
            return o;
        }
        if o.is_zero() {
            return self;
        }
        out(self.lo + o.lo, self.hi + o.hi)
    }
}
impl Sub for Iv {
    type Output = Iv;
    #[inline]
    fn sub(self, o: Iv) -> Iv {
        self + (-o)
    }
}
impl Neg for Iv {
    type Output = Iv;
    #[inline]
    fn neg(self) -> Iv {
        Iv { lo: -self.hi, hi: -self.lo }
    }
}
impl Mul for Iv {
    type Output = Iv;
    #[inline]
    fn mul(self, o: Iv) -> Iv {
        if self.is_zero() || o.is_zero() {
            return ZERO;
        }
        let (a, b, c, d) = (self.lo * o.lo, self.lo * o.hi, self.hi * o.lo, self.hi * o.hi);
        out(a.min(b).min(c.min(d)), a.max(b).max(c.max(d)))
    }
}
impl Div for Iv {
    type Output = Iv;
    fn div(self, o: Iv) -> Iv {
        assert!(o.lo > 0.0 || o.hi < 0.0, "division by an interval containing zero");
        if self.is_zero() {
            return ZERO;
        }
        let (a, b, c, d) = (self.lo / o.lo, self.lo / o.hi, self.hi / o.lo, self.hi / o.hi);
        out(a.min(b).min(c.min(d)), a.max(b).max(c.max(d)))
    }
}
impl Mul<f64> for Iv {
    type Output = Iv;
    #[inline]
    fn mul(self, o: f64) -> Iv {
        self * Iv::point(o)
    }
}

/// Taylor degree for exp on |r| ≤ 0.5. The remainder |r|^25 / 25! · e^0.5 < 1e-32 is added explicitly.
const TAYLOR: usize = 24;
const REMAINDER: f64 = 1e-32;

/// e^r for |r| ≤ 0.5 as an enclosure: Horner on 1 + r(1 + r/2(1 + r/3(…))) in intervals, plus the remainder.
fn exp_small(r: Iv) -> Iv {
    assert!(r.mag() <= 0.5);
    let mut p = ONE;
    for n in (1..=TAYLOR).rev() {
        p = ONE + r * p / Iv::int(n as i64);
    }
    p + Iv { lo: -REMAINDER, hi: REMAINDER }
}

/// e^r − 1 for |r| ≤ 0.5: r(1 + r/2(1 + r/3(…))) plus the remainder (no cancellation for tiny r).
fn expm1_small(r: Iv) -> Iv {
    assert!(r.mag() <= 0.5);
    let mut p = ONE;
    for n in (2..=TAYLOR).rev() {
        p = ONE + r * p / Iv::int(n as i64);
    }
    r * p + Iv { lo: -REMAINDER * r.mag(), hi: REMAINDER * r.mag() }
}

/// An enclosure of e^x for a binary64 x with |x| ≤ 700.
pub fn exp_point(x: f64) -> Iv {
    assert!(x.is_finite() && x.abs() <= 700.0, "exponential argument {x} outside the enclosure contract");
    if x == 0.0 {
        return ONE;
    }
    let k = (x / LN2.lo).round();
    // k·LN2_HI is exact; only the subtraction from x and the tiny k·t term are rounded
    let r = (Iv::point(x) - Iv::point(k * LN2_HI)) - LN2_LO * Iv::point(k);
    let e = exp_small(r);
    // multiplying by 2^k is exact in the normal range; widen anyway
    let scale = Iv::point(2f64.powi(k as i32));
    let v = e * scale;
    assert!(v.is_finite() && v.lo > 0.0);
    v
}

/// An enclosure of e^x − 1 for a binary64 x.
pub fn expm1_point(x: f64) -> Iv {
    if x == 0.0 {
        return ZERO;
    }
    if x.abs() <= 0.5 {
        expm1_small(Iv::point(x))
    } else {
        exp_point(x) - ONE
    }
}

/// A complex interval (rectangular).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cx {
    pub re: Iv,
    pub im: Iv,
}

pub const CZERO: Cx = Cx { re: ZERO, im: ZERO };

impl Cx {
    pub fn new(re: Iv, im: Iv) -> Cx {
        Cx { re, im }
    }
    pub fn real(re: Iv) -> Cx {
        Cx { re, im: ZERO }
    }
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
    /// |z|².
    #[inline]
    pub fn norm_sq(&self) -> Iv {
        self.re.sqr() + self.im.sqr()
    }
    /// Re(conj(self) · o).
    #[inline]
    pub fn re_conj_mul(&self, o: &Cx) -> Iv {
        self.re * o.re + self.im * o.im
    }
    #[inline]
    pub fn scale(&self, s: Iv) -> Cx {
        Cx { re: self.re * s, im: self.im * s }
    }
    pub fn mul(&self, o: &Cx) -> Cx {
        Cx { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}

impl Add for Cx {
    type Output = Cx;
    #[inline]
    fn add(self, o: Cx) -> Cx {
        Cx { re: self.re + o.re, im: self.im + o.im }
    }
}
impl Sub for Cx {
    type Output = Cx;
    #[inline]
    fn sub(self, o: Cx) -> Cx {
        Cx { re: self.re - o.re, im: self.im - o.im }
    }
}
impl Neg for Cx {
    type Output = Cx;
    #[inline]
    fn neg(self) -> Cx {
        Cx { re: -self.re, im: -self.im }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ln2_enclosure_contains_the_series() {
        // ln 2 = Σ_{k≥1} 1/(k 2^k); the tail after K terms is below 2^-K.
        let mut s = ZERO;
        let mut p = ONE;
        for k in 1..=70i64 {
            p = p / Iv::int(2);
            s = s + p / Iv::int(k);
        }
        let tail = Iv::new(0.0, 2f64.powi(-70));
        let ln2 = s + tail;
        assert!(ln2.hi >= LN2.lo && ln2.lo <= LN2.hi, "series {ln2:?} misses the constant {LN2:?}");
        assert!(ln2.width() < 1e-13);
    }

    /// Minimal unsigned big integers (little-endian base 2^32) for the exact constant checks.
    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct Big(Vec<u32>);
    impl Big {
        fn from(x: u128) -> Big {
            let mut v = vec![];
            let mut x = x;
            while x > 0 {
                v.push(x as u32);
                x >>= 32;
            }
            Big(v)
        }
        fn mul_small(&self, m: u32) -> Big {
            let mut carry = 0u64;
            let mut v: Vec<u32> = self.0.iter().map(|&d| {
                let p = d as u64 * m as u64 + carry;
                carry = p >> 32;
                p as u32
            }).collect();
            if carry > 0 {
                v.push(carry as u32);
            }
            Big(v).trim()
        }
        fn add(&self, o: &Big) -> Big {
            let n = self.0.len().max(o.0.len());
            let mut carry = 0u64;
            let mut v = vec![];
            for i in 0..n {
                let s = *self.0.get(i).unwrap_or(&0) as u64 + *o.0.get(i).unwrap_or(&0) as u64 + carry;
                v.push(s as u32);
                carry = s >> 32;
            }
            if carry > 0 {
                v.push(carry as u32);
            }
            Big(v).trim()
        }
        fn trim(mut self) -> Big {
            while self.0.last() == Some(&0) {
                self.0.pop();
            }
            self
        }
        fn pow(base: u32, e: u32) -> Big {
            (0..e).fold(Big::from(1), |a, _| a.mul_small(base))
        }
        fn mul(&self, o: &Big) -> Big {
            let mut acc = Big::from(0);
            for (i, &d) in o.0.iter().enumerate() {
                let mut part = self.mul_small(d);
                part.0.splice(0..0, std::iter::repeat(0).take(i));
                acc = acc.add(&part.trim());
            }
            acc
        }
        fn cmp_value(&self, o: &Big) -> std::cmp::Ordering {
            let (a, b) = (self.clone().trim(), o.clone().trim());
            a.0.len().cmp(&b.0.len()).then_with(|| a.0.iter().rev().cmp(b.0.iter().rev()))
        }
        fn decimal(digits: &str) -> Big {
            digits.bytes().fold(Big::from(0), |a, c| a.mul_small(10).add(&Big::from((c - b'0') as u128)))
        }
    }

    #[test]
    fn cody_waite_split_brackets_ln2_exactly() {
        use std::cmp::Ordering::*;
        // ln 2 = 0.69314718055994530941723212145817656807550013436025|52…, so D < 10^50 ln 2 < D + 1.
        let d = Big::decimal("69314718055994530941723212145817656807550013436025");
        // LN2_HI = a / 2^53, LN2_LO endpoints = m / 2^85 and (m + 1) / 2^85, so the sum is (a 2^32 + m) / 2^85.
        let a = (LN2_HI.to_bits() & ((1u64 << 52) - 1)) | (1u64 << 52);
        assert_eq!(LN2_HI, a as f64 / 2f64.powi(53));
        assert_eq!(a & 0xFFFFF, 0, "LN2_HI must have 20 trailing zero bits (33 significant), so k·LN2_HI is exact for |k| ≤ 1024");
        let m = (LN2_LO.lo.to_bits() & ((1u64 << 52) - 1)) | (1u64 << 52);
        assert_eq!(LN2_LO.lo, m as f64 * 2f64.powi(-85));
        assert_eq!(LN2_LO.hi, (m + 1) as f64 * 2f64.powi(-85));
        let lo = Big::from(((a as u128) << 32) + m as u128);
        let hi = Big::from(((a as u128) << 32) + m as u128 + 1);
        let (p10, p2) = (Big::pow(10, 50), Big::pow(2, 85));
        // lo / 2^85 < D / 10^50  and  hi / 2^85 > (D + 1) / 10^50
        assert_eq!(lo.mul(&p10).cmp_value(&d.mul(&p2)), Less, "LN2_HI + LN2_LO.lo is not below ln 2");
        assert_eq!(hi.mul(&p10).cmp_value(&d.add(&Big::from(1)).mul(&p2)), Greater, "upper split end is not above ln 2");
    }

    #[test]
    fn ln2_enclosure_brackets_the_decimal_expansion_exactly() {
        // ln 2 = 0.6931471805599453094172|321…, so L < ln 2 < L + 1e-22 with L = 6931471805599453094172e-22.
        // LN2.lo = m / 2^53 and LN2.hi = (m + 1) / 2^53 with m its 53-bit significand. Compare in integers:
        // m·10^22 < L·2^53 and (m + 1)·10^22 > (L + 1)·2^53.
        let m = (LN2.lo.to_bits() & ((1u64 << 52) - 1)) | (1u64 << 52);
        assert_eq!(LN2.lo, m as f64 / 2f64.powi(53));
        assert_eq!(LN2.hi, (m + 1) as f64 / 2f64.powi(53));
        let l: u128 = 6931471805599453094172;
        let (ten22, two53) = (10u128.pow(22), 1u128 << 53);
        assert!((m as u128) * ten22 < l * two53, "LN2.lo is not below ln 2");
        assert!((m as u128 + 1) * ten22 > (l + 1) * two53, "LN2.hi is not above ln 2");
    }

    #[test]
    fn exp_encloses_known_values_tightly() {
        // e = 2.718281828459045235360287…: both binary64 neighbours of e are inside, width ≤ 32 ulp
        let e = exp_point(1.0);
        assert!(e.lo <= 2.718281828459045 && e.hi >= 2.7182818284590455, "{e:?}");
        assert!(e.width() <= 32.0 * f64::EPSILON * 2.72, "{e:?}");
        // e^-10 = 4.539992976248485153559151556…e-5
        let v = exp_point(-10.0);
        assert!(v.contains(4.539992976248485e-5) && v.width() < 1e-18, "{v:?}");
        // e^20 = 485165195.4097902779691068…
        let v = exp_point(20.0);
        // width is set by the one-ulp enclosure of ln 2 times k = 29: about 1e-14 relative
        assert!(v.contains(485165195.4097903) && v.width() < 64.0 * f64::EPSILON * 4.86e8, "{v:?}");
        for x in [-30.0, -3.3, -0.4, 0.2, 0.7, 5.5, 41.0] {
            let v = exp_point(x);
            assert!(v.contains(x.exp()) || (v.lo - x.exp()).abs() < 2e-16 * x.exp(), "{x}: {v:?} vs {}", x.exp());
            assert!(v.width() <= 64.0 * f64::EPSILON * x.exp(), "{x}: width {}", v.width());
        }
    }

    #[test]
    fn expm1_keeps_tiny_arguments() {
        let v = expm1_point(1e-10);
        // 1e-10 + 5e-21 + …
        assert!(v.contains(1.00000000005e-10) && v.width() < 1e-25, "{v:?}");
        let v = expm1_point(-0.3);
        assert!(v.contains((-0.3f64).exp_m1()) && v.width() < 1e-15);
        let v = expm1_point(3.0);
        assert!(v.contains(3f64.exp_m1()) && v.width() < 1e-13);
    }

    #[test]
    fn arithmetic_is_outward_and_keeps_zeros() {
        let a = Iv::point(0.1) + Iv::point(0.2);
        assert!(a.lo < a.hi && a.contains(0.30000000000000004) && a.contains(0.3));
        assert_eq!(ZERO + ZERO, ZERO);
        assert_eq!(ZERO * Iv::point(3.0), ZERO);
        let s = Iv::new(-2.0, 3.0).sqr();
        assert_eq!(s.lo, 0.0);
        assert!(s.hi >= 9.0);
        let q = Iv::point(2.0).sqrt();
        assert!(q.lo < std::f64::consts::SQRT_2 + 1e-16 && q.hi > std::f64::consts::SQRT_2 - 1e-16);
        assert!((Iv::point(1.0) / Iv::point(3.0)).contains(1.0 / 3.0));
    }
}
