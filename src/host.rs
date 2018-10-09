use std::sync::Arc;

use crate::EnetKeepAlive;

use enet_sys::{
    enet_host_bandwidth_limit, enet_host_channel_limit, enet_host_destroy, enet_host_flush,
    ENetHost,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Represents a bandwidth limit or unlimited.
pub enum BandwidthLimit {
    /// No limit on bandwidth
    Unlimited,
    /// Bandwidth limit in bytes/second
    Limited(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Represents a channel limit or unlimited.
pub enum ChannelLimit {
    /// Maximum limit on the number of channels
    Maximum,
    /// Channel limit
    Limited(usize),
}

impl ChannelLimit {
    pub(in crate) fn to_enet_usize(&self) -> usize {
        match *self {
            ChannelLimit::Maximum => 0,
            ChannelLimit::Limited(l) => l,
        }
    }
}

impl BandwidthLimit {
    pub(in crate) fn to_enet_u32(&self) -> u32 {
        match *self {
            BandwidthLimit::Unlimited => 0,
            BandwidthLimit::Limited(l) => l,
        }
    }
}

/// A `Host` represents one endpoint of an ENet connection. Created through `Enet`.
///
/// This type provides functionality such as connection establishment and packet transmission.
pub struct Host {
    _keep_alive: Arc<EnetKeepAlive>,
    inner: *mut ENetHost,
}

impl Host {
    pub(in crate) fn new(_keep_alive: Arc<EnetKeepAlive>, inner: *mut ENetHost) -> Host {
        assert!(!inner.is_null());

        Host { _keep_alive, inner }
    }

    /// Sends any queued packets on the host specified to its designated peers.
    ///
    /// This function need only be used in circumstances where one wishes to send queued packets earlier than in a call to `Host::service()`.
    pub fn flush(&mut self) {
        unsafe {
            enet_host_flush(self.inner);
        }
    }

    /// Sets the bandwith limits for this host.
    pub fn set_bandwith_limits(
        &mut self,
        incoming_bandwith: BandwidthLimit,
        outgoing_bandwidth: BandwidthLimit,
    ) {
        unsafe {
            enet_host_bandwidth_limit(
                self.inner,
                incoming_bandwith.to_enet_u32(),
                outgoing_bandwidth.to_enet_u32(),
            );
        }
    }

    /// Sets the maximum allowed channels of future connections.
    ///
    /// Pass `None` to use ENet defaults, which set the value to its maximum.
    pub fn set_channel_limit(&mut self, max_channel_count: Option<usize>) {
        unsafe {
            enet_host_channel_limit(self.inner, max_channel_count.unwrap_or(0));
        }
    }
}

impl Drop for Host {
    /// Call the corresponding ENet cleanup-function(s).
    fn drop(&mut self) {
        unsafe {
            enet_host_destroy(self.inner);
        }
    }
}
