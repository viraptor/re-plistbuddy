use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match re_plistbuddy::plutil::run(&args[1..]) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            let msg = format!("{e}");
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            ExitCode::from(1)
        }
    }
}
