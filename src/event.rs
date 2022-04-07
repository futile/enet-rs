#![allow(non_upper_case_globals)]
use enet_sys::{
    ENetEvent, _ENetEventType_ENET_EVENT_TYPE_CONNECT, _ENetEventType_ENET_EVENT_TYPE_DISCONNECT,
    _ENetEventType_ENET_EVENT_TYPE_NONE, _ENetEventType_ENET_EVENT_TYPE_RECEIVE,
};

use crate::{Host, Packet, Peer, PeerID};

/// This struct represents an event that can occur when servicing an `Host`.
#[derive(Debug)]
pub struct Event<'a, T> {
    peer: &'a mut Peer<T>,
    peer_id: PeerID,
    kind: EventKind,
}

/// The type of an event.
#[derive(Debug)]
pub enum EventKind {
    /// Peer has connected.
    Connect,
    /// Peer has disconnected.
    //
    /// The data of the peer will be dropped when the received `Event` is dropped.
    Disconnect {
        /// The data associated with this event. Usually a reason for disconnection.
        data: u32,
    },
    /// Peer has received a packet.
    Receive {
        /// ID of the channel that the packet was received on.
        channel_id: u8,
        /// The `Packet` that was received.
        packet: Packet,
    },
}

impl<'a, T> Event<'a, T> {
    pub(crate) fn from_sys_event(event_sys: ENetEvent, host: &'a Host<T>) -> Option<Event<'a, T>> {
        if event_sys.type_ == _ENetEventType_ENET_EVENT_TYPE_NONE {
            return None;
        }

        let peer = unsafe { Peer::new_mut(&mut *event_sys.peer) };
        let peer_id = unsafe { host.peer_id(event_sys.peer) };
        let kind = match event_sys.type_ {
            _ENetEventType_ENET_EVENT_TYPE_CONNECT => EventKind::Connect,
            _ENetEventType_ENET_EVENT_TYPE_DISCONNECT => EventKind::Disconnect {
                data: event_sys.data,
            },
            _ENetEventType_ENET_EVENT_TYPE_RECEIVE => EventKind::Receive {
                channel_id: event_sys.channelID,
                packet: Packet::from_sys_packet(event_sys.packet),
            },
            _ => panic!("unrecognized event type: {}", event_sys.type_),
        };

        Some(Event {
            peer,
            peer_id,
            kind,
        })
    }

    /// The peer that this event happened on.
    pub fn peer(&'_ self) -> &'_ Peer<T> {
        &*self.peer
    }

    /// The peer that this event happened on.
    pub fn peer_mut(&'_ mut self) -> &'_ mut Peer<T> {
        self.peer
    }

    /// The `PeerID` of the peer that this event happened on.
    pub fn peer_id(&self) -> PeerID {
        self.peer_id
    }

    /// The type of this event.
    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    /// Take the EventKind out of this event.
    /// If this peer is a Disconnect event, it will clean up the Peer.
    /// See the `Drop` implementation
    pub fn take_kind(mut self) -> EventKind {
        // Unfortunately we can't simply take the `kind` out of the Event, as otherwise the `Drop`
        // implementation would no longer work.
        // We can however, swap the actual EventKind with an empty EventKind (in this case
        // Connect).
        // As the `Drop` implementation will then do nothing, we need to call cleanup_after_disconnect before we do the swap.
        self.cleanup_after_disconnect();

        let mut kind = EventKind::Connect;
        std::mem::swap(&mut kind, &mut self.kind);
        kind
    }

    fn cleanup_after_disconnect(&mut self) {
        match self.kind {
            EventKind::Disconnect { .. } => self.peer.cleanup_after_disconnect(),
            EventKind::Connect | EventKind::Receive { .. } => {}
        }
    }
}

/// Dropping an `Event` with `EventKind::Disconnect` will clean up the Peer, by dropping
/// the data associated with the `Peer`, as well as invalidating the `PeerID`.
impl<'a, T> Drop for Event<'a, T> {
    fn drop(&mut self) {
        self.cleanup_after_disconnect();
    }
}
