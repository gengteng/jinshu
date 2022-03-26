use futures::future::join_all;
use jinshu_common::Config;
use jinshu_protocol::{Content, Message};
use jinshu_sdk::{Client, ClientConfig};
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tokio::time::sleep;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct AppClientConfig {
    user_count: usize,
    server_host: String,
    server_port: u16,
}

#[derive(Debug, Deserialize)]
struct Conf {
    tracing: TracingConfig,
    app_client: AppClientConfig,
    #[serde(default)]
    client: ClientConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracing = conf.tracing.init("app-client");

    let Conf {
        app_client, client, ..
    } = conf;

    let app_server_address = Url::parse(&format!(
        "http://{}:{}",
        app_client.server_host, app_client.server_port
    ))?;

    tracing::info!(?client);
    let client = Client::new(client)?;

    let users = Arc::new(RwLock::new(HashSet::new()));

    tracing::info!(?app_client);
    let mut vec = Vec::with_capacity(app_client.user_count);
    for i in 0..app_client.user_count {
        let c = client.clone();
        let a = app_server_address.clone();
        let u = users.clone();

        vec.push(async move {
            if let Err(error) = impersonate(c, a, format!("user{}", i), u).await {
                tracing::error!(%error, "Failed to impersonate.")
            }
        });
    }

    join_all(vec).await;

    Ok(())
}

async fn impersonate(
    client: Client,
    app_server_address: Url,
    username: impl Into<String>,
    users: Arc<RwLock<HashSet<Uuid>>>,
) -> anyhow::Result<()> {
    let username = username.into();

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(&username)
        .await?;

    let mut password = String::with_capacity(32);
    file.read_to_string(&mut password).await?;

    let password_from_file = !password.is_empty();

    let sign_param = if password_from_file {
        tracing::info!(%username, "User password from file");
        AppSignParam {
            username: &username,
            password: &password,
        }
    } else {
        password = Uuid::new_v4().simple().to_string();

        let sign_param = AppSignParam {
            username: &username,
            password: &password,
        };

        let sign_up = client
            .http_client()
            .post(app_server_address.join("/sign_up")?)
            .json(&sign_param)
            .send()
            .await?;

        if !sign_up.status().is_success() {
            anyhow::bail!(
                "Sign up error: {} {}",
                sign_up.status(),
                sign_up.text().await?
            );
        }

        file.write_all(password.as_bytes()).await?;
        tracing::info!(%username, "User signed up.");
        sign_param
    };

    drop(file);

    let sign_in = client
        .http_client()
        .post(app_server_address.join("/sign_in")?)
        .json(&sign_param)
        .send()
        .await?;

    if !sign_in.status().is_success() {
        anyhow::bail!(
            "Sign in error: {} {}",
            sign_in.status(),
            sign_in.text().await?
        );
    }

    let AppSignInResult { jinshu } = sign_in.json().await?;

    tracing::info!(%username, "User signed in");

    let mut ua = client.sign_in(jinshu.user_id, jinshu.token).await?;

    users.write().await.insert(jinshu.user_id);

    let mut shutdown = Box::pin(shutdown_signal());

    loop {
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {
                let targets = users
                    .read()
                    .await
                    .iter()
                    .filter(|u| jinshu.user_id.ne(*u))
                    .map(|to| {
                        ua.send(Message::new(
                            jinshu.user_id,
                            *to,
                            Content::string(format!("Hello, I'm {}", username)),
                        ))
                    })
                    .collect::<Vec<_>>();

                join_all(targets).await;
            }
            receive = ua.receive() => {
                match receive {
                    Ok(message) => tracing::info!(%username, ?message, "Received a message"),
                    Err(error) => {
                        tracing::error!(%error, "Failed to receive messages");
                        break;
                    },
                }
            }
            _ = &mut shutdown => {
                tracing::info!(%username, "Received a shutdown signal");
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct AppSignParam<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
struct AppSignInResult {
    pub jinshu: JinshuResult,
}

#[derive(Debug, Deserialize)]
struct JinshuResult {
    pub user_id: Uuid,
    pub token: Uuid,
}
