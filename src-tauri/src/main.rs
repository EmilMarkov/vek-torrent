// Не показывать консольное окно в release-сборке под Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vek_torrent_lib::run()
}
