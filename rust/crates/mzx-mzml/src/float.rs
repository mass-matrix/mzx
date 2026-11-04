//! pwiz's canonical float formatting: port of `PwizFloat` (pwiz/data/common/PwizFloat.cs
//! @ fb75aff), itself a port of boost::spirit::karma's `real_inserter::call_n` with the
//! `double12_policy` / `float5_policy` / `nosci_policy` pwiz configures.
//!
//! The rounding is binary: the fractional part is scaled by `10^precision` in the
//! value's own float type and rounded with `floor(x + 0.5)`.

macro_rules! generate {
    ($name:ident, $t:ty, $max_exp10:expr) => {
        fn $name(value: $t, precision: i32, force_fixed: bool) -> String {
            if value.is_nan() {
                return if value.is_sign_negative() {
                    "-nan"
                } else {
                    "nan"
                }
                .into();
            }
            if value.is_infinite() {
                return if value.is_sign_negative() {
                    "-inf"
                } else {
                    "inf"
                }
                .into();
            }
            let one: $t = 1.0;
            let ten: $t = 10.0;
            let half: $t = 0.5;
            let pow10 = |e: i32| -> $t { POW10[e as usize] as $t };
            let rem10 = |v: $t| -> i64 { ((v % ten) as f64).floor() as i64 };

            let sign = value.is_sign_negative();
            let scientific = !force_fixed && is_scientific(value as f64);
            let mut n = if sign { -value } else { value };

            let mut dim: i32 = 0;
            let mut precexp_offset = false;
            if scientific {
                let log = n.log10();
                if log > 0.0 {
                    dim = log.trunc() as i32;
                    n /= pow10(dim);
                } else if n < one {
                    let exp = (-log).trunc() as i32;
                    dim = -exp;
                    if exp > $max_exp10 {
                        n *= pow10($max_exp10);
                        n *= pow10(exp - $max_exp10);
                    } else {
                        n *= pow10(exp);
                    }
                    if n < one {
                        n *= ten;
                        dim -= 1;
                        precexp_offset = true;
                    }
                }
            }

            let mut integer_part = n.trunc();
            let mut fractional_part = n - integer_part;
            let precexp = pow10(precision);
            if precexp_offset {
                fractional_part = ((fractional_part * precexp + half) * ten).floor() / ten;
            } else {
                fractional_part = (fractional_part * precexp + half).floor();
            }
            if fractional_part >= precexp {
                fractional_part = (fractional_part - precexp).floor();
                integer_part += one;
                if integer_part >= ten && scientific {
                    integer_part /= ten;
                    dim += 1;
                }
            }

            let long_int_part = integer_part.floor();
            let mut long_frac_part = fractional_part;
            let frac_part_floor = long_frac_part;
            let mut prec = precision;
            if long_frac_part != 0.0 {
                while prec != 0 && rem10(long_frac_part) == 0 {
                    long_frac_part = (long_frac_part / ten).floor();
                    prec -= 1;
                }
            } else {
                prec = 0;
            }
            if precision != prec {
                long_frac_part = frac_part_floor / pow10(precision - prec);
            }

            let mut s = String::with_capacity(32);
            if sign && !(long_int_part == 0.0 && long_frac_part == 0.0) {
                s.push('-');
            }
            append_integer_digits(
                &mut s,
                long_int_part as f64,
                |v| rem10(v as $t),
                |value, exp| ((value as $t) / pow10(exp.min(308))).floor() as f64,
            );
            s.push('.');
            let mut digits: $t = if long_frac_part == 0.0 {
                one
            } else {
                (long_frac_part + one).log10().ceil()
            };
            let pad = prec as $t;
            while digits < pad {
                s.push('0');
                digits += one;
            }
            if precision != 0 {
                append_integer_digits(
                    &mut s,
                    long_frac_part as f64,
                    |v| rem10(v as $t),
                    |value, exp| ((value as $t) / pow10(exp.min(308))).floor() as f64,
                );
            }
            if scientific {
                s.push('e');
                if dim < 0 {
                    s.push('-');
                }
                let a = dim.abs();
                if a < 10 {
                    s.push('0');
                }
                s.push_str(&a.to_string());
            }
            s
        }
    };
}

/// 1e0..=1e308 as correctly rounded doubles (karma's pow10 table).
static POW10: std::sync::LazyLock<[f64; 309]> = std::sync::LazyLock::new(|| {
    let mut t = [0.0; 309];
    for (i, v) in t.iter_mut().enumerate() {
        *v = format!("1e{i}").parse().unwrap();
    }
    t
});

const MAX_LONG: f64 = 9.223_372_036_854_776E18;

fn is_scientific(v: f64) -> bool {
    if v == 0.0 {
        return false;
    }
    let a = v.abs();
    !(1e-3..1e5).contains(&a)
}

/// `AppendIntegerDigits`: integral-valued float, printed through i64 when in range,
/// else peeled digit by digit in the value's own type.
fn append_integer_digits(
    s: &mut String,
    d: f64,
    rem: impl Fn(f64) -> i64,
    div: impl Fn(f64, i32) -> f64,
) {
    if d.abs() < MAX_LONG {
        s.push_str(&(d as i64).to_string());
        return;
    }
    let start = s.len();
    let mut remaining = d;
    let mut exp = 0;
    loop {
        let r = rem(remaining);
        s.push((b'0' + r as u8) as char);
        exp += 1;
        remaining = div(d, exp);
        if remaining == 0.0 {
            break;
        }
    }
    let tail: String = s[start..].chars().rev().collect();
    s.truncate(start);
    s.push_str(&tail);
}

generate!(generate_f64, f64, 308);
generate!(generate_f32, f32, 38);

const DOUBLE_MIN_NORMAL: f64 = f64::MIN_POSITIVE;
const FLOAT_MIN_NORMAL: f32 = f32::MIN_POSITIVE;

/// `pwiz::util::toString(double)` (karma `double12_policy`).
pub fn double12(value: f64) -> String {
    let v = if value > 0.0 {
        value.max(DOUBLE_MIN_NORMAL)
    } else if value < 0.0 {
        value.min(-DOUBLE_MIN_NORMAL)
    } else {
        value
    };
    generate_f64(v, 12, false)
}

/// `pwiz::util::toString(float)` (karma `float5_policy`, single precision).
pub fn float5(value: f32) -> String {
    let v = if value > 0.0 {
        value.max(FLOAT_MIN_NORMAL)
    } else if value < 0.0 {
        value.min(-FLOAT_MIN_NORMAL)
    } else {
        value
    };
    generate_f32(v, 5, false)
}

/// karma `nosci_policy`: fixed notation, 9 fractional digits.
pub fn nosci9(value: f64) -> String {
    generate_f64(value, 9, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double12_matches_goldens() {
        assert_eq!(double12(0.0035445841), "0.0035445841");
        assert_eq!(double12(123.0), "123.0");
        assert_eq!(double12(120.055793762207), "120.055793762207");
        assert_eq!(double12(1323.778076171875), "1323.778076171875");
        assert_eq!(double12(296.2006143414705), "296.20061434147");
        assert_eq!(double12(1024187.1), "1.0241871e06");
        assert_eq!(double12(0.0), "0.0");
        assert_eq!(double12(-0.0), "0.0");
        assert_eq!(double12(1e-4), "1.0e-04");
        assert_eq!(double12(114.000000059605), "114.000000059605");
    }
}
