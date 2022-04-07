extern crate enet;

use anyhow::Context;
use enet::*;
use std::net::Ipv4Addr;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let enet = Enet::new().context("could not initialize ENet")?;

    let local_addr = Address::new(Ipv4Addr::LOCALHOST, 9001);

    let mut host = enet
        .create_host::<()>(
            Some(&local_addr),
            10,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .context("could not create host")?;

    loop {
        // Wait 500 ms for any events.
        if let Some(Event { kind, .. }) = host
            .service(Duration::from_secs(1))
            .context("service failed")?
        {
            match kind {
                EventKind::Connect => println!("new connection!"),
                EventKind::Disconnect { .. } => println!("disconnect!"),
                EventKind::Receive { channel_id, packet } => println!(
                    "got packet on channel {}, content: '{}'",
                    channel_id,
                    std::str::from_utf8(packet.data()).unwrap()
                ),
            }
        }
    }
}
