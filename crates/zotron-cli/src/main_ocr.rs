fn main() {
    match zotron_cli::run_ocr(std::env::args_os()) {
        Ok(output) => print!("{output}"),
        Err(message) => {
            eprintln!("{}", zotron_cli::format_error_json(&message));
            std::process::exit(1);
        }
    }
}
