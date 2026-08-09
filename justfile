# Local shortcuts. Run `just` on its own to list them.
#
# These mirror .github/workflows/ci.yml on purpose: `just ci` runs what CI
# runs, so a green `just ci` should mean a green pipeline. Change one, change
# the other — a justfile that quietly drifts from CI is worse than none.

set shell := ["bash", "-uc"]

# nvm defines `node` and `npm` as lazy shell functions that only resolve in
# interactive shells, so recipes reach the binaries through PATH instead.
node_bin := `ls -d "$HOME"/.nvm/versions/node/*/bin 2>/dev/null | sort -V | tail -1 || true`
export PATH := if node_bin != "" { node_bin + ":" + env_var("PATH") } else { env_var("PATH") }

_default:
    @just --list --unsorted

# ---------------------------------------------------------------- tests ---

# Rust and web tests: the short loop.
test: test-rust test-web

# The test profile is optimised: seconds, not the two minutes an unoptimised
# build of these numeric loops would take.

# Rust tests.
test-rust:
    cargo test --workspace

# Web tests.
test-web:
    cd web && npx vitest run

# One test by name, with its output: just test-one clearance
test-one name:
    cargo test --workspace {{ name }} -- --nocapture

# -------------------------------------------------------------- quality ---

# Everything CI checks, deployment aside.
check: check-rust check-web

check-rust:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

check-web:
    cd web && npx tsc --noEmit

# Format, and apply the fixes clippy can make on its own.
fix:
    cargo fmt
    cargo clippy --all-targets --fix --allow-dirty --allow-staged

# ---------------------------------------------------------- wasm and web ---

# The output path is relative to the crate, which is why it climbs two levels.

# Build the wasm package the web app imports.
wasm:
    wasm-pack build --target web --out-dir ../../web/src/generated crates/swept-wasm

# Install web dependencies, exactly as CI does.
install:
    cd web && npm ci

# Rebuilds the wasm first: forgetting to fails with an obscure import error
# rather than a useful one.

# Serve the app locally.
dev: wasm
    cd web && npx vite

# Production bundle.
build: wasm
    cd web && npx vite build

# --------------------------------------------------------- measurements ---

# Regenerate the golden vectors from the frozen prototype.
fixtures:
    node tools/extract-golden/extract.js

# Check the committed fixtures still match the prototype, as CI does.
fixtures-check:
    node tools/extract-golden/extract.js --check

# What refining the planning grid costs, in nodes and in clearance.
grid-cost:
    cargo run -p swept-solver --release --example grid_cost

# Open the API documentation.
doc:
    cargo doc --workspace --no-deps --open

# -------------------------------------------------------- before pushing ---

# Everything CI runs. Green here should mean green there.
ci: check test fixtures-check build
