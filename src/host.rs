use std::{marker::PhantomData, mem::MaybeUninit, sync::Arc, time::Duration};

use enet_sys::{
    enet_host_bandwidth_limit, enet_host_channel_limit, enet_host_check_events, enet_host_connect,
    enet_host_destroy, enet_host_flush, enet_host_service, ENetEvent, ENetHost, ENetPeer,
    ENET_PROTOCOL_MAXIMUM_CHANNEL_COUNT,
};

use crate::{Address, EnetKeepAlive, Error, Event, Peer, PeerID};

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
    Limited(enet_sys::size_t),
}

impl ChannelLimit {
    pub(in crate) fn to_enet_val(self) -> enet_sys::size_t {
        match self {
            ChannelLimit::Maximum => 0,
            ChannelLimit::Limited(l) => l,
        }
    }

    fn from_enet_val(enet_val: enet_sys::size_t) -> ChannelLimit {
        const MAX_COUNT: enet_sys::size_t = ENET_PROTOCOL_MAXIMUM_CHANNEL_COUNT as enet_sys::size_t;
        match enet_val {
            MAX_COUNT => ChannelLimit::Maximum,
            0 => panic!("ChannelLimit::from_enet_usize: got 0"),
            lim => ChannelLimit::Limited(lim),
        }
    }
}

impl BandwidthLimit {
    pub(in crate) fn to_enet_u32(self) -> u32 {
        match self {
            BandwidthLimit::Unlimited => 0,
            BandwidthLimit::Limited(l) => l,
        }
    }
}

/// A `Host` represents one endpoint of an ENet connection. Created through
/// `Enet`.
///
/// This type provides functionality such as connection establishment and packet
/// transmission.
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
    /// This function need only be used in circumstances where one wishes to
    /// send queued packets earlier than in a call to `Host::service()`.
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
            enet_host_channel_limit(self.inner, max_channel_count.to_enet_val());
        }
    }

    /// Returns the limit of channels per connected peer for this `Host`.
    pub fn channel_limit(&self) -> ChannelLimit {
        ChannelLimit::from_enet_val(unsafe { (*self.inner).channelLimit })
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
    pub fn peer_count(&self) -> enet_sys::size_t {
        unsafe { (*self.inner).peerCount }
    }

    /// Returns a mutable reference to a peer at the given PeerID, None if the index is invalid.
    pub fn peer_mut(&mut self, idx: PeerID) -> Option<&mut Peer<T>> {
        if !(0..self.peer_count() as isize).contains(&idx.index) {
            return None;
        }

        let peer = Peer::new_mut(unsafe { &mut *((*self.inner).peers.offset(idx.index)) });
        if peer.generation() != idx.generation {
            return None;
        }

        Some(peer)
    }

    /// Returns a reference to a peer at the given PeerID, None if the index is invalid.
    pub fn peer(&self, idx: PeerID) -> Option<&Peer<T>> {
        if !(0..self.peer_count() as isize).contains(&idx.index) {
            return None;
        }

        let peer = Peer::new(unsafe { &*((*self.inner).peers.offset(idx.index)) });
        if peer.generation() != idx.generation {
            return None;
        }

        Some(peer)
    }

    pub(crate) unsafe fn peer_id(&self, peer: *mut ENetPeer) -> PeerID {
        // We can do pointer arithmetic here to determine the offset of our new Peer in the
        // list of peers, which is it's PeerID.
        let index = peer.offset_from((*self.inner).peers);
        let peer = Peer::<T>::new_mut(&mut *peer);
        PeerID {
            index,
            generation: peer.generation(),
        }
    }

    /// Returns an iterator over all peers connected to this `Host`.
    pub fn peers_mut(&mut self) -> impl Iterator<Item = &'_ mut Peer<T>> {
        let peers = unsafe {
            std::slice::from_raw_parts_mut(
                (*self.inner).peers,
                // This conversion should basically never fail.
                // It may only fail if size_t and usize are of
                // different size and the peerCount is very large,
                // which is only possible on niche platforms.
                (*self.inner).peerCount.try_into().unwrap(),
            )
        };

        peers.iter_mut().map(|peer| Peer::new_mut(&mut *peer))
    }

    /// Returns an iterator over all peers connected to this `Host`.
    pub fn peers(&self) -> impl Iterator<Item = &'_ Peer<T>> {
        let peers = unsafe {
            std::slice::from_raw_parts(
                (*self.inner).peers,
                // This conversion should basically never fail.
                // It may only fail if size_t and usize are of
                // different size and the peerCount is very large,
                // which is only possible on niche platforms.
                (*self.inner).peerCount.try_into().unwrap(),
            )
        };

        peers.iter().map(|peer| Peer::new(&*peer))
    }

    fn process_event(&'_ mut self, sys_event: ENetEvent) -> Option<Event<'_, T>> {
        Event::from_sys_event(sys_event, self)
    }

    /// Maintains this host and delivers an event if available.
    ///
    /// This should be called regularly for ENet to work properly with good performance.
    ///
    /// The function won't block if `timeout` is less than 1ms.
    pub fn service(&'_ mut self, timeout: Duration) -> Result<Option<Event<'_, T>>, Error> {
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
            r if r > 0 => Ok(self.process_event(unsafe { sys_event.assume_init() })),
            0 => Ok(None),
            r if r < 0 => Err(Error(r)),
            _ => panic!("unreachable"),
        }

        // TODO: check `total*` fields on `inner`, these need to be reset from
        // time to time.
    }

    /// Checks for any queued events on this `Host` and dispatches one if
    /// available
    pub fn check_events(&'_ mut self) -> Result<Option<Event<'_, T>>, Error> {
        // ENetEvent is Copy (aka has no Drop impl), so we don't have to make sure we
        // `mem::forget` it later on
        let mut sys_event = MaybeUninit::uninit();

        let res = unsafe { enet_host_check_events(self.inner, sys_event.as_mut_ptr()) };

        match res {
            r if r > 0 => Ok(self.process_event(unsafe { sys_event.assume_init() })),
            0 => Ok(None),
            r if r < 0 => Err(Error(r)),
            _ => panic!("unreachable"),
        }
    }

    /// Initiates a connection to a foreign host.
    ///
    /// The connection will not be done until a `Event::Connected` for this peer
    /// was received.
    ///
    /// `channel_count` specifies how many channels to allocate for this peer.
    /// `user_data` is a user-specified value that can be chosen arbitrarily.
    pub fn connect(
        &mut self,
        address: &Address,
        channel_count: enet_sys::size_t,
        user_data: u32,
    ) -> Result<(&mut Peer<T>, PeerID), Error> {
        let res: *mut ENetPeer = unsafe {
            enet_host_connect(
                self.inner,
                &address.to_enet_address() as *const _,
                channel_count,
                user_data,
            )
        };

        if res.is_null() {
            return Err(Error(0));
        }

        Ok((Peer::new_mut(unsafe { &mut *res }), unsafe {
            self.peer_id(res)
        }))
    }
}

impl<T> Drop for Host<T> {
    /// Call the corresponding ENet cleanup-function(s).
    fn drop(&mut self) {
        for peer in self.peers_mut() {
            peer.drop_raw_data();
        }

        unsafe {
            enet_host_destroy(self.inner);
        }
    }
}
