use std::{fs, sync::Arc};

use tempfile::tempdir;

use super::{MEDIA_LIBRARY_MAX_IMAGE_BYTES, MediaLibrary, MediaLibraryError, is_valid_id};
use crate::model::MediaKind;

fn png_bytes() -> Vec<u8> {
    b"\x89PNG\r\n\x1a\nrelay-test".to_vec()
}

fn gif_bytes() -> Vec<u8> {
    b"GIF89arelay-test".to_vec()
}

#[tokio::test]
async fn imports_are_owned_and_survive_reopen() {
    let directory = tempdir().expect("temporary directory");
    let library = Arc::new(MediaLibrary::open(directory.path()).expect("open library"));
    let item = library
        .import_bytes(
            "original.png".into(),
            MediaKind::Image,
            "image/png".into(),
            png_bytes(),
            None,
            None,
        )
        .await
        .expect("import media");

    assert!(is_valid_id(&item.id));
    assert_eq!(item.extension, "png");
    assert_eq!(item.size_bytes, png_bytes().len() as u64);
    assert_eq!(library.usage().expect("usage").item_count, 1);
    assert_eq!(
        library.read_asset(&item.id).expect("asset").bytes,
        png_bytes()
    );

    let reopened = MediaLibrary::open(directory.path()).expect("reopen library");
    assert_eq!(reopened.list().expect("list")[0].id, item.id);
    assert_eq!(
        reopened.read_asset(&item.id).expect("asset").bytes,
        png_bytes()
    );
}

#[tokio::test]
async fn inferred_format_is_sniffed_and_name_is_safe() {
    let directory = tempdir().expect("temporary directory");
    let library = Arc::new(MediaLibrary::open(directory.path()).expect("open library"));
    let item = library
        .import_bytes(
            "folder\\name.gif".into(),
            MediaKind::Gif,
            "image/gif".into(),
            gif_bytes(),
            None,
            None,
        )
        .await
        .expect("import GIF");

    assert!(matches!(item.kind, MediaKind::Gif));
    assert_eq!(item.name, "name.gif");
    assert_eq!(item.content_type, "image/gif");
}

#[tokio::test]
async fn rejects_unsupported_or_invalid_media_without_manifest_entry() {
    let directory = tempdir().expect("temporary directory");
    let library = Arc::new(MediaLibrary::open(directory.path()).expect("open library"));

    let unsupported = library
        .import_bytes(
            "sound.mp3".into(),
            MediaKind::Audio,
            "audio/mpeg".into(),
            vec![1, 2, 3],
            None,
            None,
        )
        .await;
    assert!(matches!(
        unsupported,
        Err(MediaLibraryError::UnsupportedMedia)
    ));

    let invalid = library
        .import_bytes(
            "image.png".into(),
            MediaKind::Image,
            "image/png".into(),
            b"not-an-image".to_vec(),
            None,
            None,
        )
        .await;
    assert!(matches!(invalid, Err(MediaLibraryError::InvalidMedia)));
    assert!(library.list().expect("list").is_empty());
    assert!(!directory.path().join("index.json").exists());
}

#[tokio::test]
async fn enforces_visual_size_limit() {
    let directory = tempdir().expect("temporary directory");
    let library = Arc::new(MediaLibrary::open(directory.path()).expect("open library"));
    let too_large = vec![0_u8; MEDIA_LIBRARY_MAX_IMAGE_BYTES + 1];
    let result = library
        .import_bytes(
            "large.png".into(),
            MediaKind::Image,
            "image/png".into(),
            too_large,
            None,
            None,
        )
        .await;
    assert!(matches!(result, Err(MediaLibraryError::TooLarge { .. })));
    assert!(library.list().expect("list").is_empty());
}

#[tokio::test]
async fn rename_preserves_asset_and_delete_removes_only_managed_copy() {
    let directory = tempdir().expect("temporary directory");
    let library = Arc::new(MediaLibrary::open(directory.path()).expect("open library"));
    let item = library
        .import_bytes(
            "before.png".into(),
            MediaKind::Image,
            "image/png".into(),
            png_bytes(),
            None,
            None,
        )
        .await
        .expect("import media");
    let renamed = library.rename(&item.id, "after.png").expect("rename");
    assert_eq!(renamed.name, "after.png");
    assert_eq!(
        library.read_asset(&item.id).expect("asset").bytes,
        png_bytes()
    );

    let outside = directory.path().join("outside.txt");
    fs::write(&outside, b"keep me").expect("outside fixture");
    library.delete(&item.id).expect("delete");
    assert!(library.get(&item.id).is_err());
    assert_eq!(fs::read(&outside).expect("outside file"), b"keep me");
    assert!(library.usage().expect("usage").bytes == 0);
}

#[test]
fn corrupt_index_keeps_the_app_startable_without_overwriting_data() {
    let directory = tempdir().expect("temporary directory");
    let index = directory.path().join("index.json");
    fs::write(&index, b"not-json").expect("corrupt index fixture");

    let library = MediaLibrary::open_or_unavailable(directory.path());
    assert!(matches!(
        library.list(),
        Err(MediaLibraryError::CorruptManifest)
    ));
    assert_eq!(fs::read(&index).expect("index"), b"not-json");
}

#[tokio::test]
async fn corrupted_asset_is_reported_without_an_unbounded_read() {
    let directory = tempdir().expect("temporary directory");
    let library = Arc::new(MediaLibrary::open(directory.path()).expect("open library"));
    let item = library
        .import_bytes(
            "clip.png".into(),
            MediaKind::Image,
            "image/png".into(),
            png_bytes(),
            None,
            None,
        )
        .await
        .expect("import media");
    let asset_path = directory
        .path()
        .join("assets")
        .join(format!("{}.png", item.id));
    fs::write(&asset_path, vec![0_u8; MEDIA_LIBRARY_MAX_IMAGE_BYTES + 1])
        .expect("oversized asset fixture");
    assert!(matches!(
        library.read_asset(&item.id),
        Err(MediaLibraryError::AssetUnavailable)
    ));
}

#[test]
fn rejects_invalid_ids_before_touching_disk() {
    let directory = tempdir().expect("temporary directory");
    let library = MediaLibrary::open(directory.path()).expect("open library");
    assert!(matches!(
        library.read_asset("../index.json"),
        Err(MediaLibraryError::InvalidId)
    ));
    assert!(matches!(
        library.delete("not-an-id"),
        Err(MediaLibraryError::InvalidId)
    ));
}
