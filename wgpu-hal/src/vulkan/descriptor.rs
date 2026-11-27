use alloc::vec::Vec;
use arrayvec::ArrayVec;
use ash::vk;
use hashbrown::{HashMap, HashSet};

const POOL_MIN_SETS: u32 = 64;
const POOL_MAX_SETS: u32 = 512;

/// The dynamic counts are tracked separately since dynamic textures/buffers
/// have distinct descriptor types compared to their non-dynamic counterparts.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorCounts {
    pub sampler: u32,
    pub sampled_image: u32,
    pub storage_image: u32,
    pub uniform_buffer: u32,
    pub uniform_buffer_dynamic: u32,
    pub storage_buffer: u32,
    pub storage_buffer_dynamic: u32,
    pub acceleration_structure: u32,
}

impl DescriptorCounts {
    fn total(&self) -> u32 {
        self.sampler
            + self.sampled_image
            + self.storage_image
            + self.uniform_buffer
            + self.uniform_buffer_dynamic
            + self.storage_buffer
            + self.storage_buffer_dynamic
            + self.acceleration_structure
    }
}

#[derive(Debug)]
pub struct DescriptorSet {
    raw: vk::DescriptorSet,
    bucket_key: BucketKey,
    pool_index: usize,
}

impl DescriptorSet {
    pub fn raw(&self) -> vk::DescriptorSet {
        self.raw
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct BucketKey {
    counts: DescriptorCounts,
    update_after_bind: bool,
}

struct Pool {
    raw: vk::DescriptorPool,
    capacity: u32,
    available: u32,
}

/// Keeps track of all pools created with this bucket's [`BucketKey`].
#[derive(Default)]
struct Bucket {
    /// We keep track of all descriptor set layouts that might use this bucket.
    /// If the set becomes empty, the bucket is destroyed.
    layouts: HashSet<vk::DescriptorSetLayout>,
    pools: Vec<Pool>,
    available_sets: u32,
    allocated_sets: u32,
}

impl Bucket {
    fn create_pool(
        &mut self,
        device: &ash::Device,
        key: &BucketKey,
        capacity_hint: u32,
        min_capacity: u32,
        max_capacity: u32,
    ) -> Result<(usize, &mut Pool), vk::Result> {
        let pool = create_descriptor_pool(device, key, capacity_hint, min_capacity, max_capacity)?;
        self.available_sets += pool.capacity;
        // Reuse the slot of a destroyed pool, so the indices of live sets stay valid.
        let index = match self.pools.iter().position(|pool| pool.capacity == 0) {
            Some(index) => {
                self.pools[index] = pool;
                index
            }
            None => {
                self.pools.push(pool);
                self.pools.len() - 1
            }
        };
        Ok((index, &mut self.pools[index]))
    }

    /// Destroys this bucket's empty pools, keeping the slots of those in the middle so the
    /// indices of live sets stay valid. Returns the capacity, in sets, they held.
    fn destroy_empty_pools(&mut self, device: &ash::Device) -> u32 {
        let mut released = 0;
        for pool in &mut self.pools {
            if pool.capacity != 0 && pool.available == pool.capacity {
                unsafe { device.destroy_descriptor_pool(pool.raw, None) };
                released += pool.capacity;
                *pool = Pool::DESTROYED;
            }
        }
        while self.pools.last().is_some_and(|pool| pool.capacity == 0) {
            self.pools.pop();
        }
        self.available_sets -= released;
        released
    }
}

impl Pool {
    /// The slot of a destroyed pool; it has no sets to allocate.
    const DESTROYED: Pool = Pool {
        raw: vk::DescriptorPool::null(),
        capacity: 0,
        available: 0,
    };
}

pub struct DescriptorAllocator {
    buckets: HashMap<BucketKey, Bucket>,
    max_update_after_bind_descriptors_in_all_pools: u32,
    /// Descriptors that the update-after-bind pools hold, counted by pool capacity, since
    /// `maxUpdateAfterBindDescriptorsInAllPools` limits what the pools are created with.
    update_after_bind_descriptors_in_all_pools: u64,
}

impl super::BindGroupLayout {
    fn bucket_key(&self) -> BucketKey {
        let update_after_bind = self.contains_binding_arrays;
        let counts = self.desc_count.clone();
        BucketKey {
            counts,
            update_after_bind,
        }
    }
}

impl DescriptorAllocator {
    pub fn new(max_update_after_bind_descriptors_in_all_pools: u32) -> Self {
        DescriptorAllocator {
            buckets: HashMap::default(),
            max_update_after_bind_descriptors_in_all_pools,
            update_after_bind_descriptors_in_all_pools: 0,
        }
    }

    pub fn register_layout(
        &mut self,
        device: &ash::Device,
        layout: &super::BindGroupLayout,
    ) -> Result<(), crate::DeviceError> {
        let key = layout.bucket_key();
        let bucket = match self.buckets.entry(key) {
            hashbrown::hash_map::Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
            hashbrown::hash_map::Entry::Vacant(vacant_entry) => {
                let mut bucket = Bucket::default();
                // Create at least 1 pool upfront instead of doing this when creating the
                // first bind group. Update-after-bind pools are created on demand instead, so
                // a layout does not hold part of the shared limit before it allocates.
                if !vacant_entry.key().update_after_bind {
                    bucket
                        .create_pool(
                            device,
                            vacant_entry.key(),
                            POOL_MIN_SETS,
                            POOL_MIN_SETS,
                            u32::MAX,
                        )
                        .map_err(super::map_host_device_oom_and_fragmentation_err)?;
                }
                vacant_entry.insert(bucket)
            }
        };

        assert!(bucket.layouts.insert(layout.raw));

        Ok(())
    }

    pub fn unregister_layout(&mut self, device: &ash::Device, layout: &super::BindGroupLayout) {
        let key = layout.bucket_key();
        let bucket = self.buckets.get_mut(&key).unwrap();

        assert!(bucket.layouts.remove(&layout.raw));

        if bucket.layouts.is_empty() {
            // Remove the bucket and destroy any remaining pools.
            let bucket = self.buckets.remove(&key).unwrap();
            for pool in bucket.pools {
                assert_eq!(
                    pool.available, pool.capacity,
                    "pool is not empty, at least one DescriptorSet has not been freed"
                );
                unsafe { device.destroy_descriptor_pool(pool.raw, None) };
                self.release_capacity(&key, pool.capacity);
            }
        }
    }

    pub unsafe fn alloc(
        &mut self,
        device: &ash::Device,
        layout: &super::BindGroupLayout,
    ) -> Result<DescriptorSet, crate::DeviceError> {
        let key = layout.bucket_key();
        let pool_index = self.pool_with_room(device, &key)?;
        let pool = &mut self.buckets.get_mut(&key).unwrap().pools[pool_index];

        let vk_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool.raw)
            .set_layouts(core::slice::from_ref(&layout.raw));

        let raw = match unsafe { device.allocate_descriptor_sets(&vk_info) } {
            Ok(sets) => Ok(sets[0]),
            // We make sure not to exceed the size of the pool.
            Err(vk::Result::ERROR_OUT_OF_POOL_MEMORY) => unreachable!(),
            // We only allocate from a pool if the nr and type of descriptors
            // used by a layout is the same as those specified at pool
            // creation time.
            //
            // > Additionally, if all sets allocated from the pool since it was
            // created or most recently reset use the same number of
            // descriptors (of each type) and the requested allocation also
            // uses that same number of descriptors (of each type),
            // then fragmentation must not cause an allocation failure.
            //
            // from https://docs.vulkan.org/refpages/latest/refpages/source/VkDescriptorPoolCreateInfo.html#_description
            Err(vk::Result::ERROR_FRAGMENTED_POOL) => unreachable!(),
            Err(err) => Err(super::map_host_device_oom_err(err)),
        }?;

        pool.available -= 1;
        let bucket = self.buckets.get_mut(&key).unwrap();
        bucket.available_sets -= 1;
        bucket.allocated_sets += 1;

        Ok(DescriptorSet {
            raw,
            bucket_key: key,
            pool_index,
        })
    }

    pub unsafe fn free(&mut self, device: &ash::Device, set: DescriptorSet) {
        let bucket = self.buckets.get_mut(&set.bucket_key).unwrap();
        let pool = bucket.pools.get_mut(set.pool_index).unwrap();

        let result =
            unsafe { device.free_descriptor_sets(pool.raw, core::slice::from_ref(&set.raw())) };
        if let Err(err) = result {
            // vkFreeDescriptorSets is documented to return:
            // - VK_ERROR_UNKNOWN
            // - VK_ERROR_VALIDATION_FAILED (we shouldn't encounter this one)
            // wgpu-hal doesn't currently report errors in destroy functions.
            // Panic here for now. It might be ok to ignore the error but the
            // proper solution would probably be to lose the device.
            panic!("vkFreeDescriptorSets error: {err}, please report this error");
        }

        pool.available += 1;
        bucket.available_sets += 1;
        bucket.allocated_sets -= 1;

        // Do not immediately destroy empty pools since they might have
        // been recently created.
        // Destroy a pool only if it's empty and we have 1/4th its capacity
        // in other pools.
        // Note that this logic will never destroy the last pool.
        let pool = bucket.pools.last().unwrap();
        if pool.available == pool.capacity
            && bucket.available_sets - pool.capacity > pool.capacity / 4
        {
            let pool = bucket.pools.pop().unwrap();
            unsafe { device.destroy_descriptor_pool(pool.raw, None) };
            bucket.available_sets -= pool.capacity;
            self.release_capacity(&set.bucket_key, pool.capacity);
        }
    }

    /// Returns the index of a pool of `key`'s bucket with room for a set, creating one if needed.
    fn pool_with_room(
        &mut self,
        device: &ash::Device,
        key: &BucketKey,
    ) -> Result<usize, crate::DeviceError> {
        // Prefer smaller/older/fuller pools for new allocations to prevent
        // fragmentation and possibly fragmentation of hardware resources
        // (VK_ERROR_FRAGMENTATION)
        if let Some(index) = self.buckets[key]
            .pools
            .iter()
            .position(|pool| pool.available != 0)
        {
            return Ok(index);
        }
        let max_capacity = self.pool_capacity_limit(device, key)?;
        let min_capacity = self.min_pool_capacity(key);
        let capacity_hint = self.buckets[key].allocated_sets;
        let created = self.buckets.get_mut(key).unwrap().create_pool(
            device,
            key,
            capacity_hint,
            min_capacity,
            max_capacity,
        );
        let created = match created {
            // Empty update-after-bind pools may hold the memory a new one needs.
            Err(vk::Result::ERROR_FRAGMENTATION) if key.update_after_bind => {
                self.destroy_empty_update_after_bind_pools(device);
                self.buckets
                    .get_mut(key)
                    .unwrap()
                    .create_pool(device, key, 1, 1, max_capacity)
            }
            created => created,
        };
        let (index, pool) = created.map_err(super::map_host_device_oom_and_fragmentation_err)?;
        let capacity = pool.capacity;
        if key.update_after_bind {
            self.update_after_bind_descriptors_in_all_pools +=
                u64::from(capacity) * u64::from(key.counts.total());
        }
        Ok(index)
    }

    /// The fewest sets a new pool of `key` holds. Update-after-bind pools start from a single set
    /// when a pool of [`POOL_MIN_SETS`] would take more than an eighth of the limit, so one live
    /// set cannot hold a large share of it, and grow with demand from there.
    fn min_pool_capacity(&self, key: &BucketKey) -> u32 {
        let full = u64::from(POOL_MIN_SETS) * u64::from(key.counts.total());
        if key.update_after_bind
            && full > u64::from(self.max_update_after_bind_descriptors_in_all_pools) / 8
        {
            1
        } else {
            POOL_MIN_SETS
        }
    }

    /// Destroys the empty pools of every update-after-bind bucket.
    fn destroy_empty_update_after_bind_pools(&mut self, device: &ash::Device) {
        for (key, bucket) in &mut self.buckets {
            if key.update_after_bind {
                let released = bucket.destroy_empty_pools(device);
                self.update_after_bind_descriptors_in_all_pools -=
                    u64::from(released) * u64::from(key.counts.total());
            }
        }
    }

    /// Returns how many sets a new pool for `key` may hold. Update-after-bind pools are bounded
    /// by what is left of `maxUpdateAfterBindDescriptorsInAllPools`; when not even one set fits,
    /// the empty update-after-bind pools of every bucket are destroyed first.
    fn pool_capacity_limit(
        &mut self,
        device: &ash::Device,
        key: &BucketKey,
    ) -> Result<u32, crate::DeviceError> {
        if !key.update_after_bind {
            return Ok(u32::MAX);
        }
        let per_set = u64::from(key.counts.total().max(1));
        let mut fitting = self.remaining_update_after_bind_descriptors() / per_set;
        if fitting == 0 {
            self.destroy_empty_update_after_bind_pools(device);
            fitting = self.remaining_update_after_bind_descriptors() / per_set;
        }
        if fitting == 0 {
            return Err(crate::DeviceError::OutOfMemory);
        }
        Ok(u32::try_from(fitting).unwrap_or(u32::MAX))
    }

    fn remaining_update_after_bind_descriptors(&self) -> u64 {
        u64::from(self.max_update_after_bind_descriptors_in_all_pools)
            .saturating_sub(self.update_after_bind_descriptors_in_all_pools)
    }

    /// Returns the share of the update-after-bind limit held by a destroyed pool of `key`.
    fn release_capacity(&mut self, key: &BucketKey, capacity: u32) {
        if key.update_after_bind {
            self.update_after_bind_descriptors_in_all_pools -=
                u64::from(capacity) * u64::from(key.counts.total());
        }
    }
}

impl Drop for DescriptorAllocator {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(
                self.buckets.is_empty(),
                "buckets are not empty, at least one BGL has not been unregistered"
            )
        }
    }
}

fn create_descriptor_pool(
    device: &ash::Device,
    key: &BucketKey,
    capacity_hint: u32,
    min_capacity: u32,
    max_capacity: u32,
) -> Result<Pool, vk::Result> {
    let counts = &key.counts;

    const NR_OF_DESCRIPTOR_TYPES: usize = 8;

    use vk::DescriptorType as Dt;
    let counts: [_; NR_OF_DESCRIPTOR_TYPES] = [
        (Dt::SAMPLER, counts.sampler),
        (Dt::SAMPLED_IMAGE, counts.sampled_image),
        (Dt::STORAGE_IMAGE, counts.storage_image),
        (Dt::UNIFORM_BUFFER, counts.uniform_buffer),
        (Dt::UNIFORM_BUFFER_DYNAMIC, counts.uniform_buffer_dynamic),
        (Dt::STORAGE_BUFFER, counts.storage_buffer),
        (Dt::STORAGE_BUFFER_DYNAMIC, counts.storage_buffer_dynamic),
        (
            Dt::ACCELERATION_STRUCTURE_KHR,
            counts.acceleration_structure,
        ),
    ];

    // bounded doubling growth strategy
    let mut capacity = capacity_hint
        .clamp(min_capacity, POOL_MAX_SETS)
        .next_power_of_two()
        .min(max_capacity);

    // Lower capacity to avoid overflowing `count * capacity` calculations
    // below. The resulting `capacity` might not be a power of 2 but that's
    // ok.
    for (_, count) in counts {
        capacity = (u32::MAX / count.max(1)).min(capacity);
    }

    let mut pool_sizes = counts
        .into_iter()
        .filter(|&(_, count)| count != 0)
        .map(|(ty, count)| vk::DescriptorPoolSize {
            ty,
            descriptor_count: count * capacity,
        })
        .collect::<ArrayVec<_, NR_OF_DESCRIPTOR_TYPES>>();

    // Prior to Vulkan 1.3.215 `poolSizeCount` was required to be non-zero, so
    // add a single small sampler entry if `pool_sizes` would otherwise be
    // empty.
    if pool_sizes.is_empty() {
        pool_sizes.push(vk::DescriptorPoolSize {
            ty: Dt::SAMPLER,
            descriptor_count: 1,
        });
    }

    let mut flags = vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET;
    if key.update_after_bind {
        flags |= vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND;
    };

    let vk_info = vk::DescriptorPoolCreateInfo::default()
        .flags(flags)
        .max_sets(capacity)
        .pool_sizes(&pool_sizes);

    let raw = unsafe { device.create_descriptor_pool(&vk_info, None) }?;

    Ok(Pool {
        raw,
        capacity,
        available: capacity,
    })
}

#[cfg(test)]
#[path = "descriptor_tests.rs"]
mod tests;
