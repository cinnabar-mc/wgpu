//! Checks that a release whose device wait fails destroys none of the kept or retired swapchain
//! views, and keeps them retired until a release whose wait succeeds.
//!
//! The device is opened on whichever Vulkan adapter is present, with `vkDeviceWaitIdle`
//! wrapped to return a chosen result and `vkDestroyImageView` wrapped to count calls.

use alloc::{vec, vec::Vec};
use core::{
    ffi::{c_void, CStr},
    sync::atomic::{AtomicUsize, Ordering},
};
use std::sync::{Mutex as StdMutex, OnceLock};

use ash::vk;

use super::owner_tests::{image, keep_view, swapchain};
use crate::{Device as _, Instance as _};

static WAIT_RESULT: StdMutex<vk::Result> = StdMutex::new(vk::Result::SUCCESS);
static VIEWS_DESTROYED: AtomicUsize = AtomicUsize::new(0);
static DRIVER: OnceLock<ash::DeviceFnV1_0> = OnceLock::new();

unsafe extern "system" fn device_wait_idle(_device: vk::Device) -> vk::Result {
    *WAIT_RESULT.lock().unwrap()
}

unsafe extern "system" fn destroy_image_view(
    device: vk::Device,
    view: vk::ImageView,
    allocator: *const vk::AllocationCallbacks<'_>,
) {
    VIEWS_DESTROYED.fetch_add(1, Ordering::Relaxed);
    unsafe { (DRIVER.get().unwrap().destroy_image_view)(device, view, allocator) }
}

#[test]
fn a_failed_wait_keeps_kept_and_retired_views_for_a_later_release() {
    let instance = match unsafe {
        crate::vulkan::Instance::init(&crate::InstanceDescriptor {
            name: "swapchain wait test",
            flags: wgt::InstanceFlags::empty(),
            memory_budget_thresholds: wgt::MemoryBudgetThresholds::default(),
            backend_options: wgt::BackendOptions::default(),
            telemetry: None,
            display: None,
        })
    } {
        Ok(instance) => instance,
        Err(error) => {
            std::eprintln!(
                "missing fixture: Vulkan instance for swapchain release waits ({error})"
            );
            return;
        }
    };
    let Some(exposed) = unsafe { instance.enumerate_adapters(None) }
        .into_iter()
        .next()
    else {
        std::eprintln!("missing fixture: Vulkan adapter for swapchain release waits");
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
                b"vkDeviceWaitIdle" => device_wait_idle as *const c_void,
                b"vkDestroyImageView" => destroy_image_view as *const c_void,
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
    let device = &open.device;
    let functor = ash::khr::swapchain::Device::new(&adapter.instance.raw, &device.shared.raw);
    let views_destroyed = || VIEWS_DESTROYED.load(Ordering::Relaxed);

    let mut first = image(device);
    let mut second = image(device);
    keep_view(device, &mut first);
    keep_view(device, &mut second);
    // A swapchain dropped without a release has its view retired.
    drop(swapchain(device, &functor, vec![first.raw]));
    assert_eq!(device.shared.retired_swapchain_views.lock().len(), 1);

    unsafe {
        use crate::vulkan::swapchain::Swapchain as _;
        *WAIT_RESULT.lock().unwrap() = vk::Result::ERROR_OUT_OF_HOST_MEMORY;
        swapchain(device, &functor, vec![second.raw]).release_resources(device);
        assert_eq!(
            views_destroyed(),
            0,
            "a failed wait destroys no kept or retired view"
        );
        assert!(device.shared.swapchain_views.lock().is_empty());
        assert_eq!(device.shared.retired_swapchain_views.lock().len(), 2);

        *WAIT_RESULT.lock().unwrap() = vk::Result::SUCCESS;
        swapchain(device, &functor, Vec::new()).release_resources(device);
        assert_eq!(
            views_destroyed(),
            2,
            "a successful wait destroys each view once"
        );
        assert!(device.shared.retired_swapchain_views.lock().is_empty());

        device.destroy_texture(first);
        device.destroy_texture(second);
    }
}
