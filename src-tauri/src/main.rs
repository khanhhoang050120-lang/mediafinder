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
    // Run by the uninstaller: take the Startup shortcut and the scheduled task
    // with us. Leaving a task that launches a program which no longer exists is
    // the kind of litter that outlives the app on someone else's machine.
    if args.iter().any(|a| a == "--remove-setup") {
        // `--quiet` comes from a silent uninstall, where nobody is there to
        // answer a permission dialog.
        let may_prompt = !args.iter().any(|a| a == "--quiet");
        mediafinder::setup::remove_setup(may_prompt);
        return;
    }

    // The elevated half of the above: nothing but the task deletion.
    if args.iter().any(|a| a == "--remove-task") {
        mediafinder::setup::remove_task_only();
        return;
    }

    if args.iter().any(|a| a == "--index") {
        mediafinder::run_indexer();
    } else if args.iter().any(|a| a == "--watch") {
        mediafinder::run_watch(&args);
    } else if args.iter().any(|a| a == "--audit") {
        mediafinder::run_audit(&args);
    } else {
        mediafinder::run_gui();
    }
}
