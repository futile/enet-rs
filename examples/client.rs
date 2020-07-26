extern crate enet;

use std::net::Ipv4Addr;

use enet::*;
use std::time::Duration;

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

    let peer_id = loop {
        let e = host
            .service(Duration::from_secs(1))
            .expect("service failed");

        let e = match e {
            Some(ev) => ev,
            _ => continue,
        };

        println!("[client] event: {:#?}", e);

        match e.kind {
            EventKind::Connect => break e.peer_id,
            EventKind::Disconnect { data } => {
                println!(
                    "connection NOT successful, peer: {:?}, reason: {}",
                    e.peer_id, data
                );
                std::process::exit(0);
            }
            EventKind::Receive { .. } => {
                panic!("unexpected Receive-event while waiting for connection")
            }
        };
    };

    // send a "hello"-like packet
    host[peer_id]
        .send_packet(
            Packet::from_vec(b"harro".to_vec(), PacketMode::ReliableSequenced).unwrap(),
            1,
        )
        .unwrap();

    // disconnect after all outgoing packets have been sent.
    host[peer_id].disconnect_later(5);

    loop {
        let e = host.service(Duration::from_secs(1)).unwrap();
        println!("received event: {:#?}", e);
    }
}
