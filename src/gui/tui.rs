use sha1::digest::block_buffer::Lazy;
use tokio::{self, sync::RwLock};

use crate::{gui::log_panel, main::SHUTDOWN};

// ****** UTIL ******
fn init_terminal() {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    enable_raw_mode().unwrap();
}

// ****** MAIN ******
pub static UNIQUE: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
pub static TERMINAL: LazyLock<Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>> =
    LazyLock::new(|| {
        Arc::new(Mutex::new(
            Terminal::new(CrosstermBackend::new(stdout())).unwrap(),
        ))
    });

pub fn start_tui() {
    tokio::spawn(async move {
        init_terminal();
        while !SHUTDOWN.read().await {
            if UNIQUE.read().await {
                render_tui();
            } else {
                thread::sleep(Duration::from_millis(50));
            }
        }
    })
}
pub fn render_tui() {
    log_panel::render();
}
