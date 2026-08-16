# swept-path

Will this car get through that gate, and how many shunts will it take?

**[swept-path-steel.vercel.app](https://swept-path-steel.vercel.app/)**

A swept path simulator for narrow openings — a driveway gate, a courtyard
entrance, a parking box. You give it the opening and the vehicle; it returns a
drivable trajectory, the clearance left at the tightest point, and **where that
figure came from**.

*Swept path* is the English term of art; in French the field is called *épure
de giration*. The interface reads in either, and measures in metres or in US
units.

## What it answers

- Can this vehicle get in **at all**, and in how many moves?
- How much room is left at the tightest moment — and is that moment *in the
  gateway*, or against a curb six metres short of it? The two mean very
  different things to a driver, so both are reported.
- How wide would the street have to be for a single-move entry?
- Is the answer **proved or merely found**? An exhaustive sweep that returns
  nothing has established something; a heuristic planner that returns nothing
  has not. Every result carries which one produced it, and the interface never
  shows a clearance without it.

The one conclusion worth knowing before opening it: **width decides
everything**. Whatever the trajectory, the best clearance on each side is
`(opening − vehicle) / 2`. Extra manoeuvres buy none of it.

## How it works

Explained in the app, with figures drawn by the same renderer as the plan:
**[How it works](https://swept-path-steel.vercel.app/documentation.html)** and
**[Disclaimer](https://swept-path-steel.vercel.app/disclaimer.html)**.

In short: the vehicle follows a kinematic bicycle model, a state being the pose
of the rear axle. Three solvers run in turn — an exhaustive sweep of Dubins and
Reeds-Shepp curves, a hybrid A\* planner for multi-move entries, and a
bisection for the narrowest usable street. Clearance, not length, is what gets
maximised: the shortest path is the one that grazes most. Everything runs in a
Web Worker, and budgets are counted in expanded nodes rather than milliseconds,
so the same inputs always produce the same answer on any machine.

## Running it

Requires the Rust toolchain pinned in `rust-toolchain.toml`, with the
`wasm32-unknown-unknown` target, plus [`wasm-pack`], [`just`] and Node 24.

```sh
just install     # web dependencies
just dev         # rebuilds the wasm package, then serves the app
just ci          # everything CI runs
```

`just` on its own lists every recipe.

[`wasm-pack`]: https://rustwasm.github.io/wasm-pack/
[`just`]: https://just.systems/

## Layout

| Path | What lives there |
|---|---|
| `crates/swept-core` | Geometry, kinematics, clearance, Dubins and Reeds-Shepp. **Zero production dependencies.** |
| `crates/swept-solver` | The three solvers, and the confidence attached to each result |
| `crates/swept-wasm` | The WebAssembly boundary: a scene in, poses out |
| `web/` | TypeScript interface, SVG rendering, Web Worker, Tailwind |
| `data/vehicles.json` | Vehicle database, provenance recorded per field |

Code, identifiers, rustdoc, test names and commit messages are in English; the
interface reads in French or English. Inside the core, lengths are metres and
angles radians without exception — conversion happens once, where the form is
read.

## What it does not model

Stated plainly, because a confident wrong answer is worse than no answer. The
scene is assumed symmetric about the opening, so the two posts cannot yet be
placed independently. Slope, camber, suspension travel and parked vehicles are
absent. And most vehicle figures are not measured: in `data/vehicles.json`,
width over the mirrors is often `derived` and front overhang `estimated`, where
three centimetres of error flips a conclusion — which is why provenance is
recorded field by field.

## Licence

The application — `swept-solver`, `swept-wasm`, the web interface — is
**AGPL-3.0-only**, in [`LICENSE`](LICENSE).

The geometric core, `swept-core`, is **MIT OR Apache-2.0**, in
[`crates/swept-core/LICENSE-MIT`](crates/swept-core/LICENSE-MIT) and
[`LICENSE-APACHE`](crates/swept-core/LICENSE-APACHE), so that it stays reusable
on its own. Keeping it free of production dependencies is what makes that
possible.

The vehicle data in `data/vehicles.json` is licensed separately from the code,
and comes from manufacturer documents only — never from an aggregator, whose
database rights would follow it.
