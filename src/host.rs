use std::sync::Arc;

use crate::{Enet, EnetFailure, EnetAddress};

use enet_sys::{ENetAddress, ENetHost, enet_host_create};

pub struct Host {
    _enet: Arc<Enet>,
    inner: *mut ENetHost,
}

impl Host {
    pub(in crate) fn new(_enet: Arc<Enet>, inner: *mut ENetHost) -> Host {
        Host {
            _enet,
            inner,
        }
    }
}
