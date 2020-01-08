#![allow(non_upper_case_globals)]
use enet_sys::{
    ENetEvent, _ENetEventType_ENET_EVENT_TYPE_CONNECT, _ENetEventType_ENET_EVENT_TYPE_DISCONNECT,
    _ENetEventType_ENET_EVENT_TYPE_NONE, _ENetEventType_ENET_EVENT_TYPE_RECEIVE,
};

use crate::{Packet, Peer};

/// This struct represents an event that can occur when servicing an `EnetHost`.
pub struct Event<'a, T> {
    /// The peer that this event happened on.
    pub peer: &'a mut Peer<T>,
    /// The type of this event.
    pub kind: EventKind,
}

#[derive(Debug)]
pub enum EventKind {
    /// Peer has connected.
    Connect,
    /// Peer has disconnected.
    Disconnect { data: u32 },
    /// Peer has received a packet.
    Receive { channel_id: u8, packet: Packet },
}

impl<'a, T> Event<'a, T> {
    pub(crate) fn from_sys_event(event_sys: &ENetEvent) -> Option<Event<'a, T>> {
        if event_sys.type_ == _ENetEventType_ENET_EVENT_TYPE_NONE {
            return None;
        }

        let peer = Peer::new_mut(unsafe { &mut *event_sys.peer });
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

        Some(Event { peer, kind })
    }
}
