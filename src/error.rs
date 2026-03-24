// lumindd — Error types and result aliases
// Copyright (c) 2026 Lumees Lab — Hasan Kurşun
// SPDX-License-Identifier: BSD-3-Clause

//! Error types for the lumindd decision diagram library.
//!
//! This module defines a unified error type [`DdError`] covering all failure
//! modes that can arise during DD construction, manipulation, and I/O, along
//! with a convenience alias [`DdResult<T>`].

use std::fmt;

/// Errors that can occur during DD operations.
#[derive(Clone, Debug)]
pub enum DdError {
    /// Node arena capacity exceeded.
    OutOfNodes {
        /// The maximum number of nodes that could be allocated.
        limit: usize,
    },

    /// An operation exceeded the caller-specified node-count limit.
    NodeLimitExceeded {
        /// The node limit that was exceeded.
        limit: u64,
    },

    /// An operation timed out before completing.
    Timeout,

    /// An invalid variable index was supplied.
    InvalidVariable {
        /// The variable index that was requested.
        var: u16,
        /// The total number of variables currently in the manager.
        num_vars: u16,
    },

    /// A serialization or deserialization error.
    IoError(String),

    /// A generic error with a descriptive message.
    Other(String),
}

impl fmt::Display for DdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DdError::OutOfNodes { limit } => {
                write!(f, "node arena capacity exceeded (limit: {})", limit)
            }
            DdError::NodeLimitExceeded { limit } => {
                write!(f, "operation exceeded node count limit of {}", limit)
            }
            DdError::Timeout => {
                write!(f, "operation timed out")
            }
            DdError::InvalidVariable { var, num_vars } => {
                write!(
                    f,
                    "invalid variable index {} (manager has {} variables)",
                    var, num_vars
                )
            }
            DdError::IoError(msg) => {
                write!(f, "I/O error: {}", msg)
            }
            DdError::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for DdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<std::io::Error> for DdError {
    fn from(err: std::io::Error) -> Self {
        DdError::IoError(err.to_string())
    }
}

impl From<String> for DdError {
    fn from(msg: String) -> Self {
        DdError::Other(msg)
    }
}

impl From<&str> for DdError {
    fn from(msg: &str) -> Self {
        DdError::Other(msg.to_owned())
    }
}

/// A convenience alias for `Result<T, DdError>`.
pub type DdResult<T> = Result<T, DdError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_out_of_nodes() {
        let e = DdError::OutOfNodes { limit: 1_000_000 };
        assert!(e.to_string().contains("1000000"));
    }

    #[test]
    fn display_invalid_variable() {
        let e = DdError::InvalidVariable {
            var: 42,
            num_vars: 10,
        };
        let s = e.to_string();
        assert!(s.contains("42"));
        assert!(s.contains("10"));
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let dd_err: DdError = io_err.into();
        match dd_err {
            DdError::IoError(msg) => assert!(msg.contains("file missing")),
            _ => panic!("expected IoError variant"),
        }
    }

    #[test]
    fn from_string() {
        let dd_err: DdError = "something went wrong".into();
        match dd_err {
            DdError::Other(msg) => assert_eq!(msg, "something went wrong"),
            _ => panic!("expected Other variant"),
        }
    }

    #[test]
    fn error_trait() {
        let e = DdError::Timeout;
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().contains("timed out"));
    }
}
