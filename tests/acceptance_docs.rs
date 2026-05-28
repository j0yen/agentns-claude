//! AC10: README + CHANGELOG present and named correctly.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn readme_present_and_documents_install() {
    let path = repo_root().join("README.md");
    let body = fs::read_to_string(&path).unwrap_or_default();
    assert!(!body.is_empty(), "README.md is empty at {}", path.display());
    let lower = body.to_lowercase();
    assert!(
        lower.contains("install") && lower.contains("usage"),
        "README.md must document install and usage"
    );
    assert!(
        lower.contains("mock") || lower.contains("agentns_session_id_override"),
        "README.md must document the mock-mode contract"
    );
}

#[test]
fn changelog_has_v0_1_0_section() {
    let path = repo_root().join("CHANGELOG.md");
    let body = fs::read_to_string(&path).unwrap_or_default();
    assert!(!body.is_empty(), "CHANGELOG.md is empty at {}", path.display());
    assert!(
        body.contains("0.1.0"),
        "CHANGELOG.md must include a v0.1.0 section"
    );
}
