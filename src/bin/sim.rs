#[path = "../facade_cli.rs"]
mod facade_cli;

fn main() -> std::process::ExitCode {
    facade_cli::process_main()
}
