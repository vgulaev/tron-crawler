use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

#[derive(Debug, Clone)]
pub struct AppState {
  pub stop_get_block_loop: Arc<AtomicBool>,
  pub reload_watched_addresses: Arc<AtomicBool>,
}

impl AppState {
  pub fn set_stop_get_block_loop(&self, val: bool) {
    self.stop_get_block_loop.store(val, Ordering::SeqCst);
  }

  pub fn get_stop_get_block_loop(&self) -> bool {
    self.stop_get_block_loop.load(Ordering::SeqCst)
  }

  pub fn set_reload_watched_addresses(&self, val: bool) {
    self.reload_watched_addresses.store(val, Ordering::SeqCst);
  }

  pub fn get_reload_watched_addresses(&self) -> bool {
    self.reload_watched_addresses.load(Ordering::SeqCst)
  }

  pub async fn new() -> AppState {
    AppState {
      stop_get_block_loop: Arc::new(AtomicBool::new(false)),
      reload_watched_addresses: Arc::new(AtomicBool::new(true)),
    }
  }
}
