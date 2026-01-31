use std::process::Command;

fn main() {
    // Get git SHA for version tracking
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=APVD_BUILD_SHA={}", sha);

    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
}
