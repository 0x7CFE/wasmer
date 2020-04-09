use std::cmp::min;
use target_lexicon::{OperatingSystem, PointerWidth, Triple, HOST};

/// Tunable parameters for WebAssembly compilation.
#[derive(Clone)]
pub struct Tunables {
    /// For static heaps, the size in wasm pages of the heap protected by bounds checking.
    pub static_memory_bound: u32,

    /// The size in bytes of the offset guard for static heaps.
    pub static_memory_offset_guard_size: u64,

    /// The size in bytes of the offset guard for dynamic heaps.
    pub dynamic_memory_offset_guard_size: u64,
}

impl Tunables {
    /// Get the `Tunables` for a specific Target
    pub fn for_target(triple: &Triple) -> Self {
        let pointer_width: PointerWidth = triple.pointer_width().unwrap();
        let (mut static_memory_bound, mut static_memory_offset_guard_size): (u32, u64) =
            match pointer_width {
                PointerWidth::U16 => (0x400, 0x1000),
                PointerWidth::U32 => (0x4000, 0x1_0000),
                // Static Memory Bound:
                //   Allocating 4 GiB of address space let us avoid the
                //   need for explicit bounds checks.
                // Static Memory Guard size:
                //   Allocating 2 GiB of address space lets us translate wasm
                //   offsets into x86 offsets as aggressively as we can.
                PointerWidth::U64 => (0x1_0000, 0x8000_0000),
            };

        // Allocate a small guard to optimize common cases but without
        // wasting too much memory.
        let dynamic_memory_offset_guard_size: u64 = 0x1_0000;

        match triple.operating_system {
            OperatingSystem::Windows => {
                // For now, use a smaller footprint on Windows so that we don't
                // don't outstrip the paging file.
                static_memory_bound = min(static_memory_bound, 0x100);
                static_memory_offset_guard_size = min(static_memory_offset_guard_size, 0x10000);
            }
            _ => {}
        }

        Self {
            static_memory_bound,
            static_memory_offset_guard_size,
            dynamic_memory_offset_guard_size,
        }
    }
}

impl Default for Tunables {
    fn default() -> Self {
        Tunables::for_target(&HOST)
    }
}
