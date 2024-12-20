// Copyright 2015 strtod developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// Translated from
// - http://mxr.mozilla.org/mozilla-central/source/js/src/dtoa.c

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

pub mod ffi;

use std::mem::transmute;
use std::num::Wrapping;

const DBL_DIG: u32 = 15;
const DBL_MAX_10_EXP: i32 = 308;
const DBL_MAX_EXP: u32 = 1024;
const FLT_RADIX: u32 = 2;
const Exp_shift: u32 = 20;
const Exp_msk1: u32 = 0x100000;
const Exp_mask: u32 = 0x7ff00000;
const P: u32 = 53;
const Bias: i32 = 1023;
const Emin: i32 = -1022;
const Exp_1: u32 = 0x3ff00000;
const Ebits: u32 = 11;
const Frac_mask: u32 = 0xfffff;
const Frac_mask1: u32 = 0xfffff;
const Ten_pmax: i32 = 22;
const Bndry_mask: u32 = 0xfffff;
const Bndry_mask1: u32 = 0xfffff;
const LSB: u32 = 1;
const Log2P: i32 = 1;
const Tiny1: u32 = 1;
const Flt_Rounds: u32 = 1;
const Big0: u32 = Frac_mask1 | (Exp_msk1 * (DBL_MAX_EXP + Bias as u32 - 1));
const Big1: u32 = 0xffffffff;
const n_bigtens: u32 = 5;
const Scale_Bit: i32 = 0x10;

const CTAB: u32 = 9;
const CNL: u32 = 10;
const CVT: u32 = 11;
const CFF: u32 = 12;
const CCR: u32 = 13;
const CSP: u32 = 32;
const CPLUS: u32 = 43;
const CMIN: u32 = 45;
const CDOT: u32 = 46;
const C0: u32 = 48;
const C9: u32 = 57;
const CE: u32 = 69;
const Ce: u32 = 101;

static tens: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];
static bigtens: [f64; 5] = [1e16, 1e32, 1e64, 1e128, 1e256];
static tinytens: [f64; 5] = [
    1e-16,
    1e-32,
    1e-64,
    1e-128,
    9007199254740992_f64 * 9007199254740992e-256_f64,
];

/// Parses a 64-bit floating point number.
///
/// Leading whitspace and trailing characters are ignored. If the input
/// string does not contain a valid floating point number (where e.g.
/// `"."` is seen as a valid floating point number), `None` is returned.
/// Otherwise the parsed floating point number is returned.
///
/// This implementation is a translation from
/// http://mxr.mozilla.org/mozilla-central/source/js/src/dtoa.c.
///
/// Return : It returns the floating point number and the number of character needed to build that number
pub fn strtod(input: &str) -> Option<(f64, usize)> {
    let mut parser = Parser {
        sign: false,
        rv: U { d: 0_f64 },
    };

    parser.parse(input.as_bytes())
}

#[derive(Copy, Clone)]
struct Chars<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Chars<'a> {
    fn peek(&self) -> u32 {
        if self.offset == self.bytes.len() {
            0
        } else if self.offset < self.bytes.len() {
            self.bytes[self.offset] as u32
        } else {
            panic!();
        }
    }

    fn bump(&mut self) {
        self.offset += 1;
    }
}

struct Parser {
    sign: bool,
    rv: U,
}

impl Parser {
    fn parse(&mut self, input: &[u8]) -> Option<(f64, usize)> {
        let mut s = Chars {
            bytes: input,
            offset: 0,
        };
        if !self.parse_impl(&mut s) {
            None
        } else {
            Some(if self.sign {
                (-self.rv.d, s.offset)
            } else {
                (self.rv.d, s.offset)
            })
        }
    }

    fn parse_impl(&mut self, s: &mut Chars) -> bool {
        let mut nz0 = 0;
        let mut nz = 0;

        loop {
            match s.peek() {
                0 => return false,
                CPLUS | CMIN => {
                    if s.peek() == CMIN {
                        self.sign = true;
                    }
                    s.bump();
                    if s.peek() == 0 {
                        return false;
                    }
                    break;
                }
                CTAB | CNL | CVT | CFF | CCR | CSP => {}
                _ => break,
            }

            s.bump();
        }

        let start = s.clone();

        if s.peek() == C0 {
            nz0 = 1;

            s.bump();
            while s.peek() == C0 {
                s.bump();
            }

            if s.peek() == 0 {
                return true;
            }
        }

        let mut s0 = s.clone();
        let mut y = 0;
        let mut z = 0;

        let mut nd = 0;
        let mut nf = 0;
        let mut c;

        loop {
            c = s.peek();
            if c < C0 || c > C9 {
                break;
            }

            if nd < 9 {
                y = 10 * y + c - C0;
            } else if nd < 16 {
                z = 10 * z + c - C0;
            }

            nd += 1;
            s.bump();
        }

        let mut nd0 = nd;

        if c == CDOT {
            s.bump();
            c = s.peek();

            if nd == 0 {
                while c == C0 {
                    s.bump();
                    c = s.peek();
                    nz += 1;
                }

                if c > C0 && c <= C9 {
                    s0 = s.clone();
                    nf += nz;
                    nz = 0;
                }
            }

            while c >= C0 && c <= C9 {
                nz += 1;

                if c > C0 {
                    nf += nz;

                    for _ in 1..nz {
                        if nd < 9 {
                            y *= 10;
                        } else if nd < DBL_DIG + 1 {
                            z *= 10;
                        }
                        nd += 1;
                    }
                    if nd < 9 {
                        y = 10 * y + c - C0;
                    } else if nd < DBL_DIG + 1 {
                        z = 10 * z + c - C0;
                    }
                    nd += 1;
                    nz = 0;
                }

                s.bump();
                c = s.peek();
            }
        }

        let mut e = 0_i32;
        if c == Ce || c == CE {
            if nd == 0 && nz == 0 && nz0 == 0 {
                self.sign = false;
                return true;
            }

            let mut esign = false;

            s.bump();
            c = s.peek();

            match c {
                CPLUS | CMIN => {
                    if c == CMIN {
                        esign = true;
                    }
                    s.bump();
                    c = s.peek();
                }
                _ => {}
            }

            if c >= C0 && c <= C9 {
                while c == C0 {
                    s.bump();
                    c = s.peek();
                }

                if c > C0 && c <= C9 {
                    let mut L = c - C0;
                    let s1 = s.clone();

                    s.bump();
                    c = s.peek();

                    while c >= C0 && c <= C9 {
                        L = u32::wrapping_sub(u32::wrapping_add(u32::wrapping_mul(L, 10), c), C0);

                        s.bump();
                        c = s.peek();
                    }

                    if s.offset - s1.offset > 8 || L > 19999 {
                        // Avoid confusion from exponents
                        // so large that e might overflow.

                        e = 19999; // safe for 16 bit ints
                    } else {
                        e = L as i32;
                    }

                    if esign {
                        e = -e;
                    }
                } else {
                    e = 0;
                }
            }
        }

        if nd == 0 {
            if nz == 0 && nz0 == 0 {
                self.sign = false;
            }
            return s.offset > start.offset;
        }

        e -= nf;
        let mut e1 = e;

        // Now we have nd0 digits, starting at s0, followed by a
        // decimal point, followed by nd-nd0 digits.  The number we're
        // after is the integer represented by those digits times
        // 10**e

        if nd0 == 0 {
            nd0 = nd;
        }

        let k = if nd < DBL_DIG + 1 { nd } else { DBL_DIG + 1 };
        self.rv.d = y as f64;
        if k > 9 {
            self.rv.d = tens[k as usize - 9] * self.rv.d + z as f64;
        }
        if nd <= DBL_DIG && Flt_Rounds == 1 {
            if e == 0 {
                return true;
            }
            if e > 0 {
                if e <= Ten_pmax {
                    self.rv.d *= tens[e as usize];
                    return true;
                }

                let i = DBL_DIG - nd;
                if e <= Ten_pmax + i as i32 {
                    // A fancier test would sometimes let us do
                    // this for larger i values.
                    e -= i as i32;
                    self.rv.d *= tens[i as usize];
                    self.rv.d *= tens[e as usize];
                    return true;
                }
            } else if e >= -Ten_pmax {
                self.rv.d /= tens[-e as usize];
                return true;
            }
        }

        e1 += (nd - k) as i32;

        let mut scale = 0_i32;

        // Get starting approximation = rv * 10**e1

        if e1 > 0 {
            let i = e1 & 15;
            if i != 0 {
                self.rv.d *= tens[i as usize];
            }
            e1 &= !15;
            if e1 != 0 {
                if e1 > DBL_MAX_10_EXP {
                    self.overflow();
                    return true;
                }

                e1 >>= 4;

                let mut j = 0;

                while e1 > 1 {
                    if e1 & 1 != 0 {
                        self.rv.d *= bigtens[j];
                    }

                    j += 1;
                    e1 >>= 1;
                }

                // The last multiplication could overflow.
                let w = self.rv.word0() - P * Exp_msk1;
                self.rv.set_word0(w);
                self.rv.d *= bigtens[j];

                let z = self.rv.word0() & Exp_mask;
                if z > Exp_msk1 * (DBL_MAX_EXP + Bias as u32 - P) {
                    self.overflow();
                    return true;
                }

                if z > Exp_msk1 * (DBL_MAX_EXP + Bias as u32 - 1 - P) {
                    self.rv.set_word0(Big0);
                    self.rv.set_word1(Big1);
                } else {
                    let w = self.rv.word0() + P * Exp_msk1;
                    self.rv.set_word0(w);
                }
            }
        } else if e1 < 0 {
            e1 = -e1;
            let i = e1 & 15;
            if i != 0 {
                self.rv.d /= tens[i as usize];
            }
            e1 >>= 4;
            if e1 != 0 {
                if e1 >= 1 << n_bigtens {
                    self.rv.d = 0_f64;
                    return true;
                }

                if e1 & Scale_Bit != 0 {
                    scale = (2 * P) as i32;
                }

                let mut j = 0;
                while e1 > 0 {
                    if e1 & 1 != 0 {
                        self.rv.d *= tinytens[j];
                    }

                    j += 1;
                    e1 >>= 1;
                }

                if scale != 0 {
                    let j = 2 * P as i32 + 1 - ((self.rv.word0() & Exp_mask) >> Exp_shift) as i32;
                    if j > 0 {
                        // scaled rv is denormal; zap j low bits
                        if j >= 32 {
                            self.rv.set_word1(0);
                            if j >= 53 {
                                self.rv.set_word0((P + 2) * Exp_msk1);
                            } else {
                                let w = self.rv.word0() & 0xffffffff << (j - 32);
                                self.rv.set_word0(w);
                            }
                        } else {
                            let w = self.rv.word1() & 0xffffffff << j;
                            self.rv.set_word1(w);
                        }

                        if self.rv.d == 0_f64 {
                            return true;
                        }
                    }
                }
            }
        }

        // Now the hard part -- adjusting rv to the correct value.

        // Put digits into bd: true value = bd * 10^e

        let bd0 = s2b(&mut s0, nd0, nd, y);

        loop {
            let mut bd = bd0.clone();
            let (mut bb, bbe, bbbits) = d2b(self.rv); // rv = bb * 2^bbe
            let mut bs = i2b(1);

            let (mut bb2, bb5, mut bd2, bd5) = if e >= 0 {
                (0_i32, 0_i32, e, e)
            } else {
                (-e, -e, 0_i32, 0_i32)
            };

            if bbe >= 0 {
                bb2 += bbe;
            } else {
                bd2 -= bbe;
            }

            let mut bs2 = bb2;

            let mut j = bbe - scale;
            let i = j + bbbits as i32 - 1; // logb(rv)
            if i < Emin {
                j += P as i32 - Emin;
            } else {
                j = (P + 1 - bbbits) as i32;
            }
            bb2 += j;
            bd2 += j;
            bd2 += scale;
            let mut i = if bb2 < bd2 { bb2 } else { bd2 };
            if i > bs2 {
                i = bs2;
            }
            if i > 0 {
                bb2 -= i;
                bd2 -= i;
                bs2 -= i;
            }

            if bb5 > 0 {
                bs = pow5mult(bs, bb5);
                bb = mult(&bs, &bb);
            }
            if bb2 > 0 {
                bb = lshift(&bb, bb2);
            }
            if bd5 > 0 {
                bd = pow5mult(bd, bd5);
            }
            if bd2 > 0 {
                bd = lshift(&bd, bd2);
            }
            if bs2 > 0 {
                bs = lshift(&bs, bs2);
            }

            let mut delta = diff(&bb, &bd);
            let dsign = delta.sign;
            delta.sign = false;

            let i = cmp(&delta, &bs);
            if i < 0 {
                // Error is less than half an ulp -- check for
                // special case of mantissa a power of two.

                if dsign
                    || self.rv.word1() != 0
                    || self.rv.word0() & Bndry_mask != 0
                    || (self.rv.word0() & Exp_mask) <= (2 * P + 1) * Exp_msk1
                {
                    break;
                }

                if delta.x[0] == 0 && delta.x.len() <= 1 {
                    // exact result
                    break;
                }

                delta = lshift(&delta, Log2P);
                if cmp(&delta, &bs) > 0 {
                    if self.drop_down(scale) {
                        return true;
                    }
                }
                break;
            }
            if i == 0 {
                // exactly half-way between
                if dsign {
                    if (self.rv.word0() & Bndry_mask1) == Bndry_mask1 {
                        let y = self.rv.word0() & Exp_mask;
                        if self.rv.word1()
                            == (if scale != 0 && y <= 2 * P * Exp_msk1 {
                                0xffffffff & (0xffffffff << (2 * P + 1 - (y >> Exp_shift)))
                            } else {
                                0xffffffff
                            })
                        {
                            // boundary case -- increment exponent
                            let w = (self.rv.word0() & Exp_mask) + Exp_msk1;
                            self.rv.set_word0(w);
                            self.rv.set_word1(0);
                            break;
                        }
                    }
                } else if self.rv.word0() & Bndry_mask == 0 && self.rv.word1() == 0 {
                    if self.drop_down(scale) {
                        return true;
                    }
                    break;
                }

                if self.rv.word1() & LSB == 0 {
                    break;
                }

                if dsign {
                    self.rv.d += ulp(self.rv);
                } else {
                    self.rv.d -= ulp(self.rv);
                    if self.rv.d == 0_f64 {
                        return true;
                    }
                }

                break;
            }

            let mut aadj = ratio(&delta, &bs);
            let mut aadj1 = U { d: 0_f64 };
            if aadj <= 2_f64 {
                if dsign {
                    aadj = 1_f64;
                    aadj1.d = 1_f64;
                } else if self.rv.word1() != 0 || self.rv.word0() & Bndry_mask != 0 {
                    if self.rv.word1() == Tiny1 && self.rv.word0() == 0 {
                        self.rv.d = 0_f64;
                        return true;
                    }

                    aadj = 1_f64;
                    aadj1.d = -1_f64;
                } else {
                    // special case -- power of FLT_RADIX to be
                    // rounded down...

                    if aadj < 2_f64 / FLT_RADIX as f64 {
                        aadj = 1_f64 / FLT_RADIX as f64;
                    } else {
                        aadj *= 0.5_f64;
                    }
                    aadj1.d = -aadj;
                }
            } else {
                aadj *= 0.5_f64;
                aadj1.d = if dsign { aadj } else { -aadj };
                if Flt_Rounds == 0 {
                    aadj1.d += 0.5_f64;
                }
            }

            let y = self.rv.word0() & Exp_mask;

            // Check for overflow

            if y == Exp_msk1 * (DBL_MAX_EXP + Bias as u32 - 1) {
                let rv0 = self.rv;
                let w = self.rv.word0() - P * Exp_msk1;
                self.rv.set_word0(w);
                let adj = aadj1.d * ulp(self.rv);
                self.rv.d += adj;
                if (self.rv.word0() & Exp_mask) >= Exp_msk1 * (DBL_MAX_EXP + Bias as u32 - P) {
                    if rv0.word0() == Big0 && rv0.word1() == Big1 {
                        self.overflow();
                        return true;
                    }
                    self.rv.set_word0(Big0);
                    self.rv.set_word1(Big1);

                    continue;
                } else {
                    let w = self.rv.word0() + P * Exp_msk1;
                    self.rv.set_word0(w);
                }
            } else {
                if scale != 0 && y <= 2 * P * Exp_msk1 {
                    if aadj <= 0x7fffffff as f64 {
                        let mut z = aadj as u32;
                        if z <= 0 {
                            z = 1;
                        }
                        aadj = z as f64;
                        aadj1.d = if dsign { aadj } else { -aadj };
                    }
                    let w = aadj1.word0() + (2 * P + 1) * Exp_msk1 - y;
                    aadj1.set_word0(w);
                }
                let adj = aadj1.d * ulp(self.rv);
                self.rv.d += adj;
            }

            let z = self.rv.word0() & Exp_mask;
            if scale == 0 {
                if y == z {
                    // Can we stop now?
                    let L = aadj as i32;
                    aadj -= L as f64;
                    // The tolerances below are conservative.
                    if dsign || self.rv.word1() != 0 || self.rv.word0() & Bndry_mask != 0 {
                        if aadj < 0.4999999_f64 || aadj > 0.5000001_f64 {
                            break;
                        }
                    } else if aadj < 0.4999999_f64 / FLT_RADIX as f64 {
                        break;
                    }
                }
            }
        }

        if scale != 0 {
            let mut rv0 = U { d: 0_f64 };
            rv0.set_word0(Exp_1 - 2 * P * Exp_msk1);
            rv0.set_word1(0);
            self.rv.d *= rv0.d;
        }

        true
    }

    fn overflow(&mut self) {
        self.rv.set_word0(Exp_mask);
        self.rv.set_word1(0);
    }

    fn drop_down(&mut self, scale: i32) -> bool {
        // boundary case -- decrement exponent
        if scale != 0 {
            let L = self.rv.word0() & Exp_mask;
            if L <= (2 * P + 1) & Exp_msk1 {
                if L > (P + 2) * Exp_msk1 {
                    // round even ==>
                    // accept rv
                    return false;
                }
                // rv = smallest denormal
                self.rv.d = 0_f64;
                return true;
            }
        }

        let L = (self.rv.word0() & Exp_mask) - Exp_msk1;

        self.rv.set_word0(L | Bndry_mask1);
        self.rv.set_word1(0xffffffff);

        false
    }
}

#[derive(Copy, Clone)]
struct U {
    d: f64,
}

impl U {
    fn word0(&self) -> u32 {
        let words = unsafe { transmute::<_, [u32; 2]>(self.d) };

        if cfg!(target_endian = "little") {
            words[1]
        } else {
            words[0]
        }
    }

    fn set_word0(&mut self, word: u32) {
        let mut words = unsafe { transmute::<_, [u32; 2]>(self.d) };

        if cfg!(target_endian = "little") {
            words[1] = word;
        } else {
            words[0] = word;
        }

        self.d = unsafe { transmute(words) };
    }

    fn word1(&self) -> u32 {
        let words = unsafe { transmute::<_, [u32; 2]>(self.d) };

        if cfg!(target_endian = "little") {
            words[0]
        } else {
            words[1]
        }
    }

    fn set_word1(&mut self, word: u32) {
        let mut words = unsafe { transmute::<_, [u32; 2]>(self.d) };

        if cfg!(target_endian = "little") {
            words[0] = word;
        } else {
            words[1] = word;
        }

        self.d = unsafe { transmute(words) };
    }
}

#[derive(Clone)]
struct BigInt {
    sign: bool,
    x: Vec<u32>,
}

impl BigInt {
    fn new() -> BigInt {
        BigInt {
            sign: false,
            x: Vec::new(),
        }
    }

    fn trim(&mut self) {
        while self.x.len() > 1 {
            if self.x[self.x.len() - 1] == 0 {
                self.x.pop();
            } else {
                break;
            }
        }
    }
}

fn ulp(x: U) -> f64 {
    let L = (x.word0() & Exp_mask) - (P - 1) * Exp_msk1;
    let mut a = U { d: 0_f64 };
    a.set_word0(L);
    a.set_word1(0);
    a.d
}

fn ratio(a: &BigInt, b: &BigInt) -> f64 {
    let (da, ka) = b2d(a);
    let mut da = U { d: da };
    let (db, kb) = b2d(b);
    let mut db = U { d: db };

    let mut k = ka - kb + 32 * (a.x.len() - b.x.len()) as i32;

    if k > 0 {
        let w = da.word0() + k as u32 * Exp_msk1;
        da.set_word0(w);
    } else {
        k = -k;
        let w = db.word0() + k as u32 * Exp_msk1;
        db.set_word0(w);
    }

    da.d / db.d
}

fn b2d(a: &BigInt) -> (f64, i32) {
    let xa0 = 0;
    let mut xa = xa0 + a.x.len() - 1;
    let mut y = a.x[xa];
    let mut k = hi0bits(y);
    let e = 32 - k as i32;

    let mut d = U { d: 0_f64 };

    if k < Ebits {
        d.set_word0(Exp_1 | y >> (Ebits - k));
        let w = if xa > xa0 {
            xa -= 1;
            a.x[xa]
        } else {
            0
        };
        d.set_word1(y << ((32 - Ebits) + k) | w >> (Ebits - k));
        return (d.d, e);
    }

    let z = if xa > xa0 {
        xa -= 1;
        a.x[xa]
    } else {
        0
    };

    k -= Ebits;
    if k != 0 {
        d.set_word0(Exp_1 | y << k | z >> (32 - k));
        y = if xa > xa0 {
            xa -= 1;
            a.x[xa]
        } else {
            0
        };
        d.set_word1(z << k | y >> (32 - k));
    } else {
        d.set_word0(Exp_1 | y);
        d.set_word1(z);
    }

    (d.d, e)
}

fn hi0bits(mut x: u32) -> u32 {
    let mut k = 0;

    if x & 0xffff0000 == 0 {
        k = 16;
        x <<= 16;
    }
    if x & 0xff000000 == 0 {
        k += 8;
        x <<= 8;
    }
    if x & 0xf0000000 == 0 {
        k += 4;
        x <<= 4;
    }
    if x & 0xc0000000 == 0 {
        k += 2;
        x <<= 2;
    }
    if x & 0x80000000 == 0 {
        k += 1;
        if x & 0x40000000 == 0 {
            return 32;
        }
    }
    return k;
}

fn s2b(s: &mut Chars, nd0: u32, nd: u32, y9: u32) -> BigInt {
    let mut b = BigInt::new();
    b.x.push(y9);

    let mut i = 9;
    if 9 < nd0 {
        s.offset += 9;

        loop {
            let c = s.peek();
            s.bump();

            multadd(&mut b, 10, c - C0);

            i += 1;
            if i >= nd0 {
                break;
            }
        }

        s.bump();
    } else {
        s.offset += 10;
    }

    while i < nd {
        let c = s.peek();
        s.bump();

        multadd(&mut b, 10, c - C0);
        i += 1;
    }

    b
}

fn multadd(b: &mut BigInt, m: i32, a: u32) {
    // multiply by m and add a
    let mut carry = a;

    for x in 0..b.x.len() {
        let y = b.x[x] as u64 * m as u64 + carry as u64;
        carry = (y >> 32) as u32;
        b.x[x] = y as u32 & 0xffffffff;
    }

    if carry != 0 {
        b.x.push(carry as u32);
    }
}

fn d2b(mut d: U) -> (BigInt, i32, u32) {
    let mut b = BigInt::new();

    let mut z = d.word0() & Frac_mask;
    let w = d.word0() & 0x7fffffff;
    d.set_word0(w); // clear sign bit, which we ignore
    let de = d.word0() >> Exp_shift;
    if de != 0 {
        z |= Exp_msk1;
    }

    let mut y = d.word1();
    let mut k;

    if y != 0 {
        k = lo0bits(&mut y);
        if k != 0 {
            b.x.push(y | z << (32 - k));
            z >>= k;
        } else {
            b.x.push(y);
        }
        if z != 0 {
            b.x.push(z);
        }
    } else {
        k = lo0bits(&mut z);
        b.x.push(z);
        k += 32;
    }

    let e;
    let bits;

    if de != 0 {
        e = de as i32 - Bias - (P - 1) as i32 + k as i32;
        bits = P - k;
    } else {
        e = de as i32 - Bias - (P - 1) as i32 + 1 + k as i32;
        bits = 32 * b.x.len() as u32 - hi0bits(b.x[b.x.len() - 1]);
    }

    (b, e, bits)
}

fn lo0bits(y: &mut u32) -> u32 {
    let mut x = *y;

    if x & 7 != 0 {
        if x & 1 != 0 {
            return 0;
        }
        if x & 2 != 0 {
            *y = x >> 1;
            return 1;
        }
        *y = x >> 2;
        return 2;
    }
    let mut k = 0;
    if x & 0xffff == 0 {
        k = 16;
        x >>= 16;
    }
    if x & 0xff == 0 {
        k += 8;
        x >>= 8;
    }
    if x & 0xf == 0 {
        k += 4;
        x >>= 4;
    }
    if x & 0x3 == 0 {
        k += 2;
        x >>= 2;
    }
    if x & 1 == 0 {
        k += 1;
        x >>= 1;
        if x == 0 {
            return 32;
        }
    }

    *y = x;

    k
}

fn i2b(i: u32) -> BigInt {
    let mut b = BigInt::new();
    b.x.push(i);
    b
}

static p05: [u32; 3] = [5, 25, 125];

fn pow5mult(mut b: BigInt, mut k: i32) -> BigInt {
    let i = k & 3;
    if i != 0 {
        multadd(&mut b, p05[i as usize - 1] as i32, 0);
    }

    k >>= 2;
    if k == 0 {
        return b;
    }

    let mut p5 = i2b(625);

    loop {
        if k & 1 != 0 {
            b = mult(&b, &p5);
        }
        k >>= 1;
        if k == 0 {
            break;
        }

        p5 = mult(&p5, &p5);
    }

    b
}

fn mult<'a>(mut a: &'a BigInt, mut b: &'a BigInt) -> BigInt {
    if a.x.len() < b.x.len() {
        let c = a;
        a = b;
        b = c;
    }

    let wa = a.x.len();
    let wb = b.x.len();
    let wc = wa + wb;
    let mut c = BigInt::new();
    for _ in 0..wc {
        c.x.push(0);
    }
    let xa = 0;
    let xae = wa;
    let mut xb = 0;
    let xbe = wb;
    let mut xc0 = 0;

    while xb < xbe {
        let y = b.x[xb];
        xb += 1;
        if y != 0 {
            let mut x = xa;
            let mut xc = xc0;
            let mut carry = 0;

            loop {
                let z = a.x[x] as u64 * y as u64 + c.x[xc] as u64 + carry;
                x += 1;
                carry = z >> 32;
                c.x[xc] = z as u32 & 0xffffffff;
                xc += 1;

                if x >= xae {
                    break;
                }
            }

            c.x[xc] = carry as u32;
        }

        xc0 += 1;
    }

    c.trim();

    c
}

fn lshift(b: &BigInt, k: i32) -> BigInt {
    let mut b1 = BigInt::new();
    for _ in 0..(k >> 5) {
        b1.x.push(0);
    }

    let k = k & 0x1f;

    if k != 0 {
        let k1 = 32 - k;
        let mut z = 0;

        for x in 0..b.x.len() {
            b1.x.push(b.x[x] << k | z);
            z = b.x[x] >> k1;
        }

        b1.x.push(z);
    } else {
        for x in 0..b.x.len() {
            b1.x.push(b.x[x]);
        }
    }

    b1.trim();

    b1
}

fn diff<'a>(mut a: &'a BigInt, mut b: &'a BigInt) -> BigInt {
    let mut i = cmp(a, b);
    if i == 0 {
        let mut c = BigInt::new();
        c.x.push(0);
        return c;
    }

    if i < 0 {
        let c = a;
        a = b;
        b = c;
        i = 1;
    } else {
        i = 0;
    }

    let mut c = BigInt::new();
    c.sign = i != 0;
    let mut borrow = 0;
    let mut xa = 0;

    for xb in 0..b.x.len() {
        let y = Wrapping(a.x[xa] as u64) - Wrapping(b.x[xb] as u64) - Wrapping(borrow as u64);
        xa += 1;

        borrow = (y.0 >> 32) as u32 & 1;
        c.x.push((y.0 & 0xffffffff) as u32);
    }

    for xa in xa..a.x.len() {
        let y = Wrapping(a.x[xa] as u64) - Wrapping(borrow as u64);
        borrow = (y.0 >> 32) as u32 & 1;
        c.x.push(y.0 as u32 & 0xffffffff);
    }

    c.trim();

    c
}

fn cmp<'a>(a: &'a BigInt, b: &'a BigInt) -> i32 {
    let mut i = a.x.len() as i32;
    let j = b.x.len() as i32;
    i -= j;
    if i != 0 {
        return i;
    }

    for x in (0..j as usize).rev() {
        if a.x[x] != b.x[x] {
            return if a.x[x] < b.x[x] { -1 } else { 1 };
        }
    }

    0
}

#[cfg(test)]
mod test {
    #![allow(overflowing_literals)]

    use super::strtod;
    use std::f64;

    #[test]
    pub fn tests() {
        test("abc10a", None);
        test("10abc10a", Some((10.0, 2)));
        test("10.12abc10a", Some((10.12, 5)));
        test("10a", Some((10.0, 2)));
        test("1,", Some((1.0, 1)));
        test("1,00", Some((1.0, 1)));
        test("0,00", Some((0.0, 1)));
        test("0.00", Some((0.0, 4)));
        test("12.345", Some((12.345, 6)));
        test("12.345e19", Some((12.345e19, 9)));
        test("-.1e+9", Some((-0.1e+9, 6)));
        test(".125", Some((0.125, 4)));
        test("1e20", Some((1e20, 4)));
        test("0e-19", Some((0.0, 5)));
        test("4\00012", Some((4.0, 1)));
        test("5.9e-76", Some((5.9e-76, 7)));
        test("Inf", None);
        test("-Inf", None);
        test("+InFiNiTy", None);
        test("1e-324", Some((0.0, 6)));
        test(
            "+1.000000000116415321826934814453125",
            Some((1.000000000116415321826934814453125, 36)),
        );
        test("42.0000000000000000001", Some((42.0000000000000000001, 22)));
        test(
            "42.00000000000000000001",
            Some((42.00000000000000000001, 23)),
        );
        test(
            "42.000000000000000000001",
            Some((42.000000000000000000001, 24)),
        );
        test("179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368", Some((179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368.000000, 309)));
        test(".y", Some((0.0, 1)));
        test("0.y", Some((0.0, 2)));
        test(".0y", Some((0.0, 2)));
        test("000,,,e1", Some((0.0, 3)));
        test("000e1", Some((0.0, 5)));
        test("000,1e1", Some((0.0, 3)));
        test("0", Some((0.0, 1)));
        test("000", Some((0.0, 3)));
        test("-0", Some((-0.0, 2)));
        test("-000", Some((-0.0, 4)));
        test("0,", Some((0.0, 1)));
        test("-0,", Some((-0.0, 2)));
        test("0,0", Some((0.0, 1)));
        test("-0,0", Some((-0.0, 2)));
        test("0e-10", Some((0.0, 5)));
        test("-0e-10", Some((-0.0, 6)));
        test("0,e-10", Some((0.0, 1)));
        test("-0,e-10", Some((-0.0, 2)));
        test("0,0e-10", Some((0.0, 1)));
        test("-0,0e-10", Some((-0.0, 2)));
        test("0e-1000000", Some((0.0, 10)));
        test("-0e-1000000", Some((-0.0, 11)));
        test("0,0e-1000000", Some((0.0, 1)));
        test("-0,0e-1000000", Some((-0.0, 2)));
        test("0", Some((0.0, 1)));
        test("000", Some((0.0, 3)));
        test("-0", Some((-0.0, 2)));
        test("-000", Some((-0.0, 4)));
        test("0e-10", Some((0.0, 5)));
        test("-0e-10", Some((-0.0, 6)));
        test("0e-1000000", Some((0.0, 10)));
        test("-0e-1000000", Some((-0.0, 11)));
        test("1", Some((1_f64, 1)));
        test("1.1", Some((1.1, 3)));
        test("1.1e1", Some((1.1e1, 5)));
        test("1234.1234", Some((1234.1234, 9)));
        test("1234.12345678", Some((1234.12345678, 13)));
        test("1234.123456789012", Some((1234.123456789012, 17)));
        test(
            "1.797693134862315708145274237317e+10",
            Some((1.797693134862315708145274237317e+10, 36)),
        );
        test(
            "1.797693134862315708145274237317e+308",
            Some((1.797693134862315708145274237317e+308_f64, 37)),
        );
        test("000000000e123", Some((0.0, 13)));
        test("0000000010000e-329", Some((0.0, 18)));
        test("000000001e-325", Some((0.0, 14)));
        test("0000000020000e-328", Some((0.0, 18)));
        test("0000000090000e-329", Some((0.0, 18)));
        test("0e+999", Some((0.0, 6)));
        test("0e1", Some((0.0, 3)));
        test("0e12345", Some((0.0, 7)));
        test("0e2", Some((0.0, 3)));
        test("0e-2", Some((0.0, 4)));
        test("0e-999", Some((0.0, 6)));
        test("10000e-329", Some((0.0, 10)));
        test("1e-325", Some((0.0, 6)));
        test("20000e-328", Some((0.0, 10)));
        test("2e-324", Some((0.0, 6)));
        test("90000e-329", Some((0.0, 10)));
        test("e1324", Some((0.0, 0)));
        test("1e0", Some((1.0, 3)));
        test("17976931348623157e292", Some((1.7976931348623157E+308, 21)));
        test("17976931348623158e292", Some((1.7976931348623158E+308, 21)));
        test("1e1", Some((10.0, 3)));
        test("1e2", Some((100.0, 3)));
        test(
            "10141204801825834086073718800384e0",
            Some((10141204801825834086073718800384.0, 34)),
        );
        test(
            "1014120480182583464902367222169599999e-5",
            Some((10141204801825834086073718800384.0, 40)),
        );
        test(
            "1014120480182583464902367222169600001e-5",
            Some((10141204801825835211973625643008.0, 40)),
        );
        test(
            "10141204801825834649023672221696e0",
            Some((10141204801825835211973625643008.0, 34)),
        );
        test(
            "10141204801825835211973625643008e0",
            Some((10141204801825835211973625643008.0, 34)),
        );
        test(
            "104110013277974872254e-225",
            Some((104110013277974872254e-225, 26)),
        );
        test("12345e0", Some((12345.0, 7)));
        test("12345e1", Some((123450.0, 7)));
        test("12345e2", Some((1234500.0, 7)));
        test("12345678901234e0", Some((12345678901234.0, 16)));
        test("12345678901234e1", Some((123456789012340.0, 16)));
        test("12345678901234e2", Some((1234567890123400.0, 16)));
        test("123456789012345e0", Some((123456789012345.0, 17)));
        test("123456789012345e1", Some((1234567890123450.0, 17)));
        test("123456789012345e2", Some((12345678901234500.0, 17)));
        test(
            "1234567890123456789012345e108",
            Some((1234567890123456789012345e108, 29)),
        );
        test(
            "1234567890123456789012345e109",
            Some((1234567890123456789012345e109, 29)),
        );
        test(
            "1234567890123456789012345e110",
            Some((1234567890123456789012345e110, 29)),
        );
        test(
            "1234567890123456789012345e111",
            Some((1234567890123456789012345e111, 29)),
        );
        test(
            "1234567890123456789012345e112",
            Some((1234567890123456789012345e112, 29)),
        );
        test(
            "1234567890123456789012345e113",
            Some((1234567890123456789012345e113, 29)),
        );
        test(
            "1234567890123456789012345e114",
            Some((1234567890123456789012345e114, 29)),
        );
        test(
            "1234567890123456789012345e115",
            Some((1234567890123456789012345e115, 29)),
        );
        test(
            "1234567890123456789052345e108",
            Some((1234567890123456789052345e108, 29)),
        );
        test(
            "1234567890123456789052345e109",
            Some((1234567890123456789052345e109, 29)),
        );
        test(
            "1234567890123456789052345e110",
            Some((1234567890123456789052345e110, 29)),
        );
        test(
            "1234567890123456789052345e111",
            Some((1234567890123456789052345e111, 29)),
        );
        test(
            "1234567890123456789052345e112",
            Some((1234567890123456789052345e112, 29)),
        );
        test(
            "1234567890123456789052345e113",
            Some((1234567890123456789052345e113, 29)),
        );
        test(
            "1234567890123456789052345e114",
            Some((1234567890123456789052345e114, 29)),
        );
        test(
            "1234567890123456789052345e115",
            Some((1234567890123456789052345e115, 29)),
        );
        test("123456789012345e-1", Some((123456789012345e-1, 18)));
        test("123456789012345e-2", Some((123456789012345e-2, 18)));
        test("123456789012345e20", Some((123456789012345e20, 18)));
        test("123456789012345e-20", Some((123456789012345e-20, 19)));
        test("123456789012345e22", Some((123456789012345e22, 18)));
        test("123456789012345e-22", Some((123456789012345e-22, 19)));
        test("123456789012345e23", Some((123456789012345e23, 18)));
        test("123456789012345e-23", Some((123456789012345e-23, 19)));
        test("123456789012345e-25", Some((123456789012345e-25, 19)));
        test("123456789012345e35", Some((123456789012345e35, 18)));
        test("123456789012345e36", Some((123456789012345e36, 18)));
        test("123456789012345e37", Some((123456789012345e37, 18)));
        test("123456789012345e39", Some((123456789012345e39, 18)));
        test("123456789012345e-39", Some((123456789012345e-39, 19)));
        test("123456789012345e-5", Some((123456789012345e-5, 18)));
        test("12345678901234e-1", Some((12345678901234e-1, 17)));
        test("12345678901234e-2", Some((12345678901234e-2, 17)));
        test("12345678901234e20", Some((12345678901234e20, 17)));
        test("12345678901234e-20", Some((12345678901234e-20, 18)));
        test("12345678901234e22", Some((12345678901234e22, 17)));
        test("12345678901234e-22", Some((12345678901234e-22, 18)));
        test("12345678901234e23", Some((12345678901234e23, 17)));
        test("12345678901234e-23", Some((12345678901234e-23, 18)));
        test("12345678901234e-25", Some((12345678901234e-25, 18)));
        test("12345678901234e30", Some((12345678901234e30, 17)));
        test("12345678901234e31", Some((12345678901234e31, 17)));
        test("12345678901234e32", Some((12345678901234e32, 17)));
        test("12345678901234e35", Some((12345678901234e35, 17)));
        test("12345678901234e36", Some((12345678901234e36, 17)));
        test("12345678901234e37", Some((12345678901234e37, 17)));
        test("12345678901234e-39", Some((12345678901234e-39, 18)));
        test("12345678901234e-5", Some((12345678901234e-5, 17)));
        test("123456789e108", Some((123456789e108, 13)));
        test("123456789e109", Some((123456789e109, 13)));
        test("123456789e110", Some((123456789e110, 13)));
        test("123456789e111", Some((123456789e111, 13)));
        test("123456789e112", Some((123456789e112, 13)));
        test("123456789e113", Some((123456789e113, 13)));
        test("123456789e114", Some((123456789e114, 13)));
        test("123456789e115", Some((123456789e115, 13)));
        test("12345e-1", Some((12345e-1, 8)));
        test("12345e-2", Some((12345e-2, 8)));
        test("12345e20", Some((12345e20, 8)));
        test("12345e-20", Some((12345e-20, 9)));
        test("12345e22", Some((12345e22, 8)));
        test("12345e-22", Some((12345e-22, 9)));
        test("12345e23", Some((12345e23, 8)));
        test("12345e-23", Some((12345e-23, 9)));
        test("12345e-25", Some((12345e-25, 9)));
        test("12345e30", Some((12345e30, 8)));
        test("12345e31", Some((12345e31, 8)));
        test("12345e32", Some((12345e32, 8)));
        test("12345e35", Some((12345e35, 8)));
        test("12345e36", Some((12345e36, 8)));
        test("12345e37", Some((12345e37, 8)));
        test("12345e-39", Some((12345e-39, 9)));
        test("12345e-5", Some((12345e-5, 8)));
        test("000000001234e304", Some((1234e304, 16)));
        test("0000000123400000e299", Some((1234e304, 20)));
        test("123400000e299", Some((1234e304, 13)));
        test("1234e304", Some((1234e304, 8)));
        test("00000000123400000e300", Some((1234e305, 21)));
        test("00000001234e305", Some((1234e305, 15)));
        test("123400000e300", Some((1234e305, 13)));
        test("1234e305", Some((1234e305, 8)));
        test("00000000170000000e300", Some((17e307, 21)));
        test("0000000017e307", Some((17e307, 14)));
        test("170000000e300", Some((17e307, 13)));
        test("17e307", Some((17e307, 6)));
        test("1e-1", Some((1e-1, 4)));
        test("1e-2", Some((1e-2, 4)));
        test("1e20", Some((1e20, 4)));
        test("1e-20", Some((1e-20, 5)));
        test("1e22", Some((1e22, 4)));
        test("1e-22", Some((1e-22, 5)));
        test("1e23", Some((1e23, 4)));
        test("1e-23", Some((1e-23, 5)));
        test("1e-25", Some((1e-25, 5)));
        test("000000000000100000e303", Some((1e308, 22)));
        test("00000001e308", Some((1e308, 12)));
        test("100000e303", Some((1e308, 10)));
        test("1e308", Some((1e308, 5)));
        test("1e35", Some((1e35, 4)));
        test("1e36", Some((1e36, 4)));
        test("1e37", Some((1e37, 4)));
        test("1e-39", Some((1e-39, 5)));
        test("1e-5", Some((1e-5, 4)));
        test("2e0", Some((2.0, 3)));
        test("22250738585072011e-324", Some((2.225073858507201e-308, 22)));
        test("2e1", Some((20.0, 3)));
        test("2e2", Some((200.0, 3)));
        test("2e-1", Some((2e-1, 4)));
        test("2e-2", Some((2e-2, 4)));
        test("2e20", Some((2e20, 4)));
        test("2e-20", Some((2e-20, 5)));
        test("2e22", Some((2e22, 4)));
        test("2e-22", Some((2e-22, 5)));
        test("2e23", Some((2e23, 4)));
        test("2e-23", Some((2e-23, 5)));
        test("2e-25", Some((2e-25, 5)));
        test("2e35", Some((2e35, 4)));
        test("2e36", Some((2e36, 4)));
        test("2e37", Some((2e37, 4)));
        test("2e-39", Some((2e-39, 5)));
        test("2e-5", Some((2e-5, 4)));
        test("358416272e-33", Some((358416272e-33, 13)));
        test("00000030000e-328", Some((40000e-328, 16)));
        test("30000e-328", Some((40000e-328, 10)));
        test("3e-324", Some((4e-324, 6)));
        test("5445618932859895362967233318697132813618813095743952975439298223406969961560047552942717636670910728746893019786283454139917900193169748259349067524939840552682198095012176093045431437495773903922425632551857520884625114624126588173520906670968542074438852601438992904761759703022688483745081090292688986958251711580854575674815074162979705098246243690189880319928315307816832576838178256307401454285988871020923752587330172447966674453785790265533466496640456213871241930958703059911787722565044368663670643970181259143319016472430928902201239474588139233890135329130660705762320235358869874608541509790266400643191187286648422874774910682648288516244021893172769161449825765517353755844373640588822904791244190695299838293263075467057383813882521706545084301049855505888186560731e-1035", Some((5.445618932859895e-255, 787)));
        test(
            "5708990770823838890407843763683279797179383808e0",
            Some((5708990770823838890407843763683279797179383808.0, 48)),
        );
        test(
            "5708990770823839207320493820740630171355185151999e-3",
            Some((5708990770823838890407843763683279797179383808.0, 52)),
        );
        test(
            "5708990770823839207320493820740630171355185152001e-3",
            Some((5708990770823839524233143877797980545530986496.0, 52)),
        );
        test(
            "5708990770823839207320493820740630171355185152e0",
            Some((5708990770823839524233143877797980545530986496.0, 48)),
        );
        test(
            "5708990770823839524233143877797980545530986496e0",
            Some((5708990770823839524233143877797980545530986496.0, 48)),
        );
        test("72057594037927928e0", Some((72057594037927928.0, 19)));
        test("7205759403792793199999e-5", Some((72057594037927928.0, 25)));
        test("7205759403792793200001e-5", Some((72057594037927936.0, 25)));
        test("72057594037927932e0", Some((72057594037927936.0, 19)));
        test("72057594037927936e0", Some((72057594037927936.0, 19)));
        test("89255e-22", Some((89255e-22, 9)));
        test("9e0", Some((9.0, 3)));
        test("9e1", Some((90.0, 3)));
        test("9e2", Some((900.0, 3)));
        test("9223372036854774784e0", Some((9223372036854774784.0, 21)));
        test(
            "922337203685477529599999e-5",
            Some((9223372036854774784.0, 27)),
        );
        test(
            "922337203685477529600001e-5",
            Some((9223372036854775808.0, 27)),
        );
        test("9223372036854775296e0", Some((9223372036854775808.0, 21)));
        test("9223372036854775808e0", Some((9223372036854775808.0, 21)));
        test("9e-1", Some((9e-1, 4)));
        test("9e-2", Some((9e-2, 4)));
        test("9e20", Some((9e20, 4)));
        test("9e-20", Some((9e-20, 5)));
        test("9e22", Some((9e22, 4)));
        test("9e-22", Some((9e-22, 5)));
        test("9e23", Some((9e23, 4)));
        test("9e-23", Some((9e-23, 5)));
        test("9e-25", Some((9e-25, 5)));
        test("9e35", Some((9e35, 4)));
        test("9e36", Some((9e36, 4)));
        test("9e37", Some((9e37, 4)));
        test("9e-39", Some((9e-39, 5)));
        test("9e-5", Some((9e-5, 4)));
        test("00000000180000000e300", Some((f64::INFINITY, 21)));
        test("0000000018e307", Some((f64::INFINITY, 14)));
        test("00000001000000e303", Some((f64::INFINITY, 18)));
        test("0000001e309", Some((f64::INFINITY, 11)));
        test("1000000e303", Some((f64::INFINITY, 11)));
        test("17976931348623159e292", Some((f64::INFINITY, 21)));
        test("180000000e300", Some((f64::INFINITY, 13)));
        test("18e307", Some((f64::INFINITY, 6)));
        test("1e309", Some((f64::INFINITY, 5)));
    }

    fn test(input: &str, val: Option<(f64, usize)>) {
        let result = strtod(input);
        assert_eq!(result, val);
        if result.is_some() {
            assert_eq!(
                result.unwrap().0.is_sign_positive(),
                val.unwrap().0.is_sign_positive()
            );
        }
    }
}
