// Console is hidden in release builds only; in debug it stays so `tracing`
// output is visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mediafinder::init_tracing();

    // One binary, two modes (see the architecture note in build.rs):
    //   mediafinder.exe            -> GUI, runs as invoker, never elevated
    //   mediafinder.exe --index    -> short-lived indexer, launched elevated
    if std::env::args().skip(1).any(|arg| arg == "--index") {
        mediafinder::run_indexer();
    } else {
        mediafinder::run_gui();
    }
}
