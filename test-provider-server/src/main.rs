// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::net::SocketAddr;

use clap::Parser;
use scim2_test_provider_server::create_http_server;

#[derive(Debug, Parser)]
#[clap(about = "SCIM 2 provider server")]
struct Args {
    // Note that port "4567" is arbitrarily chosen and is not SCIM specific.
    #[clap(long, default_value = "127.0.0.1:4567")]
    bind_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt: Args = Args::try_parse()?;

    let http_server = create_http_server(Some(opt.bind_addr))?;
    if let Err(s) = http_server.await {
        anyhow::bail!("Error from start(): {}", s);
    }

    Ok(())
}
