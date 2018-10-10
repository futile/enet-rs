use enet_sys::{
    ENetEvent, _ENetEventType_ENET_EVENT_TYPE_CONNECT, _ENetEventType_ENET_EVENT_TYPE_DISCONNECT,
    _ENetEventType_ENET_EVENT_TYPE_NONE, _ENetEventType_ENET_EVENT_TYPE_RECEIVE,
};

use crate::{EnetPacket, EnetPeer};

pub enum EnetEvent<'a, T> {
    Connect(EnetPeer<'a, T>),
    Disconnect(EnetPeer<'a, T>, u32),
    Receive {
        sender: EnetPeer<'a, T>,
        channel_id: u8,
        packet: EnetPacket,
    },
}

impl<'a, T> EnetEvent<'a, T> {
    pub(crate) fn from_sys_event<'b>(event_sys: &'b ENetEvent) -> Option<EnetEvent<'a, T>> {
        #[allow(non_upper_case_globals)]
        match event_sys.type_ {
            _ENetEventType_ENET_EVENT_TYPE_NONE => None,
            _ENetEventType_ENET_EVENT_TYPE_CONNECT => {
                Some(EnetEvent::Connect(EnetPeer::new(event_sys.peer)))
            }
            _ENetEventType_ENET_EVENT_TYPE_DISCONNECT => Some(EnetEvent::Disconnect(
                EnetPeer::new(event_sys.peer),
                event_sys.data,
            )),
            _ENetEventType_ENET_EVENT_TYPE_RECEIVE => {
                Some(EnetEvent::Receive {
                    sender: EnetPeer::new(event_sys.peer),
                    channel_id: event_sys.channelID,
                    packet: EnetPacket::new(event_sys.packet),
                })
            }
            _ => panic!("unrecognized event type: {}", event_sys.type_),
        }
    }
}

impl<'a, T> Drop for EnetEvent<'a, T> {
    fn drop(&mut self) {
        match self {
            // Seemingly, the lifetime of an ENetPeer ends with the end of the Disconnect event.
            // However, this is *not really clear* in the ENet docs!
            // It looks like the Peer *might* live longer, but not shorter, so it should be safe
            // to destroy the associated data (if any) here.
            EnetEvent::Disconnect(peer, _) => peer.set_data(None),
            _ => (),
        }
    }
}
