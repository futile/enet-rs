use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ops::{Index, IndexMut};
use std::sync::Arc;
use std::time::Duration;

use crate::{Address, EnetKeepAlive, Error, Event, EventKind, Peer};

use enet_sys::{
    enet_host_bandwidth_limit, enet_host_channel_limit, enet_host_check_events, enet_host_connect,
    enet_host_destroy, enet_host_flush, enet_host_service, ENetEvent, ENetHost, ENetPeer,
    ENET_PROTOCOL_MAXIMUM_CHANNEL_COUNT,
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
    disconnect_drop: Option<usize>,
    _keep_alive: Arc<EnetKeepAlive>,
    _peer_data: PhantomData<*const T>,
}

impl<T> Host<T> {
    pub(in crate) fn new(_keep_alive: Arc<EnetKeepAlive>, inner: *mut ENetHost) -> Host<T> {
        assert!(!inner.is_null());

        Host {
            inner,
            disconnect_drop: None,
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
    pub fn address(&self) -> Address {
        Address::from_enet_address(&unsafe { (*self.inner).address })
    }

    /// Returns the number of peers allocated for this `Host`.
    pub fn peer_count(&self) -> usize {
        unsafe { (*self.inner).peerCount }
    }

    /// Returns a mutable reference to a peer at the index, None if the index is invalid.
    pub fn peer_mut(&mut self, idx: usize) -> Option<&mut Peer<T>> {
        if idx >= self.peer_count() {
            return None;
        }

        Some(Peer::new_mut(unsafe {
            &mut *((*self.inner).peers.offset(idx as isize))
        }))
    }

    /// Returns a reference to a peer at the index, None if the index is invalid.
    pub fn peer(&self, idx: usize) -> Option<&Peer<T>> {
        if idx >= self.peer_count() {
            return None;
        }

        Some(Peer::new(unsafe {
            &*((*self.inner).peers.offset(idx as isize))
        }))
    }

    /// Returns an iterator over all peers connected to this `Host`.
    pub fn peers_mut(&mut self) -> impl Iterator<Item = &'_ mut Peer<T>> {
        let peers =
            unsafe { std::slice::from_raw_parts_mut((*self.inner).peers, (*self.inner).peerCount) };

        peers.into_iter().map(|peer| Peer::new_mut(&mut *peer))
    }

    /// Returns an iterator over all peers connected to this `Host`.
    pub fn peers(&self) -> impl Iterator<Item = &'_ Peer<T>> {
        let peers =
            unsafe { std::slice::from_raw_parts((*self.inner).peers, (*self.inner).peerCount) };

        peers.into_iter().map(|peer| Peer::new(&*peer))
    }

    fn drop_disconnected(&mut self) {
        if let Some(idx) = self.disconnect_drop.take() {
            Peer::<T>::new_mut(unsafe { &mut *((*self.inner).peers.offset(idx as isize)) })
                .set_data(None);
        }
    }

    fn process_event(&mut self, sys_event: ENetEvent) -> Option<Event<'_, T>> {
        self.drop_disconnected();

        let event = Event::from_sys_event(&sys_event);
        if let Some(EventKind::Disconnect { .. }) = event.as_ref().map(|event| &event.kind) {
            self.disconnect_drop = Some(unsafe {
                (sys_event.peer as usize - (*self.inner).peers as usize)
                    / mem::size_of::<ENetPeer>()
            });
        }

        event
    }

    /// Maintains this host and delivers an event if available.
    ///
    /// This should be called regularly for ENet to work properly with good performance.
    ///
    /// The function won't block for less than 1ms.
    pub fn service(&mut self, timeout: Duration) -> Result<Option<Event<'_, T>>, Error> {
        // ENetEvent is Copy (aka has no Drop impl), so we don't have to make sure we `mem::forget` it later on
        let mut sys_event = MaybeUninit::uninit();

        let res = unsafe {
            enet_host_service(
                self.inner,
                sys_event.as_mut_ptr(),
                timeout.as_millis() as u32,
            )
        };

        match res {
            r if r > 0 => Ok(unsafe { self.process_event(sys_event.assume_init()) }),
            0 => Ok(None),
            r if r < 0 => Err(Error(r)),
            _ => panic!("unreachable"),
        }

        // TODO: check `total*` fields on `inner`, these need to be reset from time to time.
    }

    /// Checks for any queued events on this `Host` and dispatches one if available
    pub fn check_events(&mut self) -> Result<Option<Event<'_, T>>, Error> {
        // ENetEvent is Copy (aka has no Drop impl), so we don't have to make sure we `mem::forget` it later on
        let mut sys_event = MaybeUninit::uninit();

        let res = unsafe { enet_host_check_events(self.inner, sys_event.as_mut_ptr()) };

        match res {
            r if r > 0 => Ok(unsafe { self.process_event(sys_event.assume_init()) }),
            0 => Ok(None),
            r if r < 0 => Err(Error(r)),
            _ => panic!("unreachable"),
        }
    }

    /// Initiates a connection to a foreign host.
    ///
    /// The connection will not be done until a `Event::Connected` for this peer was received.
    ///
    /// `channel_count` specifies how many channels to allocate for this peer.
    /// `data` is a user-specified value that can be chosen arbitrarily.
    pub fn connect(
        &mut self,
        address: &Address,
        channel_count: usize,
        data: u32,
    ) -> Result<&mut Peer<T>, Error> {
        let res: *mut ENetPeer = unsafe {
            enet_host_connect(
                self.inner,
                &address.to_enet_address() as *const _,
                channel_count,
                data,
            )
        };

        if res.is_null() {
            return Err(Error(0));
        }

        Ok(Peer::new_mut(unsafe { &mut *res }))
    }
}

impl<T> Index<usize> for Host<T> {
    type Output = Peer<T>;

    fn index(&self, idx: usize) -> &Peer<T> {
        self.peer(idx).expect("invalid peer index")
    }
}

impl<T> IndexMut<usize> for Host<T> {
    fn index_mut(&mut self, idx: usize) -> &mut Peer<T> {
        self.peer_mut(idx).expect("invalid peer index")
    }
}

impl<T> Drop for Host<T> {
    /// Call the corresponding ENet cleanup-function(s).
    fn drop(&mut self) {
        for peer in self.peers_mut() {
            peer.set_data(None);
        }

        unsafe {
            enet_host_destroy(self.inner);
        }
    }
}
