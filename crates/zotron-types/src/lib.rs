//! Shared Zotron wire and CLI output types.
//!
//! Split into focused modules; the crate-root API is preserved via glob
//! re-exports so consumers keep importing `zotron_types::X`.

mod wire;
mod providers;
mod evidence;
mod chunking;
mod retrieval;
mod artifacts;

pub use wire::*;
pub use providers::*;
pub use evidence::*;
pub use chunking::*;
pub use retrieval::*;
pub use artifacts::*;
