use super::errors::{WasmError, WasmResult};
use super::module::translate_module;
use super::state::ModuleTranslationState;
use crate::module::{MemoryPlan, Module, TableElements, TablePlan};
use crate::std::borrow::ToOwned;
use crate::std::string::ToString;
use crate::std::{boxed::Box, string::String, vec::Vec};
use crate::tunables::Tunables;
use std::convert::TryFrom;
use std::sync::Arc;
use wasm_common::entity::PrimaryMap;
use wasm_common::FuncType;
use wasm_common::{
    DataIndex, DataInitializer, DataInitializerLocation, DefinedFuncIndex, ElemIndex, ExportIndex,
    FuncIndex, GlobalIndex, GlobalType, ImportIndex, MemoryIndex, MemoryType, SignatureIndex,
    TableIndex, TableType,
};

/// Contains function data: bytecode and its offset in the module.
#[derive(Hash)]
pub struct FunctionBodyData<'a> {
    /// Function body bytecode.
    pub data: &'a [u8],

    /// Body offset relative to the module file.
    pub module_offset: usize,
}

/// The result of translating via `ModuleEnvironment`. Function bodies are not
/// yet translated, and data initializers have not yet been copied out of the
/// original buffer.
/// The function bodies will be translated by a specific compiler backend.
pub struct ModuleTranslation<'data> {
    /// Module information.
    pub module: Module,

    /// References to the function bodies.
    pub function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,

    /// Tunable parameters.
    pub tunables: Tunables,

    /// The decoded Wasm types for the module.
    pub module_translation: Option<ModuleTranslationState>,
}

/// Object containing the standalone environment information.
pub struct ModuleEnvironment<'data> {
    /// The result to be filled in.
    pub result: ModuleTranslation<'data>,
    imports: u32,
}

impl<'data> ModuleEnvironment<'data> {
    /// Allocates the environment data structures.
    pub fn new(tunables: Tunables) -> Self {
        Self {
            result: ModuleTranslation {
                module: Module::new(),
                function_body_inputs: PrimaryMap::new(),
                data_initializers: Vec::new(),
                tunables,
                module_translation: None,
            },
            imports: 0,
        }
    }

    /// Translate a wasm module using this environment. This consumes the
    /// `ModuleEnvironment` and produces a `ModuleTranslation`.
    pub fn translate(mut self, data: &'data [u8]) -> WasmResult<ModuleTranslation<'data>> {
        assert!(self.result.module_translation.is_none());
        let module_translation = translate_module(data, &mut self)?;
        self.result.module_translation = Some(module_translation);
        Ok(self.result)
    }

    pub(crate) fn declare_export(&mut self, export: ExportIndex, name: &str) -> WasmResult<()> {
        self.result
            .module
            .exports
            .insert(String::from(name), export);
        Ok(())
    }

    pub(crate) fn declare_import(
        &mut self,
        import: ImportIndex,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        self.result.module.imports.insert(
            (String::from(module), String::from(field), self.imports),
            import,
        );
        Ok(())
    }

    pub(crate) fn reserve_signatures(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .signatures
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_signature(&mut self, sig: FuncType) -> WasmResult<()> {
        // TODO: Deduplicate signatures.
        self.result.module.local.signatures.push(sig);
        Ok(())
    }

    pub(crate) fn declare_func_import(
        &mut self,
        sig_index: SignatureIndex,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.functions.len(),
            self.result.module.local.num_imported_funcs,
            "Imported functions must be declared first"
        );
        self.declare_import(
            ImportIndex::Function(FuncIndex::from_u32(
                self.result.module.local.num_imported_funcs as _,
            )),
            module,
            field,
        )?;
        self.result.module.local.functions.push(sig_index);
        self.result.module.local.num_imported_funcs += 1;
        self.imports += 1;
        Ok(())
    }

    pub(crate) fn declare_table_import(
        &mut self,
        table: TableType,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.table_plans.len(),
            self.result.module.local.num_imported_tables,
            "Imported tables must be declared first"
        );
        self.declare_import(
            ImportIndex::Table(TableIndex::from_u32(
                self.result.module.local.num_imported_tables as _,
            )),
            module,
            field,
        )?;
        let plan = TablePlan::for_table(table, &self.result.tunables);
        self.result.module.local.table_plans.push(plan);
        self.result.module.local.num_imported_tables += 1;
        self.imports += 1;
        Ok(())
    }

    pub(crate) fn declare_memory_import(
        &mut self,
        memory: MemoryType,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.memory_plans.len(),
            self.result.module.local.num_imported_memories,
            "Imported memories must be declared first"
        );
        self.declare_import(
            ImportIndex::Memory(MemoryIndex::from_u32(
                self.result.module.local.num_imported_memories as _,
            )),
            module,
            field,
        )?;
        let plan = MemoryPlan::for_memory(memory, &self.result.tunables);
        self.result.module.local.memory_plans.push(plan);
        self.result.module.local.num_imported_memories += 1;
        self.imports += 1;
        Ok(())
    }

    pub(crate) fn declare_global_import(
        &mut self,
        global: GlobalType,
        module: &str,
        field: &str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.result.module.local.globals.len(),
            self.result.module.local.num_imported_globals,
            "Imported globals must be declared first"
        );
        self.declare_import(
            ImportIndex::Global(GlobalIndex::from_u32(
                self.result.module.local.num_imported_globals as _,
            )),
            module,
            field,
        )?;
        self.result.module.local.globals.push(global);
        self.result.module.local.num_imported_globals += 1;
        self.imports += 1;
        Ok(())
    }

    pub(crate) fn finish_imports(&mut self) -> WasmResult<()> {
        Ok(())
    }

    pub(crate) fn reserve_func_types(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .functions
            .reserve_exact(usize::try_from(num).unwrap());
        self.result
            .function_body_inputs
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_func_type(&mut self, sig_index: SignatureIndex) -> WasmResult<()> {
        self.result.module.local.functions.push(sig_index);
        Ok(())
    }

    pub(crate) fn reserve_tables(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .table_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_table(&mut self, table: TableType) -> WasmResult<()> {
        let plan = TablePlan::for_table(table, &self.result.tunables);
        self.result.module.local.table_plans.push(plan);
        Ok(())
    }

    pub(crate) fn reserve_memories(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .memory_plans
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_memory(&mut self, memory: MemoryType) -> WasmResult<()> {
        if memory.shared {
            return Err(WasmError::Unsupported(
                "shared memories are not supported yet".to_owned(),
            ));
        }
        let plan = MemoryPlan::for_memory(memory, &self.result.tunables);
        self.result.module.local.memory_plans.push(plan);
        Ok(())
    }

    pub(crate) fn reserve_globals(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .local
            .globals
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_global(&mut self, global: GlobalType) -> WasmResult<()> {
        self.result.module.local.globals.push(global);
        Ok(())
    }

    pub(crate) fn reserve_exports(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .exports
            .reserve(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_func_export(
        &mut self,
        func_index: FuncIndex,
        name: &str,
    ) -> WasmResult<()> {
        self.declare_export(ExportIndex::Function(func_index), name)
    }

    pub(crate) fn declare_table_export(
        &mut self,
        table_index: TableIndex,
        name: &str,
    ) -> WasmResult<()> {
        self.declare_export(ExportIndex::Table(table_index), name)
    }

    pub(crate) fn declare_memory_export(
        &mut self,
        memory_index: MemoryIndex,
        name: &str,
    ) -> WasmResult<()> {
        self.declare_export(ExportIndex::Memory(memory_index), name)
    }

    pub(crate) fn declare_global_export(
        &mut self,
        global_index: GlobalIndex,
        name: &str,
    ) -> WasmResult<()> {
        self.declare_export(ExportIndex::Global(global_index), name)
    }

    pub(crate) fn declare_start_func(&mut self, func_index: FuncIndex) -> WasmResult<()> {
        debug_assert!(self.result.module.start_func.is_none());
        self.result.module.start_func = Some(func_index);
        Ok(())
    }

    pub(crate) fn reserve_table_elements(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .module
            .table_elements
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        self.result.module.table_elements.push(TableElements {
            table_index,
            base,
            offset,
            elements,
        });
        Ok(())
    }

    pub(crate) fn declare_passive_element(
        &mut self,
        elem_index: ElemIndex,
        segments: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        let old = self
            .result
            .module
            .passive_elements
            .insert(elem_index, segments);
        debug_assert!(
            old.is_none(),
            "should never get duplicate element indices, that would be a bug in `wasmer_compiler`'s \
             translation"
        );
        Ok(())
    }

    pub(crate) fn define_function_body(
        &mut self,
        _module_translation: &ModuleTranslationState,
        body_bytes: &'data [u8],
        body_offset: usize,
    ) -> WasmResult<()> {
        self.result.function_body_inputs.push(FunctionBodyData {
            data: body_bytes,
            module_offset: body_offset,
        });
        Ok(())
    }

    pub(crate) fn reserve_data_initializers(&mut self, num: u32) -> WasmResult<()> {
        self.result
            .data_initializers
            .reserve_exact(usize::try_from(num).unwrap());
        Ok(())
    }

    pub(crate) fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'data [u8],
    ) -> WasmResult<()> {
        self.result.data_initializers.push(DataInitializer {
            location: DataInitializerLocation {
                memory_index,
                base,
                offset,
            },
            data,
        });
        Ok(())
    }

    pub(crate) fn reserve_passive_data(&mut self, count: u32) -> WasmResult<()> {
        self.result.module.passive_data.reserve(count as usize);
        Ok(())
    }

    pub(crate) fn declare_passive_data(
        &mut self,
        data_index: DataIndex,
        data: &'data [u8],
    ) -> WasmResult<()> {
        let old = self
            .result
            .module
            .passive_data
            .insert(data_index, Arc::from(data));
        debug_assert!(
            old.is_none(),
            "a module can't have duplicate indices, this would be a wasmer-compiler bug"
        );
        Ok(())
    }

    pub(crate) fn declare_module_name(&mut self, name: &'data str) -> WasmResult<()> {
        self.result.module.name = Some(name.to_string());
        Ok(())
    }

    pub(crate) fn declare_func_name(
        &mut self,
        func_index: FuncIndex,
        name: &'data str,
    ) -> WasmResult<()> {
        self.result
            .module
            .func_names
            .insert(func_index, name.to_string());
        Ok(())
    }

    /// Provides the number of imports up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    pub(crate) fn reserve_imports(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Notifies the implementation that all exports have been declared.
    pub(crate) fn finish_exports(&mut self) -> WasmResult<()> {
        Ok(())
    }

    /// Indicates that a custom section has been found in the wasm file
    pub(crate) fn custom_section(
        &mut self,
        _name: &'data str,
        _data: &'data [u8],
    ) -> WasmResult<()> {
        Ok(())
    }
}
