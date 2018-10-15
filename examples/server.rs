extern crate enet;

use std::net::Ipv4Addr;

use enet::*;

fn main() {
    let enet = Enet::new().expect("could not initialize ENet");

    let local_addr = EnetAddress::new(Ipv4Addr::LOCALHOST, 9001);

    let _host = enet.create_host::<()>(
        &local_addr,
        10,
        ChannelLimit::Maximum,
        BandwidthLimit::Unlimited,
        BandwidthLimit::Unlimited,
    ).expect("could not create host");
}
