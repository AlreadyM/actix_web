//! Simple echo websocket server.
//!
//! Open `http://localhost:8080/` in browser to test.

use std::{cell::Cell, clone};
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_files::NamedFile;
use actix_web::{get, post, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, cookie::Key};
use actix_web_actors::ws;
use futures_util::pin_mut;
use tokio::time;

mod server;
use self::server::MyWebSocket;


type TokioMpscSender = tokio::sync::mpsc::Sender<String>;
async fn index() -> impl Responder {
    NamedFile::open_async("../static/index.html").await.unwrap()
}

/// WebSocket handshake and start `MyWebSocket` actor.
async fn echo_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(MyWebSocket::new(), &req, stream)
}
use serde::Deserialize;
#[derive(Deserialize, Debug)]
struct Info {
    username: String,
}

#[derive(Clone)]
struct AppState {
    sender: Cell<usize>,
}

#[get("/getall/{user_id}/{friend}")]
// async fn get(path: web::Path<String>, req: HttpRequest, query:web::Query<String>)->impl Responder {
// async fn getall(req: HttpRequest, path: web::Path<(u32,String)>, query:web::Query<(String,String)>)->impl Responder {
// async fn getall(session: Session, req: HttpRequest, path: web::Path<(String,String)>, query:web::Query<Info> ,data: web::Data<Pipechannel>)->impl Responder {
async fn getall(session: Session, req: HttpRequest, path: web::Path<(String,String)>, query:web::Query<Info> ,data: web::Data<Pipechannel>)->Result<HttpResponse, Error> {
    // access session data
    if let Some(count) = session.get::<i32>("counter")? {
        session.insert("counter", count + 1)?;
    } else {
        session.insert("counter", 1)?;
    }

    // Ok(HttpResponse::Ok().body(format!(
    //     "Count is {:?}!",
    //     session.get::<i32>("counter")?.unwrap()
    // )))
    println!("session:{:?}",session.get::<i32>("counter")?.unwrap());
    
    
// async fn getall(req: HttpRequest,  query:web::Query<(String,String)>)->impl Responder {
    let sender = &data.sender;
    sender.send(format!("msg send from tokio mpsc sender: {}","the msg detail")).await.unwrap();
    // time::sleep(time::Duration::from_secs(5)).await;
    println!("{:?}", data.sender);
    println!("{:?}",query);
        // HttpResponse::Ok().body(format!("Hello ,Get! \n {:?} \n, {:?} \n, {:?} \n", 0, path , query))
    Ok(HttpResponse::Ok().body(format!(
        "Count is {:?}!",
        session.get::<i32>("counter")?.unwrap()
    )))
    // HttpResponse::Ok().body(format!("Hello ,Get!: {:?}, {:?}", path, query))
    // HttpResponse::Ok().body(path.into_inner())
}

#[post("/echo")]
async fn echo(req_body:String) ->impl Responder {
    HttpResponse::Ok().body(req_body)
}
struct Pipechannel {
    sender: TokioMpscSender,
    // receiver:TokioMpscReceiver
}
impl Pipechannel {
    fn build(sender: TokioMpscSender) -> Self {
        Pipechannel { sender: sender }
    }
}
impl Clone for Pipechannel {
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone() }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://localhost:8080");

    // 原启动可用模式
            // HttpServer::new(|| {
            //     App::new()
            //         .service(getall)
            //         .service(echo)
            //         // WebSocket UI HTML file
            //         .service(web::resource("/").to(index))
            //         // websocket route
            //         .service(web::resource("/ws").route(web::get().to(echo_ws)))
            //         // enable logger
            //         .wrap(middleware::Logger::default())
            // })
            // .workers(2)
            // .bind(("::", 8080))?
            // .run()
            // .await
    // 原启动可用模式


    // 期待启动一个内部PIPE
    let (tx,mut rx) = tokio::sync::mpsc::channel::<String>(10);
    let pipechannel = Pipechannel{
        sender:tx
    };
    // let appState = AppState {
    //     sender:Cell::new(&tx),
    // };
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pipechannel.clone()))
            .wrap(
                // create cookie based session middleware
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    .cookie_secure(false)
                    .build()
            )
            .service(getall)
            .service(echo)
            // WebSocket UI HTML file
            .service(web::resource("/").to(index))
            // websocket route
            .service(web::resource("/ws").route(web::get().to(echo_ws)))
            // enable logger
            .wrap(middleware::Logger::default())
    })
    .workers(2)
    .bind(("::", 8080))?
    .run()
    // .await
    ;

    let reader = tokio::spawn(async move {
        let mut count = 0;
        loop {
            // time::sleep(time::Duration::from_secs(1)).await;
            count += 1;
            println!("{count}");
            let msg = rx.recv().await;
            match msg {
                Some(msg) => {
                    println!("receiver printing :{:?}",msg);
                }
                None =>{
                    // return;
                }
            }
        }
    });
    // tokio::spawn(async move {
    //     server.run().await;
    // });
    pin_mut!(server,reader);
    futures_util::future::select(server, reader).await;
    // tokio::select!(server,reader);
    // tokio::spawn(async {
    //     reader.await;
    // });


    Ok(())

}
