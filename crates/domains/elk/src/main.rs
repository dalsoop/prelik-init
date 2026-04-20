//! pxi-elk — ELK 스택 (Elasticsearch + Kibana + Logstash) 관리.

use clap::{Parser, Subcommand};
use pxi_core::common;
use std::process::Command;

#[derive(Parser)]
#[command(name = "pxi-elk", about = "ELK 스택 관리 (ES + Kibana + Logstash)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

const ELK_IP: &str = "10.0.50.190"; // LINT_ALLOW: ELK LXC
const ELK_VMID: &str = "50190";

#[derive(Subcommand)]
enum Cmd {
    /// ELK 상태 확인 (ES + Kibana + Logstash)
    Status,
    /// Kibana 웹 UI URL 표시
    Open,
    /// 특정 LXC에서 ELK로 로그 전달 설정 (rsyslog)
    Connect {
        /// LXC VMID
        vmid: String,
    },
    /// LXC의 ELK 로그 전달 해제
    Disconnect {
        /// LXC VMID
        vmid: String,
    },
    /// 최근 로그 검색 (Elasticsearch query_string)
    Search {
        /// 검색 쿼리 (예: "level:error", "medusa AND path:/store")
        query: String,
        /// 최근 N분 (기본 30)
        #[arg(long, default_value = "30")]
        minutes: u32,
        /// 결과 수 (기본 20)
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    /// 인덱스 목록 + 크기
    Indices,
    /// 오래된 인덱스 정리 (기본 30일)
    Cleanup {
        /// 보존 일수
        #[arg(long, default_value = "30")]
        days: u32,
    },
    /// ES + Kibana + Logstash 재시작
    Restart,
    /// LXC에 ELK 설치 (Elasticsearch + Kibana + Logstash)
    Install {
        /// 대상 LXC VMID (기본: 50190)
        #[arg(long, default_value = "50190")]
        vmid: String,
    },
    /// ELK 스택 진단
    Doctor,
}

fn es_url() -> String {
    format!("http://{}:9200", ELK_IP)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let es = es_url();
    // Kibana URL — config.toml network.internal_zone_pve() 기반 동적 구성.
    // 공식: elk.{zone} (zone fallback: 50.internal.kr)
    let zone = pxi_core::config::Config::load()
        .map(|c| c.network.internal_zone_pve())
        .unwrap_or_else(|_| "50.internal.kr".into());
    let kibana_url = format!("https://elk.{zone}");
    let kibana = kibana_url.as_str();

    match cli.cmd {
        Cmd::Status => {
            println!("=== Elasticsearch ===");
            common::run("curl", &["-sS", &format!("{}/_cluster/health?pretty", es)]);
            println!("\n=== Kibana ===");
            common::run("curl", &["-sS", "-o", "/dev/null", "-w", "HTTP %{http_code}\n",
                                  &format!("{}/api/status", kibana)]);
            println!("\n=== Logstash ===");
            common::run("pct", &["exec", ELK_VMID, "--", "systemctl", "is-active", "logstash"]);
        }
        Cmd::Open => {
            println!("Kibana: {}", kibana);
        }
        Cmd::Connect { vmid } => {
            let script = format!(
                "echo '*.* @@{}:5514' > /etc/rsyslog.d/90-elk.conf && \
                 apt-get install -y rsyslog 2>/dev/null; \
                 systemctl enable --now rsyslog && systemctl restart rsyslog && \
                 logger -t pxi-elk 'ELK connected from LXC {}'",
                ELK_IP, vmid
            );
            common::run("pct", &["exec", &vmid, "--", "bash", "-c", &script]);
            println!("✓ LXC {} → ELK ({}:5514) 연결됨", vmid, ELK_IP);
        }
        Cmd::Disconnect { vmid } => {
            common::run("pct", &["exec", &vmid, "--", "bash", "-c",
                        "rm -f /etc/rsyslog.d/90-elk.conf && systemctl restart rsyslog"]);
            println!("✓ LXC {} ELK 연결 해제됨", vmid);
        }
        Cmd::Search { query, minutes, limit } => {
            let body = format!(
                r#"{{"size":{},"sort":[{{"@timestamp":"desc"}}],"query":{{"bool":{{"must":[{{"query_string":{{"query":"{}"}}}},{{"range":{{"@timestamp":{{"gte":"now-{}m"}}}}}}]}}}}}}"#,
                limit, query.replace('"', "\\\""), minutes
            );
            let script = format!(
                "curl -sS '{}/syslog-*/_search' -H 'Content-Type: application/json' -d '{}' | \
                 python3 -c \"import json,sys; d=json.load(sys.stdin); \
                 [print(h['_source'].get('@timestamp','?')[:19], '|', \
                        h['_source'].get('log_level', h['_source'].get('medusa',{{}}).get('level','?'))[:5], '|', \
                        h['_source'].get('message','')[:120]) \
                  for h in d.get('hits',{{}}).get('hits',[])]\"",
                es, body
            );
            common::run("bash", &["-c", &script]);
        }
        Cmd::Indices => {
            common::run("curl", &["-sS", &format!("{}/_cat/indices?v&h=index,docs.count,store.size&s=index", es)]);
        }
        Cmd::Cleanup { days } => {
            println!("{}일 이전 syslog-* 인덱스 삭제 중...", days);
            let script = format!(
                "CUTOFF=$(date -u -d '{d} days ago' +syslog-%Y.%m.%d 2>/dev/null || \
                          date -u -v-{d}d +syslog-%Y.%m.%d); \
                 for idx in $(curl -sS '{es}/_cat/indices/syslog-*?h=index' | sort); do \
                   [[ \"$idx\" < \"$CUTOFF\" ]] && curl -sS -X DELETE '{es}/'$idx && echo \" deleted: $idx\"; \
                 done; echo \"cutoff: $CUTOFF\"",
                d = days, es = es
            );
            common::run("bash", &["-c", &script]);
        }
        Cmd::Restart => {
            common::run("pct", &["exec", ELK_VMID, "--", "bash", "-c",
                        "systemctl restart elasticsearch kibana logstash && \
                         sleep 5 && systemctl is-active elasticsearch kibana logstash"]);
        }
        Cmd::Install { vmid } => { install(&vmid)?; }
        Cmd::Doctor => { doctor(); }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// install — ELK 초기 설정 (encryption keys, locale, traefik)
// ---------------------------------------------------------------------------

fn install(vmid: &str) -> anyhow::Result<()> {
    println!("=== ELK 초기 설정 (LXC {vmid}) ===\n");

    // 1. Kibana encryption keys
    println!("[1/4] Kibana encryption keys");
    let check_key = pct_exec(vmid, "grep -q 'xpack.encryptedSavedObjects.encryptionKey' /etc/kibana/kibana.yml && echo exists");
    if check_key.trim() == "exists" {
        println!("  이미 설정됨");
    } else {
        let script = r#"
KEY1=$(openssl rand -hex 16)
KEY2=$(openssl rand -hex 16)
KEY3=$(openssl rand -hex 16)
echo "xpack.encryptedSavedObjects.encryptionKey: \"$KEY1\"" >> /etc/kibana/kibana.yml
echo "xpack.security.encryptionKey: \"$KEY2\"" >> /etc/kibana/kibana.yml
echo "xpack.reporting.encryptionKey: \"$KEY3\"" >> /etc/kibana/kibana.yml
echo done
"#;
        pct_exec(vmid, script);
        println!("  ✓ 3개 키 생성 완료");
    }

    // 2. Kibana locale (ko-KR)
    println!("[2/4] Kibana 한국어 설정");
    let has_ko = pct_exec(vmid, "test -f /usr/share/kibana/node_modules/@kbn/translations-plugin/translations/ko-KR.json && echo yes");
    if has_ko.trim() == "yes" {
        println!("  ko-KR.json 이미 존재");
    } else {
        println!("  ⚠ ko-KR.json 없음 — homelab-i18n/kibana/deploy.sh 실행 필요");
    }

    // locale 설정
    let locale_set = pct_exec(vmid, "grep -q '^i18n.locale' /etc/kibana/kibana.yml && echo yes");
    if locale_set.trim() != "yes" {
        pct_exec(vmid, "sed -i 's/#i18n.locale:.*/i18n.locale: \"ko-KR\"/' /etc/kibana/kibana.yml");
        println!("  ✓ i18n.locale: ko-KR 설정");
    } else {
        println!("  이미 설정됨");
    }

    // supportedLocale에 ko-KR 추가
    let has_supported = pct_exec(vmid, "grep -q 'ko-KR' /usr/share/kibana/node_modules/@kbn/core-i18n-server-internal/src/constants.js 2>/dev/null && echo yes");
    if has_supported.trim() != "yes" && has_ko.trim() == "yes" {
        pct_exec(vmid, r#"sed -i "s/\(supportedLocale.*\)\]/\1, 'ko-KR']/" /usr/share/kibana/node_modules/@kbn/core-i18n-server-internal/src/constants.js"#);
        println!("  ✓ supportedLocale에 ko-KR 추가");
        // x-pack/.i18nrc.json 등록
        pct_exec(vmid, r#"python3 -c "
import json
with open('/usr/share/kibana/x-pack/.i18nrc.json') as f: data = json.load(f)
entry = '@kbn/translations-plugin/translations/ko-KR.json'
if entry not in data.get('translations', []):
    data.setdefault('translations', []).append(entry)
    with open('/usr/share/kibana/x-pack/.i18nrc.json', 'w') as f: json.dump(data, f, indent=2)
""#);
        println!("  ✓ x-pack/.i18nrc.json 등록");
    }

    // 3. Traefik 라우트 — host = elk.{internal_zone_pve}
    println!("[3/4] Traefik 라우트");
    let zone = pxi_core::config::Config::load()
        .map(|c| c.network.internal_zone_pve())
        .unwrap_or_else(|_| "50.internal.kr".into());
    let elk_host = format!("elk.{zone}");
    let check_cmd = format!("curl -sf --max-time 3 -o /dev/null -w '%{{http_code}}' https://{elk_host}/ 2>/dev/null");
    let route_ok = Command::new("bash")
        .args(["-c", &check_cmd])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("302") || String::from_utf8_lossy(&o.stdout).contains("200"))
        .unwrap_or(false);
    if route_ok {
        println!("  ✓ {elk_host} 접근 가능");
    } else {
        println!("  라우트 추가 중...");
        common::run("pxi", &["run", "traefik", "add",
            "--name", "elk",
            "--domain", &elk_host,
            "--backend", &format!("http://{}:5601", ELK_IP)]);
        println!("  ✓ 라우트 추가됨");
    }

    // 4. 재시작
    println!("[4/4] Kibana 재시작");
    common::run("pct", &["exec", vmid, "--", "systemctl", "restart", "kibana"]);
    println!("  ✓ 재시작 완료 (1-2분 후 접속 가능)");

    println!("\n=== 완료 ===");
    println!("  URL: https://{elk_host}");
    println!("  진단: pxi run elk doctor");
    Ok(())
}

fn pct_exec(vmid: &str, script: &str) -> String {
    Command::new("pct")
        .args(["exec", vmid, "--", "bash", "-c", script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// doctor — ELK 스택 진단 (서비스 + 설정 체크)
// ---------------------------------------------------------------------------

fn doctor() {
    println!("=== pxi-elk doctor ===\n");

    // 1. Elasticsearch reachable
    let es_ok = Command::new("curl")
        .args(["-sf", "--max-time", "5", &format!("http://{}:9200", ELK_IP)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!("  {} Elasticsearch ({}:9200)", if es_ok { "✓" } else { "✗" }, ELK_IP);

    // 2. Kibana reachable
    let kibana_ok = Command::new("curl")
        .args(["-sf", "--max-time", "5", &format!("http://{}:5601/api/status", ELK_IP)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!("  {} Kibana ({}:5601)", if kibana_ok { "✓" } else { "✗" }, ELK_IP);

    // 3. Logstash running
    let logstash_ok = Command::new("pct")
        .args(["exec", ELK_VMID, "--", "systemctl", "is-active", "logstash"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!("  {} Logstash (systemctl)", if logstash_ok { "✓" } else { "✗" });

    // 4. syslog-* index count
    let idx_output = Command::new("curl")
        .args(["-sf", "--max-time", "5", &format!("http://{}:9200/_cat/indices/syslog-*?h=index", ELK_IP)])
        .output();
    match idx_output {
        Ok(o) if o.status.success() => {
            let body = String::from_utf8_lossy(&o.stdout);
            let count = body.lines().filter(|l| !l.trim().is_empty()).count();
            println!("  ✓ syslog-* 인덱스: {}개", count);
        }
        _ => println!("  ✗ syslog-* 인덱스 조회 실패"),
    }

    // 5. Encryption keys
    let enc_ok = pct_exec(ELK_VMID, "grep -q 'xpack.encryptedSavedObjects.encryptionKey' /etc/kibana/kibana.yml && echo yes");
    println!("  {} encryption keys", if enc_ok.trim() == "yes" { "✓" } else { "✗ 누락 — `pxi run elk install` 실행" });

    // 6. Locale
    let locale = pct_exec(ELK_VMID, "grep '^i18n.locale' /etc/kibana/kibana.yml 2>/dev/null | head -1");
    let locale = locale.trim();
    if locale.is_empty() {
        println!("  ✗ i18n.locale 미설정 (기본 영어)");
    } else {
        println!("  ✓ {}", locale);
    }

    // 7. ko-KR translation file
    let ko_exists = pct_exec(ELK_VMID, "test -f /usr/share/kibana/node_modules/@kbn/translations-plugin/translations/ko-KR.json && echo yes");
    println!("  {} ko-KR.json", if ko_exists.trim() == "yes" { "✓" } else { "✗ 없음 — homelab-i18n/kibana/deploy.sh" });

    // 8. Traefik route — host 동적 구성
    let zone = pxi_core::config::Config::load()
        .map(|c| c.network.internal_zone_pve())
        .unwrap_or_else(|_| "50.internal.kr".into());
    let elk_host = format!("elk.{zone}");
    let check_cmd = format!("curl -sf --max-time 3 -o /dev/null https://{elk_host}/ 2>/dev/null");
    let route_ok = Command::new("bash")
        .args(["-c", &check_cmd])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!("  {} Traefik 라우트 ({elk_host})", if route_ok { "✓" } else { "✗" });
}
