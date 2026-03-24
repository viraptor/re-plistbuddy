use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match re_plistbuddy::plist_buddy::run(&args[1..]) {
        Ok(any_failed) => {
            if any_failed {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            let msg = format!("{e}");
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            ExitCode::from(1)
        }
    }
}
