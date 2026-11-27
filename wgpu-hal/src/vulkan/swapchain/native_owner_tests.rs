//! Checks that releasing a swapchain retires kept views only on the device that owns the
//! swapchain, even when `release_resources` is given another device whose cache holds a view
//! keyed by the same raw image handle.

use alloc::{sync::Arc, vec, vec::Vec};

use ash::vk;

use super::NativeSwapchain;
use crate::{Adapter as _, Device as _, Instance as _};

fn image(device: &crate::vulkan::Device) -> crate::vulkan::Texture {
    unsafe {
        device.create_texture(&crate::TextureDescriptor {
            label: None,
            size: wgt::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgt::TextureDimension::D2,
            format: wgt::TextureFormat::Rgba8Unorm,
            usage: wgt::TextureUses::COLOR_TARGET,
            memory_flags: crate::MemoryFlags::empty(),
            view_formats: Vec::new(),
        })
    }
    .unwrap()
}

/// Gives `image` a kept swapchain view on `device`.
fn keep_view(device: &crate::vulkan::Device, image: &mut crate::vulkan::Texture) {
    image.swapchain_image = true;
    let view = unsafe {
        device.create_texture_view(
            image,
            &crate::TextureViewDescriptor {
                label: None,
                format: wgt::TextureFormat::Rgba8Unorm,
                dimension: wgt::TextureViewDimension::D2,
                usage: wgt::TextureUses::COLOR_TARGET,
                range: wgt::ImageSubresourceRange::default(),
            },
        )
    }
    .unwrap();
    unsafe { device.destroy_texture_view(view) };
    image.swapchain_image = false;
}

#[test]
fn releasing_a_swapchain_retires_views_only_on_its_own_device() {
    let instance = match unsafe {
        crate::vulkan::Instance::init(&crate::InstanceDescriptor {
            name: "swapchain owner test",
            flags: wgt::InstanceFlags::empty(),
            memory_budget_thresholds: wgt::MemoryBudgetThresholds::default(),
            backend_options: wgt::BackendOptions::default(),
            telemetry: None,
            display: None,
        })
    } {
        Ok(instance) => instance,
        Err(error) => {
            std::eprintln!("missing fixture: Vulkan instance for swapchain ownership ({error})");
            return;
        }
    };
    let Some(exposed) = unsafe { instance.enumerate_adapters(None) }
        .into_iter()
        .next()
    else {
        std::eprintln!("missing fixture: Vulkan adapter for swapchain ownership");
        return;
    };
    let open = |_| {
        unsafe {
            exposed.adapter.open(
                wgt::Features::empty(),
                &exposed.capabilities.limits,
                &wgt::MemoryHints::default(),
            )
        }
        .expect("test device opens")
    };
    let owner = open(0);
    let other = open(1);

    let mut owned_image = image(&owner.device);
    keep_view(&owner.device, &mut owned_image);
    // The other device keeps a view under the same raw image handle.
    let mut other_image = image(&other.device);
    keep_view(&other.device, &mut other_image);
    {
        let mut views = other.device.shared.swapchain_views.lock();
        let (key, view) = views.drain().next().unwrap();
        views.insert(
            crate::vulkan::SwapchainViewKey {
                image: owned_image.raw,
                ..key
            },
            view,
        );
    }

    let mut swapchain = NativeSwapchain {
        raw: vk::SwapchainKHR::null(),
        functor: ash::khr::swapchain::Device::new(
            &exposed.adapter.instance.raw,
            &owner.device.shared.raw,
        ),
        device: Arc::clone(&owner.device.shared),
        images: vec![owned_image.raw],
        fence: None,
        config: crate::SurfaceConfiguration {
            maximum_frame_latency: 2,
            present_mode: wgt::PresentMode::Fifo,
            composite_alpha_mode: wgt::CompositeAlphaMode::Opaque,
            format: wgt::TextureFormat::Rgba8Unorm,
            color_space: wgt::SurfaceColorSpace::Srgb,
            extent: wgt::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            usage: wgt::TextureUses::COLOR_TARGET,
            view_formats: Vec::new(),
        },
        acquire_semaphores: Vec::new(),
        next_acquire_index: 0,
        present_semaphores: Vec::new(),
        next_present_time: None,
    };
    unsafe {
        use crate::vulkan::swapchain::Swapchain as _;
        swapchain.release_resources(&other.device);
    }
    drop(swapchain);

    assert!(
        owner.device.shared.swapchain_views.lock().is_empty(),
        "the owning device's kept view is released"
    );
    assert!(owner
        .device
        .shared
        .retired_swapchain_views
        .lock()
        .is_empty());
    assert_eq!(
        other.device.shared.swapchain_views.lock().len(),
        1,
        "the other device keeps its view"
    );

    unsafe {
        owner.device.destroy_texture(owned_image);
        other.device.destroy_texture(other_image);
    }
    drop(owner);
    drop(other);
}
