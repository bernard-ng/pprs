//! Explicit target versions and finite parser budgets, independent of host PHP.

use std::{fmt, str::FromStr};

/// Supported interpretation targets. This is not a strict language-validity mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhpVersion {
    /// PHP 7.0.
    Php70,
    /// PHP 7.1.
    Php71,
    /// PHP 7.2.
    Php72,
    /// PHP 7.3.
    Php73,
    /// PHP 7.4.
    Php74,
    /// PHP 8.0.
    Php80,
    /// PHP 8.1.
    Php81,
    /// PHP 8.2.
    Php82,
    /// PHP 8.3.
    Php83,
    /// PHP 8.4, the fixed default.
    #[default]
    Php84,
}

impl PhpVersion {
    /// All ten supported targets in increasing order.
    pub const ALL: [Self; 10] = [
        Self::Php70,
        Self::Php71,
        Self::Php72,
        Self::Php73,
        Self::Php74,
        Self::Php80,
        Self::Php81,
        Self::Php82,
        Self::Php83,
        Self::Php84,
    ];
    /// Exact major/minor spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Php70 => "7.0",
            Self::Php71 => "7.1",
            Self::Php72 => "7.2",
            Self::Php73 => "7.3",
            Self::Php74 => "7.4",
            Self::Php80 => "8.0",
            Self::Php81 => "8.1",
            Self::Php82 => "8.2",
            Self::Php83 => "8.3",
            Self::Php84 => "8.4",
        }
    }
}

/// An unsupported or non-canonical version string; no rounding is performed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unsupported PHP target {0:?}; expected 7.0–7.4 or 8.0–8.4")]
pub struct UnsupportedVersion(pub String);

impl FromStr for PhpVersion {
    type Err = UnsupportedVersion;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|version| version.as_str() == value)
            .ok_or_else(|| UnsupportedVersion(value.to_owned()))
    }
}
impl fmt::Display for PhpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The resource whose finite budget was exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Resource {
    /// Input byte size, checked before copying/indexing caller input.
    #[error("input bytes")]
    InputBytes,
    /// Recursive grammar or AST depth. The parser additionally caps effective depth at 128.
    #[error("nesting depth")]
    Depth,
    /// Retained tokens, including trivia, EOF and abort bookkeeping.
    #[error("tokens")]
    Tokens,
    /// Constructed AST nodes, including recovery nodes.
    #[error("AST nodes")]
    Nodes,
    /// Collected diagnostics, including a reserved final limit diagnostic.
    #[error("diagnostics")]
    Diagnostics,
}

/// Invalid limit configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{resource} limit {value} is below minimum {minimum}")]
pub struct InvalidLimit {
    /// Affected resource.
    pub resource: Resource,
    /// Requested value.
    pub value: usize,
    /// Minimum accepted value.
    pub minimum: usize,
}

/// Validated finite budgets. Defaults are provisional until P12 stress testing.
///
/// Token limits reserve two slots for an unscanned suffix and EOF. Diagnostic
/// limits reserve one slot for the terminating resource-limit diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    input_bytes: usize,
    depth: usize,
    tokens: usize,
    nodes: usize,
    diagnostics: usize,
}

impl Limits {
    /// Validate all limits. Zero input bytes is useful for empty-input-only use.
    ///
    /// # Errors
    /// Depth/nodes/diagnostics must be at least one; tokens at least two.
    pub fn new(
        input_bytes: usize,
        depth: usize,
        tokens: usize,
        nodes: usize,
        diagnostics: usize,
    ) -> Result<Self, InvalidLimit> {
        for (resource, value, minimum) in [
            (Resource::Depth, depth, 1),
            (Resource::Tokens, tokens, 2),
            (Resource::Nodes, nodes, 1),
            (Resource::Diagnostics, diagnostics, 1),
        ] {
            if value < minimum {
                return Err(InvalidLimit {
                    resource,
                    value,
                    minimum,
                });
            }
        }
        Ok(Self {
            input_bytes,
            depth,
            tokens,
            nodes,
            diagnostics,
        })
    }
    /// Get a resource's configured bound.
    #[must_use]
    pub const fn get(self, resource: Resource) -> usize {
        match resource {
            Resource::InputBytes => self.input_bytes,
            Resource::Depth => self.depth,
            Resource::Tokens => self.tokens,
            Resource::Nodes => self.nodes,
            Resource::Diagnostics => self.diagnostics,
        }
    }
    /// Check a proposed usage before doing the corresponding allocation/work.
    ///
    /// # Errors
    /// Returns the exhausted resource if usage exceeds its configured bound.
    pub fn check(self, resource: Resource, usage: usize) -> Result<(), Resource> {
        if usage <= self.get(resource) {
            Ok(())
        } else {
            Err(resource)
        }
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 16 * 1024 * 1024,
            depth: 128,
            tokens: 1_000_000,
            nodes: 250_000,
            diagnostics: 100,
        }
    }
}

/// Explicit parser configuration. All field types already enforce validity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    /// Best-effort interpretation target, default PHP 8.4.
    pub version: PhpVersion,
    /// Recognize `<?` open tags. Default false; `<?=` is always recognized.
    pub short_open_tag: bool,
    /// Finite resource budgets.
    pub limits: Limits,
}
