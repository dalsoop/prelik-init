use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root 경로 계산 실패");
    let domains_root = workspace.join("crates/domains");

    // ─── 도메인 dir 스캔 → rustc-cfg 주입 ───
    // mac-app-init crates/core/build.rs 동일 패턴.
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&domains_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("domain.ncl").exists() {
                if let Some(n) = entry.file_name().to_str() {
                    names.push(n.to_string());
                }
            }
        }
    }
    let all: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    println!(
        "cargo::rustc-check-cfg=cfg(domain, values({}))",
        all.join(", ")
    );
    for name in &names {
        println!("cargo:rustc-cfg=domain=\"{name}\"");
    }
    println!("cargo:rerun-if-changed={}", domains_root.display());

    // ─── ncl/domains.ncl → OUT_DIR/locale.json embed ───
    // 빌드 머신에 nickel 이 있으면 export 해서 embed. 없으면 empty JSON 을 기록 —
    // runtime 의 Registry::load() 가 "embedded 유효성" 을 JSON 파싱으로 판정.
    // release 빌드 환경 (GitHub Actions) 에는 nickel 이 설치되므로 항상 embed 됨.
    // 로컬 dev 빌드 는 nickel 미설치 시 locale.json 파일시스템 tier 에 의존.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR 없음");
    let out_json = Path::new(&out_dir).join("locale.json");
    let ncl_source = workspace.join("ncl/domains.ncl");

    let exported = Command::new("nickel")
        .args([
            "export",
            "--format",
            "json",
            ncl_source.to_str().expect("ncl 경로 utf-8"),
        ])
        .output();

    match exported {
        Ok(o) if o.status.success() => {
            fs::write(&out_json, &o.stdout).expect("OUT_DIR/locale.json 쓰기 실패");
        }
        Ok(o) => {
            println!(
                "cargo:warning=nickel export 실패 (exit {}) — embedded locale 없음",
                o.status.code().unwrap_or(-1)
            );
            fs::write(&out_json, b"{}").expect("OUT_DIR/locale.json empty 쓰기 실패");
        }
        Err(_) => {
            println!(
                "cargo:warning=nickel CLI 없음 — embedded locale 없음 \
                 (런타임은 locale.json 파일시스템 tier 에만 의존)"
            );
            fs::write(&out_json, b"{}").expect("OUT_DIR/locale.json empty 쓰기 실패");
        }
    }

    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("ncl").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("ncl/contracts").display()
    );
}
