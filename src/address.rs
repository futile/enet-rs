use std::ffi::CString;
use std::net::{Ipv4Addr, SocketAddrV4};

use byteorder::{NetworkEndian, ReadBytesExt};

use crate::EnetFailure;

use enet_sys::ENetAddress;

/// An IPv4 address that can be used with the ENet API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnetAddress {
    addr: SocketAddrV4,
}

impl EnetAddress {
    pub fn new(addr: Ipv4Addr, port: u16) -> EnetAddress {
        EnetAddress {
            addr: SocketAddrV4::new(addr, port),
        }
    }

    pub fn from_hostname(hostname: &CString, port: u16) -> Result<EnetAddress, EnetFailure> {
        use enet_sys::{enet_address_set_host, ENetAddress};

        let mut addr = ENetAddress { host: 0, port };

        let res =
            unsafe { enet_address_set_host(&mut addr as *mut ENetAddress, hostname.as_ptr()) };

        if res != 0 {
            return Err(EnetFailure(res));
        }

        Ok(EnetAddress::new(
            Ipv4Addr::from(u32::from_be(addr.host)),
            addr.port,
        ))
    }

    pub fn ip(&self) -> &Ipv4Addr {
        self.addr.ip()
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub(crate) fn to_enet_address(&self) -> ENetAddress {
        ENetAddress {
            host: (&self.ip().octets() as &[u8])
                .read_u32::<NetworkEndian>()
                .unwrap()
                .to_be(),
            port: self.port(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnetAddress;

    use std::ffi::CString;
    use std::net::Ipv4Addr;

    #[test]
    fn test_from_valid_hostname() {
        let addr = EnetAddress::from_hostname(&CString::new("localhost").unwrap(), 0).unwrap();
        assert_eq!(addr.addr.ip(), &Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(addr.addr.port(), 0);
    }

    #[test]
    fn test_from_invalid_hostname() {
        assert!(EnetAddress::from_hostname(&CString::new("").unwrap(), 0).is_err());
    }
}
