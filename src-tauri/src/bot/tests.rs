use super::*;

#[test]
fn honeypot_reports_each_failure_without_masking_other_failures() {
    for dm_delivered in [false, true] {
        for action_failed in [false, true] {
            for deletion_failed in [false, true] {
                let error = honeypot_outcome_error(dm_delivered, action_failed, deletion_failed);
                assert_eq!(
                    error.is_none(),
                    dm_delivered && !action_failed && !deletion_failed
                );
                let text = error.unwrap_or_default();
                assert_eq!(text.contains("DM could not"), !dm_delivered);
                assert_eq!(text.contains("Kick, ban or timeout failed"), action_failed);
                assert_eq!(text.contains("Message deletion failed"), deletion_failed);
            }
        }
    }
}

#[test]
fn honeypot_targets_only_the_configured_channel_with_english_notices() {
    let config = AppConfig {
        honeypot_channel_id: "123456789012345678".into(),
        honeypot_action: HoneypotAction::Ban,
        ..AppConfig::default()
    };

    assert_eq!(
        honeypot_action_for_channel(&config, "123456789012345678"),
        Some(HoneypotAction::Ban)
    );
    assert_eq!(
        honeypot_action_for_channel(&config, "223456789012345678"),
        None
    );
    use crate::moderation::messages::honeypot_notice;
    assert!(honeypot_notice(HoneypotAction::Kick, "en").contains("removed from the server"));
    assert!(honeypot_notice(HoneypotAction::Ban, "en").contains("banned from the server"));
    assert!(honeypot_notice(HoneypotAction::Timeout, "en").contains("muted"));
    assert!(honeypot_notice(HoneypotAction::Ban, "fr").contains("banni du serveur"));
    for notice in [
        honeypot_notice(HoneypotAction::Kick, "en"),
        honeypot_notice(HoneypotAction::Ban, "en"),
        honeypot_notice(HoneypotAction::Timeout, "en"),
    ] {
        assert!(notice.contains("token-stealing"));
        assert!(notice.contains("Change your Discord password"));
        assert!(notice.contains("two-factor authentication"));
    }
}

#[test]
fn deferred_embed_updates_preserve_admission_without_recounting() {
    use crate::moderation::Lane;
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut config = AppConfig::default();
    config.moderation.user_cooldown_seconds = 60;
    let mut message = Message::default();
    message.id = MessageId::new(123_456_789_012_345_678);
    message.author.id = UserId::new(223_456_789_012_345_678);
    assert!(apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
    assert!(apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));

    message.id = MessageId::new(123_456_789_012_345_679);
    assert!(!apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
    config.moderation.user_cooldown_seconds = 0;
    assert!(!apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));

    message.id = MessageId::new(123_456_789_012_345_678);
    config
        .moderation
        .blocked_user_ids
        .push(message.author.id.to_string());
    assert!(!apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
    message.id = MessageId::new(123_456_789_012_345_680);
    assert!(!apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
}

#[test]
fn deferred_embed_updates_preserve_review_holds() {
    use crate::moderation::Lane;
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let mut config = AppConfig::default();
    config.moderation.min_account_age_days = 365;
    let mut message = Message::default();
    message.id = MessageId::new(123_456_789_012_345_678);
    message.author.id = UserId::new((now_ms() - 1_420_070_400_000) << 22);
    assert!(apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
    let verdict = core.moderation_verdict(&message.id.to_string()).unwrap();
    assert_eq!(verdict.hold, Some("new_account"));
    assert!(apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
    assert_eq!(
        core.moderation_verdict(&message.id.to_string()).unwrap(),
        verdict
    );
}

#[test]
fn channel_commands_reject_foreign_guilds() {
    let invoking = GuildId::new(123_456_789_012_345_678);
    assert!(require_same_guild(invoking, invoking).is_ok());
    assert!(require_same_guild(GuildId::new(223_456_789_012_345_678), invoking).is_err());
}

#[test]
fn extracts_only_enabled_discord_guild_tags() {
    let tagged_user: User = serde_json::from_value(serde_json::json!({
        "id": "123456789012345678",
        "username": "inerthel",
        "primary_guild": {
            "identity_guild_id": "987654321098765432",
            "identity_enabled": true,
            "tag": "RE",
            "badge": "7d1734ae5a615e82bc7a4033b98fade8"
        }
    }))
    .unwrap();
    let tag = guild_tag_from_user(&tagged_user).unwrap();
    assert_eq!(tag.name, "RE");
    assert_eq!(
        tag.badge_url.as_deref(),
        Some(
            "https://cdn.discordapp.com/guild-tag-badges/987654321098765432/7d1734ae5a615e82bc7a4033b98fade8.png?size=1024"
        )
    );

    let hidden_user: User = serde_json::from_value(serde_json::json!({
        "id": "123456789012345678",
        "username": "inerthel",
        "primary_guild": {
            "identity_guild_id": "987654321098765432",
            "identity_enabled": false,
            "tag": "RE",
            "badge": "7d1734ae5a615e82bc7a4033b98fade8"
        }
    }))
    .unwrap();
    assert!(guild_tag_from_user(&hidden_user).is_none());
}

#[test]
fn formats_local_overlay_urls_without_secret() {
    let config = AppConfig {
        port: 5_321,
        ..AppConfig::default()
    };
    assert_eq!(overlay_url(&config), "http://localhost:5321/obs/visual");
    assert_eq!(
        crate::widget::obs_audio_url(config.port),
        "http://127.0.0.1:5321/obs/audio"
    );
    let details = connection_details(&config);
    assert!(details.contains("http://localhost:5321/obs/visual"));
    assert!(!details.contains("secret"));
}

#[test]
fn builds_invite_url_with_required_scopes_and_permissions() {
    assert_eq!(
        invite_url("123456789012345678", &AppConfig::default()),
        "https://discord.com/oauth2/authorize?client_id=123456789012345678&permissions=268528656&scope=bot%20applications.commands"
    );
}

#[test]
fn disables_commands_individually() {
    let config = AppConfig {
        command_clear_enabled: false,
        command_status_enabled: false,
        command_test_enabled: false,
        ..AppConfig::default()
    };
    assert!(!command_enabled(&config, "clear"));
    assert!(!command_enabled(&config, "status"));
    assert!(!command_enabled(&config, "test"));
    assert!(command_enabled(&config, "lock"));
    assert!(!command_enabled(&config, "unknown"));
}

#[test]
fn default_commands_still_require_an_administrator_inside_a_guild() {
    let guild_id = Some(GuildId::new(123_456_789_012_345_678));
    assert!(default_command_authorized(
        guild_id,
        Some(Permissions::ADMINISTRATOR)
    ));
    assert!(!default_command_authorized(
        guild_id,
        Some(Permissions::MANAGE_MESSAGES)
    ));
    assert!(!default_command_authorized(
        None,
        Some(Permissions::ADMINISTRATOR)
    ));
}

#[test]
fn formats_live_status_for_obs_and_windows_outputs() {
    let config = AppConfig {
        watched_channel_id: "123456789012345678".into(),
        former_message_channel_id: "223456789012345678".into(),
        moderation_enabled: true,
        widget_visible: true,
        widget_locked: true,
        ..AppConfig::default()
    };
    let bot = BotStatus {
        connected: true,
        username: Some("Relay".into()),
        ..BotStatus::default()
    };
    let mut server = ServerStatus {
        connected: true,
        ..ServerStatus::default()
    };
    server.outputs.visual.obs_clients = 1;
    server.outputs.visual.widget_clients = 1;
    server.outputs.notification.preview_clients = 1;

    let status = format_relay_status(&config, &bot, &server, 3);

    assert!(status.contains("Bot: connected as Relay"));
    // Two different former channels are shown until the user chooses one.
    assert!(status.contains(
        "Relay channel: choice pending in the Relay application (media <#123456789012345678>, messages <#223456789012345678>)"
    ));
    let unified = AppConfig {
        former_message_channel_id: String::new(),
        ..config.clone()
    };
    let unified_status = format_relay_status(&unified, &bot, &server, 3);
    assert!(unified_status.contains("Relay channel: <#123456789012345678>"));
    assert!(!unified_status.contains("Message channel"));
    assert!(status.contains("Moderation: enabled (3 pending)"));
    // No counter that nothing ever fills.
    assert!(!status.contains("Messages preparing"));
    assert!(status.contains("Media widget: visible and locked"));
    assert!(status.contains("Visual: 1 / 1 / 0"));
    assert!(status.contains("Notifications: 0 / 0 / 1"));
    assert!(!status.contains("secret"));
}

#[test]
fn clear_command_requires_a_bounded_message_count() {
    let command = serde_json::to_value(relay_command(&AppConfig::default())).unwrap();
    let clear = command["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|option| option["name"] == "clear")
        .unwrap();
    let channel = &clear["options"][0];
    assert_eq!(channel["name"], "channel");
    assert_eq!(channel["required"], true);
    let count = &clear["options"][1];
    assert_eq!(count["name"], "count");
    assert_eq!(count["required"], true);
    assert_eq!(count["min_value"], 1);
    assert_eq!(count["max_value"], 1_000);
}

#[test]
fn nuke_command_requires_a_text_or_announcement_channel() {
    let command = serde_json::to_value(relay_command(&AppConfig::default())).unwrap();
    let nuke = command["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|option| option["name"] == "nuke")
        .expect("nuke subcommand must be registered");
    let channel = &nuke["options"][0];
    assert_eq!(channel["name"], "channel");
    assert_eq!(channel["required"], true);
    assert_eq!(channel["channel_types"], serde_json::json!([0, 5]));
}

#[test]
fn nuke_replaces_configured_channel_references() {
    let mut config = AppConfig {
        watched_channel_id: "1".into(),
        former_message_channel_id: "1".into(),
        music_channel_id: "2".into(),
        honeypot_channel_id: "1".into(),
        channel_lock: Some(ChannelLockSnapshot {
            channel_id: "1".into(),
            overwrites: Vec::new(),
        }),
        ..AppConfig::default()
    };

    replace_configured_channel_id(&mut config, ChannelId::new(1), ChannelId::new(3));

    assert_eq!(config.watched_channel_id, "3");
    assert_eq!(config.former_message_channel_id, "3");
    assert_eq!(config.music_channel_id, "2");
    assert_eq!(config.honeypot_channel_id, "3");
    assert_eq!(config.channel_lock.unwrap().channel_id, "3");
}

#[test]
fn custom_commands_share_the_relay_schema_without_a_global_admin_gate() {
    let config = AppConfig {
        custom_commands: vec![custom_commands::CustomCommandDefinition {
            name: "rules".into(),
            description: "Show the configured rules".into(),
            action: custom_commands::CustomCommandAction::Reply {
                text: "Rules".into(),
                ephemeral: true,
            },
            ..custom_commands::CustomCommandDefinition::default()
        }],
        ..AppConfig::default()
    };
    let command = serde_json::to_value(relay_command(&config)).unwrap();
    assert!(command.get("default_member_permissions").is_none());
    assert!(
        command["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option["name"] == "rules")
    );
}

#[test]
fn test_command_exposes_every_local_output() {
    let command = serde_json::to_value(relay_command(&AppConfig::default())).unwrap();
    let test = command["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|option| option["name"] == "test")
        .unwrap();
    let output = &test["options"][0];
    assert_eq!(output["name"], "output");
    assert_eq!(output["required"], true);
    let values = output["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|choice| choice["value"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values, vec!["visual", "audio", "notification", "sticker"]);
}

#[tokio::test]
async fn discord_output_tests_require_a_live_output_and_bypass_history() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();

    let unavailable = relay_output_test(&core, OutputTestTarget::Visual)
        .await
        .unwrap();
    assert!(unavailable.contains("No live media output is connected"));

    {
        let mut server = core.server_status.write().await;
        server.connected = true;
        server.outputs.visual.obs_clients = 1;
    }
    let mut events = core.relay_tx.subscribe();
    let confirmation = relay_output_test(&core, OutputTestTarget::Visual)
        .await
        .unwrap();

    assert!(confirmation.contains("Local media test sent to 1 connected output"));
    assert!(confirmation.contains("Nothing was posted to Discord"));
    assert!(matches!(
        events.recv().await.unwrap(),
        crate::model::RelayEvent::TestOutput(_)
    ));
    assert!(core.history.read().await.is_empty());
}

#[test]
fn extracts_the_latest_release_section_from_the_changelog() {
    let changelog = "# Changelog\n\nIntro text.\n\n## [Unreleased]\n\n- Pending change.\n\n## [1.1.0] - 2026-07-12\n\n### Added\n\n- New feature.\n\n## [1.0.0] - 2026-07-12\n\n- First release.\n\n[Unreleased]: https://example.com/compare\n[1.1.0]: https://example.com/tag\n";
    let section = latest_changelog_section(changelog).unwrap();
    assert_eq!(section.version, "1.1.0");
    assert_eq!(section.date.as_deref(), Some("2026-07-12"));
    assert_eq!(section.heading, "## [1.1.0] - 2026-07-12");
    assert!(section.body.contains("New feature."));
    assert!(!section.body.contains("Pending change."));
    assert!(!section.body.contains("First release."));
    assert!(!section.body.contains("example.com"));
    assert!(
        latest_changelog_section("# Changelog\n\n## [Unreleased]\n\n- Only pending.\n").is_none()
    );
}

#[test]
fn formats_changelog_markdown_for_discord_embeds() {
    let body = "### English\n\n#### Added\n\n- New feature.\n\n### Français\n\n#### Ajouté\n\n- Nouvelle fonctionnalité.\n";
    let formatted = discord_format_changelog_body(body);
    assert!(formatted.contains("**English**"));
    assert!(formatted.contains("**Added**"));
    assert!(formatted.contains("• New feature."));
    assert!(formatted.contains("**Français**"));
    assert!(formatted.contains("• Nouvelle fonctionnalité."));
    assert!(!formatted.contains("####"));
}

#[test]
fn builds_changelog_embeds_with_version_title_and_github_link() {
    let section = ChangelogSection {
        heading: "## [1.2.6] - 2026-08-14".into(),
        version: "1.2.6".into(),
        date: Some("2026-08-14".into()),
        body: "### English\n\n#### Fixed\n\n- One fix.\n\n### Français\n\n#### Corrigé\n\n- Un correctif.\n".into(),
    };
    let (embeds, truncated) = build_changelog_embeds(&section);
    assert!(!truncated);
    assert_eq!(embeds.len(), 1);
}

#[test]
fn splits_long_changelog_sections_into_discord_sized_messages() {
    let long_line = "x".repeat(80);
    let text = (0..60)
        .map(|_| long_line.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let chunks = split_message_chunks(&text, 1_900);
    assert!(chunks.len() > 1);
    assert!(chunks.iter().all(|chunk| char_len(chunk) <= 1_900));
    assert_eq!(chunks.join("\n"), text);
}

#[test]
fn hard_splits_oversized_changelog_lines() {
    let line = "y".repeat(5_000);
    let chunks = split_message_chunks(&line, 1_000);
    assert!(chunks.len() > 1);
    assert!(chunks.iter().all(|chunk| char_len(chunk) <= 1_000));
    assert_eq!(chunks.concat(), line);
}

#[test]
fn snapshots_missing_permission_overwrites_for_exact_restoration() {
    let kind = PermissionOverwriteType::Role(serenity::all::RoleId::new(42));
    let saved = snapshot_permission(&[], kind).unwrap();
    assert_eq!(saved.target_kind, "role");
    assert_eq!(saved.target_id, "42");
    assert!(!saved.existed);
    assert_eq!(saved.allow, 0);
    assert_eq!(saved.deny, 0);
}

#[test]
fn bulk_deletes_only_messages_safely_inside_discords_two_week_limit() {
    let now = now_ms() / 1_000;
    let recent = MessageId::new(((now - 60) * 1_000 - 1_420_070_400_000) << 22);
    let old = MessageId::new(((now - 14 * 24 * 60 * 60) * 1_000 - 1_420_070_400_000) << 22);
    assert!(is_bulk_deletable(recent, now));
    assert!(!is_bulk_deletable(old, now));
}

#[test]
fn clear_skips_the_optional_protected_message_and_keeps_other_messages_eligible() {
    assert!(!clear_candidate(MessageId::new(9), Some(9)));
    assert!(clear_candidate(MessageId::new(8), Some(9)));
    assert!(clear_candidate(MessageId::new(9), None));
}

#[test]
fn classifies_supported_media_by_content_type_and_extension() {
    assert!(matches!(
        classify_media("still.png", None),
        Some(MediaKind::Image)
    ));
    assert!(matches!(
        classify_media("loop.gif", None),
        Some(MediaKind::Gif)
    ));
    assert!(matches!(
        classify_media("clip.bin", Some("video/mp4")),
        Some(MediaKind::Video)
    ));
    assert!(matches!(
        classify_media("track.opus", None),
        Some(MediaKind::Audio)
    ));
    assert!(classify_media("notes.txt", Some("text/plain")).is_none());
}

#[test]
fn extracts_discord_gifv_embeds_as_muted_video_gifs() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "gifv",
        "title": "Tenor animation",
        "url": "https://tenor.com/view/example",
        "video": {
            "url": "https://media.tenor.com/example.mp4",
            "proxy_url": "https://images-ext-1.discordapp.net/example.mp4"
        },
        "thumbnail": { "url": "https://media.tenor.com/example.gif" }
    }))
    .unwrap();
    let gif = embedded_gif(&embed).expect("Discord gifv should be relayed");
    assert_eq!(gif.url, "https://media.tenor.com/example.mp4");
    assert_eq!(gif.content_type, "video/mp4");
    assert_eq!(gif.title.as_deref(), Some("Tenor animation"));
}

#[test]
fn accepts_klipy_image_embeds_even_without_gifv_type() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "provider": { "name": "KLIPY", "url": "https://klipy.com" },
        "image": { "url": "https://cdn.klipy.com/animated.webp" }
    }))
    .unwrap();
    let gif = embedded_gif(&embed).expect("KLIPY image embed should be relayed");
    assert_eq!(gif.url, "https://cdn.klipy.com/animated.webp");
    assert_eq!(gif.content_type, "image/webp");
}

#[test]
fn treats_klipy_image_proxy_mp4_as_video_gif() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "title": "Klipy picker GIF",
        "provider": { "name": "KLIPY", "url": "https://klipy.com" },
        "image": {
            "url": "https://static.klipy.com/preview.jpg",
            "proxy_url": "https://images-ext-1.discordapp.net/external/example/clip.mp4"
        }
    }))
    .unwrap();
    let gif = embedded_gif(&embed).expect("KLIPY MP4 proxy should be relayed");
    assert_eq!(
        gif.url,
        "https://images-ext-1.discordapp.net/external/example/clip.mp4"
    );
    assert_eq!(gif.content_type, "video/mp4");
}

#[test]
fn accepts_discord_favorite_gifs_stored_as_thumbnail_only_images() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "url": "https://media.tenor.com/example/john-pork-is-calling.gif",
        "thumbnail": {
            "url": "https://media.tenor.com/example/john-pork-is-calling.gif",
            "proxy_url": "https://images-ext-1.discordapp.net/external/example/john-pork-is-calling.gif",
            "height": 387,
            "width": 220
        }
    }))
    .unwrap();

    let gif = embedded_gif(&embed).expect("thumbnail-only Discord favorite should be relayed");
    assert_eq!(
        gif.url,
        "https://media.tenor.com/example/john-pork-is-calling.gif"
    );
    assert_eq!(
        gif.proxy_url,
        "https://images-ext-1.discordapp.net/external/example/john-pork-is-calling.gif"
    );
    assert_eq!(gif.content_type, "image/gif");
}

#[test]
fn accepts_thumbnail_only_direct_gifs_without_a_known_provider() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "thumbnail": { "url": "https://example.com/animation.gif" }
    }))
    .unwrap();

    let gif = embedded_gif(&embed).expect("direct GIF thumbnails should be relayed");
    assert_eq!(gif.url, "https://example.com/animation.gif");
    assert_eq!(gif.content_type, "image/gif");
}

#[test]
fn accepts_url_only_tenor_gif_embeds() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "url": "https://media.tenor.com/kPLwmExuKxAAAAAi/halloing-hello.gif"
    }))
    .unwrap();
    let gif = embedded_gif(&embed).expect("URL-only GIF embed should be relayed");
    assert_eq!(
        gif.url,
        "https://media.tenor.com/kPLwmExuKxAAAAAi/halloing-hello.gif"
    );
}

#[test]
fn accepts_masked_direct_tenor_gif_links_without_embeds() {
    let url = "https://media.tenor.com/kPLwmExuKxAAAAAi/halloing-hello.gif";
    let content = format!("[https/media.tenor.com/halloing-hello.gif\u{a0}]({url})");
    assert_eq!(direct_gif_link(&content).unwrap().url, url);
    assert_eq!(message_gifs(&content, &[])[0].url, url);
    let preview: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "url": url,
        "image": { "url": "https://media.tenor.com/static-preview.jpg" }
    }))
    .unwrap();
    let gifs = message_gifs(&content, &[preview]);
    assert_eq!(gifs.len(), 1);
    assert_eq!(gifs[0].url, url);
    assert_eq!(gifs[0].content_type, "image/gif");
    assert!(direct_gif_link("https://evil.example/halloing-hello.gif").is_none());
}

#[test]
fn sniffs_mp4_bytes_when_discord_reports_an_image() {
    let bytes = b"\0\0\0\x18ftypisom\0\0\0\0isom";
    assert_eq!(sniff_media_type(bytes, "image/gif"), "video/mp4");
}

#[test]
fn prepares_plain_notification_messages_with_an_optional_unicode_limit() {
    assert_eq!(
        prepare_notification_text("  Bonjour Relay  ", 0).as_deref(),
        Some("Bonjour Relay")
    );
    assert_eq!(
        prepare_notification_text("\u{e9}l\u{e9}phant", 3).as_deref(),
        Some("\u{e9}l\u{e9}")
    );
    assert!(prepare_notification_text("   ", 0).is_none());
}

#[test]
fn prepares_bounded_media_text_without_standalone_links() {
    assert_eq!(
        prepare_media_text("Regardez mon setup https://example.com/image"),
        Some("Regardez mon setup".into())
    );
    assert_eq!(prepare_media_text("https://example.com/image"), None);
    assert_eq!(
        prepare_media_text("Une ligne\navec\tdu texte"),
        Some("Une ligne avec du texte".into())
    );

    let caption = prepare_media_text(&"é".repeat(MEDIA_TEXT_LIMIT + 1)).unwrap();
    assert_eq!(caption.chars().count(), MEDIA_TEXT_LIMIT);
    assert!(caption.ends_with('…'));
}

#[test]
fn automatic_privacy_filter_covers_sticker_and_attachment_names_without_scan() {
    let config = AppConfig {
        privacy_scan_enabled: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "hitler".into(),
            aliases: Vec::new(),
            regexes: Vec::new(),
        }],
        ..AppConfig::default()
    };
    let report = classify_privacy_values(
        "ordinary message",
        ["safe sticker"],
        ["hitler.png"],
        &config,
    );
    assert!(privacy_action_is_blocked(&report, &config));
    assert!(report.reasons.contains(&"forbidden_concept"));
}

#[test]
fn auto_deletes_blocked_privacy_and_filter_word_messages() {
    let mut config = AppConfig {
        privacy_scan_enabled: true,
        ..AppConfig::default()
    };
    let address = privacy::classify_text(Some("1 rue canot massy"), &config);
    assert!(should_auto_delete_blocked_message(&address, &config));

    config.privacy_scan_enabled = false;
    config.privacy_concepts = vec![privacy::ForbiddenConcept {
        canonical: "blockedterm".into(),
        aliases: Vec::new(),
        regexes: Vec::new(),
    }];
    let filter_word = privacy::classify_text(Some("blockedterm"), &config);
    assert!(
        filter_word
            .categories
            .contains(&privacy::PrivacyCategory::ContentFilter)
    );
    assert!(should_auto_delete_blocked_message(&filter_word, &config));
    assert!(!should_auto_delete_blocked_message(
        &privacy::PrivacyReport::sensitive("image_limits"),
        &config,
    ));

    config.privacy_auto_delete_blocked_messages = false;
    assert!(!should_auto_delete_blocked_message(&address, &config));
    assert!(!should_auto_delete_blocked_message(&filter_word, &config));
}

#[test]
fn converts_unicode_and_custom_emojis_to_visual_segments() {
    let segments =
        parse_visual_segments("Hello 👋 <:relay:123456789012345678> <a:dance:223456789012345678>")
            .expect("message contains emojis");
    assert_eq!(
        segments
            .iter()
            .filter(|segment| segment.kind == "emoji")
            .count(),
        3
    );
    assert!(segments.iter().any(|segment| segment.value == "👋"));
    assert!(segments.iter().any(|segment| {
        segment.value == ":dance:"
            && segment.animated
            && segment
                .url
                .as_deref()
                .is_some_and(|url| url.contains("223456789012345678"))
    }));
}

#[test]
fn wraps_plain_messages_in_a_single_text_segment() {
    let segments = plain_text_segments("test".into());
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].kind, "text");
    assert_eq!(segments[0].value, "test");
    assert!(segments[0].url.is_none());
    assert!(!segments[0].animated);
}

#[test]
fn plain_text_uses_the_standard_message_path() {
    assert!(parse_visual_segments("Relay reads this message").is_none());
    assert!(parse_visual_segments("invalid <:emoji:not-an-id>").is_none());
}

#[test]
fn maps_configured_bot_presence() {
    let config = AppConfig {
        bot_online_status: "idle".into(),
        bot_activity_type: "custom".into(),
        bot_activity_text: "Send your memes".into(),
        ..AppConfig::default()
    };
    let (activity, status) = presence_from_config(&config);
    assert_eq!(status, OnlineStatus::Idle);
    assert_eq!(activity.unwrap().state.as_deref(), Some("Send your memes"));

    let hidden = AppConfig {
        bot_online_status: "invisible".into(),
        bot_activity_type: "none".into(),
        ..AppConfig::default()
    };
    let (activity, status) = presence_from_config(&hidden);
    assert_eq!(status, OnlineStatus::Invisible);
    assert!(activity.is_none());
}

#[test]
fn maps_all_discord_sticker_formats() {
    assert_eq!(sticker_format(StickerFormatType::Png), ("png", "image/png"));
    assert_eq!(
        sticker_format(StickerFormatType::Apng),
        ("apng", "image/png")
    );
    assert_eq!(
        sticker_format(StickerFormatType::Lottie),
        ("lottie", "application/json")
    );
    assert_eq!(sticker_format(StickerFormatType::Gif), ("gif", "image/gif"));
}

#[test]
fn converts_renderable_stickers_to_visual_notification_segments() {
    let gif = sticker_visual_segment(
        "Relay dance".into(),
        Some("https://media.discordapp.net/stickers/1.gif".into()),
        StickerFormatType::Gif,
    );
    assert_eq!(
        (gif.kind.as_str(), gif.value.as_str()),
        ("sticker", "Relay dance")
    );
    assert!(gif.url.is_some() && gif.animated);

    let lottie = sticker_visual_segment(
        "Relay wave".into(),
        Some("https://cdn.discordapp.com/stickers/2.json".into()),
        StickerFormatType::Lottie,
    );
    assert!(lottie.url.is_none() && !lottie.animated);
}

const RELAY_CHANNEL: &str = "323456789012345678";
const FORMER_MESSAGE_CHANNEL: &str = "423456789012345678";

fn relay_config() -> AppConfig {
    AppConfig {
        watched_channel_id: RELAY_CHANNEL.into(),
        ..AppConfig::default()
    }
}

async fn relay_core(config: AppConfig) -> (tempfile::TempDir, Arc<AppCore>) {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    core.set_config(config).await.unwrap();
    (directory, core)
}

fn relay_message(id: u64, content: &str) -> Message {
    let mut message = Message::default();
    message.id = MessageId::new(id);
    message.channel_id = ChannelId::new(RELAY_CHANNEL.parse().unwrap());
    message.author.id = UserId::new(223_456_789_012_345_678);
    message.author.name = "viewer".into();
    message.content = content.into();
    message
}

/// The host is refused by the bounded downloader, so tests never use the network.
fn attachment(id: u64, filename: &str, content_type: &str) -> serenity::all::Attachment {
    serde_json::from_value(serde_json::json!({
        "id": id.to_string(),
        "filename": filename,
        "size": 1_024,
        "url": format!("https://relay.invalid/{filename}"),
        "proxy_url": format!("https://relay.invalid/proxy/{filename}"),
        "content_type": content_type,
    }))
    .unwrap()
}

/// Lottie stickers are never downloaded.
fn lottie_sticker(id: u64, name: &str) -> serenity::all::StickerItem {
    serde_json::from_value(serde_json::json!({
        "id": id.to_string(),
        "name": name,
        "format_type": 3,
    }))
    .unwrap()
}

#[derive(Debug, PartialEq)]
enum Output {
    Notification(String),
    /// Kind and the media card text.
    Media(&'static str, Option<String>),
    Sticker(String),
}

fn media_kind_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Gif => "gif",
        MediaKind::Video => "video",
        MediaKind::Audio => "audio",
    }
}

/// Routes one message like the gateway handler and returns what reached the
/// outputs, in playback order.
async fn relay_outputs(core: &Arc<AppCore>, message: &Message) -> Vec<Output> {
    let mut events = core.relay_tx.subscribe();
    let config = core.config.read().await.clone();
    route_relay_message(
        core,
        &Http::new(""),
        message,
        &config,
        &message_role_ids(message),
        false,
    )
    .await;
    let mut outputs = Vec::new();
    while let Ok(event) = events.try_recv() {
        outputs.push(match event {
            crate::model::RelayEvent::Notification(event) => Output::Notification(event.text),
            crate::model::RelayEvent::Media(event) => {
                Output::Media(media_kind_name(event.kind), event.text)
            }
            crate::model::RelayEvent::Sticker(event) => Output::Sticker(event.name),
            _ => continue,
        });
        core.stage_scheduler.skip_active().await;
    }
    outputs
}

#[tokio::test]
async fn relay_channel_text_becomes_one_notification() {
    let (_directory, core) = relay_core(relay_config()).await;
    assert_eq!(
        relay_outputs(
            &core,
            &relay_message(1_000_000_000_000_000_001, "Hello stream")
        )
        .await,
        vec![Output::Notification("Hello stream".into())]
    );
    let mut reply = relay_message(1_000_000_000_000_000_002, "Welcome back");
    reply.kind = MessageType::InlineReply;
    assert_eq!(
        relay_outputs(&core, &reply).await,
        vec![Output::Notification("Welcome back".into())]
    );
}

#[tokio::test]
async fn relay_channel_media_alone_never_creates_an_empty_notification() {
    let (_directory, core) = relay_core(relay_config()).await;
    for (index, (filename, content_type, kind)) in [
        ("cat.png", "image/png", "image"),
        ("dance.gif", "image/gif", "gif"),
        ("clip.webm", "video/webm", "video"),
        ("song.ogg", "audio/ogg", "audio"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut message = relay_message(1_000_000_000_000_000_010 + index as u64, "");
        message.attachments = vec![attachment(10 + index as u64, filename, content_type)];
        assert_eq!(
            relay_outputs(&core, &message).await,
            vec![Output::Media(kind, None)],
            "{filename}"
        );
    }
    let mut sticker = relay_message(1_000_000_000_000_000_020, "");
    sticker.sticker_items = vec![lottie_sticker(20, "wave")];
    assert_eq!(
        relay_outputs(&core, &sticker).await,
        vec![Output::Sticker("wave".into())]
    );
}

#[tokio::test]
async fn relay_channel_text_with_media_is_one_notification_then_each_media_once_in_order() {
    let (_directory, core) = relay_core(relay_config()).await;
    let mut message = relay_message(1_000_000_000_000_000_030, "Look at these");
    message.attachments = vec![
        attachment(31, "first.png", "image/png"),
        attachment(32, "notes.zip", "application/zip"),
        attachment(33, "second.webm", "video/webm"),
        attachment(34, "third.ogg", "audio/ogg"),
        attachment(35, "fourth.png", "image/png"),
    ];
    message.sticker_items = vec![lottie_sticker(36, "wave")];
    // Unsupported files are skipped and, as before, at most three media are relayed.
    // The notification carries the text once; media cards do not repeat it.
    assert_eq!(
        relay_outputs(&core, &message).await,
        vec![
            Output::Notification("Look at these".into()),
            Output::Sticker("wave".into()),
            Output::Media("image", None),
            Output::Media("video", None),
            Output::Media("audio", None),
        ]
    );
    let feeds = relay_config().channel_feeds(RELAY_CHANNEL).unwrap();
    let plan = RelayMessagePlan::new(&message, feeds);
    assert_eq!(
        plan.attachments
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>(),
        vec![0, 2, 3]
    );
}

#[tokio::test]
async fn relay_channel_ignores_unsupported_content_commands_and_system_messages() {
    let (_directory, core) = relay_core(relay_config()).await;
    let mut archive = relay_message(1_000_000_000_000_000_040, "");
    archive.attachments = vec![attachment(40, "notes.zip", "application/zip")];
    assert!(relay_outputs(&core, &archive).await.is_empty());
    assert!(core.pending_media.read().await.is_empty());
    archive.id = MessageId::new(1_000_000_000_000_000_041);
    archive.content = "My notes".into();
    assert_eq!(
        relay_outputs(&core, &archive).await,
        vec![Output::Notification("My notes".into())]
    );

    assert!(
        relay_outputs(
            &core,
            &relay_message(1_000_000_000_000_000_042, "/relay status")
        )
        .await
        .is_empty()
    );
    let mut command_with_media = relay_message(1_000_000_000_000_000_043, "/relay status");
    command_with_media.attachments = vec![attachment(43, "cat.png", "image/png")];
    assert_eq!(
        relay_outputs(&core, &command_with_media).await,
        vec![Output::Media("image", Some("/relay status".into()))]
    );
    let mut command = relay_message(1_000_000_000_000_000_044, "Relay used /relay status");
    command.kind = MessageType::ChatInputCommand;
    assert!(relay_outputs(&core, &command).await.is_empty());
    let mut renamed = relay_message(1_000_000_000_000_000_045, "welcome to the server");
    renamed.kind = MessageType::MemberJoin;
    assert!(relay_outputs(&core, &renamed).await.is_empty());
}

#[tokio::test]
async fn gif_links_are_relayed_as_media_never_as_notifications() {
    let (_directory, core) = relay_core(relay_config()).await;
    // The Tenor embed arrives later, through a message update.
    assert!(
        relay_outputs(
            &core,
            &relay_message(
                1_000_000_000_000_000_050,
                "https://tenor.com/view/dance-gif-123"
            )
        )
        .await
        .is_empty()
    );
    assert_eq!(
        relay_outputs(
            &core,
            &relay_message(
                1_000_000_000_000_000_051,
                "lol https://tenor.com/view/dance-gif-123"
            )
        )
        .await,
        vec![Output::Notification("lol".into())]
    );

    let text = |content: &str| notification_text(&relay_message(1, content));
    assert_eq!(text("https://media.tenor.com/abc/dance.gif"), None);
    assert_eq!(text("[dance](https://example.com/a.gif)"), None);
    assert_eq!(text("<https://giphy.com/gifs/abc>"), None);
    assert_eq!(
        text("look https://klipy.com/gifs/abc\nnext line"),
        Some("look\nnext line".into())
    );
    assert_eq!(
        text("read https://example.com/page"),
        Some("read https://example.com/page".into())
    );
    let gif_embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "gifv",
        "url": "https://example.com/party",
        "video": { "url": "https://example.com/party.mp4" }
    }))
    .unwrap();
    let mut embedded = relay_message(1, "https://example.com/party");
    embedded.embeds = vec![gif_embed];
    assert_eq!(notification_text(&embedded), None);
}

#[tokio::test]
async fn other_channels_never_reach_the_relay_flow() {
    let (_directory, core) = relay_core(AppConfig {
        music_channel_id: "523456789012345678".into(),
        honeypot_channel_id: "623456789012345678".into(),
        ..relay_config()
    })
    .await;
    for channel in [
        "523456789012345678",
        "623456789012345678",
        "723456789012345678",
    ] {
        let mut message = relay_message(1_000_000_000_000_000_060, "Hello");
        message.channel_id = ChannelId::new(channel.parse().unwrap());
        message.attachments = vec![attachment(60, "cat.png", "image/png")];
        assert!(relay_outputs(&core, &message).await.is_empty(), "{channel}");
    }
    assert!(core.pending_media.read().await.is_empty());
    assert!(core.pending_texts.read().await.is_empty());
}

#[tokio::test]
async fn a_mixed_message_is_checked_with_the_rules_of_its_text_and_its_media() {
    use crate::moderation::{FilterScopes, WordPack};
    for scopes in [
        FilterScopes {
            media: false,
            ..FilterScopes::default()
        },
        FilterScopes {
            notifications: false,
            ..FilterScopes::default()
        },
    ] {
        let mut config = AppConfig {
            privacy_auto_delete_blocked_messages: false,
            ..relay_config()
        };
        config.moderation.word_packs = vec![WordPack::Scams];
        config.moderation.filter_scopes = scopes;
        let (_directory, core) = relay_core(config).await;
        let mut message = relay_message(1_000_000_000_000_000_070, "FREE N1TRO for everyone");
        message.attachments = vec![attachment(70, "cat.png", "image/png")];
        // Adding a media never lets the text skip the notification filter, and
        // the text of a media message never skips the media filter.
        assert!(
            relay_outputs(&core, &message).await.is_empty(),
            "{scopes:?}"
        );
        assert!(core.pending_media.read().await.is_empty());
        assert!(core.pending_texts.read().await.is_empty());
    }
}

#[tokio::test]
async fn mixed_messages_keep_review_holds_manual_moderation_and_panic() {
    // A hold from the gate keeps both parts for review.
    let mut config = relay_config();
    config.moderation.min_account_age_days = 365;
    let (_directory, core) = relay_core(config).await;
    let mut message = relay_message(1_000_000_000_000_000_080, "New here");
    message.author.id = UserId::new((now_ms() - 1_420_070_400_000) << 22);
    message.attachments = vec![attachment(80, "cat.png", "image/png")];
    assert!(relay_outputs(&core, &message).await.is_empty());
    assert_eq!(core.pending_texts.read().await.len(), 1);
    assert_eq!(core.pending_media.read().await.len(), 1);

    // Manual moderation still applies to the media only, as before.
    let (_directory, core) = relay_core(AppConfig {
        moderation_enabled: true,
        ..relay_config()
    })
    .await;
    let mut message = relay_message(1_000_000_000_000_000_081, "Hello");
    message.attachments = vec![attachment(81, "cat.png", "image/png")];
    assert_eq!(
        relay_outputs(&core, &message).await,
        vec![Output::Notification("Hello".into())]
    );
    assert_eq!(core.pending_media.read().await.len(), 1);

    // Panic drops both parts instead of queueing them.
    let (_directory, core) = relay_core(relay_config()).await;
    core.set_outputs_paused(true).await;
    let mut message = relay_message(1_000_000_000_000_000_082, "Hello");
    message.attachments = vec![attachment(82, "cat.png", "image/png")];
    assert!(relay_outputs(&core, &message).await.is_empty());
    assert!(core.stage_scheduler.queue_snapshot().await.is_empty());
}

#[tokio::test]
async fn former_channels_keep_their_single_role_until_the_user_chooses() {
    let (_directory, core) = relay_core(AppConfig {
        former_message_channel_id: FORMER_MESSAGE_CHANNEL.into(),
        ..relay_config()
    })
    .await;
    let mut media = relay_message(1_000_000_000_000_000_090, "Nice");
    media.attachments = vec![attachment(90, "cat.png", "image/png")];
    assert_eq!(
        relay_outputs(&core, &media).await,
        vec![Output::Media("image", Some("Nice".into()))]
    );
    let mut text = relay_message(1_000_000_000_000_000_091, "Hello");
    text.channel_id = ChannelId::new(FORMER_MESSAGE_CHANNEL.parse().unwrap());
    text.attachments = vec![attachment(91, "cat.png", "image/png")];
    assert_eq!(
        relay_outputs(&core, &text).await,
        vec![Output::Notification("Hello".into())]
    );
}

#[test]
fn relay_messages_are_gated_on_every_lane_they_feed() {
    use crate::moderation::Lane;
    let feeds = relay_config().channel_feeds(RELAY_CHANNEL).unwrap();
    let lanes = |message: &Message| RelayMessagePlan::new(message, feeds).lanes;
    assert_eq!(lanes(&relay_message(1, "hello")), vec![Lane::Notifications]);
    assert_eq!(
        lanes(&relay_message(1, "hello https://example.com")),
        vec![Lane::Notifications, Lane::Media]
    );
    let mut image = relay_message(1, "");
    image.attachments = vec![attachment(1, "cat.png", "image/png")];
    assert_eq!(lanes(&image), vec![Lane::Media]);
    image.content = "caption".into();
    assert_eq!(lanes(&image), vec![Lane::Notifications, Lane::Media]);
    assert_eq!(lanes(&relay_message(1, "/relay status")), vec![Lane::Media]);

    let former_messages = crate::config::ChannelFeeds {
        notifications: true,
        media: false,
    };
    assert_eq!(
        RelayMessagePlan::new(&image, former_messages).lanes,
        vec![Lane::Notifications]
    );
    let former_media = crate::config::ChannelFeeds {
        notifications: false,
        media: true,
    };
    let plan = RelayMessagePlan::new(&image, former_media);
    assert_eq!(plan.lanes, vec![Lane::Media]);
    assert_eq!(plan.notification_text, None);
}

#[test]
fn only_public_gif_libraries_skip_the_animated_frame_review() {
    assert!(is_gif_library_url("https://media.tenor.com/abc/monkey.gif"));
    assert!(is_gif_library_url(
        "https://media1.giphy.com/media/abc/giphy.gif"
    ));
    assert!(is_gif_library_url("https://static.klipy.com/gifs/abc.gif"));
    assert!(!is_gif_library_url("http://media.tenor.com/abc/monkey.gif"));
    assert!(!is_gif_library_url(
        "https://cdn.discordapp.com/attachments/1/2/monkey.gif"
    ));
    assert!(!is_gif_library_url("https://eviltenor.com/monkey.gif"));
    assert!(!is_gif_library_url(
        "https://tenor.com.example.com/monkey.gif"
    ));
}

#[test]
fn server_staff_skip_the_spam_checks_like_trusted_members() {
    assert!(staff_member(true, Permissions::empty()));
    assert!(staff_member(false, Permissions::ADMINISTRATOR));
    assert!(staff_member(false, Permissions::MANAGE_GUILD));
    assert!(staff_member(
        false,
        Permissions::MANAGE_MESSAGES | Permissions::SEND_MESSAGES
    ));
    assert!(!staff_member(
        false,
        Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS
    ));
    // Without a server or member (DMs, fetched updates), nobody is staff.
    let cache = Cache::new();
    assert!(!is_server_staff(&cache, &relay_message(1, "hello")));
}

#[tokio::test]
async fn staff_gifs_are_not_dropped_by_the_member_cooldown() {
    let mut config = relay_config();
    config.moderation.user_cooldown_seconds = 10;
    let (_directory, core) = relay_core(config).await;
    let gif = |id: u64| {
        let mut message = relay_message(id, "");
        message.attachments = vec![attachment(id, "dance.gif", "image/gif")];
        message
    };
    let outputs = |message: Message, staff: bool| {
        let core = core.clone();
        async move {
            let mut events = core.relay_tx.subscribe();
            let config = core.config.read().await.clone();
            route_relay_message(&core, &Http::new(""), &message, &config, &[], staff).await;
            let mut count = 0;
            while let Ok(event) = events.try_recv() {
                if matches!(event, crate::model::RelayEvent::Media(_)) {
                    count += 1;
                    core.stage_scheduler.skip_active().await;
                }
            }
            count
        }
    };
    // A viewer's second GIF inside the cooldown is dropped, as configured.
    assert_eq!(outputs(gif(1_000_000_000_000_000_100), false).await, 1);
    assert_eq!(outputs(gif(1_000_000_000_000_000_101), false).await, 0);
    // A moderator's GIFs all play.
    let mut staff_gif = gif(1_000_000_000_000_000_102);
    staff_gif.author.id = UserId::new(323_456_789_012_345_679);
    assert_eq!(outputs(staff_gif, true).await, 1);
    let mut second = gif(1_000_000_000_000_000_103);
    second.author.id = UserId::new(323_456_789_012_345_679);
    assert_eq!(outputs(second, true).await, 1);
}
