//! Checks that render passes reuse the device's framebuffers across command encoder resets,
//! that destroying a view destroys its framebuffers, and that dropping the device destroys
//! the rest.
//!
//! The device is opened on whichever Vulkan adapter is present (lavapipe in CI), with
//! `vkCreateFramebuffer` and `vkDestroyFramebuffer` wrapped to record every handle.

use alloc::vec::Vec;
use core::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};

use ash::vk::{self, Handle as _};

use crate::{CommandEncoder as _, Device as _, Instance as _};

/// Framebuffers created through the wrapped entry point.
static CREATED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// Framebuffers destroyed through the wrapped entry point.
static DESTROYED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// The driver's own entry points that the wrappers forward to.
static DRIVER: OnceLock<ash::DeviceFnV1_0> = OnceLock::new();

fn driver() -> &'static ash::DeviceFnV1_0 {
    DRIVER.get().expect("driver entry points are loaded")
}

unsafe extern "system" fn create_framebuffer(
    device: vk::Device,
    info: *const vk::FramebufferCreateInfo<'_>,
    allocator: *const vk::AllocationCallbacks<'_>,
    framebuffer: *mut vk::Framebuffer,
) -> vk::Result {
    let result = unsafe { (driver().create_framebuffer)(device, info, allocator, framebuffer) };
    if result == vk::Result::SUCCESS {
        CREATED
            .lock()
            .unwrap()
            .push(unsafe { *framebuffer }.as_raw());
    }
    result
}

unsafe extern "system" fn destroy_framebuffer(
    device: vk::Device,
    framebuffer: vk::Framebuffer,
    allocator: *const vk::AllocationCallbacks<'_>,
) {
    DESTROYED.lock().unwrap().push(framebuffer.as_raw());
    unsafe { (driver().destroy_framebuffer)(device, framebuffer, allocator) }
}

/// The number of framebuffers created and destroyed so far.
fn counts() -> (usize, usize) {
    (
        CREATED.lock().unwrap().len(),
        DESTROYED.lock().unwrap().len(),
    )
}

/// Records one command buffer drawing nothing into a render pass on `view` (into `slice` of
/// a 3D view when given) and recycles it, as `wgpu-core` does once a submission completes.
unsafe fn render_and_reset(
    encoder: &mut super::CommandEncoder,
    view: &super::TextureView,
    slice: Option<u32>,
) {
    let extent = wgt::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
    };
    let color = crate::ColorAttachment {
        target: crate::Attachment {
            view,
            usage: wgt::TextureUses::COLOR_TARGET,
        },
        depth_slice: slice,
        resolve_target: None,
        ops: crate::AttachmentOps::LOAD_CLEAR | crate::AttachmentOps::STORE,
        clear_value: wgt::Color::BLACK,
    };
    unsafe {
        encoder.begin_encoding(None).unwrap();
        encoder
            .begin_render_pass(
                &crate::RenderPassDescriptor::<super::QuerySet, super::TextureView> {
                    label: None,
                    extent,
                    sample_count: 1,
                    color_attachments: &[Some(color)],
                    depth_stencil_attachment: None,
                    multiview_mask: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            )
            .unwrap();
        encoder.end_render_pass();
        let buffer = encoder.end_encoding().unwrap();
        encoder.reset_all(core::iter::once(buffer));
    }
}

/// Creates a 4×4 color target of `dimension` and a view of its first mip level.
unsafe fn target(
    device: &super::Device,
    dimension: wgt::TextureDimension,
) -> (super::Texture, super::TextureView) {
    let texture = unsafe {
        device.create_texture(&crate::TextureDescriptor {
            label: None,
            size: wgt::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: if dimension == wgt::TextureDimension::D3 {
                    2
                } else {
                    1
                },
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format: wgt::TextureFormat::Rgba8Unorm,
            usage: wgt::TextureUses::COLOR_TARGET,
            memory_flags: crate::MemoryFlags::empty(),
            view_formats: Vec::new(),
        })
    }
    .unwrap();
    let view = unsafe {
        device.create_texture_view(
            &texture,
            &crate::TextureViewDescriptor {
                label: None,
                format: wgt::TextureFormat::Rgba8Unorm,
                dimension: match dimension {
                    wgt::TextureDimension::D3 => wgt::TextureViewDimension::D3,
                    _ => wgt::TextureViewDimension::D2,
                },
                usage: wgt::TextureUses::COLOR_TARGET,
                range: wgt::ImageSubresourceRange::default(),
            },
        )
    }
    .unwrap();
    (texture, view)
}

#[test]
fn render_passes_share_device_framebuffers_until_a_view_is_destroyed() {
    let instance = match unsafe {
        super::Instance::init(&crate::InstanceDescriptor {
            name: "framebuffer cache test",
            flags: wgt::InstanceFlags::empty(),
            memory_budget_thresholds: wgt::MemoryBudgetThresholds::default(),
            backend_options: wgt::BackendOptions::default(),
            telemetry: None,
            display: None,
        })
    } {
        Ok(instance) => instance,
        Err(error) => {
            std::eprintln!("missing fixture: Vulkan instance for the framebuffer cache ({error})");
            return;
        }
    };
    let Some(exposed) = unsafe { instance.enumerate_adapters(None) }
        .into_iter()
        .next()
    else {
        std::eprintln!("missing fixture: Vulkan adapter for the framebuffer cache");
        return;
    };
    let adapter = exposed.adapter;

    let features = wgt::Features::empty();
    let extensions = adapter.required_device_extensions(features);
    let mut device_features = adapter.physical_device_features(&extensions, features);
    let queue = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(0)
        .queue_priorities(&[1.0]);
    let extension_names = extensions
        .iter()
        .map(|name| name.as_ptr())
        .collect::<Vec<_>>();
    let info = device_features.add_to_device_create(
        vk::DeviceCreateInfo::default()
            .queue_create_infos(core::slice::from_ref(&queue))
            .enabled_extension_names(&extension_names),
    );
    let raw = unsafe { adapter.instance.raw.create_device(adapter.raw, &info, None) }
        .expect("test device opens");
    let load = |name: &CStr| unsafe {
        core::mem::transmute::<vk::PFN_vkVoidFunction, *const c_void>(
            adapter
                .instance
                .raw
                .get_device_proc_addr(raw.handle(), name.as_ptr()),
        )
    };
    let _ = DRIVER.set(ash::DeviceFnV1_0::load(load));
    let wrapped = unsafe {
        ash::Device::load_with(
            |name| match name.to_bytes() {
                b"vkCreateFramebuffer" => create_framebuffer as *const c_void,
                b"vkDestroyFramebuffer" => destroy_framebuffer as *const c_void,
                _ => load(name),
            },
            raw.handle(),
        )
    };
    let open = unsafe {
        adapter.device_from_raw(
            wrapped,
            None,
            &extensions,
            features,
            &exposed.capabilities.limits,
            &wgt::MemoryHints::default(),
            0,
            0,
        )
    }
    .expect("test device wraps");
    let device = open.device;
    let encoder_desc = crate::CommandEncoderDescriptor {
        label: None,
        queue: &open.queue,
    };

    unsafe {
        let (texture, view) = target(&device, wgt::TextureDimension::D2);
        let mut first = device.create_command_encoder(&encoder_desc).unwrap();
        let mut second = device.create_command_encoder(&encoder_desc).unwrap();

        // A pass repeated every frame, on whichever encoder the frame gets, creates one
        // framebuffer, which outlives each encoder reset.
        for _ in 0..3 {
            render_and_reset(&mut first, &view, None);
            render_and_reset(&mut second, &view, None);
        }
        assert_eq!(counts(), (1, 0), "a repeated pass creates one framebuffer");

        // Destroying the view destroys the framebuffer attaching it.
        device.destroy_texture_view(view);
        assert_eq!(
            counts(),
            (1, 1),
            "destroying a view destroys its framebuffer"
        );
        device.destroy_texture(texture);

        // A slice of a 3D view is attached through a view the encoder owns, so its framebuffer
        // lives until the encoder is reset.
        let (volume, volume_view) = target(&device, wgt::TextureDimension::D3);
        render_and_reset(&mut first, &volume_view, Some(1));
        assert_eq!(
            counts(),
            (2, 2),
            "an encoder-owned framebuffer dies at reset"
        );
        device.destroy_texture_view(volume_view);
        device.destroy_texture(volume);

        // Dropping the device destroys framebuffers whose views are still alive.
        let (texture, view) = target(&device, wgt::TextureDimension::D2);
        render_and_reset(&mut first, &view, None);
        drop(first);
        drop(second);
        device.destroy_texture(texture);
        // The view is left alive on purpose, so only dropping the device can destroy its
        // framebuffer. The view itself leaks with the device.
        core::mem::forget(view);
    }
    drop(open.queue);
    drop(device);

    let created = CREATED.lock().unwrap().clone();
    let mut destroyed = DESTROYED.lock().unwrap().clone();
    assert_eq!(created.len(), 3);
    let mut created_sorted = created;
    created_sorted.sort_unstable();
    destroyed.sort_unstable();
    assert_eq!(
        created_sorted, destroyed,
        "every framebuffer is destroyed exactly once by the time the device is dropped"
    );
}
