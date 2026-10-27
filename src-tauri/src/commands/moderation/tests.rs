// No test here reaches Discord: without a running bot only remembered names are used.
use super::*;
use crate::moderation::log::LogAction;

fn entry(author: &str) -> LogEntry {
    LogEntry {
        at: 1,
        lane: None,
        action: LogAction::Blocked,
        reason: "duplicate".into(),
        author_id: Some(author.into()),
    }
}

#[test]
fn the_display_name_comes_before_the_username() {
    let mut user = User::default();
    user.name = "alt_acc".into();
    assert_eq!(display_name(&user), "alt_acc");
    user.global_name = Some("Alt".into());
    assert_eq!(display_name(&user), "Alt");
}

#[tokio::test]
async fn only_names_relay_could_find_are_shown() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    {
        let mut names = core.author_names.lock().unwrap();
        names.insert("42".into(), Some("Alt".into()));
        names.insert("7".into(), None);
    }
    let names = author_names(&core, &[entry("42"), entry("7"), entry("9"), entry("42")]).await;
    assert_eq!(names, HashMap::from([("42".to_owned(), "Alt".to_owned())]));
}
