use std::marker::PhantomData;
use std::time::Duration;

use enet_sys::{ENetPeer};

use crate::{EnetAddress};

pub struct EnetPeer<'a, T: 'a> {
    inner: *mut ENetPeer,

    _data: PhantomData<&'a mut T>,
}

impl<'a, T> EnetPeer<'a, T> {
    pub(crate) fn new(inner: *mut ENetPeer) -> EnetPeer<'a, T> {
        EnetPeer {
            inner,
            _data: PhantomData,
        }
    }

    pub fn address(&self) -> EnetAddress {
        EnetAddress::from_enet_address(& unsafe {
            (*self.inner).address
        })
    }

    /// Returns the amout of channels allocated for this `Peer`.
    pub fn channel_count(&self) -> usize {
        unsafe {
            (*self.inner).channelCount
        }
    }

    /// Returns a reference to the data associated with this `Peer`, if set.
    pub fn data(&self) -> Option<&T> {
        unsafe {
            let raw_data = (*self.inner).data as *const T;

            if raw_data.is_null() {
                None
            } else {
                Some(&(*raw_data))
            }
        }
    }

    /// Returns a mutable reference to the data associated with this `Peer`, if set.
    pub fn data_mut(&self) -> Option<&mut T> {
        unsafe {
            let raw_data = (*self.inner).data as *mut T;

            if raw_data.is_null() {
                None
            } else {
                Some(&mut (*raw_data))
            }
        }
    }

    /// Sets or clears the data associated with this `Peer`, replacing existing data.
    pub fn set_data(&mut self, data: Option<T>) {
        unsafe {
            let raw_data = (*self.inner).data as *mut T;

            if !raw_data.is_null() {
                // free old data
                let _: Box<T> = Box::from_raw(raw_data);
            }

            let new_data = match data {
                Some(data) => Box::into_raw(Box::new(data)) as *mut _,
                None => std::ptr::null_mut(),
            };

            (*self.inner).data = new_data;
        }
    }

    /// Returns the downstream bandwidth of this `Peer` in bytes/second.
    pub fn incoming_bandwidth(&self) -> u32 {
        unsafe {
            (*self.inner).incomingBandwidth
        }
    }

    /// Returns the upstream bandwidth of this `Peer` in bytes/second.
    pub fn outgoing_bandwidth(&self) -> u32 {
        unsafe {
            (*self.inner).outgoingBandwidth
        }
    }

    /// Returns the mean round trip time between sending a reliable packet and receiving its acknowledgement.
    pub fn mean_rtt(&self) -> Duration {
        Duration::from_millis(unsafe {
            (*self.inner).roundTripTime
        } as u64)
    }
}
