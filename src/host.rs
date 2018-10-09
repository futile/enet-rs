use std::sync::Arc;

use crate::EnetKeepAlive;

use enet_sys::{enet_host_destroy, ENetHost};

/// A `Host` represents one endpoint of an ENet connection. Created through `Enet`.
///
/// This type provides functionality such as connection establishment and packet transmission.
pub struct Host {
    _keep_alive: Arc<EnetKeepAlive>,
    inner: *mut ENetHost,
}

impl Host {
    pub(in crate) fn new(_keep_alive: Arc<EnetKeepAlive>, inner: *mut ENetHost) -> Host {
        assert!(!inner.is_null());

        Host { _keep_alive, inner }
    }
}

impl Drop for Host {
    /// Call the corresponding ENet cleanup-function(s).
    fn drop(&mut self) {
        unsafe {
            enet_host_destroy(self.inner);
        }
    }
}
