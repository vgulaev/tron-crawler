use app_config;
use log::info;
use rust_decimal::prelude::*;
use serde_json::{json, Value};
use std::{collections::HashSet, io::Result, str::FromStr, sync::Arc, sync::Mutex};
use tokio_postgres::{Client, Config, NoTls};

mod address;

pub use address::{Address, Error};

pub async fn get_pg_connection() -> Result<Client> {
  let cfg = app_config::Config::new();
  let (client, connection) = Config::new()
    .host(&cfg.pg.host)
    .port(5432)
    .user(&cfg.pg.user)
    .password(&cfg.pg.pass)
    .dbname(&cfg.pg.db)
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

async fn notify_bot(
  cfg: &Arc<app_config::Config>,
  owner_address: &str,
  to: &str,
  amount: &Decimal,
) {
  info!("*************************");
  let url = format!("https://api.telegram.org/bot{}/sendMessage", cfg.bot_token);
  let http = reqwest::Client::new();
  let res = http
    .post(url)
    // .header(key, value)
    .json(&json!({
        "chat_id": "-1001813292844",
        "text": format!("Привет от rust\n\nfrom: {}\n\nto: {}\n\namount: {}", owner_address, to, amount),
        "method": "sendMessage",
    }))
    .send()
    .await
    .unwrap();
  info!("res {}", res.status());
}

pub struct DBInsert {
  pub pg: Arc<Client>,
  pub http: reqwest::Client,
  pub watched_addresses: Arc<Mutex<HashSet<String>>>,
  pub cfg: Arc<app_config::Config>,
}

impl DBInsert {
  pub async fn new() -> DBInsert {
    DBInsert {
      pg: std::sync::Arc::new(get_pg_connection().await.unwrap()),
      http: reqwest::Client::new(),
      watched_addresses: Arc::new(Mutex::new(HashSet::new())),
      cfg: std::sync::Arc::new(app_config::Config::new()),
    }
  }

  pub async fn reload_watched_addresses(&self) {
    let rows = self
      .pg
      .query(
        "SELECT id from crawler_tronaddress WHERE crawler_wathing = TRUE",
        &[],
      )
      .await
      .unwrap();
    info!("++++++++++++++++++++++++++++");
    
    let mut wa = self.watched_addresses.lock().unwrap();
    wa.clear();
    for r in rows {
      let address: &str = r.get(0);
      wa.insert(String::from(address));
      info!("{} added to scope", address);
    }
    info!("reload_watched_addresses done");
  }

  async fn insert_transactions(&self, block_num: &i64, tr: &Value) {
    if "SUCCESS" != tr["ret"][0]["contractRet"] {
      return;
    }
    let contract = tr["raw_data"]["contract"][0].clone();

    if "TriggerSmartContract" != contract["type"].as_str().unwrap() {
      return;
    }
    if self.cfg.usdt_contract_address()
      != contract["parameter"]["value"]["contract_address"]
        .as_str()
        .unwrap()
    {
      return;
    }
    info!("try to insert txID: {}", tr["txID"]);
    let owner_address = Address::from_str(
      contract["parameter"]["value"]["owner_address"]
        .as_str()
        .unwrap(),
    )
    .unwrap()
    .to_string();
    let raw_data = &contract["parameter"]["value"]["data"].as_str().unwrap();
    if "a9059cbb" != raw_data.get(0..8).unwrap() {
      return;
    }
    let slice = raw_data.get(32..72).unwrap();
    let to = Address::from_str(format!("41{slice}").as_str())
      .unwrap()
      .to_string();
    let slice = raw_data.get(72..136).unwrap().trim_start_matches("0");
    if 0 == slice.len() {
      return;
    }
    let amount = Decimal::from_str_radix(slice, 16).unwrap();
    let amount_contract = amount / Decimal::from_u128(self.cfg.currency_factor()).unwrap();

    self.pg.query(
      "INSERT INTO crawler_trontransactions (id, block_id, \"address_from\", \"address_to\", amount_raw, amount_contract) VALUES ($1, $2, $3, $4, $5, $6)",
      &[
        &tr["txID"].as_str().unwrap(),
        &block_num,
        &owner_address,
        &to,
        &amount,
        &amount_contract
      ],
    )
    .await
    .unwrap();

    info!("from: {} to: {} amount: {}", owner_address, to, amount);
    if self.watched_addresses.lock().unwrap().contains(&to) {
      let cloned = self.cfg.clone();
      actix_rt::spawn(async move {
        notify_bot(&cloned, &owner_address, &to, &amount_contract).await;
      });  
    }
  }

  pub async fn insert_block(&self, body: &[u8]) {
    let data: Value = serde_json::from_slice(body).unwrap();

    info!(
      "block_header number: {:?}",
      data["block_header"]["raw_data"]["number"]
    );
    let block_num = data["block_header"]["raw_data"]["number"].as_i64().unwrap() as i64;
    self.pg.query(
      "INSERT INTO crawler_tronblock (id, \"blockID\", \"parentHash\") VALUES ($1, $2::TEXT, $3::TEXT)",
      &[
        &block_num,
        &data["blockID"].as_str().unwrap(),
        &data["block_header"]["raw_data"]["parentHash"]
          .as_str()
          .unwrap(),
      ],
    )
    .await
    .unwrap();

    if data.as_object().unwrap().contains_key("transactions") {
      for tr in data["transactions"].as_array().unwrap() {
        self.insert_transactions(&block_num, tr).await;
      }
    }
  }
}

// #[cfg(test)]
// mod tests {
//   use super::*;

//   use serde_json::json;
//   use std::sync::Arc;
//   use tokio;

//   #[tokio::test]
//   async fn correct_insertion() {
//     let pg = Arc::new(get_pg_connection().await.unwrap());
//     let block_num = 41247769;
//     let tr = json!({
//         "ret": [
//             {
//                 "contractRet": "SUCCESS"
//             }
//         ],
//         "signature": [
//             "671feeb8d760decfecaf27c2d6f0eea41b9e7b1062e2bb24d677d2fba94352d82484cc8d956f7db7bc3b87b83568140d6dc1832cde2d529dfcac4602190665731c"
//         ],
//         "txID": "49333f48060f169f1b4c6be0a6a0e7eeb8f3ce8cee3e5d12e023d95d29e50b0c",
//         "raw_data": {
//             "contract": [
//                 {
//                     "parameter": {
//                         "value": {
//                             "data": "a9059cbb0000000000000000000000005ce5f085777890e0ea35892990d3826ad71b04e500000000000000000000000000000000000000000000000029a2241af62c0000",
//                             "owner_address": "41ea06cab892cc41244d394fe110ffec5dad3f2980",
//                             "contract_address": "4137349aeb75a32f8c4c090daff376cf975f5d2eba"
//                         },
//                         "type_url": "type.googleapis.com/protocol.TriggerSmartContract"
//                     },
//                     "type": "TriggerSmartContract"
//                 }
//             ],
//             "ref_block_bytes": "6278",
//             "ref_block_hash": "c10b2750f0a93708",
//             "expiration": 1698158268000u128,
//             "fee_limit": 1000000000,
//             "timestamp": 1698158218439u128
//         },
//         "raw_data_hex": "0a0262782208c10b2750f0a9370840e09cfb90b6315aae01081f12a9010a31747970652e676f6f676c65617069732e636f6d2f70726f746f636f6c2e54726967676572536d617274436f6e747261637412740a1541ea06cab892cc41244d394fe110ffec5dad3f298012154137349aeb75a32f8c4c090daff376cf975f5d2eba2244a9059cbb0000000000000000000000005ce5f085777890e0ea35892990d3826ad71b04e500000000000000000000000000000000000000000000000029a2241af62c000070c799f890b63190018094ebdc03"
//     });
//     pg.query("DELETE FROM transactions WHERE id = '49333f48060f169f1b4c6be0a6a0e7eeb8f3ce8cee3e5d12e023d95d29e50b0c'", &[]).await.unwrap();
//     insert_transactions(pg.clone(), &block_num, &tr).await;
//   }
// }
