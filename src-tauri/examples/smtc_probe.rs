fn main() {
    #[cfg(windows)]
    match lyricglass_lib::platform::windows::probe() {
        Ok(sessions) => println!("{}", serde_json::to_string_pretty(&sessions).unwrap()),
        Err(error) => {
            eprintln!("SMTC probe failed: {error}");
            std::process::exit(1);
        }
    }
    #[cfg(not(windows))]
    eprintln!("Windows required");
}
