//! The WebAssembly boundary.
//!
//! Three functions, JSON in and JSON out. No rule lives here: this layer
//! converts, calls the solvers, and converts back. Anything that decides
//! something belongs in `swept-core` or `swept-solver`.

pub mod dto;

use dto::{ErrorDto, SceneDto, SolveRequest};
use swept_solver::budget::Progress;
use wasm_bindgen::prelude::*;

/// Relays planner progress to a JavaScript callback.
///
/// A callback that throws is ignored: an observer must never be able to fail
/// a search it is only watching.
struct JsProgress<'a> {
    callback: &'a js_sys::Function,
}

impl Progress for JsProgress<'_> {
    fn nodes_expanded(&mut self, moves: u8, expanded: u32) {
        let _ = self.callback.call2(
            &JsValue::NULL,
            &JsValue::from(moves),
            &JsValue::from(expanded),
        );
    }
}

/// Installs a panic hook reporting to the console, in debug builds only.
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Finds every way in, one alternative per move count.
///
/// `on_progress` is called as `(moves, expanded)` while the planner works.
/// It may be omitted, and usually is outside the interface. Note that the
/// exhaustive sweep runs first and reports nothing: **the first call is the
/// signal that the sweep has finished** and planning has begun.
///
/// # Errors
///
/// Returns a serialised [`ErrorDto`] when the request cannot be decoded or
/// the vehicle dimensions are rejected.
#[wasm_bindgen]
pub fn solve(request: JsValue, on_progress: Option<js_sys::Function>) -> Result<JsValue, JsValue> {
    let request: SolveRequest =
        serde_wasm_bindgen::from_value(request).map_err(|e| decode_error(&e))?;
    let response = match on_progress {
        Some(callback) => dto::run_solve_reporting(
            request,
            &mut JsProgress {
                callback: &callback,
            },
        ),
        None => dto::run_solve(request),
    }
    .map_err(|e| domain_error(&e))?;
    serde_wasm_bindgen::to_value(&response).map_err(|e| encode_error(&e))
}

/// The narrowest carriageway admitting a one-move forward entry, in metres.
///
/// Returns `null` when no width up to the search ceiling works, which means
/// the opening itself is blocking rather than the road.
///
/// # Errors
///
/// Returns a serialised [`ErrorDto`] when the request cannot be decoded or
/// the vehicle dimensions are rejected.
#[wasm_bindgen]
pub fn min_road(request: JsValue) -> Result<JsValue, JsValue> {
    let request: SolveRequest =
        serde_wasm_bindgen::from_value(request).map_err(|e| decode_error(&e))?;
    let vehicle = request
        .vehicle
        .into_domain()
        .map_err(|e| domain_error(&e))?;
    let scene = request.scene.into_domain();
    let width = swept_solver::min_road::minimum_road_width(&vehicle, &scene);
    serde_wasm_bindgen::to_value(&width).map_err(|e| encode_error(&e))
}

/// The widest angle the leaves can open to without fouling their posts.
///
/// In **radians**, like everything crossing this boundary.
///
/// # Errors
///
/// Returns a serialised [`ErrorDto`] when the scene cannot be decoded.
#[wasm_bindgen]
pub fn max_gate_angle(scene: JsValue) -> Result<f64, JsValue> {
    let scene: SceneDto = serde_wasm_bindgen::from_value(scene).map_err(|e| decode_error(&e))?;
    Ok(scene.into_domain().max_open_angle().get())
}

/// Turns a decoding failure into an [`ErrorDto`] the interface can translate.
fn decode_error(error: &serde_wasm_bindgen::Error) -> JsValue {
    serde_wasm_bindgen::to_value(&ErrorDto {
        code: String::from("bad_request"),
        field: None,
    })
    .unwrap_or_else(|_| JsValue::from_str(&error.to_string()))
}

/// Same, for a rejected set of dimensions.
fn domain_error(error: &ErrorDto) -> JsValue {
    serde_wasm_bindgen::to_value(error).unwrap_or_else(|_| JsValue::from_str("bad_request"))
}

/// Same, for an encoding failure on the way out.
fn encode_error(error: &serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}
