//! `--budget` spec parser.
//!
//! iter-2 lands the parser and a `Display` impl; iter-3 wires it to
//! `prctl(PR_SET_AGENT_BUDGET_LIMITS)` once that wintermute prctl ships.
//!
//! Accepted form: comma-separated `key=value` pairs.
//!
//! ```text
//! wall=3600s,syscalls=1e7,write_bytes=10G,fork=1000
//! ```
//!
//! Keys: `wall` (seconds with optional `s`/`m`/`h`), `syscalls` (count with
//! `e<n>` scientific shorthand), `write_bytes` (with `K`/`M`/`G`/`T` IEC
//! multipliers), `fork` (count). Unknown keys are rejected so a typo doesn't
//! silently drop a limit.

use std::fmt;

/// Parsed budget. All fields optional; absent fields mean "no limit".
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BudgetSpec {
    /// Wall-clock limit in whole seconds.
    pub wall_secs: Option<u64>,
    /// Syscall count limit.
    pub syscalls: Option<u64>,
    /// Cumulative write-bytes limit.
    pub write_bytes: Option<u64>,
    /// Fork/clone count limit.
    pub fork: Option<u64>,
}

/// Reasons a `--budget` spec failed to parse.
#[derive(Debug)]
pub enum ParseError {
    /// The spec string was empty or whitespace.
    Empty,
    /// A comma-separated token was not in `key=value` form.
    BadPair(String),
    /// A key was not one of the four recognized budget dimensions.
    UnknownKey(String),
    /// A value parsed structurally but failed validation.
    BadValue {
        /// Key whose value was rejected.
        key: String,
        /// Raw value string from the spec.
        value: String,
        /// Human-readable reason for rejection.
        why: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "budget spec is empty"),
            Self::BadPair(s) => write!(f, "expected key=value, got {s:?}"),
            Self::UnknownKey(k) => write!(f, "unknown budget key {k:?} (want one of: wall, syscalls, write_bytes, fork)"),
            Self::BadValue { key, value, why } => {
                write!(f, "bad value for {key}={value:?}: {why}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a `--budget` spec.
///
/// # Errors
/// Returns [`ParseError`] on malformed input.
pub fn parse(spec: &str) -> Result<BudgetSpec, ParseError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut out = BudgetSpec::default();
    for pair in spec.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| ParseError::BadPair(pair.to_owned()))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "wall" => {
                out.wall_secs = Some(parse_duration(key, value)?);
            }
            "syscalls" => {
                out.syscalls = Some(parse_count(key, value)?);
            }
            "write_bytes" => {
                out.write_bytes = Some(parse_bytes(key, value)?);
            }
            "fork" => {
                out.fork = Some(parse_count(key, value)?);
            }
            other => return Err(ParseError::UnknownKey(other.to_owned())),
        }
    }
    Ok(out)
}

fn parse_count(key: &str, value: &str) -> Result<u64, ParseError> {
    if let Some((mantissa, exp)) = value.split_once('e') {
        let m: u64 = mantissa.parse().map_err(|e: std::num::ParseIntError| ParseError::BadValue {
            key: key.to_owned(),
            value: value.to_owned(),
            why: e.to_string(),
        })?;
        let e: u32 = exp.parse().map_err(|e: std::num::ParseIntError| ParseError::BadValue {
            key: key.to_owned(),
            value: value.to_owned(),
            why: e.to_string(),
        })?;
        Ok(m.saturating_mul(10u64.saturating_pow(e)))
    } else {
        value.parse().map_err(|e: std::num::ParseIntError| ParseError::BadValue {
            key: key.to_owned(),
            value: value.to_owned(),
            why: e.to_string(),
        })
    }
}

fn parse_duration(key: &str, value: &str) -> Result<u64, ParseError> {
    let (num, mul): (&str, u64) = if let Some(s) = value.strip_suffix("ms") {
        (s, 0) // ms unsupported; surface as bad value below
    } else if let Some(s) = value.strip_suffix('s') {
        (s, 1)
    } else if let Some(s) = value.strip_suffix('m') {
        (s, 60)
    } else if let Some(s) = value.strip_suffix('h') {
        (s, 3600)
    } else {
        (value, 1) // bare number → seconds
    };
    if mul == 0 {
        return Err(ParseError::BadValue {
            key: key.to_owned(),
            value: value.to_owned(),
            why: "millisecond budgets unsupported; use whole seconds".to_owned(),
        });
    }
    let n: u64 = num.parse().map_err(|e: std::num::ParseIntError| ParseError::BadValue {
        key: key.to_owned(),
        value: value.to_owned(),
        why: e.to_string(),
    })?;
    Ok(n.saturating_mul(mul))
}

fn parse_bytes(key: &str, value: &str) -> Result<u64, ParseError> {
    let (num, mul): (&str, u64) = if let Some(s) = value.strip_suffix('T') {
        (s, 1024u64.pow(4))
    } else if let Some(s) = value.strip_suffix('G') {
        (s, 1024u64.pow(3))
    } else if let Some(s) = value.strip_suffix('M') {
        (s, 1024u64.pow(2))
    } else if let Some(s) = value.strip_suffix('K') {
        (s, 1024)
    } else {
        (value, 1)
    };
    let n: u64 = num.parse().map_err(|e: std::num::ParseIntError| ParseError::BadValue {
        key: key.to_owned(),
        value: value.to_owned(),
        why: e.to_string(),
    })?;
    Ok(n.saturating_mul(mul))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_spec() {
        let b = parse("wall=3600s,syscalls=1e7,write_bytes=10G,fork=1000").unwrap();
        assert_eq!(b.wall_secs, Some(3600));
        assert_eq!(b.syscalls, Some(10_000_000));
        assert_eq!(b.write_bytes, Some(10 * 1024u64.pow(3)));
        assert_eq!(b.fork, Some(1000));
    }

    #[test]
    fn unknown_key_rejected() {
        assert!(matches!(parse("ramen=1"), Err(ParseError::UnknownKey(_))));
    }

    #[test]
    fn empty_spec_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)));
    }

    #[test]
    fn ms_duration_rejected() {
        assert!(matches!(parse("wall=500ms"), Err(ParseError::BadValue { .. })));
    }
}
