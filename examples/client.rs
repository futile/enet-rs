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

    host.connect(&Address::new(Ipv4Addr::LOCALHOST, 9001), 10, 0)
        .expect("connect failed");

    let mut peer = loop {
        let e = host.service(1000).expect("service failed");

        let e = match e {
            Some(ev) => ev,
            _ => continue,
        };

        println!("[client] event: {:#?}", e);

        match e {
            Event::Connect(ref p) => {
                break p.clone();
            }
            Event::Disconnect(ref p, r) => {
                println!("connection NOT successful, peer: {:?}, reason: {}", p, r);
                std::process::exit(0);
            }
            Event::Receive { .. } => {
                panic!("unexpected Receive-event while waiting for connection")
            }
        };
    };

    // send a "hello"-like packet
    peer.send_packet(
        Packet::new(b"harro", PacketMode::ReliableSequenced).unwrap(),
        1,
    )
    .unwrap();

    // disconnect after all outgoing packets have been sent.
    peer.disconnect_later(5);

    loop {
        let e = host.service(1000).unwrap();
        println!("received event: {:#?}", e);
    }
}
