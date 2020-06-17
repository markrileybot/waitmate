use std::borrow::{Borrow, Cow};
use std::io::Bytes;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_cors::Cors;
use actix_rt::System;
use actix_session::{CookieSession, Session};
use actix_web::{App, Error, middleware, Responder, web};
use actix_web::body::Body;
use actix_web::http::{header, Method, StatusCode};
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::Result;
use actix_web_actors::ws;
use log::info;
use mime_guess::from_path;
use rust_embed::RustEmbed;
use serde_json::{Deserializer, Value, json};
use uuid::Uuid;

use crate::waitmate::api::{Event, EventBus, Named, Waiter};
use crate::waitmate::log::{Cursor, EventLog};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// ws command
const CMD_SET_OFFSET: Option<&str> = Some("set_offset");

#[derive(RustEmbed)]
#[folder = "target/web"]
struct WebResources;

fn get_embedded_file(path: &str) -> HttpResponse {
    match WebResources::get(path) {
        Some(content) => {
            let body: Body = match content {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes.into(),
            };
            HttpResponse::Ok()
                .content_type(from_path(path).first_or_octet_stream().as_ref())
                .body(body)
        }
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[get("/api/v1/event")]
async fn get_events(_req: HttpRequest, event_log: web::Data<Arc<EventLog>>) -> impl Responder {
    let mut resp = String::from("[");
    let c = event_log.build_cursor().build();
    for (key, event) in c {
        if resp.len() > 1 {
            resp.push(',');
        }
        resp.push_str(serde_json::to_string(&event).unwrap().as_str());
    }
    resp.push(']');

    return HttpResponse::Ok()
        .content_type("application/json")
        .body(resp);
}

/// /api/v1/connect
async fn web_socket_connect(
    req: HttpRequest,
    stream: web::Payload,
    event_log: web::Data<Arc<EventLog>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WebSocketSession {
            id: 0,
            last_heard_from: Instant::now(),
            last_event_time: None,
            last_event_id: None,
            event_log: event_log.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[get("/{_:.*}")]
async fn static_file(path: web::Path<(String,)>) -> HttpResponse {
    return get_embedded_file(&path.0);
}

#[get("/")]
async fn index() -> HttpResponse {
    return get_embedded_file("index.html");
}


/// Define http actor
struct WebSocketSession {
    /// unique session id
    id: usize,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    last_heard_from: Instant,
    /// last position
    last_event_time: Option<u128>,
    last_event_id: Option<Uuid>,
    event_log: Arc<EventLog>
}
impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    /// Called when an actor gets polled the first time.
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heard_from) > CLIENT_TIMEOUT {
                info!("Client {} has gone away!", act.id);
                ctx.stop();
            } else if act.last_event_time.is_some() {
                let mut count = 0;
                let c = act.event_log.build_cursor()
                    .starting_after(act.last_event_time.unwrap(),
                                    act.last_event_id)
                    .build();
                for (key, event) in c {
                    ctx.text(serde_json::to_string(&event).unwrap().as_str());
                    act.last_event_id = Some(event.id);
                    act.last_event_time = Some(event.time);
                    count += 1;
                }
                if count > 0 {
                    info!("Sent {} to client {}", count, act.id);
                }
            }
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        self.last_heard_from = Instant::now();
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let command: Value = serde_json::from_str(&text).unwrap();
                let name = command["command"].as_str();
                let args = command["args"].as_object();
                match name {
                    CMD_SET_OFFSET => {
                        let key = args.map_or(None,
                                              |m| m["key"].as_str());
                        if key.is_some() {
                            let (t, i) = EventLog::parse_key(key.unwrap().as_bytes()).unwrap();
                            self.last_event_time = Some(t);
                            self.last_event_id = Some(i);
                        }
                    }
                    _ => {}
                }
            },
            _ => (),
        }
    }
}

pub struct Server {
    address: String,
    event_log: Arc<EventLog>
}
impl Server {
    pub fn new(address: &str, event_log: Arc<EventLog>) -> Self {
        return Self {
            address: String::from(address),
            event_log
        }
    }
}
impl Waiter for Server {
    fn wait(&self, _bus: &dyn EventBus) {
        let event_log = self.event_log.clone();
        let mut sys = System::new(format!("http://{}", self.address));
        // let el = el.clone();
        // srv is server controller type, `dev::Server`
        let srv = HttpServer::new(move || {
            App::new()
                .data(event_log.clone())
                // cookie session middleware
                .wrap(CookieSession::signed(&[0; 32]).secure(false))
                // enable logger - always register actix-web Logger middleware last
                .wrap(middleware::Logger::default())
                // cors
                .wrap(Cors::default())
                // register favicon
                // .service(favicon)
                .service(get_events)
                .service(web::resource("/api/v1/connect").to(web_socket_connect))
                .service(index)
                .service(static_file)
        })
            .bind(&self.address).unwrap()
            .run();

        // run future
        sys.block_on(srv).unwrap();
    }
}
impl Named for Server {
    fn name(&self) -> &str {
        return self.address.as_str();
    }
}