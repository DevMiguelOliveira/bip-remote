use std::env;

fn main() {
    let command = env::args().nth(1).unwrap_or_else(|| "status".to_owned());
    match command.as_str() {
        "status" => println!("bipremote-host foundation ready"),
        "check-config" => println!("bipremote-host configuration contract ready"),
        "version" => println!("bipremote-host {}", env!("CARGO_PKG_VERSION")),
        other => {
            eprintln!("unsupported command: {other}");
            std::process::exit(2);
        }
    }
}
