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
            let (envelope, exit_code) = zotron_cli::classify_error(&message);
            eprintln!("{envelope}");
            std::process::exit(exit_code);
        }
    }
}
