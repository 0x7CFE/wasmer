use crate::indexes::{FuncIndex, GlobalIndex};
use crate::values::Value;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

// Type Representations

// Value Types

/// A list of all possible value types in WebAssembly.
#[derive(Copy, Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum Type {
    /// Signed 32 bit integer.
    I32,
    /// Signed 64 bit integer.
    I64,
    /// Floating point 32 bit integer.
    F32,
    /// Floating point 64 bit integer.
    F64,
    /// A 128 bit number.
    V128,
    /// A reference to opaque data in the Wasm instance.
    AnyRef, /* = 128 */
    /// A reference to a Wasm function.
    FuncRef,
}

impl Type {
    /// Returns true if `Type` matches any of the numeric types. (e.g. `I32`,
    /// `I64`, `F32`, `F64`).
    pub fn is_num(&self) -> bool {
        match self {
            Type::I32 | Type::I64 | Type::F32 | Type::F64 => true,
            _ => false,
        }
    }

    /// Returns true if `Type` matches either of the reference types.
    pub fn is_ref(&self) -> bool {
        match self {
            Type::AnyRef | Type::FuncRef => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
/// The WebAssembly V128 type
pub struct V128(pub(crate) [u8; 16]);

impl V128 {
    /// Get the bytes corresponding to the V128 value
    pub fn bytes(&self) -> &[u8; 16] {
        &self.0
    }
    /// Iterate over the bytes in the constant.
    pub fn iter(&self) -> impl Iterator<Item = &u8> {
        self.0.iter()
    }

    /// Convert the immediate into a vector.
    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Convert the immediate into a slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }
}

impl From<&[u8]> for V128 {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 16);
        let mut buffer = [0; 16];
        buffer.copy_from_slice(slice);
        Self(buffer)
    }
}

// External Types

/// A list of all possible types which can be externally referenced from a
/// WebAssembly module.
///
/// This list can be found in [`ImportType`] or [`ExportType`], so these types
/// can either be imported or exported.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum ExternType {
    /// This external type is the type of a WebAssembly function.
    Func(FuncType),
    /// This external type is the type of a WebAssembly global.
    Global(GlobalType),
    /// This external type is the type of a WebAssembly table.
    Table(TableType),
    /// This external type is the type of a WebAssembly memory.
    Memory(MemoryType),
}

macro_rules! accessors {
    ($(($variant:ident($ty:ty) $get:ident $unwrap:ident))*) => ($(
        /// Attempt to return the underlying type of this external type,
        /// returning `None` if it is a different type.
        pub fn $get(&self) -> Option<&$ty> {
            if let ExternType::$variant(e) = self {
                Some(e)
            } else {
                None
            }
        }

        /// Returns the underlying descriptor of this [`ExternType`], panicking
        /// if it is a different type.
        ///
        /// # Panics
        ///
        /// Panics if `self` is not of the right type.
        pub fn $unwrap(&self) -> &$ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl ExternType {
    accessors! {
        (Func(FuncType) func unwrap_func)
        (Global(GlobalType) global unwrap_global)
        (Table(TableType) table unwrap_table)
        (Memory(MemoryType) memory unwrap_memory)
    }
}

/// The signature of a function that is either implemented
/// in a Wasm module or exposed to Wasm by the host.
///
/// WebAssembly functions can have 0 or more parameters and results.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct FuncType {
    /// The parameters of the function
    params: Vec<Type>,
    /// The return values of the function
    results: Vec<Type>,
}

impl FuncType {
    /// Creates a new Function Type with the given parameter and return types.
    pub fn new<Params, Returns>(params: Params, returns: Returns) -> Self
    where
        Params: Into<Vec<Type>>,
        Returns: Into<Vec<Type>>,
    {
        Self {
            params: params.into(),
            results: returns.into(),
        }
    }

    /// Parameter types.
    pub fn params(&self) -> &[Type] {
        &self.params
    }

    /// Return types.
    pub fn results(&self) -> &[Type] {
        &self.results
    }

    // /// Returns true if parameter types match the function signature.
    // pub fn check_params(&self, params: &[Value<T>]) -> bool {
    //     self.params.len() == params.len()
    //         && self
    //             .params
    //             .iter()
    //             .zip(params.iter().map(|val| val.ty()))
    //             .all(|(t0, ref t1)| t0 == t1)
    // }
}

impl std::fmt::Display for FuncType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let params = self
            .params
            .iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join(", ");
        let results = self
            .results
            .iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "[{}] -> [{}]", params, results)
    }
}

/// Indicator of whether a global is mutable or not
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum Mutability {
    /// The global is constant and its value does not change
    Const,
    /// The value of the global can change over time
    Var,
}

impl From<bool> for Mutability {
    fn from(val: bool) -> Mutability {
        match val {
            false => Mutability::Const,
            true => Mutability::Var,
        }
    }
}

impl From<Mutability> for bool {
    fn from(val: Mutability) -> bool {
        match val {
            Mutability::Const => false,
            Mutability::Var => true,
        }
    }
}

/// WebAssembly global.
#[derive(Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct GlobalType {
    /// The type of the value stored in the global.
    pub ty: Type,
    /// A flag indicating whether the value may change at runtime.
    pub mutability: Mutability,
    /// The source of the initial value.
    pub initializer: GlobalInit,
}

// Global Types

/// A WebAssembly global descriptor.
///
/// This type describes an instance of a global in a WebAssembly module. Globals
/// are local to an [`Instance`](crate::Instance) and are either immutable or
/// mutable.
impl GlobalType {
    /// Create a new Global variable
    /// # Usage:
    /// ```
    /// use wasm_common::{GlobalType, Type, Mutability, Value};
    ///
    /// // An I32 constant global
    /// let global = GlobalType::new(Type::I32, Mutability::Const);
    /// // An I64 mutable global
    /// let global = GlobalType::new(Type::I64, Mutability::Var);
    /// ```
    pub fn new(ty: Type, mutability: Mutability) -> Self {
        Self {
            ty: ty,
            mutability: mutability,
            initializer: GlobalInit::Import,
        }
    }
}

/// Globals are initialized via the `const` operators or by referring to another import.
#[derive(Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum GlobalInit {
    /// An `i32.const`.
    I32Const(i32),
    /// An `i64.const`.
    I64Const(i64),
    /// An `f32.const`.
    F32Const(u32),
    /// An `f64.const`.
    F64Const(u64),
    /// A `v128.const`.
    V128Const(V128),
    /// A `global.get` of another global.
    GetGlobal(GlobalIndex),
    /// A `ref.null`.
    RefNullConst,
    /// A `ref.func <index>`.
    RefFunc(FuncIndex),
    ///< The global is imported from, and thus initialized by, a different module.
    Import,
}

impl GlobalInit {
    /// Get the `GlobalInit` from a given `Value`
    pub fn from_value<T>(value: Value<T>) -> Self {
        match value {
            Value::I32(i) => GlobalInit::I32Const(i),
            Value::I64(i) => GlobalInit::I64Const(i),
            Value::F32(f) => GlobalInit::F32Const(f),
            Value::F64(f) => GlobalInit::F64Const(f),
            _ => unimplemented!("GlobalInit from_value for {:?}", value),
        }
    }
    /// Get the `Value` from the Global init value
    pub fn to_value<T>(&self) -> Value<T> {
        match self {
            GlobalInit::I32Const(i) => Value::I32(*i),
            GlobalInit::I64Const(i) => Value::I64(*i),
            GlobalInit::F32Const(f) => Value::F32(*f),
            GlobalInit::F64Const(f) => Value::F64(*f),
            _ => unimplemented!("GlobalInit to_value for {:?}", self),
        }
    }
}

// Table Types

/// A descriptor for a table in a WebAssembly module.
///
/// Tables are contiguous chunks of a specific element, typically a `funcref` or
/// an `anyref`. The most common use for tables is a function table through
/// which `call_indirect` can invoke other functions.
#[derive(Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct TableType {
    /// The type of data stored in elements of the table.
    pub ty: Type,
    /// The minimum number of elements in the table.
    pub minimum: u32,
    /// The maximum number of elements in the table.
    pub maximum: Option<u32>,
}

impl TableType {
    /// Creates a new table descriptor which will contain the specified
    /// `element` and have the `limits` applied to its length.
    pub fn new(ty: Type, minimum: u32, maximum: Option<u32>) -> TableType {
        TableType {
            ty,
            minimum,
            maximum,
        }
    }
}

// Memory Types

/// A descriptor for a WebAssembly memory type.
///
/// Memories are described in units of pages (64KB) and represent contiguous
/// chunks of addressable memory.
#[derive(Debug, Clone, Copy, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemoryType {
    /// The minimum number of pages in the memory.
    pub minimum: u32,
    /// The maximum number of pages in the memory.
    pub maximum: Option<u32>,
    /// Whether the memory may be shared between multiple threads.
    pub shared: bool,
}

impl MemoryType {
    /// Creates a new descriptor for a WebAssembly memory given the specified
    /// limits of the memory.
    pub fn new(minimum: u32, maximum: Option<u32>, shared: bool) -> MemoryType {
        MemoryType {
            minimum,
            maximum,
            shared,
        }
    }
}

// Import Types

/// A descriptor for an imported value into a wasm module.
///
/// This type is primarily accessed from the
/// [`Module::imports`](crate::Module::imports) API. Each [`ImportType`]
/// describes an import into the wasm module with the module/name that it's
/// imported from as well as the type of item that's being imported.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ImportType {
    module: String,
    name: String,
    ty: ExternType,
}

impl ImportType {
    /// Creates a new import descriptor which comes from `module` and `name` and
    /// is of type `ty`.
    pub fn new(module: &str, name: &str, ty: ExternType) -> ImportType {
        ImportType {
            module: module.to_owned(),
            name: name.to_owned(),
            ty,
        }
    }

    /// Returns the module name that this import is expected to come from.
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Returns the field name of the module that this import is expected to
    /// come from.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the expected type of this import.
    pub fn ty(&self) -> &ExternType {
        &self.ty
    }
}

// Export Types

/// A descriptor for an exported WebAssembly value.
///
/// This type is primarily accessed from the
/// [`Module::exports`](crate::Module::exports) accessor and describes what
/// names are exported from a wasm module and the type of the item that is
/// exported.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ExportType {
    name: String,
    ty: ExternType,
}

impl ExportType {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `ty`.
    pub fn new(name: &str, ty: ExternType) -> ExportType {
        ExportType {
            name: name.to_string(),
            ty,
        }
    }

    /// Returns the name by which this export is known by.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the type of this export.
    pub fn ty(&self) -> &ExternType {
        &self.ty
    }
}
