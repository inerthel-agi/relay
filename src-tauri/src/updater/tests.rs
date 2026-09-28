use super::*;

const TEST_PUBLIC_KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
const TEST_SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";

fn release(url: &str) -> GitHubRelease {
    GitHubRelease {
        tag_name: "v1.1.23".into(),
        assets: vec![
            GitHubAsset {
                name: "Relay_1.1.23_x64-setup.exe".into(),
                browser_download_url: url.into(),
                size: 7_472_679,
                digest: Some(format!("sha256:{}", "a".repeat(64))),
            },
            GitHubAsset {
                name: "Relay_1.1.23_x64-setup.exe.sig".into(),
                browser_download_url: format!("{url}.sig"),
                size: 300,
                digest: None,
            },
        ],
    }
}

#[test]
fn compares_numeric_release_versions() {
    assert!(is_newer_version("1.1.24", "1.1.23").unwrap());
    assert!(is_newer_version("1.2.0", "1.1.99").unwrap());
    assert!(!is_newer_version("1.1.23", "1.1.23").unwrap());
    assert!(!is_newer_version("1.1.22", "1.1.23").unwrap());
    assert!(is_newer_version("1.1-beta", "1.1.23").is_err());
    assert_eq!(release_version("v1.3.6g").unwrap(), "1.3.6g");
    assert!(is_newer_version("1.3.6g", "1.3.6").unwrap());
    assert!(is_newer_version("1.3.6h", "1.3.6g").unwrap());
    assert!(is_newer_version("1.3.7", "1.3.6g").unwrap());
    assert!(!is_newer_version("1.3.6", "1.3.6g").unwrap());
    assert!(is_newer_version("1.3.6gg", "1.3.6g").is_err());
    assert_eq!(CURRENT_VERSION, "1.4.1");
}

#[test]
fn accepts_only_the_expected_official_installer_url() {
    let official = release(
        "https://github.com/inerthel-agi/relay/releases/download/v1.1.23/Relay_1.1.23_x64-setup.exe",
    );
    let asset = installer_asset(&official, "1.1.23").unwrap();
    assert!(validated_download_url(&official, asset, "1.1.23").is_ok());

    let foreign = release(
        "https://example.com/inerthel-agi/relay/releases/download/v1.1.23/Relay_1.1.23_x64-setup.exe",
    );
    let asset = installer_asset(&foreign, "1.1.23").unwrap();
    assert!(validated_download_url(&foreign, asset, "1.1.23").is_err());

    let mut lettered = release(
        "https://github.com/inerthel-agi/relay/releases/download/v1.3.6g/Relay_1.3.6g_x64-setup.exe",
    );
    lettered.tag_name = "v1.3.6g".into();
    lettered.assets[0].name = "Relay_1.3.6g_x64-setup.exe".into();
    lettered.assets[1].name = "Relay_1.3.6g_x64-setup.exe.sig".into();
    let asset = installer_asset(&lettered, "1.3.6g").unwrap();
    assert!(validated_download_url(&lettered, asset, "1.3.6g").is_ok());
}

#[test]
fn requires_a_sha256_digest_from_github() {
    let mut release = release(
        "https://github.com/inerthel-agi/relay/releases/download/v1.1.23/Relay_1.1.23_x64-setup.exe",
    );
    assert!(expected_digest(&release.assets[0]).is_ok());
    release.assets[0].digest = None;
    assert!(expected_digest(&release.assets[0]).is_err());
}

#[test]
fn requires_the_expected_official_signature_asset() {
    let mut release = release(
        "https://github.com/inerthel-agi/relay/releases/download/v1.1.23/Relay_1.1.23_x64-setup.exe",
    );
    let asset = signature_asset(&release, "1.1.23").unwrap();
    assert!(validated_asset_url(&release, asset, "Relay_1.1.23_x64-setup.exe.sig").is_ok());
    release.assets.pop();
    assert!(signature_asset(&release, "1.1.23").is_err());
}

#[test]
fn verifies_signed_payload_and_rejects_tampering_or_missing_signature() {
    let directory = tempfile::tempdir().unwrap();
    let payload = directory.path().join("update.bin");
    let public_key = BASE64_STANDARD.encode(TEST_PUBLIC_KEY);
    let signature = BASE64_STANDARD.encode(TEST_SIGNATURE);

    fs::write(&payload, b"test").unwrap();
    assert!(verify_file_signature_with_key(&payload, &signature, &public_key, "test").is_ok());

    fs::write(&payload, b"Test").unwrap();
    assert!(verify_file_signature_with_key(&payload, &signature, &public_key, "test").is_err());
    assert!(verify_file_signature_with_key(&payload, "", &public_key, "test").is_err());

    fs::write(&payload, b"test").unwrap();
    assert!(
        verify_file_signature_with_key(
            &payload,
            &signature,
            &public_key,
            "Relay_9.9.9_x64-setup.exe"
        )
        .is_err()
    );
}

#[test]
fn pinned_updater_public_key_is_valid() {
    let decoded = decode_tauri_minisign(UPDATER_PUBLIC_KEY, "updater public key").unwrap();
    assert!(PublicKey::decode(&decoded).is_ok());
}

#[test]
#[ignore = "requires RELAY_SIGNED_INSTALLER from a signed release build"]
fn signed_release_artifact_matches_pinned_key_and_rejects_tampering() {
    let installer = PathBuf::from(
        env::var_os("RELAY_SIGNED_INSTALLER").expect("RELAY_SIGNED_INSTALLER is required"),
    );
    let mut signature_path = installer.as_os_str().to_owned();
    signature_path.push(".sig");
    let signature = fs::read_to_string(PathBuf::from(signature_path)).unwrap();

    let expected_name = installer.file_name().unwrap().to_str().unwrap();
    verify_file_signature(&installer, &signature, expected_name).unwrap();

    let directory = tempfile::tempdir().unwrap();
    let tampered = directory.path().join("tampered-update.exe");
    fs::copy(&installer, &tampered).unwrap();
    OpenOptions::new()
        .append(true)
        .open(&tampered)
        .unwrap()
        .write_all(b"tampered")
        .unwrap();
    assert!(verify_file_signature(&tampered, &signature, expected_name).is_err());
}

#[cfg(target_os = "windows")]
#[test]
fn verified_installer_lock_allows_hashing_and_launch() {
    let directory = tempfile::tempdir().unwrap();
    let source = PathBuf::from(env::var_os("SystemRoot").unwrap())
        .join("System32")
        .join("where.exe");
    let executable = directory.path().join("Relay_update_test.exe");
    fs::copy(source, &executable).unwrap();

    let mut options = OpenOptions::new();
    options.read(true).share_mode(0x0000_0001);
    let _lock = options.open(&executable).unwrap();

    assert_eq!(file_sha256(&executable).unwrap().len(), 64);
    assert!(Command::new(&executable).arg("cmd.exe").output().is_ok());
}
