use actix_web::{
  http::header::ContentType, middleware::Logger, web, App, HttpResponse, HttpServer, Responder,
};
use env_logger::Env;
use log::info;
use serde_json::{json, Value};
use std::io::Result;
use tokio_postgres::{Client, Config, NoTls};
use app_config;
use app_state::AppState;

async fn me() -> impl Responder {
  "This is amazing Tron Crawler"
}

async fn not_found() -> HttpResponse {
  HttpResponse::NotFound()
    .content_type(ContentType::plaintext())
    .body("Looks like no page here")
}


async fn get_pg_connection() -> Result<Client> {
  let (client, connection) = Config::new()
    .host(app_config::PG_HOST)
    .port(5432)
    .user(app_config::PG_USER)
    .password(app_config::PG_PASS)
    .dbname(app_config::PG_DATABASE)
    .connect(NoTls)
    .await
    .unwrap();

  tokio::spawn(async move {
    if let Err(e) = connection.await {
      eprintln!("connection error: {}", e);
    }
  });

  Ok(client)
}

async fn get_block_by_block(state: AppState) {
  let ctrlc_state = state.clone();
  actix_rt::spawn(async move {
    tokio::signal::ctrl_c().await.unwrap();
    ctrlc_state.set_stop_get_block_loop(true);
  });  

  let pg_client = get_pg_connection().await.unwrap();
  let http_client = reqwest::Client::new();
  let mut k: usize = 41060038;
  // let url = "http://5.45.75.175:8090/wallet/getblock";
  let url = "https://api.trongrid.io/wallet/getblock";
  loop {
    if state.get_stop_get_block_loop() {
      break
    }
    let res = http_client
      .post(url)
      .body(
        json!({
          "id_or_num": format!("{}", k),
          "detail": true
        })
        .to_string(),
      )
      .send()
      .await
      .unwrap();

    let data: Value = serde_json::from_slice(&res.bytes().await.unwrap()).unwrap();

    if data.as_object().unwrap().contains_key("transactions") {
      info!("Crawler get block: {}, with {} transaction", k, data["transactions"].as_array().unwrap().len());
    }

    k += 1;
  }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  println!("Hello!!!");
  println!("Server has started");
  env_logger::init_from_env(Env::default().default_filter_or("info"));

  let state = AppState::get_default();
  let web_state = web::Data::new(state.clone());

  actix_rt::spawn(get_block_by_block(state.clone()));

  HttpServer::new(move || {
    App::new()
      .wrap(Logger::default())
      .app_data(web_state.clone())
      .service(web::scope("/api").route("/me", web::post().to(me)))
      .default_service(web::route().to(not_found))
  })
  .bind(("0.0.0.0", 8081))?
  .run()
  .await
}
