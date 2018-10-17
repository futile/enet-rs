use std::marker::PhantomData;
use std::time::Duration;

use enet_sys::{
    enet_peer_disconnect, enet_peer_disconnect_later, enet_peer_disconnect_now, enet_peer_receive,
    enet_peer_reset, enet_peer_send, ENetPeer,
};

use crate::{EnetAddress, EnetFailure, EnetPacket};

/// This struct represents an endpoint in an ENet-connection.
///
/// The lifetime of these instances is not really clear from the ENet documentation.
/// Therefore, `EnetPeer`s are always borrowed, and can not really be stored anywhere.
///
/// ENet allows the association of arbitrary data with each peer.
/// The type of this associated data is chosen through `T`.
#[derive(Clone, Debug)]
pub struct EnetPeer<'a, T: 'a> {
    inner: *mut ENetPeer,

    _data: PhantomData<&'a mut T>,
}

/// A packet received directly from a `Peer`.
///
/// Contains the received packet as well as the channel on which it was received.
pub struct PeerPacket<'b, 'a, T: 'a> {
    pub packet: EnetPacket,
    pub channel_id: u8,
    _priv_guard: PhantomData<&'b EnetPeer<'a, T>>,
}

impl<'a, T> EnetPeer<'a, T> {
    pub(crate) fn new(inner: *mut ENetPeer) -> EnetPeer<'a, T> {
        EnetPeer {
            inner,
            _data: PhantomData,
        }
    }

    /// Returns the address of this `Peer`.
    pub fn address(&self) -> EnetAddress {
        EnetAddress::from_enet_address(&unsafe { (*self.inner).address })
    }

    /// Returns the amout of channels allocated for this `Peer`.
    pub fn channel_count(&self) -> usize {
        unsafe { (*self.inner).channelCount }
    }

    /// Returns a reference to the data associated with this `Peer`, if set.
    pub fn data(&self) -> Option<&T> {
        unsafe {
            let raw_data = (*self.inner).data as *const T;

            if raw_data.is_null() {
                None
            } else {
                Some(&(*raw_data))
            }
        }
    }

    /// Returns a mutable reference to the data associated with this `Peer`, if set.
    pub fn data_mut(&self) -> Option<&mut T> {
        unsafe {
            let raw_data = (*self.inner).data as *mut T;

            if raw_data.is_null() {
                None
            } else {
                Some(&mut (*raw_data))
            }
        }
    }

    /// Sets or clears the data associated with this `Peer`, replacing existing data.
    pub fn set_data(&mut self, data: Option<T>) {
        unsafe {
            let raw_data = (*self.inner).data as *mut T;

            if !raw_data.is_null() {
                // free old data
                let _: Box<T> = Box::from_raw(raw_data);
            }

            let new_data = match data {
                Some(data) => Box::into_raw(Box::new(data)) as *mut _,
                None => std::ptr::null_mut(),
            };

            (*self.inner).data = new_data;
        }
    }

    /// Returns the downstream bandwidth of this `Peer` in bytes/second.
    pub fn incoming_bandwidth(&self) -> u32 {
        unsafe { (*self.inner).incomingBandwidth }
    }

    /// Returns the upstream bandwidth of this `Peer` in bytes/second.
    pub fn outgoing_bandwidth(&self) -> u32 {
        unsafe { (*self.inner).outgoingBandwidth }
    }

    /// Returns the mean round trip time between sending a reliable packet and receiving its acknowledgement.
    pub fn mean_rtt(&self) -> Duration {
        Duration::from_millis(unsafe { (*self.inner).roundTripTime } as u64)
    }

    /// Forcefully disconnects this `Peer`.
    ///
    /// The foreign host represented by the peer is not notified of the disconnection and will timeout on its connection to the local host.
    pub fn reset(self) {
        unsafe {
            enet_peer_reset(self.inner);
        }
    }

    /// Queues a packet to be sent.
    ///
    /// Actual sending will happen during `Host::service`.
    pub fn send_packet(&mut self, packet: EnetPacket, channel_id: u8) -> Result<(), EnetFailure> {
        let res = unsafe { enet_peer_send(self.inner, channel_id, packet.into_inner()) };

        match res {
            r if r > 0 => panic!("unexpected res: {}", r),
            0 => Ok(()),
            r if r < 0 => Err(EnetFailure(r)),
            _ => panic!("unreachable"),
        }
    }

    /// Disconnects from this peer.
    ///
    /// A `Disconnect` event will be returned by `Host::service` once the disconnection is complete.
    pub fn disconnect(&mut self, user_data: u32) {
        unsafe {
            enet_peer_disconnect(self.inner, user_data);
        }
    }

    /// Disconnects from this peer immediately.
    ///
    /// No `Disconnect` event will be created. No disconnect notification for the foreign peer is guaranteed, and this `Peer` is immediately reset on return from this method.
    pub fn disconnect_now(self, user_data: u32) {
        unsafe {
            enet_peer_disconnect_now(self.inner, user_data);
        }
    }

    /// Disconnects from this peer after all outgoing packets have been sent.
    ///
    /// A `Disconnect` event will be returned by `Host::service` once the disconnection is complete.
    pub fn disconnect_later(&mut self, user_data: u32) {
        unsafe {
            enet_peer_disconnect_later(self.inner, user_data);
        }
    }

    /// Attempts to dequeue an incoming packet from this `Peer`.
    ///
    /// On success, returns the packet and the channel id of the receiving channel.
    pub fn receive<'b>(&'b mut self) -> Option<PeerPacket<'b, 'a, T>> {
        let mut channel_id = 0u8;

        let res = unsafe { enet_peer_receive(self.inner, &mut channel_id as *mut _) };

        if res.is_null() {
            return None;
        }

        Some(PeerPacket {
            packet: EnetPacket::from_sys_packet(res),
            channel_id,
            _priv_guard: PhantomData,
        })
    }
}
