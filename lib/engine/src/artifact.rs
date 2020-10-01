use crate::{
    resolve_imports, InstantiationError, Resolver, RuntimeError, SerializeError, Tunables,
};
use std::any::Any;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use wasmer_compiler::Features;
use wasmer_types::entity::{BoxedSlice, PrimaryMap};
use wasmer_types::{
    DataInitializer, FunctionIndex, LocalFunctionIndex, MemoryIndex, OwnedDataInitializer,
    SignatureIndex, TableIndex,
};
use wasmer_vm::{
    FunctionBodyPtr, InstanceHandle, MemoryStyle, ModuleInfo, TableStyle, VMSharedSignatureIndex,
};

/// An `Artifact` is the product that the `Engine`
/// implementation produce and use.
///
/// The `Artifact` contains the compiled data for a given
/// module as well as extra information needed to run the
/// module at runtime, such as [`ModuleInfo`] and [`Features`].
pub trait Artifact: Send + Sync {
    /// Return a reference-counted pointer to the module
    fn module(&self) -> Arc<ModuleInfo>;

    /// Return a pointer to a module.
    fn module_ref(&self) -> &ModuleInfo;

    /// Gets a mutable reference to the info.
    ///
    /// Note: this will return `None` if the module is already instantiated.
    fn module_mut(&mut self) -> Option<&mut ModuleInfo>;

    /// Register thie `Artifact` stack frame information into the global scope.
    ///
    /// This is required to ensure that any traps can be properly symbolicated.
    fn register_frame_info(&self);

    /// Returns the features for this Artifact
    fn features(&self) -> &Features;

    /// Returns the memory styles associated with this `Artifact`.
    fn memory_styles(&self) -> &PrimaryMap<MemoryIndex, MemoryStyle>;

    /// Returns the table plans associated with this `Artifact`.
    fn table_styles(&self) -> &PrimaryMap<TableIndex, TableStyle>;

    /// Returns data initializers to pass to `InstanceHandle::initialize`
    fn data_initializers(&self) -> &[OwnedDataInitializer];

    /// Returns the functions allocated in memory or this `Artifact`
    /// ready to be run.
    fn finished_functions(&self) -> &BoxedSlice<LocalFunctionIndex, FunctionBodyPtr>;

    /// Returns the dynamic function trampolines allocated in memory
    /// for this `Artifact`, ready to be run.
    fn finished_dynamic_function_trampolines(&self) -> &BoxedSlice<FunctionIndex, FunctionBodyPtr>;

    /// Returns the associated VM signatures for this `Artifact`.
    fn signatures(&self) -> &BoxedSlice<SignatureIndex, VMSharedSignatureIndex>;

    /// Serializes an artifact into bytes
    fn serialize(&self) -> Result<Vec<u8>, SerializeError>;

    /// Serializes an artifact into a file path
    fn serialize_to_file(&self, path: &Path) -> Result<(), SerializeError> {
        let serialized = self.serialize()?;
        fs::write(&path, serialized)?;
        Ok(())
    }

    /// Do preinstantiation logic that is executed before instantiating
    fn preinstantiate(&self) -> Result<(), InstantiationError> {
        Ok(())
    }

    /// Crate an `Instance` from this `Artifact`.
    ///
    /// # Safety
    ///
    /// See [`InstanceHandle::new`].
    unsafe fn instantiate(
        &self,
        tunables: &dyn Tunables,
        resolver: &dyn Resolver,
        host_state: Box<dyn Any>,
    ) -> Result<InstanceHandle, InstantiationError> {
        self.preinstantiate()?;

        let module = self.module();
        let imports = resolve_imports(
            &module,
            resolver,
            &self.finished_dynamic_function_trampolines(),
            self.memory_styles(),
            self.table_styles(),
        )
        .map_err(InstantiationError::Link)?;
        let finished_memories = tunables
            .create_memories(&module, self.memory_styles())
            .map_err(InstantiationError::Link)?
            .into_boxed_slice();
        let finished_tables = tunables
            .create_tables(&module, self.table_styles())
            .map_err(InstantiationError::Link)?
            .into_boxed_slice();
        let finished_globals = tunables
            .create_globals(&module)
            .map_err(InstantiationError::Link)?
            .into_boxed_slice();

        self.register_frame_info();

        InstanceHandle::new(
            module,
            self.finished_functions().clone(),
            finished_memories,
            finished_tables,
            finished_globals,
            imports,
            self.signatures().clone(),
            host_state,
        )
        .map_err(|trap| InstantiationError::Start(RuntimeError::from_trap(trap)))
    }

    /// Finishes the instantiation of a just created `InstanceHandle`.
    ///
    /// # Safety
    ///
    /// See [`InstanceHandle::finish_instantiation`].
    unsafe fn finish_instantiation(
        &self,
        handle: &InstanceHandle,
    ) -> Result<(), InstantiationError> {
        let data_initializers = self
            .data_initializers()
            .iter()
            .map(|init| DataInitializer {
                location: init.location.clone(),
                data: &*init.data,
            })
            .collect::<Vec<_>>();
        handle
            .finish_instantiation(&data_initializers)
            .map_err(|trap| InstantiationError::Start(RuntimeError::from_trap(trap)))
    }
}
