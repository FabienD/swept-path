//! Where the time goes, before deciding how to make it go faster.
use std::time::Instant;
use swept_core::clearance::ClearanceField;
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::vehicle::Vehicle;
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::exact::{Approach, Grid, search};
use swept_solver::solve::alternatives;

fn scene(o: f64) -> Scene {
    Scene {
        left_post: Post {
            inner_edge_x: -o / 2.0,
            width: 0.55,
            depth: 0.55,
        },
        right_post: Post {
            inner_edge_x: o / 2.0,
            width: 0.55,
            depth: 0.55,
        },
        wall_thickness: 0.30,
        pavement_width: 1.20,
        dropped_kerb_width: o + 0.8,
        road_width: 4.50,
        gate: GateKind::Sliding,
    }
}

/// Times the solvers and the inner loop they spend it in.
fn main() {
    let v = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).unwrap();
    let sc = scene(2.6);

    let t = Instant::now();
    let _ = search(&v, &sc, Approach::Forward, Grid::fine());
    println!(
        "exact fine      : {:>7.0} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );

    let t = Instant::now();
    let _ = search(&v, &sc, Approach::Forward, Grid::coarse());
    println!(
        "exact coarse    : {:>7.0} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );

    let t = Instant::now();
    let _ = alternatives(&v, &sc, SearchBudget::default(), &mut Silent, None);
    println!(
        "alternatives    : {:>7.0} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );

    // alternatives() runs plan() once per depth. Does each rerun the same work?
    for depth in [2u8, 3, 4] {
        let t = Instant::now();
        let _ =
            swept_solver::multi::plan(&v, &sc, depth, SearchBudget::default(), &mut Silent, None);
        println!(
            "plan(depth={depth})    : {:>7.0} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }

    // How many clearance evaluations does one fine sweep cost?
    let field = ClearanceField::new(&sc, &v);
    let pose =
        swept_core::kinematics::Pose::new(0.0, 3.0, swept_core::units::Radians::from_degrees(90.0));
    let n = 2_000_000;
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(field.at(std::hint::black_box(pose)));
    }
    let per = t.elapsed().as_secs_f64() / f64::from(n) * 1e9;
    println!("clearance::at   : {per:>7.0} ns  ({:.1} M/s)", 1000.0 / per);
}
