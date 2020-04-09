//! Relocation is the process of assigning load addresses for position-dependent
//! code and data of a program and adjusting the code and data to reflect the
//! assigned addresses.
//!
//! Source: https://en.wikipedia.org/wiki/Relocation_(computing)
//!
//! Each time a `Compiler` compiles a WebAssembly function (into machine code),
//! it also attaches if there are any relocations that need to be patched into
//! the generated machine code, so a given frontend (JIT or native) can
//! do the corresponding work to run it.

use crate::libcall::LibCall;
use crate::std::vec::Vec;
use crate::{Addend, CodeOffset, JumpTable};
use serde::{Deserialize, Serialize};
use std::fmt;
use wasm_common::entity::PrimaryMap;
use wasm_common::{DefinedFuncIndex, FuncIndex};

/// Relocation kinds for every ISA.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationKind {
    /// absolute 4-byte
    Abs4,
    /// absolute 8-byte
    Abs8,
    /// x86 PC-relative 4-byte
    X86PCRel4,
    /// x86 PC-relative 4-byte offset to trailing rodata
    X86PCRelRodata4,
    /// x86 call to PC-relative 4-byte
    X86CallPCRel4,
    // /// x86 call to PLT-relative 4-byte
    // X86CallPLTRel4,

    // /// x86 GOT PC-relative 4-byte
    // X86GOTPCRel4,

    // /// Arm32 call target
    // Arm32Call,

    // /// Arm64 call target
    // Arm64Call,

    // /// RISC-V call target
    // RiscvCall,

    // /// Elf x86_64 32 bit signed PC relative offset to two GOT entries for GD symbol.
    // ElfX86_64TlsGd,

    // /// Mach-O x86_64 32 bit signed PC relative offset to a `__thread_vars` entry.
    // MachOX86_64Tlv,
}

impl fmt::Display for RelocationKind {
    /// Display trait implementation drops the arch, since its used in contexts where the arch is
    /// already unambiguous, e.g. clif syntax with isa specified. In other contexts, use Debug.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Abs4 => write!(f, "Abs4"),
            Self::Abs8 => write!(f, "Abs8"),
            Self::X86PCRel4 => write!(f, "PCRel4"),
            Self::X86PCRelRodata4 => write!(f, "PCRelRodata4"),
            Self::X86CallPCRel4 => write!(f, "CallPCRel4"),
            // Self::X86CallPLTRel4 => write!(f, "CallPLTRel4"),
            // Self::X86GOTPCRel4 => write!(f, "GOTPCRel4"),
            // Self::Arm32Call | Self::Arm64Call | Self::RiscvCall => write!(f, "Call"),

            // Self::ElfX86_64TlsGd => write!(f, "ElfX86_64TlsGd"),
            // Self::MachOX86_64Tlv => write!(f, "MachOX86_64Tlv"),
        }
    }
}

/// A record of a relocation to perform.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// The relocation kind.
    pub kind: RelocationKind,
    /// Relocation target.
    pub reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    pub offset: CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: Addend,
}

/// Destination function. Can be either user function or some special one, like `memory.grow`.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(LibCall),
    /// Jump table index.
    JumpTable(FuncIndex, JumpTable),
}

/// Relocations to apply to function bodies.
pub type Relocations = PrimaryMap<DefinedFuncIndex, Vec<Relocation>>;
