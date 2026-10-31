use alloc::{boxed::Box, vec::Vec};

use crate::{
    command::pass_recorder::PassRecorder,
    lock::{rank, Mutex},
};

/// A pool of free [`wgpu_hal::CommandEncoder`]s, owned by a `Device`.
///
/// Each encoder in this list is in the "closed" state.
///
/// Since a raw [`CommandEncoder`][ce] is itself a pool for allocating
/// raw [`CommandBuffer`][cb]s, this is a pool of pools.
///
/// It also pools the [`PassRecorder`]s used while encoding passes.
///
/// [`wgpu_hal::CommandEncoder`]: hal::CommandEncoder
/// [ce]: hal::CommandEncoder
/// [cb]: hal::Api::CommandBuffer
pub(crate) struct CommandAllocator {
    free_encoders: Mutex<Vec<Box<dyn hal::DynCommandEncoder>>>,
    /// Empty [`PassRecorder`]s, kept to reuse their allocations.
    free_pass_recorders: Mutex<Vec<PassRecorder>>,
}

impl CommandAllocator {
    pub(crate) fn new() -> Self {
        Self {
            free_encoders: Mutex::new(rank::COMMAND_ALLOCATOR_FREE_ENCODERS, Vec::new()),
            free_pass_recorders: Mutex::new(
                rank::COMMAND_ALLOCATOR_FREE_PASS_RECORDERS,
                Vec::new(),
            ),
        }
    }

    /// Return a fresh [`wgpu_hal::CommandEncoder`] in the "closed" state.
    ///
    /// If we have free encoders in the pool, take one of those. Otherwise,
    /// create a new one on `device`.
    ///
    /// [`wgpu_hal::CommandEncoder`]: hal::CommandEncoder
    pub(crate) fn acquire_encoder(
        &self,
        device: &dyn hal::DynDevice,
        queue: &dyn hal::DynQueue,
    ) -> Result<Box<dyn hal::DynCommandEncoder>, hal::DeviceError> {
        let mut free_encoders = self.free_encoders.lock();
        match free_encoders.pop() {
            Some(encoder) => Ok(encoder),
            None => unsafe {
                let hal_desc = hal::CommandEncoderDescriptor { label: None, queue };
                device.create_command_encoder(&hal_desc)
            },
        }
    }

    /// Add `encoder` back to the free pool.
    pub(crate) fn release_encoder(&self, encoder: Box<dyn hal::DynCommandEncoder>) {
        let mut free_encoders = self.free_encoders.lock();
        free_encoders.push(encoder);
    }

    /// Return an empty [`PassRecorder`], reusing a released one if there is one.
    pub(crate) fn acquire_pass_recorder(&self) -> PassRecorder {
        self.free_pass_recorders.lock().pop().unwrap_or_default()
    }

    /// Add `recorder` back to the free pool, dropping any recording it still holds.
    pub(crate) fn release_pass_recorder(&self, mut recorder: PassRecorder) {
        recorder.clear();
        self.free_pass_recorders.lock().push(recorder);
    }
}
