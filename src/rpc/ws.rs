use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tracing::{debug, info};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct ShredWsClient {
    url: String,
}

impl ShredWsClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    pub async fn connect_and_subscribe(&self) -> Result<WsSubscription> {
        let request = self
            .url
            .as_str()
            .into_client_request()
            .context("failed to build websocket request")?;

        let (mut stream, _) = connect_async(request).await?;
        let subscribe = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": ["shreds"]
        });

        stream
            .send(Message::Text(subscribe.to_string().into()))
            .await
            .context("failed to send shred subscription request")?;

        let subscription_id = read_subscription_ack(&mut stream).await?;
        info!(%subscription_id, "subscribed to shred stream");

        Ok(WsSubscription {
            stream,
            subscription_id,
        })
    }
}

pub struct WsSubscription {
    stream: WsStream,
    subscription_id: String,
}

impl WsSubscription {
    pub fn subscription_id(&self) -> &str {
        &self.subscription_id
    }

    pub async fn next_json_message(&mut self) -> Result<Value> {
        loop {
            let message = self
                .stream
                .next()
                .await
                .context("websocket stream ended")??;

            match message {
                Message::Text(text) => {
                    let value: Value =
                        serde_json::from_str(&text).context("invalid websocket json payload")?;
                    return Ok(value);
                }
                Message::Binary(bytes) => {
                    let value: Value = serde_json::from_slice(&bytes)
                        .context("invalid websocket binary json payload")?;
                    return Ok(value);
                }
                Message::Ping(payload) => {
                    self.stream.send(Message::Pong(payload)).await?;
                }
                Message::Pong(_) => {
                    debug!("received websocket pong");
                }
                Message::Frame(_) => {}
                Message::Close(frame) => {
                    anyhow::bail!("websocket closed: {frame:?}");
                }
            }
        }
    }
}

async fn read_subscription_ack(stream: &mut WsStream) -> Result<String> {
    loop {
        let message = stream
            .next()
            .await
            .context("websocket closed before subscription ack")??;

        match message {
            Message::Text(text) => {
                let value: Value =
                    serde_json::from_str(&text).context("invalid subscription ack payload")?;
                if let Some(result) = value.get("result").and_then(Value::as_str) {
                    return Ok(result.to_owned());
                }
            }
            Message::Binary(bytes) => {
                let value: Value = serde_json::from_slice(&bytes)
                    .context("invalid binary subscription ack payload")?;
                if let Some(result) = value.get("result").and_then(Value::as_str) {
                    return Ok(result.to_owned());
                }
            }
            Message::Ping(payload) => {
                stream.send(Message::Pong(payload)).await?;
            }
            Message::Pong(_) | Message::Frame(_) => {}
            Message::Close(frame) => anyhow::bail!("websocket closed during subscribe: {frame:?}"),
        }
    }
}
