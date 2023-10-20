use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

#[derive(Debug, Clone)]
pub struct AppState {
  pub stop_get_block_loop: Arc<AtomicBool>
}

impl AppState {
  pub fn set_stop_get_block_loop(&self, val: bool) {
    self.stop_get_block_loop.store(val, Ordering::SeqCst);
  }

  pub fn get_stop_get_block_loop(&self) -> bool {
    self.stop_get_block_loop.load(Ordering::SeqCst)
  }

  pub fn get_default() -> AppState {
    AppState {
      stop_get_block_loop: Arc::new(AtomicBool::new(false))
    }
  }
}
