//! ISO 8601 ⇄ epoch milliseconds, dependency-free, UTC only.
//!
//! GIGI stores time as `Value::Timestamp(i64)` — milliseconds since the
//! Unix epoch. Nobody thinks in epoch millis, so this module is the
//! translation layer the ergonomics ride on:
//!
//! - [`parse_iso_ms`] accepts what a person types: `2026-07-02`,
//!   `2026-07-02 14:30`, `2026-07-02T14:30:05`, optional `.mmm`
//!   fraction, optional trailing `Z`. Anything else is `None` — the
//!   caller decides how loudly to refuse.
//! - [`format_iso_ms`] renders `YYYY-MM-DDTHH:MM:SS(.mmm)Z`, trimming
//!   the fraction when it is zero.
//!
//! Date math is the standard civil-from-days / days-from-civil pair
//! (Howard Hinnant's algorithms), exact over the whole i64-ms range we
//! care about. Leap seconds are ignored, like every other database.

/// Days from civil date (proleptic Gregorian). y-m-d → days since epoch.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64; // [0, 399]
    let mp = ((m + 9) % 12) as i64; // Mar=0..Feb=11
    let doy = (153 * mp + 2) / 5 + (d as i64 - 1); // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Civil date from days since epoch.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Parse an ISO-8601-ish datetime into epoch milliseconds (UTC).
///
/// Accepted: `YYYY-MM-DD`, plus optional `[ T]HH:MM`, `:SS`, `.mmm`,
/// trailing `Z`. Returns `None` for anything else — including
/// out-of-range components, so `2026-02-30` is refused, not wrapped.
pub fn parse_iso_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let seg = s.get(r)?;
        if seg.is_empty() || !seg.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        seg.parse().ok()
    };
    let y = num(0..4)?;
    let m = num(5..7)? as u32;
    let d = num(8..10)? as u32;
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    let mut rest = &s[10..];
    if let Some(r) = rest.strip_suffix('Z').or_else(|| rest.strip_suffix('z')) {
        rest = r;
    }
    let (mut hh, mut mi, mut ss, mut ms) = (0i64, 0i64, 0i64, 0i64);
    if !rest.is_empty() {
        let rest = rest
            .strip_prefix('T')
            .or_else(|| rest.strip_prefix('t'))
            .or_else(|| rest.strip_prefix(' '))?;
        let rb = rest.as_bytes();
        if rb.len() < 5 || rb[2] != b':' {
            return None;
        }
        let tnum = |seg: &str| -> Option<i64> {
            if seg.is_empty() || !seg.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            seg.parse().ok()
        };
        // Boundary-safe slicing: `.get(..)` returns None when the range
        // lands inside a multibyte char, where a raw `&rest[a..b]`
        // panics. The segments are supposed to be ASCII digits — tnum
        // rejects anything else — but the SLICE itself must not crash
        // on multibyte input first.
        hh = tnum(rest.get(0..2)?)?;
        mi = tnum(rest.get(3..5)?)?;
        let mut tail = rest.get(5..)?;
        if let Some(t) = tail.strip_prefix(':') {
            if t.len() < 2 {
                return None;
            }
            ss = tnum(t.get(0..2)?)?;
            tail = t.get(2..)?;
            if let Some(frac) = tail.strip_prefix('.') {
                if frac.is_empty() || frac.len() > 3 || !frac.bytes().all(|c| c.is_ascii_digit())
                {
                    return None;
                }
                let scale = 10i64.pow(3 - frac.len() as u32);
                ms = frac.parse::<i64>().ok()? * scale;
                tail = "";
            }
        }
        if !tail.is_empty() {
            return None;
        }
        if hh > 23 || mi > 59 || ss > 59 {
            return None;
        }
    }
    let days = days_from_civil(y, m, d);
    Some((((days * 24 + hh) * 60 + mi) * 60 + ss) * 1000 + ms)
}

/// Render epoch milliseconds as `YYYY-MM-DDTHH:MM:SSZ` (with `.mmm`
/// only when nonzero).
pub fn format_iso_ms(ms_epoch: i64) -> String {
    let days = ms_epoch.div_euclid(86_400_000);
    let mut rem = ms_epoch.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    let hh = rem / 3_600_000;
    rem %= 3_600_000;
    let mi = rem / 60_000;
    rem %= 60_000;
    let ss = rem / 1000;
    let ms = rem % 1000;
    if ms == 0 {
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mi:02}:{ss:02}Z")
    } else {
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mi:02}:{ss:02}.{ms:03}Z")
    }
}

/// Milliseconds since the Unix epoch, now. The `NOW` literal's clock.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_anchors() {
        assert_eq!(parse_iso_ms("1970-01-01"), Some(0));
        assert_eq!(parse_iso_ms("1970-01-02"), Some(86_400_000));
        // pre-epoch works (proleptic)
        assert_eq!(parse_iso_ms("1969-12-31"), Some(-86_400_000));
        // a known modern anchor: 2026-07-02T00:00:00Z
        assert_eq!(parse_iso_ms("2026-07-02"), Some(1_782_950_400_000));
    }

    #[test]
    fn accepted_forms() {
        let d = parse_iso_ms("2026-07-02").unwrap();
        assert_eq!(parse_iso_ms("2026-07-02T00:00").unwrap(), d);
        assert_eq!(parse_iso_ms("2026-07-02 00:00:00").unwrap(), d);
        assert_eq!(parse_iso_ms("2026-07-02T00:00:00Z").unwrap(), d);
        assert_eq!(
            parse_iso_ms("2026-07-02T14:30:05.250Z").unwrap(),
            d + ((14 * 60 + 30) * 60 + 5) * 1000 + 250
        );
        assert_eq!(parse_iso_ms("2026-07-02T14:30").unwrap(), d + (14 * 60 + 30) * 60_000);
    }

    #[test]
    fn refused_forms() {
        for bad in [
            "2026-02-30",        // no such day
            "2026-13-01",        // no such month
            "2026-07-02T24:00",  // no such hour
            "2026-07-02T14:61",  // no such minute
            "02-07-2026",        // wrong order
            "July 2, 2026",      // prose
            "2026-07-02TT12:00", // typo
            "",
            "now",
        ] {
            assert!(parse_iso_ms(bad).is_none(), "{bad:?} should be refused");
        }
    }

    /// Multibyte bytes where ASCII digits are expected must be refused
    /// (`None`), not panic. The old code byte-sliced the time part
    /// (`&rest[3..5]`, `&t[0..2]`), so a multibyte char straddling a
    /// slice boundary panicked the write handler — e.g. '€' (3 bytes)
    /// in the minutes or seconds position.
    #[test]
    fn multibyte_time_parts_are_refused_not_panics() {
        for bad in [
            "2026-07-02T12:€",    // '€' spans the minutes slice → old panic
            "2026-07-02T12:€0",   // same, with a trailing digit
            "2026-07-02T12:34:€", // '€' spans the seconds slice → old panic
            "2026-07-02T12:34:€5",
            "2026-07-02 12:я5", // 2-byte char inside the minute pair
            "2026-07-02T🦀0:00", // emoji in the hour pair
            "2026-07-02Tяя:00",
        ] {
            assert!(parse_iso_ms(bad).is_none(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn round_trip() {
        for ms in [
            0i64,
            1_782_950_400_000,
            1_782_950_400_000 + 52_205_250,
            -86_400_000,
            253_402_300_799_000, // 9999-12-31T23:59:59
        ] {
            let rendered = format_iso_ms(ms);
            assert_eq!(
                parse_iso_ms(&rendered),
                Some(ms),
                "round trip failed for {ms} -> {rendered}"
            );
        }
        assert_eq!(format_iso_ms(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_iso_ms(1_782_950_400_000), "2026-07-02T00:00:00Z");
    }
}
