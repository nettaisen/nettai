//! The newest release as GitHub's API says it (`releases/latest`: the newest published, not a prerelease), read for
//! Android's update (`platform/android.rs`): its APK, if the release is newer than the installed app.

use serde::Deserialize;

/// The APK each release carries, signed with the release key (packaging/android).
const APK: &str = "nettai-android.apk";

/// A newer release's APK.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The release's version (its tag's, without the `v`).
    pub version: String,
    /// Where the APK is downloaded from.
    pub url: String,
    /// The APK's size, in bytes.
    pub size: u64,
    /// The APK's SHA-256, as GitHub says it (in hexadecimal, lowercase).
    pub sha256: String,
}

impl Found {
    #[cfg_attr(not(android), allow(dead_code))]
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    /// `sha256:` and the file's SHA-256 in hexadecimal (GitHub's, on every release asset since 2025).
    digest: Option<String>,
}

/// The release's APK, if the release (GitHub's JSON for it) is newer than `installed`, the installed app's version; a
/// release that has none, or doesn't say its SHA-256, is an error.
pub fn newer(json: &[u8], installed: &str) -> Result<Option<Found>, String> {
    let release: Release = serde_json::from_slice(json).map_err(|e| format!("the release's JSON: {e}"))?;
    let tag = release.tag_name;
    let version = tag.strip_prefix('v').unwrap_or(&tag);
    let (Some(out), Some(ours)) = (numbers(version), numbers(installed)) else {
        return Err(format!("the release's version {version} and the app's {installed} don't compare"));
    };
    if out <= ours {
        return Ok(None);
    }
    let apk = release.assets.into_iter().find(|a| a.name == APK).ok_or_else(|| format!("{tag} has no {APK}"))?;
    let sha256 = apk.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"));
    let sha256 = sha256.ok_or_else(|| format!("{tag}'s {APK} has no SHA-256"))?.to_ascii_lowercase();
    Ok(Some(Found { version: version.to_owned(), url: apk.browser_download_url, size: apk.size, sha256 }))
}

/// A version's numbers: major, minor, patch.
fn numbers(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.').map(|part| part.parse().ok());
    let numbers = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(numbers)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GitHub's JSON for a release of `tag` with these assets (name and digest), as much of it as is read.
    fn release(tag: &str, assets: &[(&str, Option<&str>)]) -> Vec<u8> {
        let assets: Vec<_> = assets
            .iter()
            .map(|(name, digest)| {
                serde_json::json!({
                    "name": name,
                    "size": 97_122_818,
                    "digest": digest,
                    "browser_download_url": format!("https://github.com/nettaisen/nettai/releases/download/{tag}/{name}"),
                })
            })
            .collect();
        serde_json::json!({ "tag_name": tag, "draft": false, "prerelease": false, "assets": assets }).to_string().into_bytes()
    }

    const SUM: &str = "5b06cc304ef839b194a770ccaa01ae4cbc49ec50c9883a8276d498ef6bc8fc1e";

    fn with_apk(tag: &str) -> Vec<u8> {
        let digest = format!("sha256:{}", SUM.to_ascii_uppercase());
        release(tag, &[("nettai-ios.ipa", Some("sha256:00")), (APK, Some(&digest)), ("altstore.json", None)])
    }

    #[test]
    fn a_newer_release_gives_its_apk() {
        let found = newer(&with_apk("v0.21.0"), "0.20.0").unwrap();
        assert_eq!(
            found,
            Some(Found {
                version: "0.21.0".into(),
                url: "https://github.com/nettaisen/nettai/releases/download/v0.21.0/nettai-android.apk".into(),
                size: 97_122_818,
                sha256: SUM.into(),
            })
        );
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(newer(&with_apk("v0.10.0"), "0.9.3").unwrap().is_some());
        assert!(newer(&with_apk("v1.0.0"), "0.20.4").unwrap().is_some());
        assert!(newer(&with_apk("v0.20.1"), "0.20.0").unwrap().is_some());
        assert_eq!(newer(&with_apk("v0.9.3"), "0.10.0").unwrap(), None);
    }

    #[test]
    fn the_same_or_an_older_release_is_no_update() {
        assert_eq!(newer(&with_apk("v0.20.0"), "0.20.0").unwrap(), None);
        assert_eq!(newer(&with_apk("v0.19.3"), "0.20.0").unwrap(), None);
    }

    #[test]
    fn a_newer_release_without_its_apk_or_its_sum_is_an_error() {
        assert!(newer(&release("v0.21.0", &[("nettai-ios.ipa", Some("sha256:00"))]), "0.20.0").is_err());
        assert!(newer(&release("v0.21.0", &[(APK, None)]), "0.20.0").is_err());
    }

    #[test]
    fn a_version_that_isnt_three_numbers_doesnt_compare() {
        assert!(newer(&with_apk("v0.21"), "0.20.0").is_err());
        assert!(newer(&with_apk("v0.21.0-rc1"), "0.20.0").is_err());
    }
}
