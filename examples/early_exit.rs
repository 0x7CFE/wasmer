//! This example shows how the host can terminate execution of Wasm early from
//! inside a host function called by the Wasm.

use anyhow::bail;
use std::fmt;
use wasmer::{imports, wat2wasm, Function, Instance, Module, NativeFunc, RuntimeError, Store};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_engine_jit::JIT;

// First we need to create an error type that we'll use to signal the end of execution.
#[derive(Debug, Clone, Copy)]
struct ExitCode(u32);

// This type must implement `std::error::Error` so we must also implement `std::fmt::Display` for it.
impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// And then we implement `std::error::Error`.
impl std::error::Error for ExitCode {}

// The host function that we'll use to terminate execution.
fn early_exit() {
    // This is where it happens.
    RuntimeError::raise(Box::new(ExitCode(1)));
}

fn main() -> anyhow::Result<()> {
    // Let's declare the Wasm module with the text representation.
    let wasm_bytes = wat2wasm(
        br#"
(module
  (type $run_t (func (param i32 i32) (result i32)))
  (type $early_exit_t (func (param) (result)))
  (import "env" "early_exit" (func $early_exit (type $early_exit_t)))
  (func $run (type $run_t) (param $x i32) (param $y i32) (result i32)
    (call $early_exit)
    (i32.add
        local.get $x
        local.get $y))
  (export "run" (func $run)))
"#,
    )?;

    let store = Store::new(&JIT::new(&Cranelift::default()).engine());
    let module = Module::new(&store, wasm_bytes)?;

    let import_object = imports! {
        "env" => {
            "early_exit" => Function::new_native(&store, early_exit),
        }
    };
    let instance = Instance::new(&module, &import_object)?;

    // Get the `run` function which we'll use as our entrypoint.
    let run_func: NativeFunc<(i32, i32), i32> =
        instance.exports.get_native_function("run").unwrap();

    // When we call a function it can either succeed or fail.
    match run_func.call(1, 7) {
        Ok(result) => {
            bail!(
                "Expected early termination with `ExitCode`, found: {}",
                result
            );
        }
        // We're expecting it to fail.
        // We attempt to downcast the error into the error type that we were expecting.
        Err(e) => match e.downcast::<ExitCode>() {
            // We found the exit code used to terminate execution.
            Ok(exit_code) => {
                println!("Exited early with exit code: {}", exit_code);
                Ok(())
            }
            Err(e) => {
                bail!("Unknown error `{}` found. expected `ErrorCode`", e);
            }
        },
    }
}
