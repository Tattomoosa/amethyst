use std::env::home_dir;
use std::fs::{create_dir_all, Path, read_to_string};
use vergen::{self, ConstantsFlags};

fn main() {
    let amethyst_home = Path::new(home_dir).join(".amethyst");
    match amethyst_home.exists() {
        true => {
            match check_sentry_allowed(amethyst_home) {
                Some(v) => {
                    load_sentry_dsn()
                },
                None => {
                    let mut file = File::create(amethyst_home.join(".sentry_status.txt")).expect("Error writing Sentry status file");
                    match ask_user_data_collection() {
                        true => {
                            let _ = file.write_all(b"true");
                            load_sentry_dsn()
                        },
                        false => {
                            let _ = file.write_all(b"false");
                        }
                    }
                }
            }
        },
        false => {
            create_dir_all(amethyst_home);
            let mut file = File::create(amethyst_home.join(".sentry_status.txt")).expect("Error writing Sentry status file");
            match ask_user_data_collection() {
                true => {
                    let _ = file.write_all(b"true");
                    load_sentry_dsn()
                },
                false => {
                    let _ = file.write_all(b"false");
                }
            }
        }
    };

    vergen::generate_cargo_keys(ConstantsFlags::all())
        .unwrap_or_else(|e| panic!("Vergen crate failed to generate version information! {}", e));

    println!("cargo:rerun-if-changed=build.rs");
}


fn check_sentry_allowed(amethyst_path: Path) -> Option<bool> {
    sentry_status_file = amethyst_home.join(".sentry_status.txt");
    match sentry_status_file.exists() {
        true => {
            match read_to_string(sentry_status_file) {
                Ok(result) => {
                    match &result {
                        "true" => { Some(true) },
                        "false" => { Some(false) }
                    }
                },
                Err(e) => {
                    None
                }
            }
        },
        false => {
            None
        }
    }
}

fn ask_user_data_collection() -> bool {
    let mut response = String::new();
    print!("May we collect anonymous panic data and usage statistics to help improve Amethyst? No personal information is collected or stored.");
    print!("Please enter Y (default) or N: ");
    let _ = stdout().flush();
    stdin().read_line(&mut s).expect("There was an error getting your input");
    match response {
        "N" => { false },
        _ => { true }
    }
}

fn load_sentry_dsn() {
    let sentry_dsn = read_to_string(amethyst_home.join(".sentry_dsn.txt"));
    println!("cargo:rustc-env=SENTRY_DSN={}", sentry_dsn);
}