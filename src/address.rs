use std::{
    ffi::CString,
    net::{Ipv4Addr, SocketAddrV4},
};

use enet_sys::ENetAddress;

use crate::Error;

/// An IPv4 address that can be used with the ENet API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    addr: SocketAddrV4,
}

impl Address {
    /// Create a new address from an ip and a port.
    pub fn new(addr: Ipv4Addr, port: u16) -> Address {
        Address {
            addr: SocketAddrV4::new(addr, port),
        }
    }

    /// Create a new address from a given hostname.
    pub fn from_hostname(hostname: &CString, port: u16) -> Result<Address, Error> {
        use enet_sys::enet_address_set_host;

        let mut addr = ENetAddress { host: 0, port };

        let res =
            unsafe { enet_address_set_host(&mut addr as *mut ENetAddress, hostname.as_ptr()) };

        if res != 0 {
            return Err(Error(res));
        }

        Ok(Self::from_enet_address(&addr))
    }

    /// Return the ip of this address
    pub fn ip(&self) -> &Ipv4Addr {
        self.addr.ip()
    }

    /// Returns the port of this address
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub(crate) fn to_enet_address(&self) -> ENetAddress {
        ENetAddress {
            // Use native byte order here, the octets are already arranged in the correct byte
            // order, don't change it
            host: u32::from_ne_bytes(self.ip().octets()),
            port: self.port(),
        }
    }

    pub(crate) fn from_enet_address(addr: &ENetAddress) -> Address {
        // Split using native byte order here, as Enet guarantees that our bytes are
        // already layed out in network byte order.
        Address::new(Ipv4Addr::from(addr.host.to_ne_bytes()), addr.port)
    }
}

impl From<SocketAddrV4> for Address {
    fn from(addr: SocketAddrV4) -> Address {
        Address { addr }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::CString, net::Ipv4Addr};

    use super::Address;

    #[test]
    fn test_from_valid_hostname() {
        let addr = Address::from_hostname(&CString::new("localhost").unwrap(), 0).unwrap();
        assert_eq!(addr.addr.ip(), &Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(addr.addr.port(), 0);
    }

    #[test]
    fn test_from_invalid_hostname() {
        assert!(Address::from_hostname(&CString::new("").unwrap(), 0).is_err());
    }
}
