//! Test-only process fixture. It is compiled into a temporary `Spine.com` by the integration
//! tests and must never be treated as evidence that the proprietary external CLI is compatible.

use std::{env, fs, io::Write, thread, time::Duration};

fn main() {
    let mode = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("synthetic-mode.txt")))
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_else(|| "wrong-version".into());
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments == ["--version"] {
        match mode.trim() {
            "hang" => thread::sleep(Duration::from_secs(10)),
            "overflow" => {
                let bytes = vec![b'X'; 128 * 1024];
                std::io::stdout().write_all(&bytes).unwrap();
            }
            "ambiguous-version" => {
                println!("Spine 4.2.43 Professional");
                println!("Spine 4.2.44 Professional");
            }
            "ambient-version" => println!("diagnostic mentions Spine 4.2.43"),
            _ => println!("Spine 4.2.44 Professional"),
        }
        return;
    }
    eprintln!("synthetic fixture does not perform proprietary operations");
    std::process::exit(31);
}
