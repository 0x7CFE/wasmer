use super::engine::wasm_engine_t;
use std::ptr::NonNull;
use wasmer::Store;

/// Opaque wrapper around `Store`
#[allow(non_camel_case_types)]
pub struct wasm_store_t {
    pub(crate) inner: Store,
}

#[no_mangle]
pub unsafe extern "C" fn wasm_store_new(
    wasm_engine_ptr: Option<NonNull<wasm_engine_t>>,
) -> Option<Box<wasm_store_t>> {
    let wasm_engine_ptr = wasm_engine_ptr?;
    let wasm_engine = wasm_engine_ptr.as_ref();
    let store = Store::new(&*wasm_engine.inner);

    Some(Box::new(wasm_store_t { inner: store }))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_store_delete(_store: Option<Box<wasm_store_t>>) {}
