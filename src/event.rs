#![allow(non_upper_case_globals)]
use enet_sys::{
    ENetEvent, _ENetEventType_ENET_EVENT_TYPE_CONNECT, _ENetEventType_ENET_EVENT_TYPE_DISCONNECT,
    _ENetEventType_ENET_EVENT_TYPE_NONE, _ENetEventType_ENET_EVENT_TYPE_RECEIVE,
};

use crate::{Host, Packet, PeerID};

/// This struct represents an event that can occur when servicing an `Host`.
#[derive(Debug)]
pub struct Event {
    /// The peer that this event happened on.
    pub peer_id: PeerID,
    /// The type of this event.
    pub kind: EventKind,
}

/// The type of an event.
#[derive(Debug)]
pub enum EventKind {
    /// Peer has connected.
    Connect,
    /// Peer has disconnected.
    //
    /// The data of the peer will be dropped on the next call to Host::service or when the structure is dropped.
    Disconnect {
        /// The data associated with this event. Usually a reason for disconnection.
        data: u32,
    },
    /// Peer has received a packet.
    Receive {
        /// ID of the channel that the packet was received on.
        channel_id: u8,
        /// The received packet.
        packet: Packet,
    },
}

impl Event {
    pub(crate) fn from_sys_event<T>(event_sys: ENetEvent, host: &Host<T>) -> Option<Event> {
        if event_sys.type_ == _ENetEventType_ENET_EVENT_TYPE_NONE {
            return None;
        }

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
            _ => panic!("unexpected event type: {}", event_sys.type_),
        };

        Some(Event { peer_id, kind })
    }
}
