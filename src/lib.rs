//! CramJam documentation of python exported functions for (de)compression of bytes
//!
//! The API follows cramjam.`<<compression algorithm>>.compress` and cramjam.`<<compression algorithm>>.decompress`
//!
//! Python Example:
//!
//! ```python
//! data = b'some bytes here'
//! compressed = cramjam.snappy.compress(data)
//! decompressed = cramjam.snappy.decompress(compressed)
//! assert data == decompressed
//! ```

// TODO: There is a lot of very similar, but slightly different code for each variant
// time should be spent perhaps with a macro or other alternative.
// Each variant is similar, but sometimes has subtly different APIs/logic.

// TODO: Add output size estimation for each variant, now it's just snappy
// allow for resizing PyByteArray if over allocated; cannot resize PyBytes yet.

pub mod brotli;
pub mod deflate;
pub mod exceptions;
pub mod gzip;
pub mod lz4;
pub mod snappy;
pub mod zstd;

use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};

use exceptions::{CompressionError, DecompressionError};
use numpy::PyArray1;

#[derive(FromPyObject)]
pub enum BytesType<'a> {
    #[pyo3(transparent, annotation = "bytes")]
    Bytes(&'a PyBytes),
    #[pyo3(transparent, annotation = "bytearray")]
    ByteArray(&'a PyByteArray),
}

impl<'a> BytesType<'a> {
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
    fn as_bytes(&self) -> &'a [u8] {
        match self {
            Self::Bytes(b) => b.as_bytes(),
            Self::ByteArray(b) => unsafe { b.as_bytes() },
        }
    }
}

impl<'a> IntoPy<PyObject> for BytesType<'a> {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Bytes(bytes) => bytes.to_object(py),
            Self::ByteArray(byte_array) => byte_array.to_object(py),
        }
    }
}

/// Buffer to de/compression algorithms' output.
/// ::Vector used when the output len cannot be determined, and/or resulting
/// python object cannot be resized to what the actual bytes decoded was.
pub enum Output<'a> {
    Slice(&'a mut [u8]),
    Vector(&'a mut Vec<u8>),
}

/// Expose de/compression_into(data: BytesType<'_>, array: &PyArray1<u8>) -> PyResult<usize>
/// functions to allow de/compress bytes into a pre-allocated Python array.
///
/// This will handle gaining access to the Python's array as a buffer for an underlying de/compression
/// function which takes the normal `&[u8]` and `Output` types
pub fn de_compress_into<F>(data: &[u8], array: &PyArray1<u8>, func: F) -> PyResult<usize>
where
    F: for<'a> FnOnce(&'a [u8], Output<'a>) -> std::io::Result<usize>,
{
    let mut array_mut = unsafe { array.as_array_mut() };

    let buffer: &mut [u8] = to_py_err!(DecompressionError -> array_mut.as_slice_mut().ok_or_else(|| {
        pyo3::exceptions::PyBufferError::new_err("Failed to get mutable slice from array.")
    }))?;

    let output = Output::Slice(buffer);
    let size = to_py_err!(DecompressionError -> func(data, output))?;
    Ok(size)
}

#[macro_export]
macro_rules! to_py_err {
    ($error:ident -> $expr:expr) => {
        $expr.map_err(|err| PyErr::new::<$error, _>(err.to_string()))
    };
}

macro_rules! make_submodule {
    ($py:ident -> $parent:ident -> $submodule:ident) => {
        let sub_mod = PyModule::new($py, stringify!($submodule))?;
        $submodule::init_py_module(sub_mod)?;
        $parent.add_submodule(sub_mod)?;
    };
}

#[pymodule]
fn cramjam(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("CompressionError", py.get_type::<CompressionError>())?;
    m.add("DecompressionError", py.get_type::<DecompressionError>())?;

    make_submodule!(py -> m -> snappy);
    make_submodule!(py -> m -> brotli);
    make_submodule!(py -> m -> lz4);
    make_submodule!(py -> m -> gzip);
    make_submodule!(py -> m -> deflate);
    make_submodule!(py -> m -> zstd);

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::Output;

    // Default testing data
    fn gen_data() -> Vec<u8> {
        (0..1000000)
            .map(|_| "oh what a beautiful morning, oh what a beautiful day!!")
            .collect::<String>()
            .into_bytes()
    }

    // Single test generation
    macro_rules! round_trip {
        ($name:ident($compress_output:ident -> $decompress_output:ident), variant=$variant:ident, compressed_len=$compressed_len:literal, $(level=$level:tt)?) => {
            #[test]
            fn $name() {
                let data = gen_data();
                let mut compressed = if stringify!($compress_output) == "Slice" { vec![0; $compressed_len] } else { Vec::new() };
                let compressed_size = crate::$variant::internal::compress(&data, Output::$compress_output(&mut compressed) $(, $level)? ).unwrap();
                assert_eq!(compressed_size, $compressed_len);

                let mut decompressed = if stringify!($decompress_output) == "Slice" { vec![0; data.len()] } else { Vec::new() };
                let decompressed_size = crate::$variant::internal::decompress(&compressed, Output::$decompress_output(&mut decompressed)).unwrap();
                assert_eq!(decompressed_size, data.len());
                if &decompressed[..decompressed_size] != &data {
                    panic!("Decompressed and original data do not match! :-(")
                }
            }
        }
    }

    // macro to generate each variation of Output::* roundtrip.
    macro_rules! test_variant {
        ($variant:ident, compressed_len=$compressed_len:literal, $(level=$level:tt)?) => {
         #[cfg(test)]
         mod $variant {
            use super::*;
            round_trip!(roundtrip_compress_via_slice_decompress_via_slice(Slice -> Slice), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_slice_decompress_via_vector(Slice -> Vector), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_vector_decompress_via_slice(Vector -> Slice), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_vector_decompress_via_vector(Vector -> Vector), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
         }
        }
    }

    test_variant!(snappy, compressed_len = 2572398,);
    test_variant!(gzip, compressed_len = 157192, level = None);
    test_variant!(brotli, compressed_len = 729, level = None);
    test_variant!(deflate, compressed_len = 157174, level = None);
    test_variant!(zstd, compressed_len = 4990, level = None);
}
