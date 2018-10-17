extern crate enet;

use std::net::Ipv4Addr;

use enet::*;

fn main() {
    let enet = Enet::new().expect("could not initialize ENet");

    let local_addr = EnetAddress::new(Ipv4Addr::LOCALHOST, 9001);

    let mut host = enet
        .create_host::<()>(
            Some(&local_addr),
            10,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .expect("could not create host");

    loop {
        match host.service(1000).expect("service failed") {
            Some(EnetEvent::Connect(_)) => println!("new connection!"),
            Some(EnetEvent::Disconnect(..)) => println!("disconnect!"),
            Some(EnetEvent::Receive {
                channel_id,
                ref packet,
                ..
            }) => println!("got packet on channel {}, content: '{}'", channel_id,
                         std::str::from_utf8(packet.data()).unwrap()),
            _ => (),
        }
    }
}
