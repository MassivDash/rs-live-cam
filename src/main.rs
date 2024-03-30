use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::cookie::{Key, SameSite};
use actix_web::error::InternalError;
use actix_web::http::Error;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use structopt::StructOpt;

#[macro_use]
extern crate log;

mod broadcaster;
mod routes;
mod session;
mod cors;

use broadcaster::Broadcaster;
use routes::login::login_form;

use crate::cors::get_cors_options::get_cors_options;
use crate::session::validate_session;

#[derive(Debug, StructOpt)]
#[structopt(name = "mjpeg-rs")]
struct Opt {
    #[structopt(short, long, default_value = "640")]
    width: u32,

    #[structopt(short, long, default_value = "360")]
    height: u32,

    #[structopt(short, long, default_value = "30")]
    fps: u64,
}

fn session_middleware() -> SessionMiddleware<CookieSessionStore> {
    SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64])).cookie_name("mjpeg-rs".to_string())
        .cookie_domain(Some("109.241.23.141".to_string()))
        .cookie_path("/".to_string())
        .cookie_secure(false)
        .cookie_http_only(false)
        .cookie_same_site(SameSite::Lax)
        .build()
}

async fn index(req: HttpRequest, session: Session) -> Result<impl Responder, Error> {
    println!("{:?}", req);

    //session
    if let Some(count) = session
        .get::<i32>("counter").unwrap_or_default()
    {
        session
            .insert("counter", count + 1).unwrap();
    } else {
        session
            .insert("counter", 1)
            .unwrap();
    }


    println!("session main new_client: {:?}", session.entries());
    Ok(HttpResponse::Ok().body("Hello world!"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();


    let secret_key =
        Key::from(b"0123456789012345678901234567890123456789012345678901234567890123456789");

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

   

    HttpServer::new(move || {
        let cors = get_cors_options("dev".to_string());
        App::new()
            .wrap(cors)
            .wrap(session_middleware()) // allow the cookie to be accessed from javascript
            .wrap(message_framework.clone())
            .service(web::resource("/").to(index))
            .service(web::resource("/stream").to(new_client))
            .route("/login", web::post().to(routes::login::login))
            .route("/login", web::get().to(login_form))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

/// Register a new client and return a response
async fn new_client(
    session: Session,
) -> impl Responder {
    info!("new_client...");

    println!("session main new_client: {:?}", session.entries());
    let auth = validate_session(&session).map_err(|err| InternalError::from_response("", err));

    match auth {
        Ok(_) => {
            let opt = Opt::from_args();
            let data = Broadcaster::create(opt.width, opt.height, opt.fps);

            let rx = data.lock().unwrap().new_client();
            // now starts streaming

            HttpResponse::Ok()
                .append_header(("Cache-Control", "no-store, must-revalidate"))
                .append_header(("Pragma", "no-cache"))
                .append_header(("Expires", "0"))
                .append_header(("Connection", "close"))
                .append_header((
                    "Content-Type",
                    "multipart/x-mixed-replace;boundary=boundarydonotcross",
                ))
                .streaming(rx)
        }
        Err(err) => return err.error_response(),
    }
}
