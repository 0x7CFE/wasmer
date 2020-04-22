// Allow unused imports while developing`
#![allow(unused_imports, dead_code)]

use crate::compiler::LLVMCompiler;
use wasmer_compiler::{Compiler, CompilerConfig, CpuFeature, Features, Target};
use inkwell::targets::{Target as LLVMTarget, TargetMachine, TargetTriple, InitializationConfig, CodeModel, RelocMode};
use inkwell::OptimizationLevel;
use itertools::Itertools;
use target_lexicon::Architecture;

#[derive(Clone)]
pub struct LLVMConfig {
    /// Enable NaN canonicalization.
    ///
    /// NaN canonicalization is useful when trying to run WebAssembly
    /// deterministically across different architectures.
    pub enable_nan_canonicalization: bool,

    /// Should the LLVM IR verifier be enabled.
    ///
    /// The verifier assures that the generated LLVM IR is valid.
    pub enable_verifier: bool,

    /// The optimization levels when optimizing the IR.
    pub opt_level: OptimizationLevel,

    features: Features,
    target: Target,
}

impl LLVMConfig {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        Self {
            enable_nan_canonicalization: true,
            enable_verifier: false,
            opt_level: OptimizationLevel::Aggressive,
            features: Default::default(),
            target: Default::default(),
        }
    }
    fn reloc_mode(&self) -> RelocMode {
        RelocMode::Static
    }

    fn code_model(&self) -> CodeModel {
        CodeModel::Large
    }

    /// Generates the target machine for the current target
    pub fn target_machine(&self) -> TargetMachine {
        let target = self.target();
        let triple = target.triple();
        let cpu_features = target.cpu_features().clone();

        match triple.architecture {
            Architecture::X86_64 => LLVMTarget::initialize_x86(&InitializationConfig {
                asm_parser: true,
                asm_printer: true,
                base: true,
                disassembler: true,
                info: true,
                machine_code: true,
            }),
            Architecture::Arm(_) => LLVMTarget::initialize_aarch64(&InitializationConfig {
                asm_parser: true,
                asm_printer: true,
                base: true,
                disassembler: true,
                info: true,
                machine_code: true,
            }),
            _ => unimplemented!("target {} not supported", triple),
        }

        // The CPU features formatted as LLVM strings
        let llvm_cpu_features = cpu_features.iter().filter_map(|feature| {
            match feature {
                CpuFeature::SSE2 => Some("+sse2"),
                CpuFeature::SSE3 => Some("+sse3"),
                CpuFeature::SSSE3 => Some("+ssse3"),
                CpuFeature::SSE41 => Some("+sse41"),
                CpuFeature::SSE42 => Some("+sse42"),
                CpuFeature::POPCNT => Some("+popcnt"),
                CpuFeature::AVX => Some("+avx"),
                CpuFeature::BMI1 => Some("+bmi"),
                CpuFeature::BMI2 => Some("+bmi2"),
                CpuFeature::AVX2 => Some("+avx2"),
                CpuFeature::AVX512DQ => Some("+avx512dq"),
                CpuFeature::AVX512VL => Some("+avx512vl"),
                CpuFeature::LZCNT => Some("+lzcnt"),
            }
        }).join(",");

        let arch_string = triple.architecture.to_string();
        let llvm_target = LLVMTarget::from_name(&arch_string).unwrap();
        let target_machine = llvm_target.create_target_machine(
            &TargetTriple::create(&target.triple().to_string()),
            &arch_string,
            &llvm_cpu_features,
            self.opt_level.clone(),
            self.reloc_mode(),
            self.code_model(),
        )
        .unwrap();
        target_machine
    }
}

impl CompilerConfig for LLVMConfig {
    /// Gets the WebAssembly features
    fn features(&self) -> &Features {
        &self.features
    }

    /// Gets the WebAssembly features, mutable
    fn features_mut(&mut self) -> &mut Features {
        &mut self.features
    }

    /// Gets the target that we will use for compiling
    /// the WebAssembly module
    fn target(&self) -> &Target {
        &self.target
    }

    /// Gets the target that we will use for compiling
    /// the WebAssembly module, mutable
    fn target_mut(&mut self) -> &mut Target {
        &mut self.target
    }

    /// Transform it into the compiler
    fn compiler(&self) -> Box<dyn Compiler> {
        Box::new(LLVMCompiler::new(&self))
    }
}

impl Default for LLVMConfig {
    fn default() -> LLVMConfig {
        LLVMConfig::new()
    }
}
