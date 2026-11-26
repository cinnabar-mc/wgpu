//! Framebuffers shared by every command encoder of a device.
//!
//! A framebuffer names the exact image views it attaches, so it stays valid only while they
//! do. Users of `wgpu-hal` destroy a view only once no pending command buffer uses it, and a
//! framebuffer is only used together with its views, so destroying the cached framebuffers of
//! a view when the view is destroyed never affects pending work.

use alloc::vec::Vec;

use ash::vk;
use naga::FastHashMap;

use super::{FramebufferKey, ResourceIdentity};

/// Framebuffers kept until one of their views is destroyed or the device is dropped.
///
/// Holds at most [`Self::CAPACITY`] framebuffers. Once full, render passes use framebuffers
/// owned by their command encoder instead, which live until the encoder is reset.
#[derive(Default)]
pub(super) struct FramebufferCache {
    framebuffers: FastHashMap<FramebufferKey, vk::Framebuffer>,
    /// The cached keys that attach each view.
    by_view: FastHashMap<ResourceIdentity<vk::ImageView>, Vec<FramebufferKey>>,
}

impl FramebufferCache {
    /// The most framebuffers kept at once.
    ///
    /// Far above what a frame uses; it bounds an application that keeps many views alive and
    /// attaches them in ever-new combinations.
    pub(super) const CAPACITY: usize = 4096;

    /// The cached framebuffer for `key`, if any.
    pub(super) fn get(&self, key: &FramebufferKey) -> Option<vk::Framebuffer> {
        self.framebuffers.get(key).copied()
    }

    /// Whether the cache holds [`Self::CAPACITY`] framebuffers and takes no more.
    pub(super) fn is_full(&self) -> bool {
        self.framebuffers.len() >= Self::CAPACITY
    }

    /// Caches `raw` for `key`. The key must not be cached yet and the cache must not be full.
    pub(super) fn insert(&mut self, key: FramebufferKey, raw: vk::Framebuffer) {
        debug_assert!(!self.is_full());
        for &view in &key.attachment_identities {
            let keys = self.by_view.entry(view).or_default();
            // A view attached twice to one framebuffer is listed once.
            if !keys.contains(&key) {
                keys.push(key.clone());
            }
        }
        let previous = self.framebuffers.insert(key, raw);
        debug_assert!(previous.is_none());
    }

    /// Forgets every framebuffer attaching `view` and returns them for the caller to destroy.
    pub(super) fn remove_view(
        &mut self,
        view: ResourceIdentity<vk::ImageView>,
    ) -> Vec<vk::Framebuffer> {
        let Some(keys) = self.by_view.remove(&view) else {
            return Vec::new();
        };
        let mut removed = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(raw) = self.framebuffers.remove(&key) else {
                continue;
            };
            removed.push(raw);
            // The other views of the framebuffer may outlive this one.
            for other in &key.attachment_identities {
                if *other == view {
                    continue;
                }
                if let Some(others) = self.by_view.get_mut(other) {
                    others.retain(|cached| *cached != key);
                    if others.is_empty() {
                        self.by_view.remove(other);
                    }
                }
            }
        }
        removed
    }

    /// Forgets every framebuffer and returns them for the caller to destroy.
    pub(super) fn drain(&mut self) -> Vec<vk::Framebuffer> {
        self.by_view.clear();
        self.framebuffers.drain().map(|(_, raw)| raw).collect()
    }

    /// The number of framebuffers cached.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.framebuffers.len()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use ash::vk::Handle as _;

    use super::*;

    fn view(id: u64) -> ResourceIdentity<vk::ImageView> {
        ResourceIdentity {
            id,
            _phantom: core::marker::PhantomData,
        }
    }

    fn key(views: &[u64]) -> FramebufferKey {
        FramebufferKey {
            raw_pass: vk::RenderPass::from_raw(1),
            attachment_identities: views.iter().map(|&id| view(id)).collect(),
            attachment_views: views
                .iter()
                .map(|&id| vk::ImageView::from_raw(id))
                .collect(),
            extent: wgt::Extent3d::default(),
        }
    }

    fn sorted(raws: Vec<vk::Framebuffer>) -> Vec<u64> {
        let mut raws = raws.into_iter().map(|raw| raw.as_raw()).collect::<Vec<_>>();
        raws.sort_unstable();
        raws
    }

    #[test]
    fn a_repeated_key_hits_the_cached_framebuffer() {
        let mut cache = FramebufferCache::default();
        assert_eq!(cache.get(&key(&[1, 2])), None);
        cache.insert(key(&[1, 2]), vk::Framebuffer::from_raw(10));
        assert_eq!(
            cache.get(&key(&[1, 2])),
            Some(vk::Framebuffer::from_raw(10))
        );
        assert_eq!(cache.get(&key(&[1, 3])), None);
    }

    #[test]
    fn destroying_a_view_removes_only_its_framebuffers() {
        let mut cache = FramebufferCache::default();
        cache.insert(key(&[1, 2]), vk::Framebuffer::from_raw(10));
        cache.insert(key(&[3, 2]), vk::Framebuffer::from_raw(11));
        cache.insert(key(&[4]), vk::Framebuffer::from_raw(12));

        assert_eq!(sorted(cache.remove_view(view(2))), vec![10, 11]);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key(&[4])), Some(vk::Framebuffer::from_raw(12)));
        // The surviving views no longer list the removed framebuffers.
        assert!(cache.by_view.get(&view(1)).is_none());
        assert!(cache.by_view.get(&view(3)).is_none());
        assert!(cache.remove_view(view(1)).is_empty());
    }

    #[test]
    fn a_long_lived_view_does_not_accumulate_keys_of_short_lived_ones() {
        let mut cache = FramebufferCache::default();
        for frame in 0..100 {
            let swapchain_view = 1000 + frame;
            cache.insert(
                key(&[swapchain_view, 1]),
                vk::Framebuffer::from_raw(frame + 1),
            );
            assert_eq!(cache.remove_view(view(swapchain_view)).len(), 1);
        }
        assert_eq!(cache.len(), 0);
        assert!(cache.by_view.is_empty());
    }

    #[test]
    fn a_view_attached_twice_is_listed_once() {
        let mut cache = FramebufferCache::default();
        cache.insert(key(&[1, 1]), vk::Framebuffer::from_raw(10));
        assert_eq!(cache.by_view[&view(1)].len(), 1);
        assert_eq!(sorted(cache.remove_view(view(1))), vec![10]);
        assert!(cache.by_view.is_empty());
    }

    #[test]
    fn the_cache_fills_at_capacity_and_drains_completely() {
        let mut cache = FramebufferCache::default();
        for id in 0..FramebufferCache::CAPACITY as u64 {
            assert!(!cache.is_full());
            cache.insert(key(&[id + 1]), vk::Framebuffer::from_raw(id + 1));
        }
        assert!(cache.is_full());
        assert_eq!(cache.drain().len(), FramebufferCache::CAPACITY);
        assert_eq!(cache.len(), 0);
        assert!(cache.by_view.is_empty());
        assert!(!cache.is_full());
    }
}
