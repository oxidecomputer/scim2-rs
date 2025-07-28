// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::Parser;
use dropshot::ApiDescription;
use dropshot::Body;
use dropshot::ConfigDropshot;
use dropshot::HttpError;
use dropshot::HttpResponseOk;
use dropshot::HttpServerStarter;
use dropshot::Path;
use dropshot::Query;
use dropshot::RequestContext;
use dropshot::TypedBody;
use dropshot::endpoint;
use http::Response;
use http::StatusCode;
use schemars::JsonSchema;
use serde::Deserialize;
use slog::Drain;
use std::net::SocketAddr;
use std::sync::Arc;

mod groups;
mod server;
mod users;

pub struct ServerContext {
    provider: scim2_rs::Provider<scim2_rs::InMemoryProviderStore>,
}

fn register_endpoints(
    api_description: &mut ApiDescription<Arc<ServerContext>>,
) -> Result<(), anyhow::Error> {
    // RFC 7644, section 3.2: SCIM Endpoints and HTTP Methods

    api_description.register(users::list_users)?;
    api_description.register(users::get_user)?;
    api_description.register(users::create_user)?;
    api_description.register(users::put_user)?;
    api_description.register(users::delete_user)?;

    api_description.register(groups::list_groups)?;
    api_description.register(groups::get_group)?;
    api_description.register(groups::create_group)?;
    api_description.register(groups::put_group)?;
    api_description.register(groups::delete_group)?;

    api_description.register(server::get_resource_types)?;
    api_description.register(server::get_resource_type_user)?;
    api_description.register(server::get_resource_type_group)?;
    api_description.register(server::get_schemas)?;
    api_description.register(server::get_service_provider_config)?;

    api_description.register(state)?;

    Ok(())
}

#[endpoint {
    method = GET,
    path = "/state"
}]
pub async fn state(
    rqctx: RequestContext<Arc<ServerContext>>,
) -> Result<HttpResponseOk<scim2_rs::InMemoryProviderStoreState>, HttpError> {
    let apictx = rqctx.context();
    Ok(HttpResponseOk(apictx.provider.state()))
}

#[derive(Debug, Parser)]
#[clap(about = "SCIM 2 provider server")]
struct Args {
    #[clap(long, default_value = "127.0.0.1:4567")]
    bind_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt: Args = Args::try_parse()?;

    // from https://docs.rs/slog/latest/slog/ - terminal out
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = slog::Logger::root(drain, slog::o!());

    let config =
        ConfigDropshot { bind_address: opt.bind_addr, ..Default::default() };

    let mut api_description = ApiDescription::<Arc<ServerContext>>::new();

    if let Err(s) = register_endpoints(&mut api_description) {
        anyhow::bail!("Error from register_endpoints: {}", s);
    }

    let store = scim2_rs::InMemoryProviderStore::new();

    let ctx =
        Arc::new(ServerContext { provider: scim2_rs::Provider::new(store) });

    let http_server = HttpServerStarter::new(
        &config,
        api_description,
        Arc::clone(&ctx),
        &log,
    );

    if let Err(e) = http_server {
        anyhow::bail!("Error from HttpServerStarter::new: {:?}", e);
    }

    if let Err(s) = http_server.unwrap().start().await {
        anyhow::bail!("Error from start(): {}", s);
    }

    Ok(())
}
