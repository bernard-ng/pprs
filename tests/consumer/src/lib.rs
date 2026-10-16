//! Compile-check the public library dependency from an independent Cargo package.

// Re-exporting the crate requires a resolvable library dependency, even before
// parser APIs exist. Replace this with real consumer scenarios as APIs arrive.
pub use pprs;
