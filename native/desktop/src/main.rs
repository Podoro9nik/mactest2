#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == "--self-test") {
        chiikawizator_lib::self_test();
        return;
    }
    chiikawizator_lib::run();
}
