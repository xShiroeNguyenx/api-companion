// Ngăn cửa sổ console phụ hiện lên trên Windows ở bản release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    api_companion_lib::run();
}
