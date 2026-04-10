use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    #[default]
    Unknown,
}

impl std::str::FromStr for Severity {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "TRACE" | "TRC" => Severity::Trace,
            "DEBUG" | "DBG" => Severity::Debug,
            "INFO" | "INF" => Severity::Info,
            "WARN" | "WARNING" | "WRN" => Severity::Warn,
            "ERROR" | "ERR" => Severity::Error,
            "FATAL" | "CRITICAL" | "CRIT" => Severity::Fatal,
            _ => Severity::Unknown,
        })
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Trace => write!(f, "TRACE"),
            Severity::Debug => write!(f, "DEBUG"),
            Severity::Info => write!(f, "INFO"),
            Severity::Warn => write!(f, "WARN"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Fatal => write!(f, "FATAL"),
            Severity::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_from_str() {
        use std::str::FromStr;
        assert_eq!(Severity::from_str("ERROR").unwrap(), Severity::Error);
        assert_eq!(Severity::from_str("error").unwrap(), Severity::Error);
        assert_eq!(Severity::from_str("ERR").unwrap(), Severity::Error);
        assert_eq!(Severity::from_str("FATAL").unwrap(), Severity::Fatal);
        assert_eq!(Severity::from_str("unknown").unwrap(), Severity::Unknown);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Error > Severity::Warn);
        assert!(Severity::Fatal > Severity::Error);
        assert!(Severity::Debug < Severity::Info);
    }
}
