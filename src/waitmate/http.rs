use std::sync::Arc;

use actix_rt::System;
use actix_session::{CookieSession, Session};
use actix_web::{App, Responder, web};
use actix_web::http::{header, Method, StatusCode};
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::Result;

use crate::waitmate::api::{EventBus, Named, Waiter, Event};
use crate::waitmate::log::{Cursor, EventLog};
use std::thread;
use std::io::Bytes;
use std::rc::Rc;
use std::borrow::Borrow;

/// simple index handler
#[get("/welcome")]
async fn welcome(session: Session, req: HttpRequest) -> Result<HttpResponse> {
    // session
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
    }

    // set counter to session
    session.set("counter", counter)?;

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/plain; charset=utf-8")
        .body(format!("HI {}", counter)))
}

#[get("/api/v1/event")]
async fn get_events(req: HttpRequest, event_log: web::Data<Arc<EventLog>>) -> impl Responder {
    let mut resp = String::from("[");
    let c = event_log.build_cursor().build();
    for (key, event) in c {
        if resp.len() > 1 {
            resp.push(',');
        }
        resp.push_str(serde_json::to_string(&event).unwrap().as_str());
    }
    resp.push(']');
    return HttpResponse::Ok().body(resp);
}

fn run(address: & str, el: Arc<EventLog>) {
    let mut sys = System::new(format!("http://{}", address));
    // let el = el.clone();
    // srv is server controller type, `dev::Server`
    let srv = HttpServer::new(move || {
        App::new()
            .data(el.clone())
            // cookie session middleware
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            // enable logger - always register actix-web Logger middleware last
            // .wrap(middleware::Logger::default())
            // register favicon
            // .service(favicon)
            // register simple route, handle all methods
            .service(welcome)
            .service(get_events)
        // with path parameters
        // .service(web::resource("/user/{name}").route(web::get().to(with_param)))
        // async response body
        // .service(
        //     web::resource("/async-body/{name}").route(web::get().to(response_body)),
        // )
        // .service(
        //     web::resource("/test").to(|req: HttpRequest| match *req.method() {
        //         Method::GET => HttpResponse::Ok(),
        //         Method::POST => HttpResponse::MethodNotAllowed(),
        //         _ => HttpResponse::NotFound(),
        //     }),
        // )
        // .service(web::resource("/error").to(|| async {
        //     error::InternalError::new(
        //         io::Error::new(io::ErrorKind::Other, "test"),
        //         StatusCode::INTERNAL_SERVER_ERROR,
        //     )
        // }))
        // static files
        // .service(fs::Files::new("/static", "static").show_files_listing())
        // redirect
        // .service(web::resource("/").route(web::get().to(|req: HttpRequest| {
        //     println!("{:?}", req);
        //     HttpResponse::Found()
        //         .header(header::LOCATION, "static/welcome.html")
        //         .finish()
        // })))
        // default
        // .default_service(
        // 404 for GET request
        // web::resource("")
        //     .route(web::get().to(p404))
        // all requests that are not `GET`
        // .route(
        //     web::route()
        //         .guard(guard::Not(guard::Get()))
        //         .to(HttpResponse::MethodNotAllowed),
        // ),
        // )
    })
        .bind(address).unwrap()
        .run();

    // run future
    sys.block_on(srv).unwrap();
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
        run(self.address.as_str(), self.event_log.clone());
    }
}
impl Named for Server {
    fn name(&self) -> &str {
        return self.address.as_str();
    }
}