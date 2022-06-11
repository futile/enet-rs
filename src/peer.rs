use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    time::Duration,
};

use enet_sys::{
    enet_peer_disconnect, enet_peer_disconnect_later, enet_peer_disconnect_now, enet_peer_receive,
    enet_peer_reset, enet_peer_send, ENetPeer, _ENetPeerState,
    _ENetPeerState_ENET_PEER_STATE_ACKNOWLEDGING_CONNECT,
    _ENetPeerState_ENET_PEER_STATE_ACKNOWLEDGING_DISCONNECT,
    _ENetPeerState_ENET_PEER_STATE_CONNECTED, _ENetPeerState_ENET_PEER_STATE_CONNECTING,
    _ENetPeerState_ENET_PEER_STATE_CONNECTION_PENDING,
    _ENetPeerState_ENET_PEER_STATE_CONNECTION_SUCCEEDED,
    _ENetPeerState_ENET_PEER_STATE_DISCONNECTED, _ENetPeerState_ENET_PEER_STATE_DISCONNECTING,
    _ENetPeerState_ENET_PEER_STATE_DISCONNECT_LATER, _ENetPeerState_ENET_PEER_STATE_ZOMBIE,
};

use crate::{Address, Error, Packet};

/// This struct represents an endpoint in an ENet-connection.
///
/// A `Peer` is owned by the `Host` it is returned from.
/// Therefore, `Peer`s are always borrowed, and can not really be stored anywhere.
/// For this purpose, use [PeerID](struct.PeerID.html) instead.
///
/// ENet allows the association of arbitrary data with each peer.
/// The type of this associated data is chosen through `T`.
#[repr(transparent)]
pub struct Peer<T> {
    inner: ENetPeer,

    _data: PhantomData<T>,
}

struct PeerData<T> {
    peer_generation: usize,
    user_data: Option<T>,
}

/// A packet received directly from a `Peer`.
///
/// Contains the received packet as well as the channel on which it was
/// received.
#[derive(Debug)]
pub struct PeerPacket {
    /// The packet that was received.
    pub packet: Packet,
    /// The channel on which the packet was received.
    pub channel_id: u8,
}

impl<'a, T> Peer<T>
where
    T: 'a,
{
    // -------- Note: Peer lifetimes -------------
    // An enet `Peer` technically has the same lifetime as the Host it belongs to.
    // This however also means that a Peer may be in an ill-defined state, when it
    // does not currently represent an active connection.
    //
    // With enet-rs, a specific `PeerID` will only ever reference a `Peer` that corresponds to an
    // active connection.
    // As soon as an `Event` that contains an `EventKind::Disconnect` is dropped, the `PeerID` of the
    // corresponding `Peer` will be invalidated.
    // The data associated with a Peer will also be dropped at that point.
    pub(crate) fn new(inner: &'a ENetPeer) -> &'a Peer<T> {
        // Safety:
        // This code interprets the `ENetPeer` reference as a `Peer` reference instead.
        // As `Peer` is `repr(transparent)` and we only return a reference, so the `Peer`
        // can't be moved, this is safe.
        unsafe { &*(inner as *const _ as *const Peer<T>) }
    }

    pub(crate) fn new_mut(inner: &'a mut ENetPeer) -> &'a mut Peer<T> {
        // Safety: See `new`
        unsafe { &mut *(inner as *mut _ as *mut Peer<T>) }
    }

    /// Returns the address of this `Peer`.
    pub fn address(&self) -> Address {
        Address::from_enet_address(&self.inner.address)
    }

    /// Returns the amout of channels allocated for this `Peer`.
    pub fn channel_count(&self) -> enet_sys::size_t {
        self.inner.channelCount
    }

    fn raw_data(&self) -> Option<&PeerData<T>> {
        unsafe {
            let raw_data = self.inner.data as *const PeerData<T>;

            if raw_data.is_null() {
                None
            } else {
                Some(&(*raw_data))
            }
        }
    }

    // automatically initializes the Peer Data if it doesn't exist
    fn raw_data_mut(&mut self) -> &mut PeerData<T> {
        unsafe {
            let mut raw_data = self.inner.data as *mut PeerData<T>;

            if raw_data.is_null() {
                raw_data = Box::into_raw(Box::new(PeerData {
                    peer_generation: 0,
                    user_data: None,
                }));
                self.inner.data = raw_data as *mut _;
            }

            &mut (*raw_data)
        }
    }

    pub(crate) fn drop_raw_data(&mut self) {
        let raw_data = self.inner.data as *mut PeerData<T>;

        if !raw_data.is_null() {
            unsafe {
                drop(Box::from_raw(raw_data));
            }
        }
    }

    pub(crate) fn generation(&self) -> usize {
        if let Some(peer_data) = self.raw_data() {
            peer_data.peer_generation
        } else {
            0
        }
    }

    /// This will do all necessary cleanup after a Peer
    /// has been disconnected, including increasing the generation,
    /// as well as dropping the data associated with this peer.
    pub(crate) fn cleanup_after_disconnect(&mut self) {
        self.raw_data_mut().peer_generation += 1;
        self.take_data();
    }

    /// Returns a reference to the data associated with this `Peer`, if set.
    pub fn data(&self) -> Option<&T> {
        if let Some(peer_data) = self.raw_data() {
            peer_data.user_data.as_ref()
        } else {
            None
        }
    }

    /// Returns a mutable reference to the data associated with this `Peer`, if set.
    pub fn data_mut(&mut self) -> Option<&mut T> {
        self.raw_data_mut().user_data.as_mut()
    }

    /// Sets the data associated with this `Peer`, replacing existing data.
    ///
    /// To clear the data associated with this Peer, use `take_data` instead.
    pub fn set_data(&mut self, data: T) {
        self.raw_data_mut().user_data = Some(data);
    }

    /// Take the data associated with this `Peer` out of it.
    /// No more data will be associated with this Peer after this call.
    pub fn take_data(&mut self) -> Option<T> {
        self.raw_data_mut().user_data.take()
    }

    /// Returns the downstream bandwidth of this `Peer` in bytes/second.
    pub fn incoming_bandwidth(&self) -> u32 {
        self.inner.incomingBandwidth
    }

    /// Returns the upstream bandwidth of this `Peer` in bytes/second.
    pub fn outgoing_bandwidth(&self) -> u32 {
        self.inner.outgoingBandwidth
    }

    /// Returns the mean round trip time between sending a reliable packet and receiving its acknowledgement.
    pub fn mean_rtt(&self) -> Duration {
        Duration::from_millis(self.inner.roundTripTime as u64)
    }

    /// Forcefully disconnects this `Peer`.
    ///
    /// The foreign host represented by the peer is not notified of the disconnection and will timeout on its connection to the local host.
    pub fn reset(&mut self) {
        unsafe {
            enet_peer_reset(&mut self.inner as *mut _);
        }
    }

    /// Returns the state this `Peer` is in.
    pub fn state(&self) -> PeerState {
        PeerState::from_sys_state(self.inner.state)
    }

    /// Queues a packet to be sent.
    ///
    /// Actual sending will happen during `Host::service`.
    pub fn send_packet(&mut self, packet: Packet, channel_id: u8) -> Result<(), Error> {
        let res =
            unsafe { enet_peer_send(&mut self.inner as *mut _, channel_id, packet.into_inner()) };

        match res {
            r if r > 0 => panic!("unexpected res: {}", r),
            0 => Ok(()),
            r if r < 0 => Err(Error(r)),
            _ => panic!("unreachable"),
        }
    }

    /// Disconnects from this peer.
    ///
    /// A `Disconnect` event will be returned by `Host::service` once the disconnection is complete.
    pub fn disconnect(&mut self, user_data: u32) {
        unsafe {
            enet_peer_disconnect(&mut self.inner as *mut _, user_data);
        }
    }

    /// Disconnects from this peer immediately.
    ///
    /// No `Disconnect` event will be created.
    /// No disconnect notification for the foreign peer is guaranteed, and this
    /// `Peer` is immediately reset on return from this method.
    ///
    /// Any `PeerID` referencing this `Peer` will be invalid after this method is executed and all
    /// data associated with this `Peer` will be dropped.
    pub fn disconnect_now(mut self, user_data: u32) {
        unsafe {
            enet_peer_disconnect_now(&mut self.inner as *mut _, user_data);
        }
        // Because no disconnect event is received, we have to clean up manually here.
        self.cleanup_after_disconnect();
    }

    /// Disconnects from this peer after all outgoing packets have been sent.
    ///
    /// A `Disconnect` event will be returned by `Host::service` once the disconnection is complete.
    pub fn disconnect_later(&mut self, user_data: u32) {
        unsafe {
            enet_peer_disconnect_later(&mut self.inner as *mut _, user_data);
        }
    }

    /// Attempts to dequeue an incoming packet from this `Peer`.
    ///
    /// On success, returns the packet and the channel id of the receiving channel.
    pub fn receive(&mut self) -> Option<PeerPacket> {
        let mut channel_id = 0u8;
        let res =
            unsafe { enet_peer_receive(&mut self.inner as *mut _, &mut channel_id as *mut _) };
        if res.is_null() {
            None
        } else {
            Some(PeerPacket {
                packet: Packet::from_sys_packet(res),
                channel_id,
            })
        }
    }
}

impl<T> Debug for Peer<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Peer").field("data", &self.data()).finish()
    }
}

/// The ID of a [Peer](struct.Peer.html).
///
/// Can be used with the [peer](struct.Host.html#method.peer)/[peer_mut](struct.Host.html#method.peer_mut)-methods of Host, to retrieve references to a Peer.
/// As the lifetime semantics of Peers aren't clear in Enet and they cannot be owned, PeerID's are the
/// primary way of storing owned references to Peers.
///
/// When connecting to a host, both a reference to the peer, and its ID are returned.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct PeerID {
    pub(crate) index: isize,
    pub(crate) generation: usize,
}

/// Describes the state a `Peer` is in.
///
/// The states should be self-explanatory, ENet doesn't explain them more
/// either.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum PeerState {
    Disconnected,
    Connected,
    Connecting,
    AcknowledgingConnect,
    ConnectionPending,
    ConnectionSucceeded,
    DisconnectLater,
    Disconnecting,
    AcknowledgingDisconnect,
    Zombie,
}

impl PeerState {
    fn from_sys_state(enet_sys_state: _ENetPeerState) -> PeerState {
        #[allow(non_upper_case_globals)]
        match enet_sys_state {
            _ENetPeerState_ENET_PEER_STATE_DISCONNECTED => PeerState::Disconnected,
            _ENetPeerState_ENET_PEER_STATE_CONNECTING => PeerState::Connecting,
            _ENetPeerState_ENET_PEER_STATE_ACKNOWLEDGING_CONNECT => PeerState::AcknowledgingConnect,
            _ENetPeerState_ENET_PEER_STATE_CONNECTION_PENDING => PeerState::ConnectionPending,
            _ENetPeerState_ENET_PEER_STATE_CONNECTION_SUCCEEDED => PeerState::ConnectionSucceeded,
            _ENetPeerState_ENET_PEER_STATE_CONNECTED => PeerState::Connected,
            _ENetPeerState_ENET_PEER_STATE_DISCONNECT_LATER => PeerState::DisconnectLater,
            _ENetPeerState_ENET_PEER_STATE_DISCONNECTING => PeerState::Disconnecting,
            _ENetPeerState_ENET_PEER_STATE_ACKNOWLEDGING_DISCONNECT => {
                PeerState::AcknowledgingDisconnect
            }
            _ENetPeerState_ENET_PEER_STATE_ZOMBIE => PeerState::Zombie,
            val => panic!("unexpected peer state: {}", val),
        }
    }
}
