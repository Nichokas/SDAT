use std::path::Path;
use std::process::Command;

fn main() {
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");

    // GSettings schema: compiled into OUT_DIR for running from the build tree.
    let schema_src = Path::new(&manifest).join("internal/settings");
    println!("cargo:rerun-if-changed=internal/settings/dev.dergs.Tonearm.gschema.xml");
    let schema_out = Path::new(&out).join("schemas");
    std::fs::create_dir_all(&schema_out).expect("create schema dir in OUT_DIR");
    std::fs::copy(schema_src.join("dev.dergs.Tonearm.gschema.xml"), schema_out.join("dev.dergs.Tonearm.gschema.xml")).expect("copy gschema into OUT_DIR");
    let ok = Command::new("glib-compile-schemas").arg(&schema_out).status().is_ok_and(|s| s.success());
    if !ok {
        println!("cargo:warning=glib-compile-schemas failed; install the schema system-wide or set GSETTINGS_SCHEMA_DIR");
    }
    println!("cargo:rustc-env=TONEARM_SCHEMA_DIR={}", schema_out.display());

    // Commit shown in the about dialog (Go: -ldflags -X ui.Commit=...).
    println!("cargo:rerun-if-env-changed=TONEARM_COMMIT");
    println!("cargo:rerun-if-env-changed=TONEARM_VERSION");
    let commit = std::env::var("TONEARM_COMMIT").ok().filter(|c| !c.is_empty()).or_else(|| Command::new("git")
        .args(["describe", "--tags", "--long", "--abbrev=7"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .or_else(|| Command::new("git").args(["rev-parse", "HEAD"]).output().ok().filter(|o| o.status.success()))
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or_default();
    println!("cargo:rustc-env=TONEARM_COMMIT={commit}");
}
