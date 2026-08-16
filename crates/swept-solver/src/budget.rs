//! What bounds a search, and how it reports on itself.
//!
//! Budgets are counted in expanded nodes, never in elapsed time. That is what
//! makes a search reproducible: the prototype gave up after 2.2 seconds per
//! depth, so its answer depended on the machine it ran on and could not be
//! asserted in a test.

use swept_core::units::Radians;

/// How finely the planner chops space and motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Discretisation {
    /// Length of one motion primitive, in metres. Shorter primitives find
    /// tighter solutions and cost more nodes.
    pub primitive_length: f64,
    /// Heading resolution of the visited-state grid.
    pub heading_step: Radians,
    /// Position resolution of the visited-state grid, in metres.
    pub position_step: f64,
    /// Spacing between poses sampled along a segment for collision checks, in
    /// metres.
    pub sample_step: f64,
}

impl Default for Discretisation {
    /// The grid that actually performs, measured against the alternative.
    ///
    /// `CLAUDE.md` called for 20 cm primitives and a 1° heading step, expecting
    /// finer search to find the centimetre-margin solutions the prototype
    /// missed. Measurement says otherwise: see [`Discretisation::fine`]. Until
    /// progressive refinement exists, the planner keeps the prototype's
    /// resolution, which finds better plans for a fiftieth of the nodes.
    ///
    /// One departure: collision sampling is stepped at 8 cm rather than the
    /// prototype's 18 cm. That does not drive search cost the way the other
    /// three do, and 18 cm is coarse enough to step over a thin obstacle.
    fn default() -> Self {
        Self {
            primitive_length: 0.90,
            heading_step: Radians::from_degrees(6.0),
            position_step: 0.18,
            sample_step: 0.08,
        }
    }
}

impl Discretisation {
    /// The fine grid `CLAUDE.md` asked for: 20 cm primitives, 1° of heading.
    ///
    /// **Not the default, on measurement.** Running
    /// `cargo run -p swept-solver --release --example grid_cost` shows it
    /// costs tens of times more nodes and returns *worse* plans: about 1 mm of
    /// clearance where the default finds 12 to 55 mm.
    ///
    /// The cause is not the resolution but the cost function. Nothing in
    /// `moves × 5 + distance × 0.18 + heuristic` rewards clearance, so the
    /// planner takes the shortest path, which means shaving obstacles. Shorter
    /// primitives simply let it shave more finely — the coarse grid is
    /// protected by its own clumsiness, not by any virtue.
    ///
    /// Measured: on a 4 m opening the tightest point of a fine-grid plan sits
    /// at pose 14 of 327, at `(-5.90, -2.47)` — grazing the curb six metres
    /// short of the gateway, nowhere near the opening.
    ///
    /// The fix is therefore not a bigger budget but a cost function that
    /// values room, or a filter on plans that shave. Progressive refinement
    /// would help with the node count, not with this. Kept here so the
    /// comparison stays reproducible.
    ///
    /// The position step is not simply scaled down with the rest. It has to
    /// stay well finer than one primitive, or two states a whole move apart
    /// land in the same cell and the planner drops one of them.
    #[must_use]
    pub fn fine() -> Self {
        Self {
            primitive_length: 0.20,
            heading_step: Radians::from_degrees(1.0),
            position_step: 0.06,
            sample_step: 0.04,
        }
    }
}

/// Node ceiling for one planning depth.
///
/// MEASURED, unlike most constants here. On the default grid the hardest scene
/// measured — a 2.20 m opening planned to four moves — settles after about
/// 29 500 nodes. This ceiling is twice that, leaving room for scenes harder
/// than any tried.
///
/// Reproduce with `cargo run -p swept-solver --release --example grid_cost`.
pub const DEFAULT_MAX_NODES: u32 = 60_000;

/// How many landing solutions are collected before a depth stops early.
///
/// MEASURED. The prototype stopped at 14 (`index.html:496`), which is enough
/// at its own resolution but throttles anything finer: on a 4 m opening,
/// raising it to 200 turned a 1 mm plan in two moves into a 66 mm plan in one.
/// The extra candidates cost little, since reaching the landing zone is what
/// is expensive, not testing another landing once there.
pub const DEFAULT_MAX_SOLUTIONS: u16 = 200;

/// What bounds one planning run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchBudget {
    /// Largest number of nodes the planner may expand per depth.
    pub max_nodes: u32,
    /// How many landings to collect before settling for the best so far.
    pub max_solutions: u16,
    /// How finely to chop space and motion.
    pub discretisation: Discretisation,
}

impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_NODES,
            max_solutions: DEFAULT_MAX_SOLUTIONS,
            discretisation: Discretisation::default(),
        }
    }
}

/// Receives progress reports from a running search.
///
/// This is an output port: the solver knows nothing about workers or
/// messages, it only says how far it has got.
pub trait Progress {
    /// Called periodically with the depth being searched and the number of
    /// nodes expanded so far.
    fn nodes_expanded(&mut self, moves: u8, expanded: u32);
}

/// A [`Progress`] that discards everything, for callers that do not care.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Progress for Silent {
    fn nodes_expanded(&mut self, _moves: u8, _expanded: u32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fine_grid_really_is_finer() {
        let fine = Discretisation::fine();
        let coarse = Discretisation::default();
        assert!(fine.primitive_length < coarse.primitive_length);
        assert!(fine.heading_step.get() < coarse.heading_step.get());
        assert!(fine.position_step < coarse.position_step);
    }

    #[test]
    fn every_visit_grid_stays_finer_than_a_whole_primitive() {
        // Otherwise two states a full move apart collapse into one cell and
        // the planner discards one of them.
        for d in [Discretisation::default(), Discretisation::fine()] {
            assert!(
                d.position_step * 2.0 < d.primitive_length,
                "position step {} is too coarse for primitives of {}",
                d.position_step,
                d.primitive_length
            );
        }
    }

    #[test]
    fn collision_sampling_never_steps_over_a_post() {
        // A 0.55 m post must never fall between two sampled poses.
        for d in [Discretisation::default(), Discretisation::fine()] {
            assert!(d.sample_step < 0.55 / 2.0, "got {}", d.sample_step);
        }
    }

    #[test]
    fn silent_progress_accepts_reports_and_does_nothing() {
        let mut silent = Silent;
        silent.nodes_expanded(3, 12_000);
    }

    #[test]
    fn counting_progress_records_the_last_report() {
        #[derive(Default)]
        struct Counter {
            calls: u32,
            last: u32,
        }
        impl Progress for Counter {
            fn nodes_expanded(&mut self, _moves: u8, expanded: u32) {
                self.calls += 1;
                self.last = expanded;
            }
        }

        let mut counter = Counter::default();
        counter.nodes_expanded(2, 100);
        counter.nodes_expanded(2, 250);
        assert_eq!((counter.calls, counter.last), (2, 250));
    }
}
