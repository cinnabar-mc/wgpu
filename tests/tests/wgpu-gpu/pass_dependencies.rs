//! A chain of render and compute passes in one command encoder, each reading
//! what the previous pass wrote, so every pass depends on the barriers encoded
//! before it.

use wgpu::util::DeviceExt as _;
use wgpu_test::{
    gpu_test, GpuTestConfiguration, GpuTestInitializer, TestParameters, TestingContext,
};

pub fn all_tests(vec: &mut Vec<GpuTestInitializer>) {
    vec.push(DEPENDENT_PASSES_IN_ONE_ENCODER);
}

/// Width and height of the texture, and the number of buffer elements per row.
const SIZE: u32 = 8;

/// Passes alternate compute (texture to buffer) and render (buffer to texture),
/// and each adds one to the value it reads.
const PASS_COUNT: u32 = 14;

const SHADER: &str = "
const SIZE: u32 = 8u;

@group(0) @binding(0) var input: texture_2d<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    output[id.y * SIZE + id.x] = textureLoad(input, id.xy, 0).x + 1u;
}

@group(0) @binding(2) var<storage, read> values: array<u32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let uv = vec2f(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4f(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4f) -> @location(0) u32 {
    let texel = vec2u(position.xy);
    return values[texel.y * SIZE + texel.x] + 1u;
}
";

#[gpu_test]
static DEPENDENT_PASSES_IN_ONE_ENCODER: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(TestParameters::default().test_features_limits())
    .run_async(dependent_passes_in_one_encoder);

async fn dependent_passes_in_one_encoder(ctx: TestingContext) {
    let device = &ctx.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::TextureFormat::R32Uint.into())],
        }),
        multiview_mask: None,
        cache: None,
    });

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    // A non-indexed draw of the full-screen triangle, then a 1x1x1 dispatch.
    let indirect = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[3u32, 1, 0, 0, 1, 1, 1]),
        usage: wgpu::BufferUsages::INDIRECT,
    });

    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &compute_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer.as_entire_binding(),
            },
        ],
    });
    let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &render_pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 2,
            resource: buffer.as_entire_binding(),
        }],
    });

    let padded_bytes_per_row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(padded_bytes_per_row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for pass in 0..PASS_COUNT {
        // Alternate direct and indirect calls of each pass kind.
        let indirect_call = pass % 4 >= 2;
        if pass % 2 == 0 {
            let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute.set_pipeline(&compute_pipeline);
            compute.set_bind_group(0, &compute_bind_group, &[]);
            if indirect_call {
                compute.dispatch_workgroups_indirect(&indirect, 16);
            } else {
                compute.dispatch_workgroups(1, 1, 1);
            }
        } else {
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render.set_pipeline(&render_pipeline);
            render.set_bind_group(0, &render_bind_group, &[]);
            if indirect_call {
                render.draw_indirect(&indirect, 0);
            } else {
                render.draw(0..3, 0..1);
            }
        }
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    ctx.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    ctx.async_poll(wgpu::PollType::wait_indefinitely())
        .await
        .unwrap();

    let data = slice.get_mapped_range().unwrap();
    for (row, bytes) in data.chunks(padded_bytes_per_row as usize).enumerate() {
        let texels: &[u32] = bytemuck::cast_slice(&bytes[..(SIZE * 4) as usize]);
        assert_eq!(texels, [PASS_COUNT; SIZE as usize], "row {row}");
    }
}
