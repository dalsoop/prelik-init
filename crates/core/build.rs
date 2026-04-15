use std::fs;
use std::path::Path;

fn main() {
    // workspace 루트 기준 crates/domains/ 스캔
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domains_root = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("crates/domains"));

    let mut names: Vec<String> = Vec::new();
    if let Some(dir) = domains_root.as_ref() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("domain.ncl").exists() {
                    if let Some(n) = entry.file_name().to_str() {
                        names.push(n.to_string());
                    }
                }
            }
        }
    }

    // cfg 등록 + 발행
    let all: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    println!(
        "cargo::rustc-check-cfg=cfg(domain, values({}))",
        all.join(", ")
    );
    for name in &names {
        println!("cargo:rustc-cfg=domain=\"{name}\"");
    }

    // domain.ncl 변경 시 재빌드
    if let Some(dir) = domains_root {
        println!("cargo:rerun-if-changed={}", dir.display());
    }
}
