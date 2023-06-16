use cw_orch::{
    anyhow,
    prelude::{networks::parse_network, DaemonBuilder},
    tokio::runtime::Runtime,
};

use abstract_interface::AppDeployer;
use app::contract::{interface::CroncatApp, CRONCAT_ID};
use semver::Version;

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();
    let chain = parse_network("juno-1");
    use dotenv::dotenv;
    let version: Version = CONTRACT_VERSION.parse().unwrap();
    let rt = Runtime::new()?;
    let chain = DaemonBuilder::default()
        .chain(chain)
        .handle(rt.handle())
        .build()?;
    let app = CroncatApp::new(CRONCAT_ID, chain);

    app.deploy(version)?;
    Ok(())
}
