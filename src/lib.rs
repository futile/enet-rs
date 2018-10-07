#![feature(arbitrary_self_types)]

#[macro_use]
extern crate failure_derive;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use std::{
    os::raw::c_int,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use enet_sys::{enet_deinitialize, enet_initialize, enet_linked_version, enet_host_create};

mod host;
mod address;

pub use crate::host::Host;
pub use crate::address::EnetAddress;

pub use enet_sys::{ENetVersion as EnetVersion, ENetAddress};

const ENET_UNINITIALIZED: usize = 1;
const ENET_INITIALIZED: usize = 2;
const ENET_DEINITIALIZED: usize = 3;

static ENET_STATUS: AtomicUsize = AtomicUsize::new(ENET_UNINITIALIZED);

#[derive(Debug)]
pub struct Enet {
    _reserved: ()
}

#[derive(Fail, Debug)]
#[fail(display = "enet failure, returned '{}'", _0)]
pub struct EnetFailure(c_int);

#[derive(Fail, Debug)]
pub enum InitializationError {
    #[fail(display = "ENet has already been initialized before")]
    AlreadyInitialized,
    #[fail(display = "ENet has already been deinitialized before")]
    AlreadyDeinitialized,
    #[fail(display = "enet_initialize failed (with '{}')", _0)]
    EnetFailure(c_int),
}

impl Enet {
    pub fn new() -> Result<Arc<Enet>, InitializationError> {
        match ENET_STATUS.compare_and_swap(ENET_UNINITIALIZED, ENET_INITIALIZED, Ordering::SeqCst) {
            ENET_UNINITIALIZED => (),
            ENET_INITIALIZED => return Err(InitializationError::AlreadyInitialized),
            ENET_DEINITIALIZED => return Err(InitializationError::AlreadyDeinitialized),
            u => panic!(
                "enet-rs internal error; unexpected value in ENET_STATUS (new): {}",
                u
            ),
        };

        let r = unsafe { enet_initialize() };

        if r != 0 {
            return Err(InitializationError::EnetFailure(r));
        }

        Ok(Arc::new(Enet {
            _reserved: (),
        }))
    }

    pub fn linked_version() -> EnetVersion {
        unsafe { enet_linked_version() }
    }

    pub fn create_host(self: &Arc<Self>,
                       address: &EnetAddress,
                       max_peer_count: usize,
                       max_channel_count: Option<usize>,
                       incoming_bandwidth: Option<u32>,
                       outgoing_bandwidth: Option<u32>,
    ) -> Result<Host, EnetFailure> {
        let addr = address.to_enet_address();
        let inner = unsafe {
            enet_host_create(
                &addr as *const ENetAddress,
                max_peer_count,
                max_channel_count.unwrap_or(0),
                incoming_bandwidth.unwrap_or(0),
                outgoing_bandwidth.unwrap_or(0),
            )
        };

        if inner.is_null() {
            return Err(EnetFailure(0));
        }

        Ok(Host::new(self.clone(), inner))
    }
}

impl Drop for Enet {
    fn drop(&mut self) {
        match ENET_STATUS.compare_and_swap(ENET_INITIALIZED, ENET_DEINITIALIZED, Ordering::SeqCst) {
            ENET_INITIALIZED => (),
            other => panic!(
                "enet-rs internal error; unexpected value in ENET_STATUS (drop): {}",
                other
            ),
        };

        unsafe {
            enet_deinitialize();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::Enet;

    lazy_static! {
        static ref ENET: Arc<Enet> = Enet::new().unwrap();
    }

    #[test]
    fn test_enet_new() {
        let _ = *ENET; // make sure the lazy_static is initialized
        assert!(Enet::new().is_err());
    }

    #[test]
    fn test_host_create_localhost() {
        use crate::EnetAddress;
        use std::net::Ipv4Addr;

        let enet = &*ENET;
        enet.create_host(&EnetAddress::new(Ipv4Addr::LOCALHOST, 12345), 1, None, None, None).unwrap();
    }
}
