//! This module contains the types used to manage eviction state for the frame eviction algorithm.

use crate::page::PageRef;
use std::ops::Deref;
use tokio::sync::Mutex;

/// The type representing a [`Frame`](super::frame::Frame)'s eviction state.
#[derive(Debug)]
pub struct EvictionState {
    /// A mutex-protected [`FrameTemperature`] enum to ensure atomic operations.
    pub inner: Mutex<FrameTemperature>,
}

/// The enum representing the possible values for [`EvictionState`].
///
/// The reason this is separate from the [`EvictionState`] struct is because we cannot represent do
/// atomic operations on enums in Rust.
#[derive(Debug)]
pub enum FrameTemperature {
    /// Represents a frequently / recently accessed [`Frame`](super::frame::Frame) that currently
    /// holds a [`Page`](crate::page::Page)'s data.
    Hot(PageRef),
    /// Represents an infrequently or old [`Frame`](super::frame::Frame) that might be evicted soon,
    /// and also still currently holds a [`Page`](crate::page::Page)'s data.
    Cool(PageRef),
    /// Represents a [`Frame`](super::frame::Frame) that does not hold any
    /// [`Page`](crate::page::Page)'s data.
    Cold,
}

impl Default for EvictionState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(FrameTemperature::Cold),
        }
    }
}

impl EvictionState {
    /// Updates the eviction state after this frame has been accessed.
    pub async fn record_access(&self) {
        let mut guard = self.inner.lock().await;
        match guard.deref() {
            FrameTemperature::Hot(_) => (),
            FrameTemperature::Cool(page) => *guard = FrameTemperature::Hot(page.clone()),
            FrameTemperature::Cold => (),
        }
    }

    /// Atomically sets the temperature as [`FrameTemperature::Hot`] and then stores the page that
    /// owns the [`Frame`](super::frame::Frame) into the state.
    pub async fn set_owner(&self, page: PageRef) {
        let mut guard = self.inner.lock().await;
        *guard = FrameTemperature::Hot(page)
    }

    /// Atomically loads the [`Page`](crate::page::Page) that owns the
    /// [`Frame`](super::frame::Frame), if an owner exists.
    pub async fn get_owner(&self) -> Option<PageRef> {
        let guard = self.inner.lock().await;
        match guard.deref() {
            FrameTemperature::Hot(page) => Some(page.clone()),
            FrameTemperature::Cool(page) => Some(page.clone()),
            FrameTemperature::Cold => None,
        }
    }

    /// Atomically runs the cooling algorithm, returning a [`PageRef`] if we want to evict the page.
    ///
    /// If the state is [`Hot`](FrameTemperature::Hot), then this function cools it down to be
    /// [`Cool`](FrameTemperature::Cool), and if it was already [`Cool`](FrameTemperature::Cool),
    /// then this function will cool it down further to [`Cold`](FrameTemperature::Cold).
    ///
    /// If the state transitions to [`Cold`](FrameTemperature::Cold), this function will return the
    /// [`PageRef`] that it used to hold.
    pub async fn cool(&self) -> Option<PageRef> {
        let mut guard = self.inner.lock().await;

        match guard.deref() {
            FrameTemperature::Hot(page) => {
                *guard = FrameTemperature::Cool(page.clone());
                None
            }
            FrameTemperature::Cool(page) => {
                let page = page.clone();
                *guard = FrameTemperature::Cold;
                Some(page)
            }
            FrameTemperature::Cold => None,
        }
    }

    /// Atomically cools down the eviction state all the way to [`Cold`](FrameTemperature::Cold),
    /// returning the owning [`PageRef`] if it wasn't already [`Cold`](FrameTemperature::Cold).
    pub async fn evict(&self) -> Option<PageRef> {
        let mut guard = self.inner.lock().await;

        match guard.deref() {
            FrameTemperature::Hot(page) => {
                let page = page.clone();
                *guard = FrameTemperature::Cold;
                Some(page)
            }
            FrameTemperature::Cool(page) => {
                let page = page.clone();
                *guard = FrameTemperature::Cold;
                Some(page)
            }
            FrameTemperature::Cold => None,
        }
    }
}
