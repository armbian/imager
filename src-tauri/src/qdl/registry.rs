//! Resolves a board's QDL facts from the Armbian API (the served `qdl` block),
//! falling back to the bundled default registry when the API has no metadata
//! (offline, older API). The API is authoritative; the bundle keeps known
//! boards flashable without a network round-trip.

use crate::images::{self, ApiQdl};
use crate::qdl::boards;
use crate::qdl::loader::family_for_soc;
use crate::qdl::QdlStorage;

/// A board's QDL facts, resolved from the API or the bundled fallback. Paths are
/// family-relative; digests are present only when the API supplied them.
pub struct ResolvedQdl {
    pub family: String,
    pub storage: QdlStorage,
    pub loader_rel: Option<String>,
    pub provision_rel: Option<String>,
    pub loader_sha256: Option<String>,
    pub provision_sha256: Option<String>,
}

fn from_api(q: ApiQdl) -> Option<ResolvedQdl> {
    Some(ResolvedQdl {
        storage: QdlStorage::from_storage_str(&q.storage)?,
        family: q.family,
        loader_rel: q.loader_rel,
        provision_rel: q.provision_rel,
        loader_sha256: q.loader_sha256,
        provision_sha256: q.provision_sha256,
    })
}

fn bundled(board_slug: &str) -> Option<ResolvedQdl> {
    let b = boards::find(board_slug)?;
    Some(ResolvedQdl {
        family: family_for_soc(b.soc)?.to_string(),
        storage: b.storage,
        loader_rel: None,
        provision_rel: b.provision_rel.map(str::to_string),
        loader_sha256: None,
        provision_sha256: None,
    })
}

/// Resolve a board's QDL facts: the API `qdl` block first, bundled default as fallback.
pub async fn resolve(board_slug: &str) -> Option<ResolvedQdl> {
    if let Some(resolved) = images::fetch_board_qdl(board_slug).await.and_then(from_api) {
        return Some(resolved);
    }
    bundled(board_slug)
}

/// Verify `bytes` against an expected hex SHA-256 when the API provided one; a
/// missing digest (bundled/offline path) is accepted.
pub fn verify_digest(bytes: &[u8], expected: Option<&str>) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    use sha2::{Digest, Sha256};
    let got = hex::encode(Sha256::digest(bytes));
    if got.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!("SHA-256 mismatch (expected {expected}, got {got})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_parses_known_values() {
        assert_eq!(QdlStorage::from_storage_str("ufs"), Some(QdlStorage::Ufs));
        assert_eq!(QdlStorage::from_storage_str("emmc"), Some(QdlStorage::Emmc));
        assert_eq!(QdlStorage::from_storage_str("nvme"), None);
    }

    // Integration check against a local API: `ARMBIAN_API_BASE=http://localhost/api/v1
    // cargo test resolves_and_fetches_from_local_api -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "requires a running API at ARMBIAN_API_BASE"]
    async fn resolves_and_fetches_from_local_api() {
        let r = resolve("radxa-dragon-q6a").await.expect("q6a resolves");
        assert_eq!(r.family, "Kodiak");
        assert_eq!(r.storage, QdlStorage::Ufs);
        assert!(r.loader_sha256.is_some(), "API supplies a loader digest");

        let loader = crate::qdl::loader::ensure_loader(&r)
            .await
            .expect("loader downloads from the API blob proxy and its digest verifies");
        assert!(loader.exists());

        use crate::qdl::provision::ProvisionSource;
        match crate::qdl::provision::ensure_provision_xml(&r).await {
            ProvisionSource::Ready(p) => assert!(p.exists()),
            ProvisionSource::Absent => panic!("q6a should declare a provision descriptor"),
            ProvisionSource::Unavailable(e) => panic!("provision XML unavailable: {e}"),
        }
    }

    #[test]
    fn digest_matches_case_insensitively_and_skips_when_absent() {
        // sha256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(verify_digest(b"", Some(empty)).is_ok());
        assert!(verify_digest(b"", Some(&empty.to_uppercase())).is_ok());
        assert!(verify_digest(b"x", Some(empty)).is_err());
        assert!(verify_digest(b"anything", None).is_ok());
    }
}
