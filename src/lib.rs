use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub mod board;
pub mod generator;
pub mod instrument;
pub mod solver;

/// Find all axis-aligned rectangles whose cell values sum to the target.
///
/// The fruitbox board uses small non-negative values, so this deterministic
/// primitive is a good fit for Rust and can later be reused from Wasm.
#[pyfunction]
fn find_sum_rectangles(
    cells: Vec<u8>,
    width: usize,
    target: u16,
) -> PyResult<Vec<(usize, usize, usize, usize)>> {
    if width == 0 {
        return Err(PyValueError::new_err("width must be greater than zero"));
    }
    if target == 0 {
        return Err(PyValueError::new_err("target must be greater than zero"));
    }
    board::find_sum_rectangles_core(&cells, width, target)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_sum_rectangles, m)?)?;
    Ok(())
}
