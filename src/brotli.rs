use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType, WriteablePyByteArray};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};
use std::io::Cursor;

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    Ok(())
}

/// Brotli decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.brotli.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress(data), py = py, output_len = output_len)
}

/// Brotli compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.brotli.compress(b'some bytes here', level=9, output_len=Option[int])  # level defaults to 11
/// ```
#[pyfunction]
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    crate::generic!(compress(data), py = py, output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(
    _py: Python<'a>,
    data: BytesType<'a>,
    array: &PyArray1<u8>,
    level: Option<u32>,
) -> PyResult<usize> {
    crate::generic_into!(compress(data -> array), level)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(decompress(data -> array))
}

pub(crate) mod internal {

    use brotli2::read::{BrotliDecoder, BrotliEncoder};
    use std::io::prelude::*;
    use std::io::Error;

    /// Decompress via Brotli
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = BrotliDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress via Brotli
    pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| 11);
        let mut encoder = BrotliEncoder::new(input, level);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
