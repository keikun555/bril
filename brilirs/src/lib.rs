#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![warn(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments)]
#![doc = include_str!("../README.md")]

use basic_block::BBProgram;
use bril_rs::Program;
use error::PositionalInterpError;
use pyo3::prelude::*;

/// The internal representation of brilirs, provided a ```TryFrom<Program>``` conversion
pub mod basic_block;
/// Provides ```check::type_check``` to validate [Program]
pub mod check;
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod error;
/// Provides ```interp::execute_main``` to execute [Program] that have been converted into [`BBProgram`]
pub mod interp;

#[doc(hidden)]
pub fn run_input<T: std::io::Write, U: std::io::Write>(
  input: impl std::io::Read,
  out: T,
  input_args: &[String],
  profiling: bool,
  profiling_out: U,
  check: bool,
  text: bool,
  src_name: Option<String>,
) -> Result<(), PositionalInterpError> {
  // It's a little confusing because of the naming conventions.
  //      - bril_rs takes file.json as input
  //      - bril2json takes file.bril as input
  let prog: Program = if text {
    bril2json::parse_abstract_program_from_read(input, true, true, src_name).try_into()?
  } else {
    bril_rs::load_abstract_program_from_read(input).try_into()?
  };
  let bbprog: BBProgram = prog.try_into()?;
  check::type_check(&bbprog)?;

  if !check {
    interp::execute_main(&bbprog, out, input_args, profiling, profiling_out)?;
  }

  Ok(())
}

/// Used for IO with Python
pub struct StringIO<'lifetime> {
    /// The vector to write to
    pub buffer: &'lifetime pyo3::types::PyList,
}

impl<'lifetime> std::io::Write for StringIO<'lifetime> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        let s: String = String::from_utf8_lossy(buf).to_string();
        self.buffer.append(s)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

impl std::convert::From<PositionalInterpError> for pyo3::PyErr {
    fn from(err: PositionalInterpError) -> pyo3::PyErr {
        pyo3::exceptions::PyRuntimeError::new_err(err.to_string())
    }
}

impl std::convert::From<error::InterpError> for pyo3::PyErr {
    fn from(err: error::InterpError) -> pyo3::PyErr {
        pyo3::exceptions::PyRuntimeError::new_err(err.to_string())
    }
}

 #[pyfunction]
/// PyO3 bindings for Python
 pub fn run_program(
   prog_string: String,
   out: &pyo3::types::PyList,
   input_args: Vec<String>,
   profiling: bool,
   profiling_out: &pyo3::types::PyList,
 ) -> Result<Option<i64>, pyo3::PyErr> {
   let prog: Program = bril_rs::load_abstract_program_from_string(&prog_string).try_into()?;
   let bbprog: BBProgram = prog.try_into()?;
   check::type_check(&bbprog)?;

   let out_write = StringIO {
       buffer: out,
   };
   let profiling_out_write = StringIO {
       buffer: profiling_out,
   };

   let return_code = interp::execute_main(&bbprog, out_write, &input_args, profiling, profiling_out_write)?;

   Ok(return_code)
 }


/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn brilirs(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_program, m)?)?;
    Ok(())
}

