use dioxus::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

impl Severity {
    pub fn css_class(&self) -> &'static str {
        match self {
            Severity::Critical => "sev-critical",
            Severity::High => "sev-high",
            Severity::Medium => "sev-medium",
            Severity::Low => "sev-low",
            Severity::Info => "sev-info",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum FindingStatus {
    Pass,
    Fail,
    Warning,
}

impl fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FindingStatus::Pass => write!(f, "PASS"),
            FindingStatus::Fail => write!(f, "FAIL"),
            FindingStatus::Warning => write!(f, "WARN"),
        }
    }
}

impl FindingStatus {
    pub fn css_class(&self) -> &'static str {
        match self {
            FindingStatus::Pass => "status-pass",
            FindingStatus::Fail => "status-fail",
            FindingStatus::Warning => "status-warn",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SecurityFinding {
    pub category: &'static str,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub status: FindingStatus,
    pub remediation: Option<String>,
}

// ── Scanner ─────────────────────────────────────────────────────────────────

fn read_source(cache: &mut HashMap<String, String>, root: &str, relative: &str) -> String {
    if let Some(cached) = cache.get(relative) {
        return cached.clone();
    }
    let path = Path::new(root).join(relative);
    let content = std::fs::read_to_string(path).unwrap_or_default();
    cache.insert(relative.to_string(), content.clone());
    content
}

fn source_contains(
    cache: &mut HashMap<String, String>,
    root: &str,
    relative: &str,
    pattern: &str,
) -> bool {
    read_source(cache, root, relative).contains(pattern)
}

pub fn run_security_scan(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut cache = HashMap::new();

    scan_tls_config(root, targets, &mut cache, &mut findings);
    scan_policy_enforcement(root, targets, &mut cache, &mut findings);
    scan_bundle_integrity(root, targets, &mut cache, &mut findings);
    scan_endpoint_auth(root, targets, &mut cache, &mut findings);
    scan_isolation(root, targets, &mut cache, &mut findings);
    scan_secrets_management(root, targets, &mut cache, &mut findings);
    scan_wire_protocol(root, targets, &mut cache, &mut findings);
    scan_ssrf_protection(root, targets, &mut cache, &mut findings);

    // Sort: failures first, then by severity.
    findings.sort_by_key(|f| {
        let status_ord = match f.status {
            FindingStatus::Fail => 0,
            FindingStatus::Warning => 1,
            FindingStatus::Pass => 2,
        };
        let sev_ord = match f.severity {
            Severity::Critical => 0,
            Severity::High => 1,
            Severity::Medium => 2,
            Severity::Low => 3,
            Severity::Info => 4,
        };
        (status_ord, sev_ord)
    });

    findings
}

fn scan_tls_config(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let edge = read_source(cache, root, &targets.edge);
    let has_tls = edge.contains("rustls") || edge.contains("TlsAcceptor") || edge.contains("https");

    findings.push(SecurityFinding {
        category: "TLS/mTLS",
        severity: if has_tls { Severity::Info } else { Severity::Medium },
        title: "Edge node TLS termination".into(),
        description: if has_tls {
            "TLS termination is configured on the edge node for external traffic.".into()
        } else {
            "Edge node serves plain HTTP. External traffic is unencrypted unless a TLS-terminating proxy sits in front.".into()
        },
        status: if has_tls { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_tls { None } else {
            Some("Configure TLS on the edge node or place it behind a TLS-terminating load balancer / reverse proxy for production.".into())
        },
    });

    // Internal mesh encryption (KEM + AEAD).
    let has_kem = source_contains(cache, root, &targets.network_kem, "ml_kem")
        || source_contains(cache, root, &targets.network_kem, "MlKem768");
    let has_aead = source_contains(cache, root, &targets.network_crypto, "ChaCha20")
        || source_contains(cache, root, &targets.network_crypto, "chacha20");

    findings.push(SecurityFinding {
        category: "TLS/mTLS",
        severity: Severity::Info,
        title: "Internal mesh encryption".into(),
        description: format!(
            "Node-to-node communication uses {} key exchange and {} encryption.",
            if has_kem {
                "ML-KEM-768 (post-quantum)"
            } else {
                "standard"
            },
            if has_aead {
                "ChaCha20-Poly1305 AEAD"
            } else {
                "unknown"
            }
        ),
        status: if has_kem && has_aead {
            FindingStatus::Pass
        } else {
            FindingStatus::Warning
        },
        remediation: None,
    });

    // Service-to-service mTLS.
    let service = read_source(cache, root, &targets.service);
    let has_mtls = service.contains("mtls")
        || service.contains("client_cert")
        || service.contains("mutual_tls");

    findings.push(SecurityFinding {
        category: "TLS/mTLS",
        severity: Severity::Low,
        title: "Service-to-service mTLS".into(),
        description: if has_mtls {
            "Mutual TLS is configured between services for zero-trust identity verification.".into()
        } else {
            "Services rely on mesh encryption (KEM + AEAD) instead of mTLS. Secure within the mesh but no per-service identity verification.".into()
        },
        status: if has_mtls { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_mtls { None } else {
            Some("Consider adding mTLS for per-service identity verification in high-security environments.".into())
        },
    });
}

fn scan_policy_enforcement(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let edge = read_source(cache, root, &targets.edge);

    // Rate limiting.
    let has_rate_limit =
        edge.contains("RATE_LIMIT") || edge.contains("RateLimit") || edge.contains("rate_limiter");
    findings.push(SecurityFinding {
        category: "Policy",
        severity: if has_rate_limit { Severity::Info } else { Severity::High },
        title: "Per-IP rate limiting".into(),
        description: if has_rate_limit {
            "Per-IP rate limiting is active on the edge node, protecting against request flooding.".into()
        } else {
            "No rate limiting detected on the edge node. The cluster is vulnerable to request flooding.".into()
        },
        status: if has_rate_limit { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_rate_limit { None } else {
            Some("Add per-IP rate limiting on the edge node to prevent denial-of-service attacks.".into())
        },
    });

    // CORS.
    let has_cors = edge.contains("CorsLayer") || edge.contains("cors");
    findings.push(SecurityFinding {
        category: "Policy",
        severity: if has_cors { Severity::Info } else { Severity::Medium },
        title: "CORS configuration".into(),
        description: if has_cors {
            "CORS headers are configured on the edge node for cross-origin request control.".into()
        } else {
            "No CORS configuration detected. Browser-based clients may face cross-origin issues, or the API may be over-permissive.".into()
        },
        status: if has_cors { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_cors { None } else {
            Some("Configure CORS headers on the edge node to restrict cross-origin access.".into())
        },
    });

    // CSP headers.
    let has_csp =
        edge.contains("content-security-policy") || edge.contains("Content-Security-Policy");
    findings.push(SecurityFinding {
        category: "Policy",
        severity: Severity::Low,
        title: "Content Security Policy headers".into(),
        description: if has_csp {
            "CSP headers are set to mitigate XSS and code injection.".into()
        } else {
            "No Content-Security-Policy headers detected. Consider adding CSP for browser-facing endpoints.".into()
        },
        status: if has_csp { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_csp { None } else {
            Some("Add Content-Security-Policy response headers for browser-facing endpoints to mitigate XSS.".into())
        },
    });

    // Circuit breaker.
    let has_circuit_breaker = edge.contains("CircuitBreaker")
        || edge.contains("circuit_breaker")
        || edge.contains("CircuitState");
    findings.push(SecurityFinding {
        category: "Policy",
        severity: Severity::Info,
        title: "Circuit breaker".into(),
        description: if has_circuit_breaker {
            "Circuit breaker pattern is implemented for fault tolerance and cascading failure prevention.".into()
        } else {
            "No circuit breaker detected. Cascading failures from downstream services are possible.".into()
        },
        status: if has_circuit_breaker { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_circuit_breaker { None } else {
            Some("Implement a circuit breaker on the edge node to prevent cascading failures.".into())
        },
    });
}

fn scan_bundle_integrity(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let gateway = read_source(cache, root, &targets.gateway);

    // Content-addressable storage.
    let has_ca = gateway.contains("sha256")
        || gateway.contains("Sha256")
        || gateway.contains("content-addressable");
    findings.push(SecurityFinding {
        category: "Bundle Integrity",
        severity: if has_ca { Severity::Info } else { Severity::High },
        title: "Content-addressable bundle storage".into(),
        description: if has_ca {
            "Bundles are stored by SHA-256 content hash, ensuring integrity and deduplication.".into()
        } else {
            "No content-addressable hashing detected for bundle storage. Bundle integrity cannot be verified.".into()
        },
        status: if has_ca { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_ca { None } else {
            Some("Store bundles using content-addressable hashing (SHA-256) to ensure integrity.".into())
        },
    });

    // Deterministic builds.
    let cli = read_source(cache, root, &targets.cli_build);
    let has_deterministic = cli.contains("build-metadata")
        || cli.contains("deterministic")
        || cli.contains("module_digests");
    findings.push(SecurityFinding {
        category: "Bundle Integrity",
        severity: Severity::Info,
        title: "Deterministic build outputs".into(),
        description: if has_deterministic {
            "Build outputs are deterministic with normalized module ordering and per-module digests.".into()
        } else {
            "Build determinism is not verified. Repeated builds may produce different hashes.".into()
        },
        status: if has_deterministic { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_deterministic { None } else {
            Some("Normalize module ordering in build output and emit per-module content digests.".into())
        },
    });
}

fn scan_endpoint_auth(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let gateway = read_source(cache, root, &targets.gateway);

    // API key auth on management endpoints.
    let has_auth =
        gateway.contains("X-API-Key") || gateway.contains("Bearer") || gateway.contains("api_key");
    findings.push(SecurityFinding {
        category: "Endpoint Auth",
        severity: if has_auth { Severity::Info } else { Severity::Critical },
        title: "Management API authentication".into(),
        description: if has_auth {
            "Management API endpoints require authentication (API key / Bearer token).".into()
        } else {
            "Management API endpoints may be unprotected. Unauthorized users could deploy or modify applications.".into()
        },
        status: if has_auth { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_auth { None } else {
            Some("Require API key or bearer token authentication on all management endpoints.".into())
        },
    });

    // API key hashing at rest.
    let has_key_hashing = gateway.contains("sha256") && gateway.contains("api_key");
    let users = read_source(cache, root, &targets.gateway_users);
    let has_key_hash_users = users.contains("sha256") || users.contains("Sha256");
    let key_hashed = has_key_hashing || has_key_hash_users;

    findings.push(SecurityFinding {
        category: "Endpoint Auth",
        severity: if key_hashed {
            Severity::Info
        } else {
            Severity::Medium
        },
        title: "API key storage security".into(),
        description: if key_hashed {
            "API keys are SHA-256 hashed at rest. Raw keys are only shown at creation time.".into()
        } else {
            "Could not confirm API keys are hashed at rest.".into()
        },
        status: if key_hashed {
            FindingStatus::Pass
        } else {
            FindingStatus::Warning
        },
        remediation: if key_hashed {
            None
        } else {
            Some("Hash API keys (SHA-256) before storing; only show raw key at creation.".into())
        },
    });

    // Password hashing.
    let has_argon2 = users.contains("argon2") || users.contains("Argon2");
    findings.push(SecurityFinding {
        category: "Endpoint Auth",
        severity: if has_argon2 {
            Severity::Info
        } else {
            Severity::High
        },
        title: "Password hashing algorithm".into(),
        description: if has_argon2 {
            "User passwords are hashed with Argon2id (memory-hard KDF).".into()
        } else {
            "Could not confirm Argon2id password hashing. Weak hashing may allow offline attacks."
                .into()
        },
        status: if has_argon2 {
            FindingStatus::Pass
        } else {
            FindingStatus::Warning
        },
        remediation: if has_argon2 {
            None
        } else {
            Some("Use Argon2id for password hashing with recommended parameters.".into())
        },
    });
}

fn scan_isolation(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let compute = read_source(cache, root, &targets.compute);

    // Per-tenant KV isolation.
    let has_tenant_kv = compute.contains("kv_") && compute.contains("tenant");
    findings.push(SecurityFinding {
        category: "Isolation",
        severity: if has_tenant_kv { Severity::Info } else { Severity::High },
        title: "Per-tenant database isolation".into(),
        description: if has_tenant_kv {
            "Each tenant gets an isolated SQLite database (physical separation, not just key prefixing).".into()
        } else {
            "Could not confirm per-tenant database isolation. Tenants may share database access.".into()
        },
        status: if has_tenant_kv { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_tenant_kv { None } else {
            Some("Use per-tenant isolated databases to prevent cross-tenant data access.".into())
        },
    });

    // Resource quotas.
    let has_quotas = compute.contains("FREQ_STORAGE_QUOTA") || compute.contains("FREQ_MAX_KV_KEYS");
    findings.push(SecurityFinding {
        category: "Isolation",
        severity: if has_quotas { Severity::Info } else { Severity::Medium },
        title: "Per-tenant resource quotas".into(),
        description: if has_quotas {
            "Per-tenant resource quotas are enforced (storage, KV keys, SQL size).".into()
        } else {
            "No per-tenant resource quotas detected. A single tenant could exhaust cluster resources.".into()
        },
        status: if has_quotas { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_quotas { None } else {
            Some("Configure FREQ_STORAGE_QUOTA_BYTES, FREQ_MAX_KV_KEYS, FREQ_MAX_SQL_BYTES per tenant.".into())
        },
    });

    // V8 heap limits.
    let has_heap_limit = compute.contains("FREQ_HEAP_LIMIT_MB") || compute.contains("heap_limits");
    findings.push(SecurityFinding {
        category: "Isolation",
        severity: if has_heap_limit {
            Severity::Info
        } else {
            Severity::Medium
        },
        title: "V8 heap memory limits".into(),
        description: if has_heap_limit {
            "V8 isolate heap limits are configured per tenant to prevent memory exhaustion.".into()
        } else {
            "No V8 heap limits detected. A runaway script could exhaust node memory.".into()
        },
        status: if has_heap_limit {
            FindingStatus::Pass
        } else {
            FindingStatus::Fail
        },
        remediation: if has_heap_limit {
            None
        } else {
            Some("Set FREQ_HEAP_LIMIT_MB to cap V8 isolate memory per tenant.".into())
        },
    });

    // CPU time limits.
    let has_cpu_limit = compute.contains("RequestTermination") || compute.contains("cpu_limit");
    findings.push(SecurityFinding {
        category: "Isolation",
        severity: if has_cpu_limit {
            Severity::Info
        } else {
            Severity::Medium
        },
        title: "CPU time limits".into(),
        description: if has_cpu_limit {
            "CPU time monitoring with V8 RequestTermination prevents runaway scripts.".into()
        } else {
            "No CPU time limits detected. Long-running scripts could starve other tenants.".into()
        },
        status: if has_cpu_limit {
            FindingStatus::Pass
        } else {
            FindingStatus::Fail
        },
        remediation: if has_cpu_limit {
            None
        } else {
            Some(
                "Implement CPU time limits with V8 RequestTermination for tenant isolation.".into(),
            )
        },
    });

    // Capability policies.
    let has_policies = compute.contains("FREQ_POLICY_NET") || compute.contains("FREQ_POLICY_FS");
    findings.push(SecurityFinding {
        category: "Isolation",
        severity: if has_policies { Severity::Info } else { Severity::Medium },
        title: "Namespace capability policies".into(),
        description: if has_policies {
            "Per-tenant capability controls restrict network, filesystem, KV, SQL, WebSocket, serve, and env access.".into()
        } else {
            "No per-tenant capability policies detected. Tenants may have unrestricted access.".into()
        },
        status: if has_policies { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_policies { None } else {
            Some("Configure FREQ_POLICY_* environment variables to restrict per-tenant capabilities.".into())
        },
    });
}

fn scan_secrets_management(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let kms = read_source(cache, root, &targets.gateway_kms);

    // Encryption at rest.
    let has_aead = kms.contains("aead_seal") || kms.contains("ChaCha20");
    findings.push(SecurityFinding {
        category: "Secrets",
        severity: if has_aead { Severity::Info } else { Severity::Critical },
        title: "Secrets encrypted at rest".into(),
        description: if has_aead {
            "Secrets are encrypted at rest using ChaCha20-Poly1305 AEAD with per-secret nonces.".into()
        } else {
            "Could not confirm secrets encryption at rest. Secrets may be stored in plaintext.".into()
        },
        status: if has_aead { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_aead { None } else {
            Some("Encrypt all secrets at rest using AEAD (ChaCha20-Poly1305) with per-secret nonces.".into())
        },
    });

    // Master key fallback warning.
    let has_dev_fallback = kms.contains("do-not-use-in-production");
    let has_master_key_loading = kms.contains("FREQ_MASTER_KEY");
    findings.push(SecurityFinding {
        category: "Secrets",
        severity: if has_dev_fallback { Severity::High } else if has_master_key_loading { Severity::Info } else { Severity::Medium },
        title: "Master key configuration".into(),
        description: if has_dev_fallback {
            "A hardcoded dev fallback master key exists. If FREQ_MASTER_KEY is not set, secrets use an insecure default key.".into()
        } else if has_master_key_loading {
            "Master key is loaded from FREQ_MASTER_KEY without a hardcoded fallback.".into()
        } else {
            "Could not confirm master key configuration.".into()
        },
        status: if has_dev_fallback { FindingStatus::Warning } else if has_master_key_loading { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_dev_fallback {
            Some("Always set FREQ_MASTER_KEY in production. The dev fallback is insecure and logged as a warning.".into())
        } else if !has_master_key_loading {
            Some("Configure FREQ_MASTER_KEY environment variable for secret encryption.".into())
        } else { None },
    });

    // Secret masking in logs.
    let compute = read_source(cache, root, &targets.compute);
    let has_masking = compute.contains("mask") || compute.contains("redact");
    findings.push(SecurityFinding {
        category: "Secrets",
        severity: if has_masking { Severity::Info } else { Severity::Medium },
        title: "Secret masking in log output".into(),
        description: if has_masking {
            "Compute node redacts known secret values from log output.".into()
        } else {
            "Could not confirm secret masking in logs. Secrets may leak through application logging.".into()
        },
        status: if has_masking { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_masking { None } else {
            Some("Redact known secret values from log output before emitting.".into())
        },
    });
}

fn scan_wire_protocol(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let network = read_source(cache, root, &targets.network);
    let crypto = read_source(cache, root, &targets.network_crypto);

    // Replay protection.
    let has_replay = network.contains("nonce") || network.contains("replay");
    findings.push(SecurityFinding {
        category: "Wire Protocol",
        severity: if has_replay {
            Severity::Info
        } else {
            Severity::High
        },
        title: "Replay protection".into(),
        description: if has_replay {
            "Nonce-based replay protection is implemented per peer.".into()
        } else {
            "No replay protection detected. Captured packets could be replayed.".into()
        },
        status: if has_replay {
            FindingStatus::Pass
        } else {
            FindingStatus::Fail
        },
        remediation: if has_replay {
            None
        } else {
            Some("Implement per-peer nonce tracking to reject replayed messages.".into())
        },
    });

    // Reliable delivery / retry.
    let has_retry =
        network.contains("retry") || network.contains("Retry") || network.contains("ACK");
    findings.push(SecurityFinding {
        category: "Wire Protocol",
        severity: Severity::Info,
        title: "Reliable message delivery".into(),
        description: if has_retry {
            "Retry queue with exponential backoff and ACK-based delivery ensures reliable communication.".into()
        } else {
            "No reliable delivery mechanism detected. Messages may be lost.".into()
        },
        status: if has_retry { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_retry { None } else {
            Some("Add retry queues with ACK-based delivery for reliable communication.".into())
        },
    });

    // Per-peer rate limiting.
    let has_peer_rl = network.contains("rate_limit") || network.contains("RateLimit");
    findings.push(SecurityFinding {
        category: "Wire Protocol",
        severity: if has_peer_rl {
            Severity::Info
        } else {
            Severity::Medium
        },
        title: "Per-peer wire rate limiting".into(),
        description: if has_peer_rl {
            "Per-peer rate limiting on the wire protocol prevents flooding from compromised nodes."
                .into()
        } else {
            "No per-peer rate limiting detected on the wire protocol.".into()
        },
        status: if has_peer_rl {
            FindingStatus::Pass
        } else {
            FindingStatus::Warning
        },
        remediation: if has_peer_rl {
            None
        } else {
            Some(
                "Add per-peer rate limiting on the wire protocol to prevent internal flooding."
                    .into(),
            )
        },
    });

    // AEAD tag verification.
    let has_aead_open = crypto.contains("aead_open") || crypto.contains("poly1305_verify");
    findings.push(SecurityFinding {
        category: "Wire Protocol",
        severity: if has_aead_open { Severity::Info } else { Severity::Critical },
        title: "AEAD authentication tag verification".into(),
        description: if has_aead_open {
            "Wire protocol messages are authenticated via Poly1305 tag verification before processing.".into()
        } else {
            "Could not confirm AEAD tag verification. Tampered messages may be accepted.".into()
        },
        status: if has_aead_open { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_aead_open { None } else {
            Some("Verify AEAD authentication tags on all received wire protocol messages.".into())
        },
    });
}

fn scan_ssrf_protection(
    root: &str,
    targets: &crate::agent::types::ScanTargets,
    cache: &mut HashMap<String, String>,
    findings: &mut Vec<SecurityFinding>,
) {
    let compute = read_source(cache, root, &targets.compute);

    // SSRF protection on fetch.
    let has_ssrf = compute.contains("is_private_host") || compute.contains("SSRF");
    findings.push(SecurityFinding {
        category: "SSRF Protection",
        severity: if has_ssrf { Severity::Info } else { Severity::Critical },
        title: "Fetch SSRF protection".into(),
        description: if has_ssrf {
            "fetch() blocks requests to private/internal IP ranges (loopback, RFC 1918, link-local) preventing SSRF against the mesh.".into()
        } else {
            "No SSRF protection detected on fetch. Tenant code could access internal mesh services.".into()
        },
        status: if has_ssrf { FindingStatus::Pass } else { FindingStatus::Fail },
        remediation: if has_ssrf { None } else {
            Some("Block fetch requests to private, loopback, and link-local IP ranges.".into())
        },
    });

    // Scheme restriction.
    let has_scheme_check =
        compute.contains("http://") && compute.contains("https://") && compute.contains("SSRF");
    findings.push(SecurityFinding {
        category: "SSRF Protection",
        severity: if has_scheme_check { Severity::Info } else { Severity::Medium },
        title: "URL scheme restriction".into(),
        description: if has_scheme_check {
            "Only http:// and https:// schemes are permitted for fetch, blocking file:// and other dangerous schemes.".into()
        } else {
            "Could not confirm URL scheme restrictions on fetch.".into()
        },
        status: if has_scheme_check { FindingStatus::Pass } else { FindingStatus::Warning },
        remediation: if has_scheme_check { None } else {
            Some("Restrict fetch to http:// and https:// schemes only.".into())
        },
    });
}

// ── Export ───────────────────────────────────────────────────────────────────

pub fn export_findings_json(root: &str, findings: &[SecurityFinding]) -> Option<String> {
    let entries: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "category": f.category,
                "severity": f.severity.to_string(),
                "status": f.status.to_string(),
                "title": f.title,
                "description": f.description,
                "remediation": f.remediation,
            })
        })
        .collect();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();

    let report = serde_json::json!({
        "timestamp_unix": timestamp,
        "total_findings": findings.len(),
        "summary": {
            "critical": findings.iter().filter(|f| f.severity == Severity::Critical).count(),
            "high": findings.iter().filter(|f| f.severity == Severity::High).count(),
            "medium": findings.iter().filter(|f| f.severity == Severity::Medium).count(),
            "low": findings.iter().filter(|f| f.severity == Severity::Low).count(),
            "info": findings.iter().filter(|f| f.severity == Severity::Info).count(),
            "pass": findings.iter().filter(|f| f.status == FindingStatus::Pass).count(),
            "fail": findings.iter().filter(|f| f.status == FindingStatus::Fail).count(),
            "warn": findings.iter().filter(|f| f.status == FindingStatus::Warning).count(),
        },
        "findings": entries,
    });

    let json = serde_json::to_string_pretty(&report).ok()?;
    let path = Path::new(root).join("security-review.json");
    std::fs::write(&path, &json).ok()?;
    Some(path.to_string_lossy().to_string())
}

// ── UI Component ────────────────────────────────────────────────────────────

/// Owned category group for rendering.
struct CategoryGroup {
    name: String,
    items: Vec<SecurityFinding>,
}

#[component]
pub fn SecurityPanel(findings: Signal<Vec<SecurityFinding>>, root: Signal<String>) -> Element {
    let findings_read = findings.read();

    if findings_read.is_empty() {
        return rsx! {
            div { class: "editor-content",
                div { class: "text-muted editor-empty", "Click \"Security Review\" to scan the project." }
            }
        };
    }

    let total = findings_read.len();
    let pass_count = findings_read
        .iter()
        .filter(|f| f.status == FindingStatus::Pass)
        .count();
    let fail_count = findings_read
        .iter()
        .filter(|f| f.status == FindingStatus::Fail)
        .count();
    let warn_count = findings_read
        .iter()
        .filter(|f| f.status == FindingStatus::Warning)
        .count();
    let critical_count = findings_read
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high_count = findings_read
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();

    // Group by category (clone into owned data so the borrow can end).
    let mut categories: Vec<CategoryGroup> = Vec::new();
    for f in findings_read.iter() {
        if let Some(group) = categories.iter_mut().find(|g| g.name == f.category) {
            group.items.push(f.clone());
        } else {
            categories.push(CategoryGroup {
                name: f.category.to_string(),
                items: vec![f.clone()],
            });
        }
    }

    let score_class = if fail_count > 0 || critical_count > 0 {
        "score-badge score-bad"
    } else if warn_count > 3 || high_count > 0 {
        "score-badge score-mixed"
    } else {
        "score-badge score-good"
    };
    let score_label = if fail_count > 0 || critical_count > 0 {
        "Needs Attention"
    } else if warn_count > 3 || high_count > 0 {
        "Fair"
    } else {
        "Good"
    };

    // Drop the read guard before entering rsx.
    drop(findings_read);

    rsx! {
        div { class: "editor-content security-panel",
            // Summary bar.
            div { class: "sec-summary",
                span { class: "{score_class}", "{score_label}" }
                span { class: "sec-stat", "{total} checks" }
                if pass_count > 0 {
                    span { class: "sec-stat status-pass", "{pass_count} pass" }
                }
                if fail_count > 0 {
                    span { class: "sec-stat status-fail", "{fail_count} fail" }
                }
                if warn_count > 0 {
                    span { class: "sec-stat status-warn", "{warn_count} warn" }
                }
                button {
                    class: "btn btn-xs sec-export",
                    onclick: move |_| {
                        let r = root.read().clone();
                        let f = findings.read();
                        if let Some(path) = export_findings_json(&r, &f) {
                            tracing::info!("Exported security report to {path}");
                        }
                    },
                    "Export JSON"
                }
            }

            // Category sections.
            for group in categories {
                div { class: "sec-category",
                    div { class: "sec-category-header", "{group.name}" }
                    for finding in group.items {
                        div { class: "sec-finding",
                            div { class: "sec-finding-header",
                                span { class: "sec-sev-badge {finding.severity.css_class()}", "{finding.severity}" }
                                span { class: "sec-status-badge {finding.status.css_class()}", "{finding.status}" }
                                span { class: "sec-finding-title", "{finding.title}" }
                            }
                            div { class: "sec-finding-desc", "{finding.description}" }
                            if let Some(ref rem) = finding.remediation {
                                div { class: "sec-remediation",
                                    span { class: "sec-remediation-label", "Remediation: " }
                                    "{rem}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Critical.to_string(), "CRITICAL");
        assert_eq!(Severity::High.to_string(), "HIGH");
        assert_eq!(Severity::Medium.to_string(), "MEDIUM");
        assert_eq!(Severity::Low.to_string(), "LOW");
        assert_eq!(Severity::Info.to_string(), "INFO");
    }

    #[test]
    fn finding_status_display() {
        assert_eq!(FindingStatus::Pass.to_string(), "PASS");
        assert_eq!(FindingStatus::Fail.to_string(), "FAIL");
        assert_eq!(FindingStatus::Warning.to_string(), "WARN");
    }

    #[test]
    fn scan_produces_findings_for_real_project() {
        // Use the actual project root if available.
        let root = std::env::var("CARGO_MANIFEST_DIR")
            .map(|d| {
                std::path::PathBuf::from(d)
                    .join("../..")
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|_| ".".into());
        let targets = crate::agent::types::ScanTargets::default();
        let findings = run_security_scan(&root, &targets);
        // The scan should always produce at least a few findings.
        assert!(!findings.is_empty(), "expected at least one finding");
    }

    #[test]
    fn scan_empty_dir_produces_findings() {
        let dir = tempfile::tempdir().unwrap();
        let targets = crate::agent::types::ScanTargets::default();
        let findings = run_security_scan(dir.path().to_str().unwrap(), &targets);
        // Even for an empty dir, we should get findings (failures/warnings).
        assert!(!findings.is_empty());
        // All findings for a missing project should be warnings or failures.
        for f in &findings {
            assert!(
                f.status == FindingStatus::Fail || f.status == FindingStatus::Warning,
                "expected fail/warn for empty project, got {:?} for '{}'",
                f.status,
                f.title
            );
        }
    }

    #[test]
    fn findings_sorted_failures_first() {
        let dir = tempfile::tempdir().unwrap();
        let targets = crate::agent::types::ScanTargets::default();
        let findings = run_security_scan(dir.path().to_str().unwrap(), &targets);
        let mut saw_pass = false;
        for f in &findings {
            if f.status == FindingStatus::Pass {
                saw_pass = true;
            }
            if saw_pass {
                assert_ne!(
                    f.status,
                    FindingStatus::Fail,
                    "fail finding after pass: '{}'",
                    f.title
                );
            }
        }
    }

    #[test]
    fn export_json_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let findings = vec![SecurityFinding {
            category: "Test",
            severity: Severity::Info,
            title: "Test finding".into(),
            description: "Test description".into(),
            status: FindingStatus::Pass,
            remediation: None,
        }];
        let path = export_findings_json(dir.path().to_str().unwrap(), &findings);
        assert!(path.is_some());
        let content = std::fs::read_to_string(path.unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["total_findings"], 1);
        assert_eq!(v["findings"][0]["title"], "Test finding");
    }
}
