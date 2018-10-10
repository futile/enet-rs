use enet_sys::{enet_packet_destroy, ENetPacket};

pub struct EnetPacket {
    inner: *mut ENetPacket,
}

impl EnetPacket {
    pub(crate) fn new(inner: *mut ENetPacket) -> EnetPacket {
        EnetPacket {
            inner
        }
    }

    pub(crate) fn into_inner(self) -> *mut ENetPacket {
        self.inner
    }

    pub fn data<'a>(&'a self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts((*self.inner).data, (*self.inner).dataLength) }
    }
}

impl Drop for EnetPacket {
    fn drop(&mut self) {
        unsafe {
            enet_packet_destroy(self.inner);
        }
    }
}
