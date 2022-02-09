use clap::Parser;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use jinshu_protocol::{
    Body, Codec, Content, Message, Pdu, PduCodec, Request, Response, TransactionId,
    TransactionIdGenerator,
};
use jinshu_utils::current_millisecond;
use log::LevelFilter;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::codec::Framed;
use uuid::Uuid;

/// 锦书命令行客户端
#[derive(Debug, Parser)]
struct Opts {
    /// 服务端地址及端口
    #[clap(short = 'a', long, default_value = "localhost:9000")]
    addr: String,

    /// 用户 ID
    #[clap(short = 'u', long)]
    user_id: Uuid,

    /// 服务端签发的 token
    #[clap(short = 't', long)]
    token: Uuid,

    /// 接收方 ID
    #[clap(short = 'r', long)]
    target: Uuid,

    /// 使用的编码, 0.json | 1.msgpack | 2.cbor | 3.flexbuffers
    #[clap(short = 'c', long, default_value = "cbor")]
    codec: Codec,

    /// 日志级别
    #[clap(short = 'l', long, default_value = "INFO")]
    log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Opts {
        addr,
        user_id,
        token,
        target,
        codec,
        log_level,
    } = Opts::parse();

    env_logger::builder().filter_level(log_level).try_init()?;

    let socket = TcpStream::connect(addr).await?;
    let mut framed = Framed::new(socket, PduCodec::new(codec));
    let mut id_gen = TransactionIdGenerator::default();

    let sign_in = Request::SignIn { user_id, token }.to_pdu(id_gen.next_id());

    framed.send(sign_in).await?;

    match framed.next().await {
        Some(Ok(Pdu {
            body: Body::Resp(Response::SignedIn { extension }),
            ..
        })) => {
            log::info!("Sign in ok");
            if let Some(extension) = extension {
                log::info!("extension: {}", extension);
            }
        }
        Some(Ok(Pdu {
            body: Body::Resp(Response::InvalidToken { user_id }),
            ..
        })) => {
            anyhow::bail!("Sign in error: invalid token (user_id: {})", user_id);
        }
        Some(Ok(pdu)) => {
            anyhow::bail!("Sign in error: unexpected response: {:?}", pdu);
        }
        Some(Err(e)) => {
            anyhow::bail!("Sign in error: {}", e);
        }
        None => {
            anyhow::bail!("Connection closed");
        }
    }

    // loop {
    //     let now = Instant::now();
    //     framed.send(Request::Ping.to_pdu(id_gen.next_id())).await?;
    //
    //     if let Some(result) = framed.next().await {
    //         let pdu = result?;
    //         log::info!("pdu received: {:?} ({}ms)", pdu, now.elapsed().as_millis());
    //     } else {
    //         log::error!("The connection was closed before receiving Pong");
    //     }
    //
    //     if id_gen.seq() >= 10 {
    //         break;
    //     }
    // }

    let from = user_id;
    let to = target;

    let (writer, reader) = framed.split();

    let waiting = Arc::new(Mutex::new(HashMap::new()));

    tokio::spawn(write_loop(from, to, waiting.clone(), writer));

    read_loop(waiting, reader).await

    //Ok(())
}
