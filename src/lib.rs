use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

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
    if cells.is_empty() || cells.len() % width != 0 {
        return Err(PyValueError::new_err(
            "cells length must be a non-empty multiple of width",
        ));
    }

    let height = cells.len() / width;
    let mut rectangles = Vec::new();

    for top in 0..height {
        let mut column_sums = vec![0_u16; width];

        for bottom in top..height {
            for x in 0..width {
                column_sums[x] += cells[bottom * width + x] as u16;
            }

            for left in 0..width {
                let mut sum = 0_u16;

                for right in left..width {
                    sum += column_sums[right];

                    if sum == target {
                        rectangles.push((left, top, right, bottom));
                    }
                    if sum > target {
                        break;
                    }
                }
            }
        }
    }

    Ok(rectangles)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_sum_rectangles, m)?)?;
    Ok(())
}
