#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if dsh_desktop_lib::run_update_helper_if_requested() {
        return;
    }
    dsh_desktop_lib::run();
}
