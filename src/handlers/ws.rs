use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;
use log::{error, debug};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};
use futures::{StreamExt, SinkExt};
use actix::prelude::*;
use crate::config::Config;

struct WebSocketSession {
    target_url: String,
    target_tx: Option<futures::channel::mpsc::UnboundedSender<TungsteniteMessage>>,
}

impl WebSocketSession {
    fn new(config: &Config, original_path: &str) -> Self {
        let target_url = format!("{}{}", config.target_ws_url, original_path);
        debug!("Creating WebSocketSession with target URL: {}", target_url);
        WebSocketSession {
            target_url,
            target_tx: None,
        }
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let target_url = self.target_url.clone();
        let addr = ctx.address();
        debug!("WebSocketSession started, connecting to: {}", target_url);
        ctx.spawn(
            async move {
                match connect_async(&target_url).await {
                    Ok((ws_stream, _)) => {
                        debug!("Connected to target WebSocket: {}", target_url);
                        let (mut write, mut read) = ws_stream.split();
                        let (tx, mut rx) = futures::channel::mpsc::unbounded();
                        
                        addr.do_send(SetTargetTx(tx));

                        tokio::spawn(async move {
                            while let Some(msg) = rx.next().await {
                                if write.send(msg).await.is_err() {
                                    break;
                                }
                            }
                        });

                        while let Some(message) = read.next().await {
                            match message {
                                Ok(msg) => {
                                    debug!("Received message from target: {:?}", msg);
                                    addr.do_send(ForwardMessage(msg));
                                }
                                Err(e) => error!("Error receiving message from target: {}", e),
                            }
                        }
                    }
                    Err(e) => error!("Failed to connect to target WebSocket: {}", e),
                }
            }
            .into_actor(self)
        );
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct SetTargetTx(futures::channel::mpsc::UnboundedSender<TungsteniteMessage>);

impl Handler<SetTargetTx> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: SetTargetTx, _: &mut Self::Context) {
        self.target_tx = Some(msg.0);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ForwardMessage(TungsteniteMessage);

impl Handler<ForwardMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: ForwardMessage, ctx: &mut Self::Context) {
        match msg.0 {
            TungsteniteMessage::Text(text) => ctx.text(text),
            TungsteniteMessage::Binary(bin) => ctx.binary(bin),
            TungsteniteMessage::Ping(data) => ctx.ping(&data),
            TungsteniteMessage::Pong(data) => ctx.pong(&data),
            TungsteniteMessage::Close(reason) => {
                if let Some(frame) = reason {
                    ctx.close(Some(ws::CloseReason {
                        code: ws::CloseCode::from(u16::from(frame.code)),
                        description: Some(frame.reason.into_owned()),
                    }));
                } else {
                    ctx.close(None);
                }
            },
            TungsteniteMessage::Frame(_) => {}
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                debug!("Received text message from client: {}", text);
                if let Some(tx) = &self.target_tx {
                    let _ = tx.unbounded_send(TungsteniteMessage::Text(text.to_string()));
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                debug!("Received binary message from client: {} bytes", bin.len());
                if let Some(tx) = &self.target_tx {
                    let _ = tx.unbounded_send(TungsteniteMessage::Binary(bin.to_vec()));
                }
            }
            Ok(ws::Message::Close(reason)) => {
                debug!("Client closed connection: {:?}", reason);
                if let Some(tx) = &self.target_tx {
                    let close_frame = reason.map(|r| tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(u16::from(r.code)),
                        reason: r.description.unwrap_or_default().into(),
                    });
                    let _ = tx.unbounded_send(TungsteniteMessage::Close(close_frame));
                }
                ctx.stop();
            }
            _ => (),
        }
    }
}

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    config: web::Data<Arc<Config>>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path().to_owned();
    debug!("WebSocket handler called with path: {}", path);
    let session = WebSocketSession::new(&config, &path);
    ws::start(session, &req, stream)
}