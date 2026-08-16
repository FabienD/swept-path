# swept-path

Will this car get through that gate, and how many shunts will it take?

A swept path simulator for narrow openings — a driveway gate, a courtyard
entrance, a parking box. You give it the opening and the vehicle; it returns a
drivable trajectory, the clearance left at the tightest point, and **where that
figure came from**.

*Swept path* is the English term of art; in French the field is called
*épure de giration*. The interface speaks either, and measures in metres or in
US units — see [Language](#language).

## What it answers

- Can this vehicle get in **at all**, and in how many moves?
- How much room is left at the tightest moment — and is that moment *in the
  gateway*, or against a kerb six metres short of it? The two mean very
  different things to a driver, so both are reported.
- How wide would the street have to be for a single-move entry?
- Is the answer **proved or merely found**? An exhaustive sweep that returns
  nothing has established something; a heuristic planner that returns nothing
  has not. Every result carries which one produced it, and the interface never
  shows a clearance without it.

## Status

Working today: the geometric core, the three solvers, the WebAssembly boundary
and a browser interface that draws the manoeuvre.

Not built yet: the usability envelope and its three verdicts (*does not fit* /
*fits but unusable* / *fits and usable*), obstacle heights, parking boxes, the
vehicle database beyond a handful of seed entries, and on the interface side
translation and a choice of metric or imperial units.

## Running it

Requires the Rust toolchain pinned in `rust-toolchain.toml` (1.97.1, with the
`wasm32-unknown-unknown` target), [`wasm-pack`], [`just`] and Node 24.

```sh
just install     # web dependencies
just dev         # rebuilds the wasm package, then serves the app
just ci          # everything CI runs — a green run here should mean a green pipeline
```

`just` on its own lists every recipe.

[`wasm-pack`]: https://rustwasm.github.io/wasm-pack/
[`just`]: https://just.systems/

## How it works

The vehicle follows a kinematic bicycle model; a state is the pose of the rear
axle, `(x, y, θ)`. The frame puts the origin at the middle of the opening, with
`y = 0` on the outer face of the wall and `y > 0` into the yard.

Three solvers, and each proves a different amount:

| Solver | Method | What a failure means |
|---|---|---|
| `exact` | Every Dubins curve between a swept grid of start and goal poses | No one-move entry exists **on that grid** |
| `multi` | Hybrid A\* over `(x, y, θ, gear)`, up to four moves | Nothing — what it finds is collision-checked, what it misses may still exist |
| `min_road` | Bisection on carriageway width, with `exact` as the predicate | Inherits `exact`'s guarantee |

The planner is always seeded from the exact search, which is what guarantees a
multi-move plan is never worse than the one-move answer.

Clearance, not length, is what gets maximised. Dubins curves minimise *length*,
and the shortest path is the one that grazes most — so the sweep enumerates all
of them, discards the ones that collide, and keeps the roomiest.

Computation runs in a Web Worker. Budgets are counted in expanded nodes, never
in milliseconds: the same inputs always produce the same output, on any machine.

[`docs/ALGORITHME.md`](docs/ALGORITHME.md) has the detail, in French, including
what was measured and what turned out to be wrong.

## What it does not model

Stated plainly, because a confident wrong answer is worse than no answer.

- **The model is pure 2D.** Every obstacle is a wall of infinite height, so a
  kerb stops a mirror that would sail a metre above it in reality. On a real
  measured gateway this is the difference between "no single-move entry exists"
  and a car that goes in every day. Obstacle heights are the next thing to
  build.
- **The scene is assumed symmetric about `x = 0`.** The two posts cannot yet be
  placed independently.
- **The vehicle figures are mostly not measured.** In `data/vehicles.json`,
  width over the mirrors is `derived` and front overhang `estimated` for almost
  every model. Three centimetres of error there flips a conclusion, so
  provenance is recorded field by field and shown as such.

## Repository layout

| Path | What lives there |
|---|---|
| `crates/swept-core` | Geometry, kinematics, clearance, Dubins curves. **Zero production dependencies.** |
| `crates/swept-solver` | The three solvers, and the confidence attached to each result |
| `crates/swept-wasm` | The WebAssembly boundary: a scene in, poses out |
| `web/` | TypeScript interface, SVG rendering, Web Worker, Tailwind |
| `data/vehicles.json` | Vehicle database, provenance per field |
| `prototype/index.html` | The original single-file prototype, kept as an oracle |
| `tools/extract-golden` | Extracts golden vectors from the prototype; CI checks the port still matches to 1e-12 |
| `docs/` | Functional spec and algorithm notes, in French |

## Language

Code, identifiers, rustdoc, test names and commit messages are in English. The
notes under `docs/` are in French, because the domain vocabulary — *épure de
giration*, *vantail*, *bateau* — is where the precision lives for the people
this was first built for.

The interface reads in French or English, chosen in the page and remembered in
the browser. That was cheap because no French had leaked into the domain layer,
which returns codes rather than sentences: the whole translation lives in
`web/src/i18n/` and `web/src/domain/labels.ts`.

Units are a separate setting, and metric by default for everyone. The two are
independent on purpose — reading in English says nothing about the tape someone
measured their gate with. In US units, what a length *is* decides how it is
shown: inches for the vehicle and the margins, as a manufacturer's sheet gives
them, feet for the roadway and the trip. One unit throughout would report a
5,90 m street as 232 in, which is arithmetically right and unreadable.

Inside the core, lengths are metres and angles are radians, without exception —
degrees appear only on screen. Conversion happens at the single boundary every
measurement crosses, when the form is read, for the same reason: a solver that
has to ask which units it is in is a solver that will one day get it wrong.

## Licence

The application — `swept-solver`, `swept-wasm`, the web interface — is
**AGPL-3.0-only**, in [`LICENSE`](LICENSE).

The geometric core, `swept-core`, is **MIT OR Apache-2.0**, in
[`crates/swept-core/LICENSE-MIT`](crates/swept-core/LICENSE-MIT) and
[`LICENSE-APACHE`](crates/swept-core/LICENSE-APACHE), so that it stays reusable
on its own. Keeping it free of production dependencies is what makes that
possible.

Vehicle data is licensed separately and is **not** covered by either. Figures
come from manufacturer documents only, never from an aggregator: a compiled
database attracts its own *sui generis* right in the EU (Directive 96/9/EC),
independently of the copyright on its contents.
