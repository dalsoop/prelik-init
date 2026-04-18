/// 도메인 바이너리 이름 → 설명 매핑 (fallback doctor 등에 사용)
pub fn known_domains() -> Vec<(&'static str, &'static str)> {
    vec![
        ("code-server", "code-server (VS Code 웹) 설치/제거"),
        ("wordpress", "WordPress LXC 설치/설정/관리"),
    ]
}

/// 도메인 바이너리 이름 조회
pub fn binary_name(domain: &str) -> String {
    format!("pxi-{domain}")
}
