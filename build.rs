use std::path::Path;
use std::process::Command;

fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // GSettings schema: compiled into OUT_DIR for running from the build tree.
    let schema_src = Path::new(&manifest).join("internal/settings");
    println!("cargo:rerun-if-changed=internal/settings/dev.dergs.Tonearm.gschema.xml");
    let schema_out = Path::new(&out).join("schemas");
    std::fs::create_dir_all(&schema_out).unwrap();
    std::fs::copy(schema_src.join("dev.dergs.Tonearm.gschema.xml"), schema_out.join("dev.dergs.Tonearm.gschema.xml")).unwrap();
    let ok = Command::new("glib-compile-schemas").arg(&schema_out).status().map(|s| s.success()).unwrap_or(false);
    if !ok {
        println!("cargo:warning=glib-compile-schemas failed; install the schema system-wide or set GSETTINGS_SCHEMA_DIR");
    }
    println!("cargo:rustc-env=TONEARM_SCHEMA_DIR={}", schema_out.display());

    // Embedded translations (like Go's //go:embed of the .po files).
    let loc_dir = Path::new(&manifest).join("internal/gettext/locales");
    println!("cargo:rerun-if-changed=internal/gettext/locales");
    let mut entries: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&loc_dir) {
        for e in rd.flatten() {
            let po = e.path().join("default.po");
            if po.is_file() {
                println!("cargo:rerun-if-changed={}", po.display());
                entries.push(format!("({:?}, include_str!({:?})),", e.file_name().to_string_lossy(), po.display().to_string()));
            }
        }
    }
    entries.sort();
    std::fs::write(
        Path::new(&out).join("locales.rs"),
        format!("pub static LOCALES: &[(&str, &str)] = &[\n{}\n];\n", entries.join("\n")),
    )
    .unwrap();

    // Commit shown in the about dialog (Go: -ldflags -X ui.Commit=...).
    println!("cargo:rerun-if-env-changed=TONEARM_COMMIT");
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
