//! This crate provides high-level bindings for the [ENet](http://enet.bespin.org/) networking library.
//!
//! ENet provides features that are most typically used by games, such as
//! unreliable but sequenced data transfer over UDP. ENet also provides optional
//! reliability, and provides multiple channels over a single connection. For more info see the [ENet website](http://enet.bespin.org/).
//!
//! This crate aims to provide high-level, rust-y binding for the ENet library, based on existing low-level [C-bindings](https://crates.io/crates/enet-sys), so users don't have to deal with ffi.
//!
//! The official ENet documentation and tutorials are a good starting point to
//! working with ENet. Most principles and names should be straight-forward to
//! transfer to this library.
//!
//! # Examples
//! This will initialize ENet and deinitialize it when the `Enet`-instance - and
//! all references to it - are dropped:
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
//! Also check out the examples in the code, as well as the examples from the official ENet website and from the enet-sys crate. There are also an example server and client in the `examples` directory on [github](https://github.com/futile/enet-rs).
//!
//! # Thread-safety
//! ENet claims to be "mostly" thread-safe as long as access to individual
//! `Host`-instances is handled in a synchronized manner. This is kind of an
//! unclear statement, but this API tries to follow that as good as possible. So
//! if the rust compilers allows you to send/sync an object between threads, it
//! should be safe to do so.
//!
//! If you used no unsafe code and the library blows up in your face, that is considered a bug. Please report any bug you encounter via [github](https://github.com/futile/enet-rs).

#![warn(missing_docs)]

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
mod event;
mod host;
mod packet;
mod peer;

pub use enet_sys::ENetVersion as EnetVersion;

pub use crate::{
    address::Address,
    event::Event,
    host::{BandwidthLimit, ChannelLimit, Host},
    packet::{Packet, PacketMode},
    peer::{Peer, PeerPacket, PeerState},
};

const ENET_UNINITIALIZED: usize = 1;
const ENET_INITIALIZED: usize = 2;
const ENET_DEINITIALIZED: usize = 3;

static ENET_STATUS: AtomicUsize = AtomicUsize::new(ENET_UNINITIALIZED);

#[derive(Debug, Clone)]
struct EnetKeepAlive;

/// Main API entry point. Provides methods such as host and peer creation.
///
/// Creating an instance of this struct for the first time (using `new()`) will
/// initialize ENet. Further attempts to create instances will result in errors,
/// so it can only be constructed once (but it can be cloned).
///
/// This struct can be used to performs most top-level ENet functionality, such
/// as host creation and connection establishment.
#[derive(Debug, Clone)]
pub struct Enet {
    keep_alive: Arc<EnetKeepAlive>,
}

/// Generic ENet error, returned by many API functions.
///
/// Contains the return value of the failed function call.
#[derive(thiserror::Error, Debug)]
#[error("enet failure, returned '{}'", .0)]
pub struct Error(pub c_int);

/// An error that can occur when initializing ENet.
#[derive(thiserror::Error, Debug)]
pub enum InitializationError {
    /// ENet was already initialized. `Enet::new()` can only (successfully) be
    /// called once, so reuse that object.
    #[error("ENet has already been initialized before")]
    AlreadyInitialized,
    /// ENet was already deinitialized. Probably continue using your previous
    /// `Enet`-instance.
    #[error("ENet has already been deinitialized before")]
    AlreadyDeinitialized,
    /// Internal ENet failure (`enet_initialize` failed), containing the return
    /// code.
    #[error("enet_initialize failed (with '{}')", .0)]
    Error(c_int),
}

impl Enet {
    /// Initializes ENet and returns a handle to the top-level functionality, in
    /// the form of an `Enet`-instance.
    pub fn new() -> Result<Enet, InitializationError> {
        match ENET_STATUS.compare_exchange(
            ENET_UNINITIALIZED,
            ENET_INITIALIZED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => (),
            Err(ENET_INITIALIZED) => return Err(InitializationError::AlreadyInitialized),
            Err(ENET_DEINITIALIZED) => return Err(InitializationError::AlreadyDeinitialized),
            Err(u) => panic!(
                "enet-rs internal error; unexpected value in ENET_STATUS (new): {}",
                u
            ),
        };

        let r = unsafe { enet_initialize() };

        if r != 0 {
            return Err(InitializationError::Error(r));
        }

        Ok(Enet {
            keep_alive: Arc::new(EnetKeepAlive),
        })
    }

    /// Creates a `Host`. A `Host` is an endpoint of an ENet connection. For
    /// more information consult the official ENet-documentation.
    ///
    /// `address` specifies the address to listen on. Client-only endpoints can
    /// choose `None`. `max_channel_count` will be set to its
    /// (ENet-specified) default value if `None`.
    ///
    /// The type `T` specifies the data associated with corresponding `Peer`s.
    pub fn create_host<T>(
        &self,
        address: Option<&Address>,
        max_peer_count: enet_sys::size_t,
        max_channel_count: ChannelLimit,
        incoming_bandwidth: BandwidthLimit,
        outgoing_bandwidth: BandwidthLimit,
    ) -> Result<Host<T>, Error> {
        let addr = address.map(Address::to_enet_address);
        let inner = unsafe {
            enet_host_create(
                addr.as_ref()
                    .map(|p| p as *const _)
                    .unwrap_or(std::ptr::null()),
                max_peer_count,
                max_channel_count.to_enet_val(),
                incoming_bandwidth.to_enet_u32(),
                outgoing_bandwidth.to_enet_u32(),
            )
        };

        if inner.is_null() {
            return Err(Error(0));
        }

        Ok(Host::new(self.keep_alive.clone(), inner))
    }
}

/// Returns the version of the linked ENet library.
pub fn linked_version() -> EnetVersion {
    unsafe { enet_linked_version() }
}

impl Drop for EnetKeepAlive {
    fn drop(&mut self) {
        match ENET_STATUS.compare_exchange(
            ENET_INITIALIZED,
            ENET_DEINITIALIZED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => (),
            Err(other) => panic!(
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
    use super::{BandwidthLimit, ChannelLimit, Enet};

    lazy_static! {
        static ref ENET: Enet = Enet::new().unwrap();
    }

    #[test]
    fn test_enet_new() {
        let _ = *ENET; // make sure the lazy_static is initialized
        assert!(Enet::new().is_err());
    }

    #[test]
    fn test_host_create_localhost() {
        use std::net::Ipv4Addr;

        use crate::Address;

        let enet = &ENET;
        enet.create_host::<()>(
            Some(&Address::new(Ipv4Addr::LOCALHOST, 12345)),
            1,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .unwrap();
    }
}
