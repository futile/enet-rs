#![allow(non_upper_case_globals)]
use enet_sys::{
    ENetEvent, _ENetEventType_ENET_EVENT_TYPE_CONNECT, _ENetEventType_ENET_EVENT_TYPE_DISCONNECT,
    _ENetEventType_ENET_EVENT_TYPE_NONE, _ENetEventType_ENET_EVENT_TYPE_RECEIVE,
};

use crate::{Packet, Peer};

pub struct Event<'a, T> {
    pub peer: &'a mut Peer<T>,
    pub kind: EventKind,
}

#[derive(Debug)]
pub enum EventKind {
    Connect,
    Disconnect { data: u32 },
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
