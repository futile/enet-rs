extern crate enet;

use std::net::Ipv4Addr;

use anyhow::Context;
use enet::*;

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
        match host.service(1000).context("service failed")? {
            Some(Event::Connect(_)) => println!("new connection!"),
            Some(Event::Disconnect(..)) => println!("disconnect!"),
            Some(Event::Receive {
                channel_id,
                ref packet,
                ..
            }) => println!(
                "got packet on channel {}, content: '{}'",
                channel_id,
                std::str::from_utf8(packet.data()).unwrap()
            ),
            _ => (),
        }
    }
}
