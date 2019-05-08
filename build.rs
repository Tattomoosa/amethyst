use vergen::{self, ConstantsFlags};

fn main() {
    let sentry_dsn = include_str!(".sentry_dsn.txt");
    println!("cargo:rustc-env=SENTRY_DSN={}", sentry_dsn);
    vergen::generate_cargo_keys(ConstantsFlags::all())
        .unwrap_or_else(|e| panic!("Vergen crate failed to generate version information! {}", e));

    println!("cargo:rerun-if-changed=build.rs");
}
