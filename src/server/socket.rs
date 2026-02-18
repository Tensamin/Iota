use crate::{
    data::communication::{CommunicationType, CommunicationValue},
    log,
};
use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web_actors::ws;
use std::time::{Duration, Instant};

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsSendMessage(pub String);

impl actix::Handler<WsSendMessage> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: WsSendMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

pub struct WsSession {
    path: String,
    last_heartbeat: Instant,
}

impl WsSession {
    pub fn new(path: String) -> Self {
        Self {
            path,
            last_heartbeat: Instant::now(),
        }
    }
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > IDLE_TIMEOUT {
                ctx.close(None);
                ctx.stop();
                return;
            }

            let ping = CommunicationValue::new(CommunicationType::ping)
                .to_json()
                .to_string();

            ctx.text(ping);
        });
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_heartbeat(ctx);

        log!("WebSocket session started for path: {}", self.path);

        if self.path.starts_with("/ws/users/") {
            log!("UserConnection handling is not yet implemented.");
        } else if self.path.starts_with("/ws/community/") {
            let community_id = self.path.split('/').nth(3).unwrap_or_default();
            log!(
                "CommunityConnection handling for {} is not yet implemented.",
                community_id
            );
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log!("WebSocket session stopped for path: {}", self.path);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => self.last_heartbeat = Instant::now(),
            Ok(ws::Message::Text(_)) => self.last_heartbeat = Instant::now(),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(_) => {
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}
