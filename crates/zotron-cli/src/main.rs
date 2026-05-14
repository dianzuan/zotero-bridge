fn main() {
    // Reset SIGPIPE to default behavior (terminate silently) so piping
    // to head/tail/jq doesn't cause a panic.
    #[cfg(unix)]
    {
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }

    match zotron_cli::run(std::env::args_os()) {
        Ok(output) => print!("{output}"),
        Err(message) => {
            eprintln!("{}", zotron_cli::format_error_json(&message));
            std::process::exit(1);
        }
    }
}
