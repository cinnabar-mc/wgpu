//! Checks what tearing down a native swapchain destroys: everything when the device goes idle
//! or is lost, nothing when the wait fails otherwise, and only the swapchain when it is dropped
//! without a release.
//!
//! The device is opened on whichever Vulkan adapter is present, with the entry points that
//! wait for the device or destroy the swapchain's objects wrapped to record each call. The
//! swapchain handle is null, so destroying it only records the call.

use alloc::{sync::Arc, vec, vec::Vec};
use core::ffi::{c_char, c_void, CStr};
use std::sync::{Mutex as StdMutex, OnceLock};

use ash::vk;
use parking_lot::Mutex;

use super::{NativeSwapchain, SwapchainAcquireSemaphore, SwapchainPresentSemaphores};
use crate::{Device as _, Instance as _};

/// What the wrapped `vkDeviceWaitIdle` returns next.
static WAIT_RESULT: StdMutex<vk::Result> = StdMutex::new(vk::Result::SUCCESS);
/// Calls recorded by the wrappers, by entry point name.
static CALLS: StdMutex<Vec<&'static str>> = StdMutex::new(Vec::new());
static DEVICE: OnceLock<ash::DeviceFnV1_0> = OnceLock::new();
static GET_INSTANCE_PROC_ADDR: OnceLock<vk::PFN_vkGetInstanceProcAddr> = OnceLock::new();
static GET_DEVICE_PROC_ADDR: OnceLock<vk::PFN_vkGetDeviceProcAddr> = OnceLock::new();

fn record(name: &'static str) {
    CALLS.lock().unwrap().push(name);
}

fn calls(name: &str) -> usize {
    CALLS
        .lock()
        .unwrap()
        .iter()
        .filter(|call| **call == name)
        .count()
}

unsafe extern "system" fn device_wait_idle(_device: vk::Device) -> vk::Result {
    record("wait");
    *WAIT_RESULT.lock().unwrap()
}

unsafe extern "system" fn destroy_fence(
    device: vk::Device,
    fence: vk::Fence,
    allocator: *const vk::AllocationCallbacks<'_>,
) {
    record("fence");
    unsafe { (DEVICE.get().unwrap().destroy_fence)(device, fence, allocator) }
}

unsafe extern "system" fn destroy_semaphore(
    device: vk::Device,
    semaphore: vk::Semaphore,
    allocator: *const vk::AllocationCallbacks<'_>,
) {
    record("semaphore");
    unsafe { (DEVICE.get().unwrap().destroy_semaphore)(device, semaphore, allocator) }
}

unsafe extern "system" fn destroy_image_view(
    device: vk::Device,
    view: vk::ImageView,
    allocator: *const vk::AllocationCallbacks<'_>,
) {
    record("view");
    unsafe { (DEVICE.get().unwrap().destroy_image_view)(device, view, allocator) }
}

unsafe extern "system" fn destroy_swapchain(
    _device: vk::Device,
    _swapchain: vk::SwapchainKHR,
    _allocator: *const vk::AllocationCallbacks<'_>,
) {
    record("swapchain");
}

unsafe extern "system" fn get_device_proc_addr(
    device: vk::Device,
    name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    if unsafe { CStr::from_ptr(name) } == c"vkDestroySwapchainKHR" {
        return unsafe {
            core::mem::transmute::<*const c_void, vk::PFN_vkVoidFunction>(
                destroy_swapchain as *const c_void,
            )
        };
    }
    unsafe { (GET_DEVICE_PROC_ADDR.get().unwrap())(device, name) }
}

unsafe extern "system" fn get_instance_proc_addr(
    instance: vk::Instance,
    name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    if unsafe { CStr::from_ptr(name) } == c"vkGetDeviceProcAddr" {
        return unsafe {
            core::mem::transmute::<*const c_void, vk::PFN_vkVoidFunction>(
                get_device_proc_addr as *const c_void,
            )
        };
    }
    unsafe { (GET_INSTANCE_PROC_ADDR.get().unwrap())(instance, name) }
}

/// A swapchain over `image` with a fence, one acquire semaphore and one kept view.
unsafe fn swapchain(
    device: &crate::vulkan::Device,
    functor: &ash::khr::swapchain::Device,
    image: &mut crate::vulkan::Texture,
) -> NativeSwapchain {
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
    let fence = unsafe {
        device
            .shared
            .raw
            .create_fence(&vk::FenceCreateInfo::default(), None)
    }
    .unwrap();
    NativeSwapchain {
        raw: vk::SwapchainKHR::null(),
        functor: functor.clone(),
        device: Arc::clone(&device.shared),
        images: vec![image.raw],
        fence: Some(fence),
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
        acquire_semaphores: vec![Arc::new(Mutex::new(
            SwapchainAcquireSemaphore::new(&device.shared, 0).unwrap(),
        ))],
        next_acquire_index: 0,
        present_semaphores: vec![Arc::new(Mutex::new(SwapchainPresentSemaphores::new(0)))],
        next_present_time: None,
        leaked: false,
    }
}

#[test]
fn a_failed_idle_wait_leaks_the_swapchain_and_a_lost_device_does_not() {
    let instance = match unsafe {
        crate::vulkan::Instance::init(&crate::InstanceDescriptor {
            name: "swapchain teardown test",
            flags: wgt::InstanceFlags::empty(),
            memory_budget_thresholds: wgt::MemoryBudgetThresholds::default(),
            backend_options: wgt::BackendOptions::default(),
            telemetry: None,
            display: None,
        })
    } {
        Ok(instance) => instance,
        Err(error) => {
            std::eprintln!("missing fixture: Vulkan instance for swapchain teardown ({error})");
            return;
        }
    };
    let Some(exposed) = unsafe { instance.enumerate_adapters(None) }
        .into_iter()
        .next()
    else {
        std::eprintln!("missing fixture: Vulkan adapter for swapchain teardown");
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
    let _ = DEVICE.set(ash::DeviceFnV1_0::load(load));
    let wrapped = unsafe {
        ash::Device::load_with(
            |name| match name.to_bytes() {
                b"vkDeviceWaitIdle" => device_wait_idle as *const c_void,
                b"vkDestroyFence" => destroy_fence as *const c_void,
                b"vkDestroySemaphore" => destroy_semaphore as *const c_void,
                b"vkDestroyImageView" => destroy_image_view as *const c_void,
                _ => load(name),
            },
            raw.handle(),
        )
    };
    let _ = GET_INSTANCE_PROC_ADDR.set(adapter.instance.entry.static_fn().get_instance_proc_addr);
    let _ = GET_DEVICE_PROC_ADDR.set(adapter.instance.raw.fp_v1_0().get_device_proc_addr);
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
    let recording_instance = unsafe {
        ash::Instance::load(
            &ash::StaticFn {
                get_instance_proc_addr,
            },
            adapter.instance.raw.handle(),
        )
    };
    let functor = ash::khr::swapchain::Device::new(&recording_instance, &device.shared.raw);

    unsafe {
        let mut image = device
            .create_texture(&crate::TextureDescriptor {
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
            .unwrap();
        use crate::vulkan::swapchain::Swapchain as _;

        // A failed wait leaves in-flight work that may use any of the swapchain's objects.
        *WAIT_RESULT.lock().unwrap() = vk::Result::ERROR_OUT_OF_HOST_MEMORY;
        let mut leaked = swapchain(&device, &functor, &mut image);
        leaked.release_resources(&device);
        drop(leaked);
        assert_eq!(calls("wait"), 1);
        for object in ["fence", "semaphore", "view", "swapchain"] {
            assert_eq!(calls(object), 0, "a failed wait destroys no {object}");
        }

        // A lost device has completed its work, so everything is destroyed.
        *WAIT_RESULT.lock().unwrap() = vk::Result::ERROR_DEVICE_LOST;
        let mut lost = swapchain(&device, &functor, &mut image);
        lost.release_resources(&device);
        drop(lost);
        assert_eq!(calls("wait"), 2);
        assert_eq!(calls("fence"), 1);
        assert!(calls("semaphore") >= 1);
        assert_eq!(calls("view"), 1);
        assert_eq!(calls("swapchain"), 1);

        // Dropping without a release does not wait, so the kept views it cannot know to be
        // unused are leaked.
        *WAIT_RESULT.lock().unwrap() = vk::Result::SUCCESS;
        let semaphores = calls("semaphore");
        let unreleased = swapchain(&device, &functor, &mut image);
        drop(unreleased);
        assert_eq!(calls("wait"), 2, "dropping does not wait for the device");
        assert_eq!(calls("view"), 1, "dropping leaks the kept view");
        assert_eq!(calls("semaphore"), semaphores);
        assert_eq!(calls("swapchain"), 2);

        device.destroy_texture(image);
    }
    drop(open.queue);
    drop(device);
}
