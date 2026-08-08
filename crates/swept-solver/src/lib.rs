//! Manoeuvre search: can this vehicle get through this opening, and how.
//!
//! Three solvers live here. `exact` sweeps every one-move approach on a grid
//! and is therefore trustworthy in both directions: when it finds nothing,
//! nothing exists on that grid. `multi` plans up to four moves with a hybrid
//! A\*, and is trustworthy in one direction only — what it finds is verified,
//! what it fails to find may still exist. `min_road` bisects on carriageway
//! width.
//!
//! # No clock
//!
//! Nothing here reads wall-clock time. Budgets are counted in expanded nodes,
//! so the same inputs always produce the same output. The prototype stopped
//! after 2.2 seconds per depth and returned whatever the machine had managed
//! to reach by then, which made its results impossible to assert in a test.

pub mod budget;
pub mod exact;
pub mod min_road;
pub mod path;
pub mod result;
