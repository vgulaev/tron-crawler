use actix_web::{
  http::header::ContentType, middleware::Logger, web, App, HttpResponse, HttpServer, Responder,
};
use app_config;
use app_db::DBInsert;
use app_state::AppState;
use env_logger::Env;
use log::info;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn me() -> impl Responder {
  "This is amazing Tron Crawler"
}

async fn not_found() -> HttpResponse {
  HttpResponse::NotFound()
    .content_type(ContentType::plaintext())
    .body("Looks like no page here")
}

async fn getblockbylatestnum(http_client: &Client) -> usize {
  let cfg = app_config::Config::new();
  let host = cfg.api_host();
  let url = format!("{host}wallet/getblockbylatestnum");
  let res = http_client
    .post(url)
    .body(json!({"num": 1}).to_string())
    .send()
    .await
    .unwrap();
  let data: Value = serde_json::from_slice(&res.bytes().await.unwrap()).unwrap();
  return data["block"].as_array().unwrap()[0]["block_header"]["raw_data"]["number"]
    .as_i64()
    .unwrap() as usize;
}

async fn get_block_by_block(state: AppState) {
  let ctrlc_state = state.clone();
  actix_rt::spawn(async move {
    tokio::signal::ctrl_c().await.unwrap();
    ctrlc_state.set_stop_get_block_loop(true);
  });

  let http_client = reqwest::Client::new();
  let cfg = app_config::Config::new();
  let host = cfg.api_host();
  let url = format!("{host}wallet/getblock");
  let db_insert = Arc::new(DBInsert::new().await);

  let mut k: usize = getblockbylatestnum(&http_client).await;
  println!("k: {}", k);
  // return
  loop {
    if state.get_stop_get_block_loop() {
      break;
    }
    if state.get_reload_watched_addresses() {
      state.set_reload_watched_addresses(false);
      db_insert.reload_watched_addresses().await
    }
    let res = http_client
      .post(&url)
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

    if 200 != res.status() {
      info!("{} status: {}", url, res.status());
      sleep(Duration::from_secs(4)).await;
      continue;
    }

    let body = res.bytes().await.unwrap();
    if 3 == body.len() {
      sleep(Duration::from_secs(4)).await;
      continue;
    }

    let cloned = db_insert.clone();
    actix_rt::spawn(async move {
      cloned.insert_block(body.as_ref()).await;
    });

    k += 1;
  }
}

async fn reload_watched_addresses(app_state: web::Data<AppState>) -> impl Responder {
  app_state.set_reload_watched_addresses(true);
  "reload_watched_addresses"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  println!("Hello!!!");
  println!("Server has started");
  let cfg = app_config::Config::new();
  env_logger::init_from_env(Env::default().default_filter_or("info"));

  let state = AppState::new().await;
  let web_state = web::Data::new(state.clone());

  actix_rt::spawn(get_block_by_block(state.clone()));

  HttpServer::new(move || {
    App::new()
      .wrap(Logger::default())
      .app_data(web_state.clone())
      .service(web::scope("/api").route("/me", web::post().to(me)).route(
        "/reload_watched_addresses",
        web::post().to(reload_watched_addresses),
      ))
      .default_service(web::route().to(not_found))
  })
  .bind(("0.0.0.0", cfg.http_server.port))?
  .run()
  .await
}
