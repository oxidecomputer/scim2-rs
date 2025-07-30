// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::Parser;
use scim2_test_client::Tester;

#[derive(Debug, Parser)]
#[clap(about = "SCIM 2 test client")]
struct Args {
    #[clap(long, default_value = "http://127.0.0.1:4567/v2")]
    url: String,

    /// A Bearer token
    #[clap(long)]
    bearer: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let opt: Args = Args::try_parse()?;

    let tester = match opt.bearer {
        Some(bearer) => Tester::new_with_bearer_auth(opt.url, bearer)?,

        None => Tester::new(opt.url),
    };

    tester.run()?;

    println!("SUCCESS");

    Ok(())
}
