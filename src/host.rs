use std::marker::PhantomData;
use std::sync::Arc;

use crate::{EnetAddress, EnetEvent, EnetFailure, EnetKeepAlive};

use enet_sys::{
    enet_host_bandwidth_limit, enet_host_channel_limit, enet_host_destroy, enet_host_flush,
    enet_host_service, ENetEvent, ENetHost, ENET_PROTOCOL_MAXIMUM_CHANNEL_COUNT,
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

    fn from_enet_usize(enet_val: usize) -> ChannelLimit {
        const MAX_COUNT: usize = ENET_PROTOCOL_MAXIMUM_CHANNEL_COUNT as usize;
        match enet_val {
            MAX_COUNT => ChannelLimit::Maximum,
            0 => panic!("ChannelLimit::from_enet_usize: got 0"),
            lim => ChannelLimit::Limited(lim),
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
pub struct Host<T> {
    inner: *mut ENetHost,

    _keep_alive: Arc<EnetKeepAlive>,
    _peer_data: PhantomData<*const T>,
}

impl<T> Host<T> {
    pub(in crate) fn new(_keep_alive: Arc<EnetKeepAlive>, inner: *mut ENetHost) -> Host<T> {
        assert!(!inner.is_null());

        Host {
            inner,
            _keep_alive,
            _peer_data: PhantomData,
        }
    }

    /// Sends any queued packets on the host specified to its designated peers.
    ///
    /// This function need only be used in circumstances where one wishes to send queued packets earlier than in a call to `Host::service()`.
    pub fn flush(&mut self) {
        unsafe {
            enet_host_flush(self.inner);
        }
    }

    /// Sets the bandwith limits for this `Host`.
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
    pub fn set_channel_limit(&mut self, max_channel_count: ChannelLimit) {
        unsafe {
            enet_host_channel_limit(self.inner, max_channel_count.to_enet_usize());
        }
    }

    /// Returns the limit of channels per connected peer for this `Host`.
    pub fn channel_limit(&self) -> ChannelLimit {
        ChannelLimit::from_enet_usize(unsafe { (*self.inner).channelLimit })
    }

    /// Returns the downstream bandwidth of this `Host` in bytes/second.
    pub fn incoming_bandwidth(&self) -> u32 {
        unsafe { (*self.inner).incomingBandwidth }
    }

    /// Returns the upstream bandwidth of this `Host` in bytes/second.
    pub fn outgoing_bandwidth(&self) -> u32 {
        unsafe { (*self.inner).outgoingBandwidth }
    }

    /// Returns the internet address of this `Host`.
    pub fn address(&self) -> EnetAddress {
        EnetAddress::from_enet_address(&unsafe { (*self.inner).address })
    }

    /// Returns the number of peers allocated for this `Host`.
    pub fn peer_count(&self) -> usize {
        unsafe { (*self.inner).peerCount }
    }

    /// Maintains this host and delivers an event if available.
    ///
    /// This should be called regularly for ENet to work properly with good performance.
    pub fn service(&'_ mut self, timeout_ms: u32) -> Result<Option<EnetEvent<'_, T>>, EnetFailure> {
        let mut sys_event: ENetEvent = unsafe { std::mem::uninitialized() };

        let res =
            unsafe { enet_host_service(self.inner, &mut sys_event as *mut ENetEvent, timeout_ms) };

        match res {
            r if r > 0 => Ok(EnetEvent::from_sys_event(&sys_event)),
            0 => Ok(None),
            r if r < 0 => Err(EnetFailure(r)),
            _ => panic!("unreachable"),
        }
    }
}

impl<T> Drop for Host<T> {
    /// Call the corresponding ENet cleanup-function(s).
    fn drop(&mut self) {
        unsafe {
            enet_host_destroy(self.inner);
        }
    }
}
