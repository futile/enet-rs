//! This crate provides high-level bindings for the [ENet](http://enet.bespin.org/) networking library.
//!
//! ENet provides features that are most typically used by games, such as unreliable but sequenced data transfer over UDP.
//! ENet also provides optional reliability, and provides multiple channels over a single connection.
//! For more info see the [ENet website](http://enet.bespin.org/).
//!
//! This crate aims to provide high-level, rust-y binding for the ENet library, based on existing low-level [C-bindings](https://crates.io/crates/enet-sys), so users don't have to deal with ffi.
//!
//! The official ENet documentation and tutorials are a good starting point to working with ENet.
//! Most principles and names should be straight-forward to transfer to this library.
//!
//! # Examples
//! This will initialize ENet and deinitialize it when the `Enet`-instance - and all references to it - are dropped:
//!
//! ```
//! use enet::Enet;
//!
//! // `Enet::new()` initializes ENet when it is first called.
//! let enet = Enet::new().unwrap();
//!
//! // Deinitialization is handled automatically (using Arc).
//! ```
//!
//! Also check out the examples in the code, as well as the examples from the official ENet website and from the enet-sys crate.
//!
//! # Thread-safety
//! ENet claims to be "mostly" thread-safe as long as access to individual `Host`-instances is handled in a synchronized manner.
//! This is kind of an unclear statement, but this API tries to follow that as good as possible.
//! So if the rust compilers allows you to send/sync an object between threads, it should be safe to do so.
//!
//! If you used no unsafe code and the library blows up in your face, that is considered a bug. Please report any such bug you encounter via [github](https://github.com/futile/enet-rs).

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

use enet_sys::{enet_deinitialize, enet_host_create, enet_initialize, enet_linked_version};

mod address;
mod host;

pub use crate::address::EnetAddress;
pub use crate::host::Host;

pub use enet_sys::ENetVersion as EnetVersion;

const ENET_UNINITIALIZED: usize = 1;
const ENET_INITIALIZED: usize = 2;
const ENET_DEINITIALIZED: usize = 3;

static ENET_STATUS: AtomicUsize = AtomicUsize::new(ENET_UNINITIALIZED);

/// Main API entry point. Provides methods such as host and peer creation.
///
/// Creating an instance of this struct for the first time (using `new`) will initialize ENet.
/// Afterwards, this struct can be used to performs tasks 
#[derive(Debug)]
pub struct Enet {
    _reserved: (),
}

/// Generic ENet error, returned by many API functions.
#[derive(Fail, Debug)]
#[fail(display = "enet failure, returned '{}'", _0)]
pub struct EnetFailure(pub c_int);

/// An error that can occur when initializing ENet.
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

        Ok(Arc::new(Enet { _reserved: () }))
    }

    pub fn linked_version() -> EnetVersion {
        unsafe { enet_linked_version() }
    }

    pub fn create_host(
        self: &Arc<Self>,
        address: &EnetAddress,
        max_peer_count: usize,
        max_channel_count: Option<usize>,
        incoming_bandwidth: Option<u32>,
        outgoing_bandwidth: Option<u32>,
    ) -> Result<Host, EnetFailure> {
        use enet_sys::ENetAddress;

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
    use super::Enet;
    use std::sync::Arc;

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
        enet.create_host(
            &EnetAddress::new(Ipv4Addr::LOCALHOST, 12345),
            1,
            None,
            None,
            None,
        )
        .unwrap();
    }
}
