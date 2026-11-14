use alloc::{string::String, vec::Vec};
#[cfg(feature = "counters")]
use core::sync::atomic::{AtomicIsize, Ordering};
use core::{fmt, ops::Range};

/// An internal counter for debugging purposes
///
/// Internally represented as an atomic isize if the `counters` feature is enabled,
/// or compiles to nothing otherwise.
pub struct InternalCounter {
    #[cfg(feature = "counters")]
    value: AtomicIsize,
}

impl InternalCounter {
    /// Creates a counter with value 0.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        InternalCounter {
            #[cfg(feature = "counters")]
            value: AtomicIsize::new(0),
        }
    }

    /// Get the counter's value.
    #[cfg(feature = "counters")]
    #[inline]
    pub fn read(&self) -> isize {
        self.value.load(Ordering::Relaxed)
    }

    /// Get the counter's value.
    ///
    /// Always returns 0 if the `counters` feature is not enabled.
    #[cfg(not(feature = "counters"))]
    #[inline]
    #[must_use]
    pub fn read(&self) -> isize {
        0
    }

    /// Get and reset the counter's value.
    ///
    /// Always returns 0 if the `counters` feature is not enabled.
    #[cfg(feature = "counters")]
    #[inline]
    pub fn take(&self) -> isize {
        self.value.swap(0, Ordering::Relaxed)
    }

    /// Get and reset the counter's value.
    ///
    /// Always returns 0 if the `counters` feature is not enabled.
    #[cfg(not(feature = "counters"))]
    #[inline]
    #[must_use]
    pub fn take(&self) -> isize {
        0
    }

    /// Increment the counter by the provided amount.
    #[inline]
    pub fn add(&self, _val: isize) {
        #[cfg(feature = "counters")]
        self.value.fetch_add(_val, Ordering::Relaxed);
    }

    /// Decrement the counter by the provided amount.
    #[inline]
    pub fn sub(&self, _val: isize) {
        #[cfg(feature = "counters")]
        self.value.fetch_add(-_val, Ordering::Relaxed);
    }

    /// Sets the counter to the provided value.
    #[inline]
    pub fn set(&self, _val: isize) {
        #[cfg(feature = "counters")]
        self.value.store(_val, Ordering::Relaxed);
    }
}

/// A point in time to measure an [`InternalCounter`] duration from.
///
/// Reads the clock only with the `counters` and `std` features on a target other than
/// `wasm32`; otherwise it is empty and every duration is zero.
#[derive(Clone, Copy, Debug)]
pub struct CounterInstant {
    #[cfg(all(feature = "counters", feature = "std", not(target_arch = "wasm32")))]
    at: std::time::Instant,
}

impl CounterInstant {
    /// The current time.
    #[inline]
    #[must_use]
    pub fn now() -> Self {
        Self {
            #[cfg(all(feature = "counters", feature = "std", not(target_arch = "wasm32")))]
            at: std::time::Instant::now(),
        }
    }
}

impl InternalCounter {
    /// Adds the nanoseconds elapsed since `start` and returns the time it read, so
    /// consecutive phases cost one clock read each.
    #[inline]
    pub fn add_elapsed_since(&self, start: CounterInstant) -> CounterInstant {
        let now = CounterInstant::now();
        #[cfg(all(feature = "counters", feature = "std", not(target_arch = "wasm32")))]
        self.add(isize::try_from(now.at.duration_since(start.at).as_nanos()).unwrap_or(isize::MAX));
        #[cfg(not(all(feature = "counters", feature = "std", not(target_arch = "wasm32"))))]
        let _ = start;
        now
    }
}

impl Clone for InternalCounter {
    fn clone(&self) -> Self {
        InternalCounter {
            #[cfg(feature = "counters")]
            value: AtomicIsize::new(self.read()),
        }
    }
}

impl Default for InternalCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for InternalCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

/// `wgpu-hal`'s part of [`InternalCounters`].
#[allow(missing_docs)]
#[derive(Clone, Debug, Default)]
pub struct HalCounters {
    // API objects
    pub buffers: InternalCounter,
    pub textures: InternalCounter,
    pub texture_views: InternalCounter,
    pub bind_groups: InternalCounter,
    pub bind_group_layouts: InternalCounter,
    pub render_pipelines: InternalCounter,
    pub compute_pipelines: InternalCounter,
    pub ray_tracing_pipelines: InternalCounter,
    pub pipeline_layouts: InternalCounter,
    pub samplers: InternalCounter,
    pub command_encoders: InternalCounter,
    pub shader_modules: InternalCounter,
    pub query_sets: InternalCounter,
    pub fences: InternalCounter,

    // Resources
    /// Amount of allocated gpu memory attributed to buffers, in bytes.
    pub buffer_memory: InternalCounter,
    /// Amount of allocated gpu memory attributed to textures, in bytes.
    pub texture_memory: InternalCounter,
    /// Amount of allocated gpu memory attributed to acceleration structures, in bytes.
    pub acceleration_structure_memory: InternalCounter,
    /// Number of gpu memory allocations.
    pub memory_allocations: InternalCounter,

    /// Command encoding work.
    pub encode: HalEncodeCounters,
}

/// Command encoding work done by a `wgpu-hal` backend, summed since the device was created.
///
/// Read two snapshots and subtract them to get the work of an interval, such as one frame.
/// Fields ending in `_ns` are nanoseconds spent, including the driver calls made. Only the
/// Vulkan backend sets these.
#[derive(Clone, Debug, Default)]
pub struct HalEncodeCounters {
    /// Render passes begun.
    pub render_passes: InternalCounter,
    /// Time beginning render passes, including looking up or creating their render pass and
    /// framebuffer objects.
    pub render_pass_begin_ns: InternalCounter,
    /// Framebuffers created for render passes because none was cached for their attachments.
    pub framebuffers_created: InternalCounter,
    /// Time creating those framebuffers, part of `render_pass_begin_ns`.
    pub framebuffer_create_ns: InternalCounter,
    /// Texture-to-texture copies encoded.
    pub texture_copies: InternalCounter,
    /// Buffer-to-buffer, buffer-to-texture and texture-to-buffer copies encoded.
    pub other_copies: InternalCounter,
    /// Time encoding all those copies.
    pub copy_ns: InternalCounter,
    /// Buffer and texture barriers encoded.
    pub barriers: InternalCounter,
    /// Time encoding barriers.
    pub barrier_ns: InternalCounter,
    /// Time ending command buffers.
    pub command_buffer_end_ns: InternalCounter,
    /// Time resetting the command pools of finished command encoders for reuse, including
    /// destroying the framebuffers they owned.
    pub command_pool_reset_ns: InternalCounter,
}

/// `wgpu-core`'s part of [`InternalCounters`].
#[derive(Clone, Debug, Default)]
pub struct CoreCounters {
    /// Command encoding work.
    pub encode: CoreEncodeCounters,
}

/// Work done by `wgpu-core` to finish command encoders, summed since the device was created.
///
/// Finishing an encoder validates the commands recorded into it and encodes them into the
/// backend. Read two snapshots and subtract them to get the work of an interval, such as one
/// frame. Fields ending in `_ns` are nanoseconds spent.
#[derive(Clone, Debug, Default)]
pub struct CoreEncodeCounters {
    /// Command encoders finished.
    pub encoders: InternalCounter,
    /// Time finishing command encoders, including the two pass phases below.
    pub finish_ns: InternalCounter,
    /// Time encoding what must precede each pass: discarded-surface fixups, query resets,
    /// merging the pass's resource usage into the encoder's tracker, the resulting barriers
    /// and indirect draw validation.
    pub pass_prepare_ns: InternalCounter,
    /// Time replaying recorded passes into the backend, from the start to the end of each
    /// pass.
    pub pass_replay_ns: InternalCounter,
}

/// All internal counters, exposed for debugging purposes.
///
/// Obtain this from
/// [`Device::get_internal_counters()`](../wgpu/struct.Device.html#method.get_internal_counters).
#[derive(Clone, Debug, Default)]
pub struct InternalCounters {
    /// `wgpu-core` counters.
    pub core: CoreCounters,
    /// `wgpu-hal` counters.
    pub hal: HalCounters,
}

/// Describes an allocation in the [`AllocatorReport`].
#[derive(Clone)]
pub struct AllocationReport {
    /// The name provided to the `allocate()` function.
    pub name: String,
    /// The offset in bytes of the allocation in its memory block.
    pub offset: u64,
    /// The size in bytes of the allocation.
    pub size: u64,
}

/// Describes a memory block in the [`AllocatorReport`].
#[derive(Clone, Debug)]
pub struct MemoryBlockReport {
    /// The size in bytes of this memory block.
    pub size: u64,
    /// The range of allocations in [`AllocatorReport::allocations`] that are associated
    /// to this memory block.
    pub allocations: Range<usize>,
}

/// A report that can be generated for informational purposes using `Allocator::generate_report()`.
#[derive(Clone)]
pub struct AllocatorReport {
    /// All live allocations, sub-allocated from memory blocks.
    pub allocations: Vec<AllocationReport>,
    /// All memory blocks.
    pub blocks: Vec<MemoryBlockReport>,
    /// Sum of the memory used by all allocations, in bytes.
    pub total_allocated_bytes: u64,
    /// Sum of the memory reserved by all memory blocks including unallocated regions, in bytes.
    // XXX: Rename to total_capacity_bytes following the rename at https://github.com/Traverse-Research/gpu-allocator/pull/266?
    pub total_reserved_bytes: u64,
}

impl fmt::Debug for AllocationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = if !self.name.is_empty() {
            self.name.as_str()
        } else {
            "--"
        };
        write!(f, "{name:?}: {}", FmtBytes(self.size))
    }
}

impl fmt::Debug for AllocatorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut allocations = self.allocations.clone();
        allocations.sort_by_key(|alloc| core::cmp::Reverse(alloc.size));

        let max_num_allocations_to_print = f.precision().unwrap_or(usize::MAX);
        allocations.truncate(max_num_allocations_to_print);

        f.debug_struct("AllocatorReport")
            .field(
                "summary",
                &core::format_args!(
                    "{} / {}",
                    FmtBytes(self.total_allocated_bytes),
                    FmtBytes(self.total_reserved_bytes)
                ),
            )
            .field("blocks", &self.blocks.len())
            .field("allocations", &self.allocations.len())
            .field("largest", &allocations.as_slice())
            .finish()
    }
}

struct FmtBytes(u64);

impl fmt::Display for FmtBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SUFFIX: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        let mut idx = 0;
        let mut amount = self.0 as f64;
        loop {
            if amount < 1024.0 || idx == SUFFIX.len() - 1 {
                return write!(f, "{:.2} {}", amount, SUFFIX[idx]);
            }

            amount /= 1024.0;
            idx += 1;
        }
    }
}
