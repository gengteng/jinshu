use jinshu_common::Config;
use jinshu_sdk::{Client, ClientConfig};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Conf {
    #[serde(default)]
    client: ClientConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Conf { client } = Conf::from_cli()?;

    let client = Client::new(client)?;

    let ua1 = client.sign_in(Uuid::new_v4(), Uuid::new_v4()).await?;

    Ok(())
}
