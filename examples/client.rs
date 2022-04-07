extern crate enet;

use std::net::Ipv4Addr;

use anyhow::Context;
use enet::*;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let enet = Enet::new().context("could not initialize ENet")?;

    let mut host = enet
        .create_host::<()>(
            None,
            10,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .context("could not create host")?;

    host.connect(&Address::new(Ipv4Addr::LOCALHOST, 9001), 10, 0)
        .context("connect failed")?;

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
                anyhow::bail!("unexpected Receive-event while waiting for connection")
            }
        };
    };

    let peer = host.peer_mut(peer_id).unwrap();
    // send a "hello"-like packet
    peer.send_packet(
        Packet::new(b"harro".to_vec(), PacketMode::ReliableSequenced).unwrap(),
        1,
    )
    .context("sending packet failed")?;

    // disconnect after all outgoing packets have been sent.
    peer.disconnect_later(5);

    loop {
        let e = host
            .service(Duration::from_secs(1))
            .context("service failed");
        println!("received event: {:#?}", e);
    }
}
