//! Filesystem core: no UI code lives here.

pub mod fs_ops;
pub mod model;
pub mod ops;
pub mod plan;
pub mod scan;

pub use model::{Entry, EntryState, Occupant, Package, PackageStatus};
pub use plan::{Action, Plan};
