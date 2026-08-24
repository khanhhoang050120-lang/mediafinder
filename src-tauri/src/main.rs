// Console is hidden in release builds only; in debug it stays so `tracing`
// output is visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mediafinder::init_tracing();

    // One binary, two modes (see the architecture note in build.rs):
    //   mediafinder.exe            -> GUI, runs as invoker, never elevated
    //   mediafinder.exe --index    -> short-lived indexer, launched elevated
    //   mediafinder.exe --watch    -> follow the change journal and print what
    //                                 it says; needs Administrator, and exists
    //                                 so the journal reader can be checked
    //                                 against a real volume rather than only
    //                                 against hand-built records
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--index") {
        mediafinder::run_indexer();
    } else if args.iter().any(|a| a == "--watch") {
        mediafinder::run_watch(&args);
    } else {
        mediafinder::run_gui();
    }
}
