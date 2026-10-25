//! Recording of render and compute pass commands, so a pass can be encoded after the
//! barriers it needs, in the same raw command buffer.

use alloc::{boxed::Box, string::String, vec::Vec};
use core::{any::Any, mem, num::NonZeroU32, ops::Range};

use hal::{
    DynBindGroup, DynBuffer, DynCommandBuffer, DynComputePipeline, DynPipelineLayout, DynQuerySet,
    DynRenderPipeline, DynTexture, DynTextureView,
};

/// The HAL commands of one render or compute pass, recorded for [replay] into a raw
/// command encoder.
///
/// A pass's barriers, discarded-surface fixups, query resets, and indirect draw
/// validation are only known once the whole pass has been validated. wgpu-core encodes the
/// pass into this recorder first, then encodes that preparation into the open raw command
/// buffer, then replays the pass after it. Encoding the pass straight into the raw encoder
/// would instead need a separate command buffer for the preparation, inserted before the
/// pass's command buffer.
///
/// Replaying issues exactly the recorded calls, in order. Only calls that are valid
/// inside a pass are recorded; the others panic.
///
/// # Lifetimes
///
/// The recorder keeps references to raw HAL objects after the calls that passed them
/// return, with their lifetimes erased to `'static`. Every such object belongs to a
/// resource that the command buffer's trackers, the command buffer itself, or the device
/// keeps alive, and the snatch guard held for all of command encoding keeps it from being
/// destroyed. A recording must therefore be replayed or [cleared] before the snatch guard
/// it was recorded under is released.
///
/// [replay]: PassRecorder::replay
/// [cleared]: PassRecorder::clear
#[derive(Debug, Default)]
pub(crate) struct PassRecorder {
    commands: Vec<Command>,
    /// Dynamic offsets and immediate data.
    words: Vec<u32>,
    /// Debug marker and pass labels.
    strings: String,
    buffer_barriers: Vec<hal::BufferBarrier<'static, dyn DynBuffer>>,
    texture_barriers: Vec<hal::TextureBarrier<'static, dyn DynTexture>>,
    color_attachments: Vec<Option<hal::ColorAttachment<'static, dyn DynTextureView>>>,
    render_passes: Vec<RenderPass>,
}

#[derive(Debug)]
struct RenderPass {
    label: Option<Range<usize>>,
    extent: wgt::Extent3d,
    sample_count: u32,
    color_attachments: Range<usize>,
    depth_stencil_attachment: Option<hal::DepthStencilAttachment<'static, dyn DynTextureView>>,
    multiview_mask: Option<NonZeroU32>,
    timestamp_writes: Option<hal::PassTimestampWrites<'static, dyn DynQuerySet>>,
    occlusion_query_set: Option<&'static dyn DynQuerySet>,
}

#[derive(Debug)]
enum Command {
    TransitionBuffers(Range<usize>),
    TransitionTextures(Range<usize>),
    SetBindGroup {
        layout: &'static dyn DynPipelineLayout,
        index: u32,
        group: &'static dyn DynBindGroup,
        dynamic_offsets: Range<usize>,
    },
    SetImmediates {
        layout: &'static dyn DynPipelineLayout,
        offset_bytes: u32,
        data: Range<usize>,
    },
    InsertDebugMarker(Range<usize>),
    BeginDebugMarker(Range<usize>),
    EndDebugMarker,
    BeginQuery(&'static dyn DynQuerySet, u32),
    EndQuery(&'static dyn DynQuerySet, u32),
    WriteTimestamp(&'static dyn DynQuerySet, u32),
    ResetQueries(&'static dyn DynQuerySet, Range<u32>),
    /// Index into [`PassRecorder::render_passes`].
    BeginRenderPass(usize),
    EndRenderPass,
    SetRenderPipeline(&'static dyn DynRenderPipeline),
    SetIndexBuffer(hal::BufferBinding<'static, dyn DynBuffer>, wgt::IndexFormat),
    SetVertexBuffer(u32, hal::BufferBinding<'static, dyn DynBuffer>),
    SetViewport(hal::Rect<f32>, Range<f32>),
    SetScissorRect(hal::Rect<u32>),
    SetStencilReference(u32),
    SetBlendConstants([f32; 4]),
    Draw {
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    },
    DrawIndexed {
        first_index: u32,
        index_count: u32,
        base_vertex: i32,
        first_instance: u32,
        instance_count: u32,
    },
    DrawMeshTasks([u32; 3]),
    DrawIndirect(IndirectDraw),
    DrawIndexedIndirect(IndirectDraw),
    DrawMeshTasksIndirect(IndirectDraw),
    DrawIndirectCount(IndirectCountDraw),
    DrawIndexedIndirectCount(IndirectCountDraw),
    DrawMeshTasksIndirectCount(IndirectCountDraw),
    BeginComputePass {
        label: Option<Range<usize>>,
        timestamp_writes: Option<hal::PassTimestampWrites<'static, dyn DynQuerySet>>,
    },
    EndComputePass,
    SetComputePipeline(&'static dyn DynComputePipeline),
    DispatchWorkgroups([u32; 3]),
    DispatchWorkgroupsIndirect(&'static dyn DynBuffer, wgt::BufferAddress),
}

#[derive(Debug)]
struct IndirectDraw {
    buffer: &'static dyn DynBuffer,
    offset: wgt::BufferAddress,
    draw_count: u32,
}

#[derive(Debug)]
struct IndirectCountDraw {
    buffer: &'static dyn DynBuffer,
    offset: wgt::BufferAddress,
    count_buffer: &'static dyn DynBuffer,
    count_offset: wgt::BufferAddress,
    max_count: u32,
}

/// Extends the lifetime of a reference to a raw HAL object to `'static`.
///
/// # Safety
///
/// The referent must stay alive until the recording holding the returned reference is
/// replayed or cleared. See [`PassRecorder`].
unsafe fn erase<T: ?Sized>(object: &T) -> &'static T {
    // SAFETY: the caller keeps `object` alive for as long as the result is used.
    unsafe { &*core::ptr::from_ref(object) }
}

/// # Safety
///
/// See [`erase`].
unsafe fn erase_attachment(
    attachment: &hal::Attachment<'_, dyn DynTextureView>,
) -> hal::Attachment<'static, dyn DynTextureView> {
    hal::Attachment {
        view: unsafe { erase(attachment.view) },
        usage: attachment.usage,
    }
}

/// # Safety
///
/// See [`erase`].
unsafe fn erase_timestamp_writes(
    timestamp_writes: &hal::PassTimestampWrites<'_, dyn DynQuerySet>,
) -> hal::PassTimestampWrites<'static, dyn DynQuerySet> {
    hal::PassTimestampWrites {
        query_set: unsafe { erase(timestamp_writes.query_set) },
        beginning_of_pass_write_index: timestamp_writes.beginning_of_pass_write_index,
        end_of_pass_write_index: timestamp_writes.end_of_pass_write_index,
    }
}

fn copy_attachment<'a>(
    attachment: &hal::Attachment<'a, dyn DynTextureView>,
) -> hal::Attachment<'a, dyn DynTextureView> {
    hal::Attachment {
        view: attachment.view,
        usage: attachment.usage,
    }
}

fn copy_timestamp_writes<'a>(
    timestamp_writes: &hal::PassTimestampWrites<'a, dyn DynQuerySet>,
) -> hal::PassTimestampWrites<'a, dyn DynQuerySet> {
    hal::PassTimestampWrites {
        query_set: timestamp_writes.query_set,
        beginning_of_pass_write_index: timestamp_writes.beginning_of_pass_write_index,
        end_of_pass_write_index: timestamp_writes.end_of_pass_write_index,
    }
}

impl PassRecorder {
    /// Encodes the recorded commands into `encoder`, in order, and clears the recording.
    ///
    /// # Safety
    ///
    /// `encoder` must be open, and every call in the recording must be valid to encode
    /// at this point of `encoder`'s command stream.
    pub(crate) unsafe fn replay(
        &mut self,
        encoder: &mut dyn hal::DynCommandEncoder,
    ) -> Result<(), hal::DeviceError> {
        let result = unsafe { self.encode(encoder) };
        self.clear();
        result
    }

    /// Drops the recording without encoding it.
    pub(crate) fn clear(&mut self) {
        self.commands.clear();
        self.words.clear();
        self.strings.clear();
        self.buffer_barriers.clear();
        self.texture_barriers.clear();
        self.color_attachments.clear();
        self.render_passes.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    fn push_words(&mut self, words: &[u32]) -> Range<usize> {
        let start = self.words.len();
        self.words.extend_from_slice(words);
        start..self.words.len()
    }

    fn push_str(&mut self, string: &str) -> Range<usize> {
        let start = self.strings.len();
        self.strings.push_str(string);
        start..self.strings.len()
    }

    unsafe fn encode(
        &self,
        encoder: &mut dyn hal::DynCommandEncoder,
    ) -> Result<(), hal::DeviceError> {
        for command in &self.commands {
            match command {
                Command::TransitionBuffers(range) => unsafe {
                    encoder.transition_buffers(&self.buffer_barriers[range.clone()]);
                },
                Command::TransitionTextures(range) => unsafe {
                    encoder.transition_textures(&self.texture_barriers[range.clone()]);
                },
                Command::SetBindGroup {
                    layout,
                    index,
                    group,
                    dynamic_offsets,
                } => unsafe {
                    encoder.set_bind_group(
                        *layout,
                        *index,
                        *group,
                        &self.words[dynamic_offsets.clone()],
                    );
                },
                Command::SetImmediates {
                    layout,
                    offset_bytes,
                    data,
                } => unsafe {
                    encoder.set_immediates(*layout, *offset_bytes, &self.words[data.clone()]);
                },
                Command::InsertDebugMarker(label) => unsafe {
                    encoder.insert_debug_marker(&self.strings[label.clone()]);
                },
                Command::BeginDebugMarker(label) => unsafe {
                    encoder.begin_debug_marker(&self.strings[label.clone()]);
                },
                Command::EndDebugMarker => unsafe { encoder.end_debug_marker() },
                Command::BeginQuery(set, index) => unsafe { encoder.begin_query(*set, *index) },
                Command::EndQuery(set, index) => unsafe { encoder.end_query(*set, *index) },
                Command::WriteTimestamp(set, index) => unsafe {
                    encoder.write_timestamp(*set, *index);
                },
                Command::ResetQueries(set, range) => unsafe {
                    encoder.reset_queries(*set, range.clone());
                },
                Command::BeginRenderPass(index) => {
                    let pass = &self.render_passes[*index];
                    let desc = hal::RenderPassDescriptor {
                        label: pass.label.clone().map(|label| &self.strings[label]),
                        extent: pass.extent,
                        sample_count: pass.sample_count,
                        color_attachments: &self.color_attachments[pass.color_attachments.clone()],
                        depth_stencil_attachment: pass.depth_stencil_attachment.as_ref().map(
                            |ds| hal::DepthStencilAttachment {
                                target: copy_attachment(&ds.target),
                                depth_ops: ds.depth_ops,
                                stencil_ops: ds.stencil_ops,
                                clear_value: ds.clear_value,
                            },
                        ),
                        multiview_mask: pass.multiview_mask,
                        timestamp_writes: pass.timestamp_writes.as_ref().map(copy_timestamp_writes),
                        occlusion_query_set: pass.occlusion_query_set,
                    };
                    unsafe { encoder.begin_render_pass(&desc) }?;
                }
                Command::EndRenderPass => unsafe { encoder.end_render_pass() },
                Command::SetRenderPipeline(pipeline) => unsafe {
                    encoder.set_render_pipeline(*pipeline);
                },
                Command::SetIndexBuffer(binding, format) => unsafe {
                    encoder.set_index_buffer(binding.clone(), *format);
                },
                Command::SetVertexBuffer(index, binding) => unsafe {
                    encoder.set_vertex_buffer(*index, binding.clone());
                },
                Command::SetViewport(rect, depth_range) => unsafe {
                    encoder.set_viewport(rect, depth_range.clone());
                },
                Command::SetScissorRect(rect) => unsafe { encoder.set_scissor_rect(rect) },
                Command::SetStencilReference(value) => unsafe {
                    encoder.set_stencil_reference(*value);
                },
                Command::SetBlendConstants(color) => unsafe {
                    encoder.set_blend_constants(color);
                },
                &Command::Draw {
                    first_vertex,
                    vertex_count,
                    first_instance,
                    instance_count,
                } => unsafe {
                    encoder.draw(first_vertex, vertex_count, first_instance, instance_count);
                },
                &Command::DrawIndexed {
                    first_index,
                    index_count,
                    base_vertex,
                    first_instance,
                    instance_count,
                } => unsafe {
                    encoder.draw_indexed(
                        first_index,
                        index_count,
                        base_vertex,
                        first_instance,
                        instance_count,
                    );
                },
                &Command::DrawMeshTasks([x, y, z]) => unsafe { encoder.draw_mesh_tasks(x, y, z) },
                Command::DrawIndirect(draw) => unsafe {
                    encoder.draw_indirect(draw.buffer, draw.offset, draw.draw_count);
                },
                Command::DrawIndexedIndirect(draw) => unsafe {
                    encoder.draw_indexed_indirect(draw.buffer, draw.offset, draw.draw_count);
                },
                Command::DrawMeshTasksIndirect(draw) => unsafe {
                    encoder.draw_mesh_tasks_indirect(draw.buffer, draw.offset, draw.draw_count);
                },
                Command::DrawIndirectCount(draw) => unsafe {
                    encoder.draw_indirect_count(
                        draw.buffer,
                        draw.offset,
                        draw.count_buffer,
                        draw.count_offset,
                        draw.max_count,
                    );
                },
                Command::DrawIndexedIndirectCount(draw) => unsafe {
                    encoder.draw_indexed_indirect_count(
                        draw.buffer,
                        draw.offset,
                        draw.count_buffer,
                        draw.count_offset,
                        draw.max_count,
                    );
                },
                Command::DrawMeshTasksIndirectCount(draw) => unsafe {
                    encoder.draw_mesh_tasks_indirect_count(
                        draw.buffer,
                        draw.offset,
                        draw.count_buffer,
                        draw.count_offset,
                        draw.max_count,
                    );
                },
                Command::BeginComputePass {
                    label,
                    timestamp_writes,
                } => {
                    let desc = hal::ComputePassDescriptor {
                        label: label.clone().map(|label| &self.strings[label]),
                        timestamp_writes: timestamp_writes.as_ref().map(copy_timestamp_writes),
                    };
                    unsafe { encoder.begin_compute_pass(&desc) };
                }
                Command::EndComputePass => unsafe { encoder.end_compute_pass() },
                Command::SetComputePipeline(pipeline) => unsafe {
                    encoder.set_compute_pipeline(*pipeline);
                },
                Command::DispatchWorkgroups(count) => unsafe {
                    encoder.dispatch_workgroups(*count);
                },
                Command::DispatchWorkgroupsIndirect(buffer, offset) => unsafe {
                    encoder.dispatch_workgroups_indirect(*buffer, *offset);
                },
            }
        }
        Ok(())
    }
}

impl hal::DynResource for PassRecorder {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Panics for HAL calls that are never made while encoding a pass.
fn not_recorded(operation: &str) -> ! {
    unreachable!("`{operation}` is not recorded in render or compute passes")
}

// SAFETY (for every method): the trait's contract requires the arguments to stay valid
// until the command buffer is executed, which covers the recording's lifetime; see
// `PassRecorder` for why the referenced objects outlive the recording.
impl hal::DynCommandEncoder for PassRecorder {
    unsafe fn begin_encoding(&mut self, _label: hal::Label) -> Result<(), hal::DeviceError> {
        not_recorded("begin_encoding")
    }

    unsafe fn discard_encoding(&mut self) {
        not_recorded("discard_encoding")
    }

    unsafe fn end_encoding(&mut self) -> Result<Box<dyn DynCommandBuffer>, hal::DeviceError> {
        not_recorded("end_encoding")
    }

    unsafe fn reset_all(&mut self, _command_buffers: Vec<Box<dyn DynCommandBuffer>>) {
        not_recorded("reset_all")
    }

    unsafe fn transition_buffers(&mut self, barriers: &[hal::BufferBarrier<'_, dyn DynBuffer>]) {
        let start = self.buffer_barriers.len();
        self.buffer_barriers
            .extend(barriers.iter().map(|barrier| hal::BufferBarrier {
                buffer: unsafe { erase(barrier.buffer) },
                usage: barrier.usage.clone(),
            }));
        self.commands.push(Command::TransitionBuffers(
            start..self.buffer_barriers.len(),
        ));
    }

    unsafe fn transition_textures(&mut self, barriers: &[hal::TextureBarrier<'_, dyn DynTexture>]) {
        let start = self.texture_barriers.len();
        self.texture_barriers
            .extend(barriers.iter().map(|barrier| hal::TextureBarrier {
                texture: unsafe { erase(barrier.texture) },
                range: barrier.range,
                usage: barrier.usage.clone(),
            }));
        self.commands.push(Command::TransitionTextures(
            start..self.texture_barriers.len(),
        ));
    }

    unsafe fn clear_buffer(&mut self, _buffer: &dyn DynBuffer, _range: hal::MemoryRange) {
        not_recorded("clear_buffer")
    }

    unsafe fn copy_buffer_to_buffer(
        &mut self,
        _src: &dyn DynBuffer,
        _dst: &dyn DynBuffer,
        _regions: &[hal::BufferCopy],
    ) {
        not_recorded("copy_buffer_to_buffer")
    }

    unsafe fn copy_texture_to_texture(
        &mut self,
        _src: &dyn DynTexture,
        _src_usage: wgt::TextureUses,
        _dst: &dyn DynTexture,
        _regions: &[hal::TextureCopy],
    ) {
        not_recorded("copy_texture_to_texture")
    }

    unsafe fn copy_buffer_to_texture(
        &mut self,
        _src: &dyn DynBuffer,
        _dst: &dyn DynTexture,
        _regions: &[hal::BufferTextureCopy],
    ) {
        not_recorded("copy_buffer_to_texture")
    }

    unsafe fn copy_texture_to_buffer(
        &mut self,
        _src: &dyn DynTexture,
        _src_usage: wgt::TextureUses,
        _dst: &dyn DynBuffer,
        _regions: &[hal::BufferTextureCopy],
    ) {
        not_recorded("copy_texture_to_buffer")
    }

    unsafe fn set_bind_group(
        &mut self,
        layout: &dyn DynPipelineLayout,
        index: u32,
        group: &dyn DynBindGroup,
        dynamic_offsets: &[wgt::DynamicOffset],
    ) {
        let dynamic_offsets = self.push_words(dynamic_offsets);
        self.commands.push(Command::SetBindGroup {
            layout: unsafe { erase(layout) },
            index,
            group: unsafe { erase(group) },
            dynamic_offsets,
        });
    }

    unsafe fn set_immediates(
        &mut self,
        layout: &dyn DynPipelineLayout,
        offset_bytes: u32,
        data: &[u32],
    ) {
        let data = self.push_words(data);
        self.commands.push(Command::SetImmediates {
            layout: unsafe { erase(layout) },
            offset_bytes,
            data,
        });
    }

    unsafe fn insert_debug_marker(&mut self, label: &str) {
        let label = self.push_str(label);
        self.commands.push(Command::InsertDebugMarker(label));
    }

    unsafe fn begin_debug_marker(&mut self, group_label: &str) {
        let label = self.push_str(group_label);
        self.commands.push(Command::BeginDebugMarker(label));
    }

    unsafe fn end_debug_marker(&mut self) {
        self.commands.push(Command::EndDebugMarker);
    }

    unsafe fn begin_query(&mut self, set: &dyn DynQuerySet, index: u32) {
        self.commands
            .push(Command::BeginQuery(unsafe { erase(set) }, index));
    }

    unsafe fn end_query(&mut self, set: &dyn DynQuerySet, index: u32) {
        self.commands
            .push(Command::EndQuery(unsafe { erase(set) }, index));
    }

    unsafe fn write_timestamp(&mut self, set: &dyn DynQuerySet, index: u32) {
        self.commands
            .push(Command::WriteTimestamp(unsafe { erase(set) }, index));
    }

    unsafe fn reset_queries(&mut self, set: &dyn DynQuerySet, range: Range<u32>) {
        self.commands
            .push(Command::ResetQueries(unsafe { erase(set) }, range));
    }

    unsafe fn copy_query_results(
        &mut self,
        _set: &dyn DynQuerySet,
        _range: Range<u32>,
        _buffer: &dyn DynBuffer,
        _offset: wgt::BufferAddress,
        _stride: wgt::BufferSize,
    ) {
        not_recorded("copy_query_results")
    }

    unsafe fn begin_render_pass(
        &mut self,
        desc: &hal::RenderPassDescriptor<dyn DynQuerySet, dyn DynTextureView>,
    ) -> Result<(), hal::DeviceError> {
        let label = desc.label.map(|label| self.push_str(label));
        let start = self.color_attachments.len();
        self.color_attachments
            .extend(desc.color_attachments.iter().map(|attachment| {
                attachment.as_ref().map(|at| hal::ColorAttachment {
                    target: unsafe { erase_attachment(&at.target) },
                    depth_slice: at.depth_slice,
                    resolve_target: at
                        .resolve_target
                        .as_ref()
                        .map(|target| unsafe { erase_attachment(target) }),
                    ops: at.ops,
                    clear_value: at.clear_value,
                })
            }));
        let pass = RenderPass {
            label,
            extent: desc.extent,
            sample_count: desc.sample_count,
            color_attachments: start..self.color_attachments.len(),
            depth_stencil_attachment: desc.depth_stencil_attachment.as_ref().map(|ds| {
                hal::DepthStencilAttachment {
                    target: unsafe { erase_attachment(&ds.target) },
                    depth_ops: ds.depth_ops,
                    stencil_ops: ds.stencil_ops,
                    clear_value: ds.clear_value,
                }
            }),
            multiview_mask: desc.multiview_mask,
            timestamp_writes: desc
                .timestamp_writes
                .as_ref()
                .map(|tw| unsafe { erase_timestamp_writes(tw) }),
            occlusion_query_set: desc.occlusion_query_set.map(|set| unsafe { erase(set) }),
        };
        self.commands
            .push(Command::BeginRenderPass(self.render_passes.len()));
        self.render_passes.push(pass);
        Ok(())
    }

    unsafe fn end_render_pass(&mut self) {
        self.commands.push(Command::EndRenderPass);
    }

    unsafe fn set_render_pipeline(&mut self, pipeline: &dyn DynRenderPipeline) {
        self.commands
            .push(Command::SetRenderPipeline(unsafe { erase(pipeline) }));
    }

    unsafe fn set_index_buffer<'a>(
        &mut self,
        binding: hal::BufferBinding<'a, dyn DynBuffer>,
        format: wgt::IndexFormat,
    ) {
        // SAFETY: only the lifetime changes; see `erase`.
        let binding = unsafe {
            mem::transmute::<
                hal::BufferBinding<'a, dyn DynBuffer>,
                hal::BufferBinding<'static, dyn DynBuffer>,
            >(binding)
        };
        self.commands.push(Command::SetIndexBuffer(binding, format));
    }

    unsafe fn set_vertex_buffer<'a>(
        &mut self,
        index: u32,
        binding: hal::BufferBinding<'a, dyn DynBuffer>,
    ) {
        // SAFETY: only the lifetime changes; see `erase`.
        let binding = unsafe {
            mem::transmute::<
                hal::BufferBinding<'a, dyn DynBuffer>,
                hal::BufferBinding<'static, dyn DynBuffer>,
            >(binding)
        };
        self.commands.push(Command::SetVertexBuffer(index, binding));
    }

    unsafe fn set_viewport(&mut self, rect: &hal::Rect<f32>, depth_range: Range<f32>) {
        self.commands
            .push(Command::SetViewport(rect.clone(), depth_range));
    }

    unsafe fn set_scissor_rect(&mut self, rect: &hal::Rect<u32>) {
        self.commands.push(Command::SetScissorRect(rect.clone()));
    }

    unsafe fn set_stencil_reference(&mut self, value: u32) {
        self.commands.push(Command::SetStencilReference(value));
    }

    unsafe fn set_blend_constants(&mut self, color: &[f32; 4]) {
        self.commands.push(Command::SetBlendConstants(*color));
    }

    unsafe fn draw(
        &mut self,
        first_vertex: u32,
        vertex_count: u32,
        first_instance: u32,
        instance_count: u32,
    ) {
        self.commands.push(Command::Draw {
            first_vertex,
            vertex_count,
            first_instance,
            instance_count,
        });
    }

    unsafe fn draw_indexed(
        &mut self,
        first_index: u32,
        index_count: u32,
        base_vertex: i32,
        first_instance: u32,
        instance_count: u32,
    ) {
        self.commands.push(Command::DrawIndexed {
            first_index,
            index_count,
            base_vertex,
            first_instance,
            instance_count,
        });
    }

    unsafe fn draw_mesh_tasks(
        &mut self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) {
        self.commands.push(Command::DrawMeshTasks([
            group_count_x,
            group_count_y,
            group_count_z,
        ]));
    }

    unsafe fn draw_indirect(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.commands.push(Command::DrawIndirect(IndirectDraw {
            buffer: unsafe { erase(buffer) },
            offset,
            draw_count,
        }));
    }

    unsafe fn draw_indexed_indirect(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.commands
            .push(Command::DrawIndexedIndirect(IndirectDraw {
                buffer: unsafe { erase(buffer) },
                offset,
                draw_count,
            }));
    }

    unsafe fn draw_mesh_tasks_indirect(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
        draw_count: u32,
    ) {
        self.commands
            .push(Command::DrawMeshTasksIndirect(IndirectDraw {
                buffer: unsafe { erase(buffer) },
                offset,
                draw_count,
            }));
    }

    unsafe fn draw_indirect_count(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
        count_buffer: &dyn DynBuffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.commands
            .push(Command::DrawIndirectCount(IndirectCountDraw {
                buffer: unsafe { erase(buffer) },
                offset,
                count_buffer: unsafe { erase(count_buffer) },
                count_offset,
                max_count,
            }));
    }

    unsafe fn draw_indexed_indirect_count(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
        count_buffer: &dyn DynBuffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.commands
            .push(Command::DrawIndexedIndirectCount(IndirectCountDraw {
                buffer: unsafe { erase(buffer) },
                offset,
                count_buffer: unsafe { erase(count_buffer) },
                count_offset,
                max_count,
            }));
    }

    unsafe fn draw_mesh_tasks_indirect_count(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
        count_buffer: &dyn DynBuffer,
        count_offset: wgt::BufferAddress,
        max_count: u32,
    ) {
        self.commands
            .push(Command::DrawMeshTasksIndirectCount(IndirectCountDraw {
                buffer: unsafe { erase(buffer) },
                offset,
                count_buffer: unsafe { erase(count_buffer) },
                count_offset,
                max_count,
            }));
    }

    unsafe fn begin_compute_pass(&mut self, desc: &hal::ComputePassDescriptor<dyn DynQuerySet>) {
        let label = desc.label.map(|label| self.push_str(label));
        let timestamp_writes = desc
            .timestamp_writes
            .as_ref()
            .map(|tw| unsafe { erase_timestamp_writes(tw) });
        self.commands.push(Command::BeginComputePass {
            label,
            timestamp_writes,
        });
    }

    unsafe fn end_compute_pass(&mut self) {
        self.commands.push(Command::EndComputePass);
    }

    unsafe fn set_compute_pipeline(&mut self, pipeline: &dyn DynComputePipeline) {
        self.commands
            .push(Command::SetComputePipeline(unsafe { erase(pipeline) }));
    }

    unsafe fn dispatch_workgroups(&mut self, count: [u32; 3]) {
        self.commands.push(Command::DispatchWorkgroups(count));
    }

    unsafe fn dispatch_workgroups_indirect(
        &mut self,
        buffer: &dyn DynBuffer,
        offset: wgt::BufferAddress,
    ) {
        self.commands.push(Command::DispatchWorkgroupsIndirect(
            unsafe { erase(buffer) },
            offset,
        ));
    }

    unsafe fn begin_ray_tracing_pass(&mut self, _desc: &hal::RayTracingPassDescriptor) {
        not_recorded("begin_ray_tracing_pass")
    }

    unsafe fn end_ray_tracing_pass(&mut self) {
        not_recorded("end_ray_tracing_pass")
    }

    unsafe fn trace_rays(
        &mut self,
        _count: [u32; 3],
        _ray_generation_group_data: hal::PipelineGroupData<dyn DynBuffer>,
        _miss_group_data: hal::PipelineGroupData<dyn DynBuffer>,
        _intersection_group_data: hal::PipelineGroupData<dyn DynBuffer>,
    ) {
        not_recorded("trace_rays")
    }

    unsafe fn set_ray_tracing_pipeline(&mut self, _pipeline: &dyn hal::DynRayTracingPipeline) {
        not_recorded("set_ray_tracing_pipeline")
    }

    unsafe fn build_acceleration_structures<'a>(
        &mut self,
        _descriptors: &'a [hal::BuildAccelerationStructureDescriptor<
            'a,
            dyn DynBuffer,
            dyn hal::DynAccelerationStructure,
        >],
    ) {
        not_recorded("build_acceleration_structures")
    }

    unsafe fn place_acceleration_structure_barrier(
        &mut self,
        _barrier: hal::AccelerationStructureBarrier,
    ) {
        not_recorded("place_acceleration_structure_barrier")
    }

    unsafe fn copy_acceleration_structure_to_acceleration_structure(
        &mut self,
        _src: &dyn hal::DynAccelerationStructure,
        _dst: &dyn hal::DynAccelerationStructure,
        _copy: wgt::AccelerationStructureCopy,
    ) {
        not_recorded("copy_acceleration_structure_to_acceleration_structure")
    }

    unsafe fn read_acceleration_structure_compact_size(
        &mut self,
        _acceleration_structure: &dyn hal::DynAccelerationStructure,
        _buf: &dyn DynBuffer,
    ) {
        not_recorded("read_acceleration_structure_compact_size")
    }

    unsafe fn set_acceleration_structure_dependencies(
        &self,
        _command_buffers: &[Box<dyn DynCommandBuffer>],
        _dependencies: &[&dyn hal::DynAccelerationStructure],
    ) {
        not_recorded("set_acceleration_structure_dependencies")
    }
}
