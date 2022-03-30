use enet_sys::{
    enet_packet_create, enet_packet_destroy, ENetPacket, _ENetPacketFlag_ENET_PACKET_FLAG_RELIABLE,
    _ENetPacketFlag_ENET_PACKET_FLAG_UNSEQUENCED,
};

use crate::Error;

/// A packet that can be sent or retrieved on an ENet-connection.
#[derive(Debug)]
pub struct Packet {
    inner: *mut ENetPacket,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
/// Mode that can be set when transmitting a packet.
///
/// ENet does not support reliable but unsequenced packets.
pub enum PacketMode {
    /// The packet will be sent unreliably but sequenced (ENet default).
    UnreliableSequenced,
    /// The packet will be sent unreliably and unsequenced.
    UnreliableUnsequenced,
    /// The packet will be sent reliably and sequenced with other reliable
    /// packets.
    ReliableSequenced,
}

impl PacketMode {
    /// Returns whether this represents a reliable mode.
    pub fn is_reliable(&self) -> bool {
        match self {
            PacketMode::UnreliableSequenced => false,
            PacketMode::UnreliableUnsequenced => false,
            PacketMode::ReliableSequenced => true,
        }
    }

    /// Returns whether this represents a sequenced mode.
    pub fn is_sequenced(&self) -> bool {
        match self {
            PacketMode::UnreliableSequenced => true,
            PacketMode::UnreliableUnsequenced => false,
            PacketMode::ReliableSequenced => true,
        }
    }

    fn to_sys_flags(self) -> u32 {
        match self {
            PacketMode::UnreliableSequenced => 0,
            PacketMode::UnreliableUnsequenced => {
                _ENetPacketFlag_ENET_PACKET_FLAG_UNSEQUENCED as u32
            }
            PacketMode::ReliableSequenced => _ENetPacketFlag_ENET_PACKET_FLAG_RELIABLE as u32,
        }
    }
}

impl Packet {
    /// Creates a new Packet with optional reliability settings.
    pub fn new(data: &[u8], mode: PacketMode) -> Result<Packet, Error> {
        let data_len: enet_sys::size_t = data
            .len()
            .try_into()
            // this can only happen on 64-bit systems if `size_t` is 32-bit wide (windows?)
            .expect("packet data too long for ENet (`size_t`)");

        let res =
            unsafe { enet_packet_create(data.as_ptr() as *const _, data_len, mode.to_sys_flags()) };

        if res.is_null() {
            return Err(Error(0));
        }

        Ok(Packet::from_sys_packet(res))
    }

    pub(crate) fn from_sys_packet(inner: *mut ENetPacket) -> Packet {
        Packet { inner }
    }

    /// Does NOT run this `Packet`'s destructor.
    pub(crate) fn into_inner(self) -> *mut ENetPacket {
        let res = self.inner;
        std::mem::forget(self);
        res
    }

    /// Returns a reference to the bytes inside this packet.
    pub fn data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (*self.inner).data,
                (*self.inner)
                    .dataLength
                    .try_into()
                    // this can only happen when a too long packet is received on a 32-bit system I
                    // think
                    .expect("packet data too long for an `usize`"),
            )
        }
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe {
            enet_packet_destroy(self.inner);
        }
    }
}
