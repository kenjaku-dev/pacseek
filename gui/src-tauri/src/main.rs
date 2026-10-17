// Prevents an additional console window on Windows in release — template
// parity with the upstream Tauri scaffold; harmless on Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pakmap_gui::run()
}
