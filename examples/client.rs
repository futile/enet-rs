extern crate enet;

use std::net::Ipv4Addr;

use enet::*;

fn main() {
    let enet = Enet::new().expect("could not initialize ENet");

    let mut host = enet
        .create_host::<()>(
            None,
            10,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .expect("could not create host");

    host.connect(&EnetAddress::new(Ipv4Addr::LOCALHOST, 9001), 10, 0);

    loop {
        let e = host.service(1000).expect("service failed");

        println!("[client] event: {:#?}", e);
    }
}
