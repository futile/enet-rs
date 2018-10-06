#[macro_use]
extern crate failure_derive;

use core::marker::PhantomData;

use std::{
    os::raw::c_int,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use enet_sys::{enet_deinitialize, enet_initialize, enet_linked_version};

const ENET_UNINITIALIZED: usize = 1;
const ENET_INITIALIZED: usize = 2;
const ENET_DEINITIALIZED: usize = 3;

static ENET_STATUS: AtomicUsize = AtomicUsize::new(ENET_UNINITIALIZED);

pub use enet_sys::ENetVersion;

pub struct Enet {
    _not_send_and_sync: PhantomData<*const ()>,
}

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
            _not_send_and_sync: Default::default(),
        }))
    }

    pub fn linked_version() -> ENetVersion {
        unsafe { enet_linked_version() }
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

    #[test]
    fn test_enet_new() {
        {
            let _ = Enet::new().unwrap();
            assert!(Enet::new().is_err());
        }
        assert!(Enet::new().is_err());
    }
}
