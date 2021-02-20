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

/// Deflate decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.deflate.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(_) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let mut cursor = Cursor::new(buffer);
                    to_py_err!(DecompressionError -> self::internal::decompress(data.as_bytes(), &mut cursor))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len());
                to_py_err!(DecompressionError -> self::internal::decompress(data.as_bytes(), &mut buffer))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(_) => {
            let mut pybytes = WriteablePyByteArray::new(py, output_len.unwrap_or_else(|| data.len()));
            to_py_err!(DecompressionError -> self::internal::decompress(data.as_bytes(), &mut pybytes))?;
            Ok(BytesType::ByteArray(pybytes.into_inner()?))
        }
    }
}

/// Deflate compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.deflate.compress(b'some bytes here', level=5, output_len=Optional[int])  # level defaults to 6
/// ```
#[pyfunction]
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(_) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let mut cursor = Cursor::new(buffer);
                    to_py_err!(CompressionError -> self::internal::compress(data.as_bytes(), &mut cursor, level))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::new();
                to_py_err!(CompressionError -> self::internal::compress(data.as_bytes(), &mut buffer, level))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(_) => {
            let mut pybytes = WriteablePyByteArray::new(py, output_len.unwrap_or_else(|| 0));
            to_py_err!(CompressionError -> self::internal::compress(data.as_bytes(), &mut pybytes, level))?;
            Ok(BytesType::ByteArray(pybytes.into_inner()?))
        }
    }
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(
    _py: Python<'a>,
    data: BytesType<'a>,
    array: &PyArray1<u8>,
    level: Option<u32>,
) -> PyResult<usize> {
    crate::de_comp_into!(compress(data -> array), level)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::de_comp_into!(decompress(data -> array))
}

pub(crate) mod internal {

    use flate2::read::{DeflateDecoder, DeflateEncoder};
    use flate2::Compression;
    use std::io::prelude::*;
    use std::io::Error;

    /// Decompress gzip data
    pub fn decompress<W: Write + ?Sized>(input: &[u8], output: &mut W) -> Result<usize, Error> {
        let mut decoder = DeflateDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress gzip data
    pub fn compress<W: Write + ?Sized>(input: &[u8], output: &mut W, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| 6);

        let mut encoder = DeflateEncoder::new(input, Compression::new(level));
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
