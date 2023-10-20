use serde::Deserialize;

const SETTINGS: &str = include_str!("settings.json5");

#[derive(Deserialize, Debug)]
pub struct Pg {
  pub host: String,
  pub user: String,
  pub pass: String,
  pub db: String,
}

#[derive(Deserialize, Debug)]
pub struct Net {
  pub host: String,
  pub usdt_contract_address: String,
  pub currency_factor: u128,
}

#[derive(Deserialize, Debug)]
pub struct HttpServer {
  pub port: u16,
}

#[derive(Deserialize, Debug)]
pub struct Config {
  pub pg: Pg,
  pub nile: Net,
  pub prod: Net,
  pub current_net: String,
  pub bot_token: String,
  pub http_server: HttpServer
}

impl Config {
  pub fn new() -> Config {
    serde_json5::from_str(SETTINGS).unwrap()
  }

  pub fn usdt_contract_address(&self) -> &String {
    match self.current_net.as_str() {
      "nile" => &self.nile.usdt_contract_address,
      "prod" => &self.prod.usdt_contract_address,
      _x => panic!("")
    } 
  }

  pub fn currency_factor(&self) -> u128 {
    match self.current_net.as_str() {
      "nile" => self.nile.currency_factor,
      "prod" => self.prod.currency_factor,
      _x => panic!("")
    } 
  }

  pub fn api_host(&self) -> &String {
    match self.current_net.as_str() {
      "nile" => &self.nile.host,
      "prod" => &self.prod.host,
      _x => panic!("")
    } 
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn check_get_settings() {
    let cfg = Config::new();
    assert_eq!(cfg.nile.currency_factor, 1000000000000000000u128)
  }
}
