//! Checks that update-after-bind pools stay within `maxUpdateAfterBindDescriptorsInAllPools`,
//! counted by what the pools are created with, and that empty pools of one layout give way to
//! another layout under pressure.

use super::{DescriptorAllocator, POOL_MIN_SETS};
use crate::{Adapter as _, Device as _, Instance as _};

/// Opens a device with update-after-bind binding arrays, or `None` without one.
fn device() -> Option<crate::OpenDevice<super::super::Api>> {
    let instance = match unsafe {
        super::super::Instance::init(&crate::InstanceDescriptor {
            name: "descriptor allocator test",
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
                "missing fixture: Vulkan instance for the descriptor allocator ({error})"
            );
            return None;
        }
    };
    let wanted = wgt::Features::TEXTURE_BINDING_ARRAY
        | wgt::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
    let Some(exposed) = unsafe { instance.enumerate_adapters(None) }
        .into_iter()
        .find(|exposed| exposed.features.contains(wanted))
    else {
        std::eprintln!(
            "missing fixture: Vulkan adapter with update-after-bind binding arrays for the descriptor allocator"
        );
        return None;
    };
    let open = unsafe {
        exposed.adapter.open(
            wanted,
            &exposed.capabilities.limits,
            &wgt::MemoryHints::default(),
        )
    }
    .expect("test device opens");
    Some(open)
}

/// A layout with one binding array of `count` sampled textures.
fn layout(device: &super::super::Device, count: u32) -> super::super::BindGroupLayout {
    let entries = [wgt::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgt::ShaderStages::FRAGMENT,
        ty: wgt::BindingType::Texture {
            sample_type: wgt::TextureSampleType::Float { filterable: true },
            view_dimension: wgt::TextureViewDimension::D2,
            multisampled: false,
        },
        count: core::num::NonZeroU32::new(count),
    }];
    unsafe {
        device.create_bind_group_layout(&crate::BindGroupLayoutDescriptor {
            label: None,
            flags: crate::BindGroupLayoutFlags::PARTIALLY_BOUND,
            entries: &entries,
        })
    }
    .expect("test layout")
}

#[test]
fn update_after_bind_pools_fit_the_limit_and_yield_when_empty() {
    let Some(open) = device() else {
        return;
    };
    let device = &open.device;
    let raw = &device.shared.raw;
    let small = layout(device, 8);
    let large = layout(device, 16);
    let oversized = layout(device, 600);
    assert!(small.contains_binding_arrays);
    // One full-size pool of the small layout, plus a little.
    let budget = POOL_MIN_SETS * 8 + 8;
    let mut allocator = DescriptorAllocator::new(budget);

    unsafe {
        allocator.register_layout(raw, &small).unwrap();
        allocator.register_layout(raw, &large).unwrap();
        allocator.register_layout(raw, &oversized).unwrap();
        assert_eq!(
            allocator.update_after_bind_descriptors_in_all_pools, 0,
            "registering a layout reserves nothing"
        );

        let set = allocator.alloc(raw, &small).unwrap();
        assert_eq!(
            allocator.update_after_bind_descriptors_in_all_pools,
            u64::from(POOL_MIN_SETS * 8),
            "a pool counts all the sets it can hold"
        );
        allocator.free(raw, set);

        // The small layout's empty pool is kept, and leaves no room for a large set until
        // it is destroyed.
        let large_set = allocator.alloc(raw, &large).unwrap();
        assert!(allocator.update_after_bind_descriptors_in_all_pools <= u64::from(budget));

        // What is left still holds a pool sized to fit it.
        let small_set = allocator.alloc(raw, &small).unwrap();
        assert!(allocator.update_after_bind_descriptors_in_all_pools <= u64::from(budget));

        assert!(matches!(
            allocator.alloc(raw, &oversized),
            Err(crate::DeviceError::OutOfMemory)
        ));

        allocator.free(raw, large_set);
        allocator.free(raw, small_set);
        allocator.unregister_layout(raw, &small);
        allocator.unregister_layout(raw, &large);
        allocator.unregister_layout(raw, &oversized);
        assert_eq!(allocator.update_after_bind_descriptors_in_all_pools, 0);

        device.destroy_bind_group_layout(small);
        device.destroy_bind_group_layout(large);
        device.destroy_bind_group_layout(oversized);
    }
}
