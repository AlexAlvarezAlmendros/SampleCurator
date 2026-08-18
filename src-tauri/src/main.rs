// Sin consola en Windows para una build de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    samplecurator_lib::run()
}
