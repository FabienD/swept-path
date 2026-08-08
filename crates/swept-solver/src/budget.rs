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
    /// The target grid from `CLAUDE.md`: 20 cm primitives and a 1° heading
    /// step.
    ///
    /// The position step is not simply scaled down with the rest. It has to
    /// stay well finer than one primitive, or two states a whole move apart
    /// land in the same cell and the planner drops one of them. The prototype
    /// held that ratio at a fifth; a third is kept here, which is more
    /// cautious.
    fn default() -> Self {
        Self {
            primitive_length: 0.20,
            heading_step: Radians::from_degrees(1.0),
            position_step: 0.06,
            sample_step: 0.04,
        }
    }
}

impl Discretisation {
    /// The prototype's own grid, kept so that the cost of refining it can be
    /// measured rather than guessed.
    #[must_use]
    pub fn prototype() -> Self {
        Self {
            primitive_length: 0.90,
            heading_step: Radians::from_degrees(6.0),
            position_step: 0.18,
            sample_step: 0.18,
        }
    }
}

/// Node ceiling for one planning depth.
///
/// ARBITRARY — carried over from the prototype (`index.html:809`), which
/// allowed 18 000 nodes per depth alongside a 2.2 second deadline. Only the
/// node count survives.
pub const DEFAULT_MAX_NODES: u32 = 18_000;

/// How many landing solutions are collected before a depth stops early.
///
/// ARBITRARY — carried over from the prototype (`index.html:496`).
pub const DEFAULT_MAX_SOLUTIONS: u16 = 14;

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
    fn the_default_grid_is_finer_than_the_prototype() {
        let now = Discretisation::default();
        let before = Discretisation::prototype();
        assert!(now.primitive_length < before.primitive_length);
        assert!(now.heading_step.get() < before.heading_step.get());
        assert!(now.position_step < before.position_step);
    }

    #[test]
    fn the_visit_grid_stays_finer_than_a_whole_primitive() {
        // Otherwise two states a full move apart collapse into one cell and
        // the planner discards one of them.
        let d = Discretisation::default();
        assert!(
            d.position_step * 2.0 < d.primitive_length,
            "position step {} is too coarse for primitives of {}",
            d.position_step,
            d.primitive_length
        );
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
