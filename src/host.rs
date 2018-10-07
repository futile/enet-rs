use std::sync::Arc;

use crate::{Enet, EnetFailure};

use enet_sys::{enet_host_destroy, ENetHost};

/// A `Host` represents one endpoint of an ENet connection. Created through `Enet`.
pub struct Host {
    _enet: Arc<Enet>,
    inner: *mut ENetHost,
}

impl Host {
    pub(in crate) fn new(_enet: Arc<Enet>, inner: *mut ENetHost) -> Host {
        assert!(!inner.is_null());

        Host { _enet, inner }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        unsafe {
            enet_host_destroy(self.inner);
        }
    }
}
