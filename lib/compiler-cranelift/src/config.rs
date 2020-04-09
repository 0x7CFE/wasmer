use crate::compiler::CraneliftCompiler;
use cranelift_codegen::isa::{lookup, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use wasmer_compiler::{Compiler, CompilerConfig, CpuFeature, Features, Target};

// Runtime Environment

/// Possible optimization levels for the Cranelift codegen backend.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum OptLevel {
    /// No optimizations performed, minimizes compilation time by disabling most
    /// optimizations.
    None,
    /// Generates the fastest possible code, but may take longer.
    Speed,
    /// Similar to `speed`, but also performs transformations aimed at reducing
    /// code size.
    SpeedAndSize,
}

/// Global configuration options used to create an [`Engine`] and customize its
/// behavior.
///
/// This structure exposed a builder-like interface and is primarily consumed by
/// [`Engine::new()`]
#[derive(Clone)]
pub struct CraneliftConfig {
    /// Enable NaN canonicalization.
    ///
    /// NaN canonicalization is useful when trying to run WebAssembly
    /// deterministically across different architectures.
    pub enable_nan_canonicalization: bool,

    /// Should the Cranelift verifier be enabled.
    ///
    /// The verifier assures that the generated Cranelift IR is valid.
    pub enable_verifier: bool,

    /// The optimization levels when optimizing the IR.
    pub opt_level: OptLevel,

    features: Features,
    target: Target,
}

impl CraneliftConfig {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        Self {
            enable_nan_canonicalization: false,
            enable_verifier: false,
            opt_level: OptLevel::Speed,
            features: Default::default(),
            target: Default::default(),
        }
    }

    /// Generates the ISA for the current target
    pub fn isa(&self) -> Box<dyn TargetIsa> {
        let target = self.target();
        let mut builder =
            lookup(target.triple().clone()).expect("construct Cranelift ISA for triple");
        // Cpu Features

        let cpu_features = target.cpu_features();
        if !cpu_features.contains(CpuFeature::SSE2) {
            panic!("x86 support requires SSE2");
        }
        if cpu_features.contains(CpuFeature::SSE3) {
            builder.enable("has_sse3").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::SSSE3) {
            builder.enable("has_ssse3").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::SSE41) {
            builder.enable("has_sse41").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::SSE42) {
            builder.enable("has_sse42").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::POPCNT) {
            builder.enable("has_popcnt").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::AVX) {
            builder.enable("has_avx").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::BMI1) {
            builder.enable("has_bmi1").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::BMI2) {
            builder.enable("has_bmi2").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::AVX2) {
            builder.enable("has_avx2").expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::AVX512DQ) {
            builder
                .enable("has_avx512dq")
                .expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::AVX512VL) {
            builder
                .enable("has_avx512vl")
                .expect("should be valid flag");
        }
        if cpu_features.contains(CpuFeature::LZCNT) {
            builder.enable("has_lzcnt").expect("should be valid flag");
        }

        builder.finish(self.flags())
    }

    /// Generates the flags for the current target
    pub fn flags(&self) -> settings::Flags {
        let mut flags = settings::builder();

        // There are two possible traps for division, and this way
        // we get the proper one if code traps.
        flags
            .enable("avoid_div_traps")
            .expect("should be valid flag");

        // Invert cranelift's default-on verification to instead default off.
        let enable_verifier = if self.enable_verifier {
            "true"
        } else {
            "false"
        };
        flags
            .set("enable_verifier", enable_verifier)
            .expect("should be valid flag");

        let opt_level = if self.features.simd {
            "none"
        } else {
            match self.opt_level {
                OptLevel::None => "none",
                OptLevel::Speed => "speed",
                OptLevel::SpeedAndSize => "speed_and_size",
            }
        };

        flags
            .set("opt_level", opt_level)
            .expect("should be valid flag");

        let enable_simd = if self.features.simd { "true" } else { "false" };
        flags
            .set("enable_simd", enable_simd)
            .expect("should be valid flag");

        settings::Flags::new(flags)
    }
}

impl CompilerConfig for CraneliftConfig {
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
        Box::new(CraneliftCompiler::new(&self))
    }
}

impl Default for CraneliftConfig {
    fn default() -> CraneliftConfig {
        CraneliftConfig::new()
    }
}
