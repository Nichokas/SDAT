use std::path::Path;

fn main() {
    // Embedded translations (like Go's //go:embed of the .po files).
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let loc_dir = Path::new(&manifest).join("../../internal/gettext/locales");
    println!("cargo:rerun-if-changed={}", loc_dir.display());
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
    .expect("write locales.rs");
}
