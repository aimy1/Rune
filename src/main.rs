pub mod config;
pub mod core;
pub mod plugins;
pub mod search;
pub mod storage;
pub mod ui;

use config::{get_cache_dir, Config};
use core::app::App;
use plugins::run_clipboard_daemon;
use std::env;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Set panic hook to ensure terminal is cleaned up if the app crashes
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let args: Vec<String> = env::args().collect();

    // Check if running in daemon mode (for clipboard collection)
    if args.len() > 1 && (args[1] == "daemon" || args[1] == "--daemon") {
        let cache_dir = get_cache_dir();
        println!("Rune: Clipboard daemon active. Monitoring clipboard events...");
        // Run blocking loop in main thread
        run_clipboard_daemon(&cache_dir);
        return Ok(());
    }

    // Normal Mode: Launch TUI palette
    let config = Config::load();
    let cache_dir = get_cache_dir();
    
    let mut app = App::new(config, cache_dir);
    app.run()?;

    Ok(())
}
