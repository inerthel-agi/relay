use super::*;

#[tokio::test]
async fn relay_media_message_slots_bound_concurrent_ingestion() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let all_slots = core.media_ingest_slots.acquire_many(2).await.unwrap();
    assert!(core.media_ingest_slots.try_acquire().is_err());
    drop(all_slots);
    assert_eq!(core.media_ingest_slots.available_permits(), 2);
}

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
fn relay_commands_are_bound_to_the_relay_channel_server() {
    let watched = "123456789012345678";
    // Configured: every Relay-wide command must run in that channel's server.
    for subcommand in ["channel", "test", "status", "show", "url", "regenerate"] {
        assert_eq!(
            relay_command_scope(subcommand, watched),
            RelayCommandScope::RelayServer(watched)
        );
        // Not configured yet: an administrator of any other server running
        // the bot could otherwise pick the stream's source.
        assert_eq!(
            relay_command_scope(subcommand, ""),
            RelayCommandScope::Unanchored
        );
    }
    // These act on a channel of the invoking server only.
    for subcommand in ["clear", "nuke", "changelog", "lock"] {
        assert_eq!(
            relay_command_scope(subcommand, ""),
            RelayCommandScope::InvokingServer
        );
    }
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
fn extra_trap_and_music_channels_take_the_main_channel_role() {
    let config = AppConfig {
        music_channel_id: "123456789012345678".into(),
        extra_music_channel_ids: vec!["223456789012345678".into()],
        honeypot_channel_id: "323456789012345678".into(),
        extra_honeypot_channel_ids: vec!["423456789012345678".into()],
        honeypot_action: HoneypotAction::Timeout,
        ..AppConfig::default()
    };
    assert_eq!(
        honeypot_action_for_channel(&config, "423456789012345678"),
        Some(HoneypotAction::Timeout)
    );
    assert_eq!(
        honeypot_action_for_channel(&config, "223456789012345678"),
        None
    );
    let mut request = relay_message(1, "never gonna give you up");
    request.channel_id = ChannelId::new(223_456_789_012_345_678);
    assert_eq!(
        decision_log_lane(&config, &request),
        Some(crate::moderation::Lane::Music)
    );

    let mut nuked = config.clone();
    replace_configured_channel_id(&mut nuked, ChannelId::new(2), ChannelId::new(9));
    assert_eq!(nuked, config);
    replace_configured_channel_id(
        &mut nuked,
        ChannelId::new(423_456_789_012_345_678),
        ChannelId::new(5),
    );
    assert_eq!(nuked.extra_honeypot_channel_ids, ["5"]);
}

#[test]
fn nuke_replaces_an_extra_relay_channel_and_show_lists_it() {
    let mut config = AppConfig {
        watched_channel_id: "1".into(),
        extra_relay_channel_ids: vec!["4".into(), "5".into()],
        ..AppConfig::default()
    };
    replace_configured_channel_id(&mut config, ChannelId::new(5), ChannelId::new(6));
    assert_eq!(config.watched_channel_id, "1");
    assert_eq!(config.extra_relay_channel_ids, ["4", "6"]);
    assert_eq!(relay_channel_summary(&config), "<#1>, also <#4>, <#6>");
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
fn only_a_fresh_message_update_can_show_its_gif() {
    let now = now_ms() / 1_000;
    let sent =
        |seconds_ago: u64| MessageId::new(((now - seconds_ago) * 1_000 - 1_420_070_400_000) << 22);
    // Discord's GIF preview arrives a few seconds after the message.
    assert!(fresh_for_embeds(sent(5), now));
    assert!(fresh_for_embeds(sent(120), now));
    // An old message edited later does not bring its GIF back.
    assert!(!fresh_for_embeds(sent(121), now));
    assert!(!fresh_for_embeds(sent(3 * 60 * 60), now));
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
fn an_expired_gif_link_falls_back_to_the_urls_of_its_own_embed() {
    let link = "https://cdn.discordapp.com/attachments/1/2/dance.gif?ex=old&is=old&hm=old";
    let content = format!("look {link}");
    let own: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "url": "https://cdn.discordapp.com/attachments/1/2/dance.gif?ex=new&is=new&hm=new",
        "thumbnail": {
            "url": "https://cdn.discordapp.com/attachments/1/2/dance.gif?ex=new&is=new&hm=new",
            "proxy_url": "https://media.discordapp.net/attachments/1/2/dance.gif?ex=new"
        }
    }))
    .unwrap();
    let other: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "url": "https://media.tenor.com/other/other.gif",
    }))
    .unwrap();
    let embeds = [other, own];
    let gifs = message_gifs(&content, &embeds);
    assert_eq!(gifs.len(), 1);
    let fallbacks = link_embed_gifs(&gifs[0].url, &embeds);
    assert_eq!(fallbacks.len(), 1);
    assert_eq!(
        fallbacks[0].proxy_url,
        "https://media.discordapp.net/attachments/1/2/dance.gif?ex=new"
    );
    assert!(
        link_embed_gifs(
            "https://cdn.discordapp.com/attachments/1/2/other.gif",
            &embeds
        )
        .is_empty()
    );
    assert!(link_embed_gifs(link, &[]).is_empty());
}

#[test]
fn a_failed_gif_link_waits_for_its_embed_only_before_discord_sends_it() {
    let embed: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "image",
        "url": "https://media.tenor.com/a/dance.gif",
    }))
    .unwrap();
    // Message create: no embed yet, so the update may retry.
    assert!(wait_for_link_embed(true, &[]));
    // Embed update: every source failed, the GIF goes to review.
    assert!(!wait_for_link_embed(true, std::slice::from_ref(&embed)));
    // A GIF taken from an embed is never left for a later retry.
    assert!(!wait_for_link_embed(false, &[]));
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
async fn every_relay_server_feeds_the_same_outputs_through_the_same_filter() {
    const EXTRA_RELAY_CHANNEL: &str = "523456789012345678";
    let mut config = AppConfig {
        privacy_auto_delete_blocked_messages: false,
        extra_relay_channel_ids: vec![EXTRA_RELAY_CHANNEL.into()],
        ..relay_config()
    };
    config.moderation.word_packs = vec![crate::moderation::WordPack::Scams];
    let (_directory, core) = relay_core(config).await;
    let from = |channel: &str, id: u64, content: &str| {
        let mut message = relay_message(id, content);
        message.channel_id = ChannelId::new(channel.parse().unwrap());
        message
    };
    assert_eq!(
        relay_outputs(
            &core,
            &from(RELAY_CHANNEL, 1_000_000_000_000_000_090, "salut")
        )
        .await,
        vec![Output::Notification("salut".into())]
    );
    let mut image = from(EXTRA_RELAY_CHANNEL, 1_000_000_000_000_000_091, "");
    image.attachments = vec![attachment(91, "cat.png", "image/png")];
    assert_eq!(
        relay_outputs(&core, &image).await,
        vec![Output::Media("image", None)]
    );
    let scam = from(
        EXTRA_RELAY_CHANNEL,
        1_000_000_000_000_000_092,
        "FREE N1TRO for everyone",
    );
    assert!(relay_outputs(&core, &scam).await.is_empty());
    // A channel that is not configured stays ignored.
    let unknown = from("623456789012345678", 1_000_000_000_000_000_093, "ça va ?");
    assert!(relay_outputs(&core, &unknown).await.is_empty());
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

#[tokio::test]
async fn a_staff_image_that_could_not_be_scanned_plays_without_review() {
    let mut config = relay_config();
    config.privacy_scan_enabled = true;
    config.privacy_auto_delete_blocked_messages = false;
    config.moderation.trusted_role_ids = vec!["423456789012345678".into()];
    let (_directory, core) = relay_core(config).await;
    let image = |id: u64| {
        let mut message = relay_message(id, "");
        // Refused host: the download fails, so the scan ends in a review.
        message.attachments = vec![attachment(id, "photo.png", "image/png")];
        message
    };
    let relay = |message: Message, roles: Vec<String>, staff: bool| {
        let core = core.clone();
        async move {
            let config = core.config.read().await.clone();
            route_relay_message(&core, &Http::new(""), &message, &config, &roles, staff).await;
        }
    };
    // An ordinary member waits for review.
    relay(image(1_000_000_000_000_000_199), Vec::new(), false).await;
    assert_eq!(core.pending_media.read().await.len(), 1);
    assert!(core.history.read().await.is_empty());
    // A trusted-role member's image plays.
    relay(
        image(1_000_000_000_000_000_200),
        vec!["423456789012345678".into()],
        false,
    )
    .await;
    assert_eq!(core.pending_media.read().await.len(), 1);
    assert_eq!(core.history.read().await.len(), 1);
    // A moderator's image plays.
    relay(image(1_000_000_000_000_000_201), Vec::new(), true).await;
    assert_eq!(core.pending_media.read().await.len(), 1);
    assert_eq!(core.history.read().await.len(), 2);
}

#[tokio::test]
async fn the_final_media_check_keeps_the_media_filter_scope() {
    use crate::moderation::FilterScopes;
    let mut config = AppConfig {
        privacy_auto_delete_blocked_messages: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "blockedname".into(),
            aliases: Vec::new(),
            regexes: vec!["blockedname".into()],
        }],
        ..relay_config()
    };
    config.moderation.filter_scopes = FilterScopes {
        media: false,
        ..FilterScopes::default()
    };
    let (_directory, core) = relay_core(config).await;
    // No text: the message only feeds the media output, whose filter is off.
    let mut message = relay_message(1_000_000_000_000_000_091, "");
    message.attachments = vec![attachment(92, "blockedname.png", "image/png")];
    assert_eq!(
        relay_outputs(&core, &message).await,
        vec![Output::Media("image", None)]
    );
}

/// Personal-data relay on for GIFs and videos (the default scope), no cooldown.
fn ai_config(provider: Option<crate::ai::AiProvider>) -> AppConfig {
    let mut config = AppConfig {
        privacy_auto_delete_blocked_messages: false,
        ..relay_config()
    };
    config.moderation.user_cooldown_seconds = 0;
    config.ai.provider = provider;
    config.ai.personal_data_relay = true;
    config
}

#[tokio::test]
async fn gif_embeds_get_the_ai_image_check_unless_the_assistant_is_off() {
    let gif_message = |id: u64| {
        let mut message = relay_message(id, "");
        message.embeds = vec![
            serde_json::from_value(serde_json::json!({
                "type": "gifv",
                "url": "https://relay.invalid/dance.gif",
                "video": { "url": "https://relay.invalid/dance.mp4" }
            }))
            .unwrap(),
        ];
        message
    };
    // The test host cannot be downloaded, so the check cannot run: held.
    let (_directory, core) = relay_core(ai_config(Some(crate::ai::AiProvider::OpenAi))).await;
    let message = gif_message(1_000_000_000_000_000_200);
    relay_outputs(&core, &message).await;
    let verdict = core.moderation_verdict(&message.id.to_string()).unwrap();
    assert_eq!(verdict.hold, Some("ai_image_unchecked"));
    // Assistant off: nothing is checked, although the image options are on.
    let (_directory, core) = relay_core(ai_config(None)).await;
    let message = gif_message(1_000_000_000_000_000_201);
    relay_outputs(&core, &message).await;
    let verdict = core.moderation_verdict(&message.id.to_string()).unwrap();
    assert_eq!(verdict.hold, None);
}

#[tokio::test]
async fn library_gifs_are_not_sent_for_personal_data() {
    use crate::moderation::Lane;
    let (_directory, core) = relay_core(ai_config(Some(crate::ai::AiProvider::OpenAi))).await;
    let config = core.config.read().await.clone();
    // Unreadable bytes: a check that runs cannot extract frames and holds.
    for (id, url, hold) in [
        (
            1_000_000_000_000_000_210,
            "https://media.tenor.com/dance.gif",
            None,
        ),
        (
            1_000_000_000_000_000_211,
            "https://relay.invalid/dance.gif",
            Some("ai_image_unchecked"),
        ),
    ] {
        let message = relay_message(id, "");
        assert!(apply_moderation_gate(
            &core,
            &message,
            &[],
            false,
            &[Lane::Media],
            &config
        ));
        let mut report = privacy::PrivacyReport::safe();
        let candidate = (MediaKind::Gif, url, url, Some(&b"not a gif"[..]));
        assert!(
            !ai_image_check(
                &core,
                &Http::new(""),
                &message,
                false,
                candidate,
                &mut report
            )
            .await
        );
        let verdict = core.moderation_verdict(&message.id.to_string()).unwrap();
        assert_eq!(verdict.hold, hold, "{url}");
    }
}

#[tokio::test]
async fn a_sound_that_cannot_be_listened_to_waits_for_review() {
    use crate::moderation::Lane;
    let config = AppConfig {
        voice_scan_enabled: true,
        privacy_scan_enabled: true,
        ..relay_config()
    };
    let (_directory, core) = relay_core(config.clone()).await;
    let url = "https://relay.invalid/voice-message.ogg";
    for (id, kind, hold) in [
        // The voice model was never downloaded: nothing can listen to it.
        (
            1_000_000_000_000_000_220,
            MediaKind::Audio,
            Some("voice_unchecked"),
        ),
        (
            1_000_000_000_000_000_221,
            MediaKind::Video,
            Some("voice_unchecked"),
        ),
        // Images never wait for the voice check.
        (1_000_000_000_000_000_222, MediaKind::Image, None),
    ] {
        let message = relay_message(id, "");
        assert!(apply_moderation_gate(
            &core,
            &message,
            &[],
            false,
            &[Lane::Media],
            &config
        ));
        let mut report = privacy::PrivacyReport::safe();
        let candidate = (kind, url, url, Some(&b"OggS"[..]));
        assert!(
            !voice_check(
                &core,
                &Http::new(""),
                &message,
                candidate,
                &config,
                &mut report
            )
            .await
        );
        let verdict = core.moderation_verdict(&message.id.to_string()).unwrap();
        assert_eq!(verdict.hold, hold, "{kind:?}");
    }
}

#[tokio::test]
async fn the_ai_reads_a_members_transcript_and_can_only_hold_it() {
    use crate::moderation::Lane;
    let mut config = relay_config();
    config.ai.provider = Some(crate::ai::AiProvider::Anthropic);
    config.ai.watch_messages = true;
    let (_directory, core) = relay_core(config.clone()).await;
    core.ai_watch_stub.lock().unwrap().answer = Some(crate::ai::advice::AiAdvice::Done {
        verdict: crate::ai::advice::AiVerdict::ClearProblem,
        categories: vec![crate::ai::advice::AiCategory::PersonalData],
        reason: "Raison.".into(),
    });
    let message = relay_message(1_000_000_000_000_000_230, "");
    assert!(apply_moderation_gate(
        &core,
        &message,
        &[],
        false,
        &[Lane::Media],
        &config
    ));
    let transcript = "je serai devant le lycée Victor-Hugo à 17 heures";
    assert!(
        core.ai_holds_transcript(&message.id.to_string(), transcript, &config)
            .await
    );
    let sent = core.ai_watch_stub.lock().unwrap().sent.clone();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("lycée Victor-Hugo"));
    // Off, the transcript never leaves the PC.
    config.ai.watch_messages = false;
    assert!(
        !core
            .ai_holds_transcript(&message.id.to_string(), transcript, &config)
            .await
    );
    assert_eq!(core.ai_watch_stub.lock().unwrap().sent.len(), 1);
}

#[tokio::test]
async fn a_relay_message_never_waits_for_a_second_media_slot() {
    let (_directory, core) = relay_core(relay_config()).await;
    // Another message is being processed and holds one of the two slots.
    let _other_message = core.media_ingest_slots.acquire().await.unwrap();
    // An attachment: the message downloads media, so it takes the other slot.
    let mut message = relay_message(1_000_000_000_000_000_240, "salut");
    message.attachments = vec![attachment(240, "wave.webm", "video/webm")];
    let outputs = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        relay_outputs(&core, &message),
    )
    .await
    .expect("the message took a second slot and waited forever");
    assert_eq!(
        outputs,
        vec![
            Output::Notification("salut".into()),
            Output::Media("video", None)
        ]
    );
}

/// Audit finding 12: an MP4 that may need a transcode (up to two minutes) is
/// prepared without holding a shared media slot.
#[tokio::test]
async fn a_video_being_prepared_leaves_its_media_slot_free() {
    let (_directory, core) = relay_core(relay_config()).await;
    let (delivery, mut requests) = tokio::sync::mpsc::unbounded_channel();
    core.set_media_delivery(delivery).await;
    let mut message = relay_message(1_000_000_000_000_000_255, "");
    message.attachments = vec![
        attachment(255, "clip.mp4", "video/mp4"),
        attachment(256, "wave.webm", "video/webm"),
    ];
    let relay = tokio::spawn({
        let core = core.clone();
        async move { relay_outputs(&core, &message).await }
    });
    // The MP4 is past the transcode step, its slot already given back.
    let request = requests.recv().await.unwrap();
    assert_eq!(core.media_ingest_slots.available_permits(), 2);
    request.ready.send(()).unwrap();
    // The next attachment is handled with a slot again.
    let request = requests.recv().await.unwrap();
    assert_eq!(core.media_ingest_slots.available_permits(), 1);
    request.ready.send(()).unwrap();
    assert_eq!(
        relay.await.unwrap(),
        vec![Output::Media("video", None), Output::Media("video", None)]
    );
    assert_eq!(core.media_ingest_slots.available_permits(), 2);
}

#[tokio::test]
async fn a_text_message_never_waits_behind_media_checks() {
    let (_directory, core) = relay_core(relay_config()).await;
    // Two media messages are still being checked.
    let _busy = core.media_ingest_slots.acquire_many(2).await.unwrap();
    let message = relay_message(1_000_000_000_000_000_241, "salut");
    let outputs = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        relay_outputs(&core, &message),
    )
    .await
    .expect("a plain text waited for a media slot");
    assert_eq!(outputs, vec![Output::Notification("salut".into())]);
}

/// Privacy on, a content rule, and a word-filter scope that leaves `lane` out.
fn filter_scope_without(lane: crate::moderation::Lane) -> AppConfig {
    let mut config = AppConfig {
        privacy_scan_enabled: true,
        privacy_auto_delete_blocked_messages: false,
        former_message_channel_id: FORMER_MESSAGE_CHANNEL.into(),
        ..relay_config()
    };
    config.moderation.block_invites = true;
    match lane {
        crate::moderation::Lane::Media => config.moderation.filter_scopes.media = false,
        crate::moderation::Lane::Notifications => {
            config.moderation.filter_scopes.notifications = false;
        }
        crate::moderation::Lane::Music => config.moderation.filter_scopes.music = false,
    }
    config
}

#[tokio::test]
async fn a_gif_embed_is_not_held_when_the_word_filter_skips_media() {
    let (_directory, core) = relay_core(filter_scope_without(crate::moderation::Lane::Media)).await;
    let mut message = relay_message(1_000_000_000_000_000_250, "");
    message.embeds = vec![
        serde_json::from_value(serde_json::json!({
            "type": "gifv",
            "url": "https://relay.invalid/dance",
            "video": { "url": "https://relay.invalid/dance.webm" }
        }))
        .unwrap(),
    ];
    assert_eq!(
        relay_outputs(&core, &message).await,
        vec![Output::Media("gif", None)]
    );
    assert!(core.pending_media.read().await.is_empty());
}

/// Audit finding 5: with the media scope off and intermediate review off, the
/// image findings of a GIF must survive the second check.
#[tokio::test]
async fn a_gif_keeps_its_image_findings_when_the_word_filter_skips_media() {
    let config = AppConfig {
        privacy_review_intermediate: false,
        privacy_protection_level: crate::privacy::ProtectionLevel::Strict,
        ..filter_scope_without(crate::moderation::Lane::Media)
    };
    let scoped = crate::moderation::scope_config(config.clone(), crate::moderation::Lane::Media);
    // Text read from the image (OCR), not from the message.
    let image_report = privacy::classify_text(Some("appelle-moi au 06 12 34 56 78"), &scoped);
    assert_ne!(
        image_report.classification,
        privacy::PrivacyClassification::Safe
    );
    let rechecked = reclassify_privacy_report(image_report.clone(), "", &scoped);
    assert_eq!(rechecked.classification, image_report.classification);
    assert_eq!(rechecked.reasons, image_report.reasons);
    assert_eq!(rechecked.categories, image_report.categories);

    // Through the bot: the GIF image cannot be fetched, an image-level finding
    // that asks for review whatever the intermediate setting.
    let (_directory, core) = relay_core(config).await;
    let mut message = relay_message(1_000_000_000_000_000_254, "");
    message.embeds = vec![
        serde_json::from_value(serde_json::json!({
            "type": "image",
            "url": "https://relay.invalid/dance.gif",
            "image": { "url": "https://relay.invalid/dance.gif" }
        }))
        .unwrap(),
    ];
    assert!(relay_outputs(&core, &message).await.is_empty());
    assert_eq!(core.pending_media.read().await.len(), 1);
}

#[tokio::test]
async fn a_former_channel_sticker_is_not_held_when_the_word_filter_skips_text() {
    // Without the image scan, so only the word filter scope decides.
    let (_directory, core) = relay_core(AppConfig {
        privacy_scan_enabled: false,
        ..filter_scope_without(crate::moderation::Lane::Notifications)
    })
    .await;
    let mut message = relay_message(1_000_000_000_000_000_251, "Hello");
    message.channel_id = ChannelId::new(FORMER_MESSAGE_CHANNEL.parse().unwrap());
    message.sticker_items = vec![lottie_sticker(251, "wave")];
    assert_eq!(
        relay_outputs(&core, &message).await,
        vec![Output::Notification("Hello".into())]
    );
}

#[tokio::test]
async fn a_former_channel_staff_sticker_that_would_wait_is_shown() {
    // With the image scan on, a phone number in the sticker name asks for review.
    let (_directory, core) = relay_core(AppConfig {
        privacy_scan_enabled: true,
        privacy_auto_delete_blocked_messages: false,
        former_message_channel_id: FORMER_MESSAGE_CHANNEL.into(),
        ..relay_config()
    })
    .await;
    let notifications = |id: u64, staff: bool| {
        let core = core.clone();
        async move {
            let mut message = relay_message(id, "Hello");
            message.channel_id = ChannelId::new(FORMER_MESSAGE_CHANNEL.parse().unwrap());
            message.sticker_items = vec![lottie_sticker(id, "06 12 34 56 78")];
            let mut events = core.relay_tx.subscribe();
            let config = core.config.read().await.clone();
            route_relay_message(&core, &Http::new(""), &message, &config, &[], staff).await;
            let mut count = 0;
            while let Ok(event) = events.try_recv() {
                if matches!(event, crate::model::RelayEvent::Notification(_)) {
                    count += 1;
                    core.stage_scheduler.skip_active().await;
                }
            }
            count
        }
    };
    assert_eq!(notifications(1_000_000_000_000_000_252, false).await, 0);
    assert_eq!(notifications(1_000_000_000_000_000_253, true).await, 1);
}

#[test]
fn a_text_checked_again_or_named_twice_counts_once() {
    let config = AppConfig {
        privacy_scan_enabled: true,
        privacy_protection_level: crate::privacy::ProtectionLevel::Strict,
        ..AppConfig::default()
    };
    let text = "appelle-moi au 06 12 34 56 78";
    let report = privacy::classify_text(Some(text), &config);
    assert_eq!(
        report.classification,
        privacy::PrivacyClassification::Medium
    );
    let rechecked = reclassify_privacy_report(report.clone(), text, &config);
    assert_eq!(rechecked.score, report.score);
    assert_eq!(
        rechecked.classification,
        privacy::PrivacyClassification::Medium
    );
    // The file name and its stem are one finding.
    let named = classify_privacy_values("", [], ["06 12 34 56 78.png"], &config);
    assert_eq!(named.classification, privacy::PrivacyClassification::Medium);
    assert!(!should_auto_delete_blocked_message(&named, &config));
}

#[tokio::test]
async fn a_lottie_sticker_is_checked_on_its_name_instead_of_held() {
    let (_directory, core) = relay_core(AppConfig {
        privacy_scan_enabled: true,
        privacy_auto_delete_blocked_messages: false,
        ..relay_config()
    })
    .await;
    let mut message = relay_message(1_000_000_000_000_000_260, "");
    message.sticker_items = vec![lottie_sticker(260, "wave")];
    assert_eq!(
        relay_outputs(&core, &message).await,
        vec![Output::Sticker("wave".into())]
    );
    assert!(core.pending_media.read().await.is_empty());
    // A name that asks for review still waits.
    let mut held = relay_message(1_000_000_000_000_000_261, "");
    held.sticker_items = vec![lottie_sticker(261, "06 12 34 56 78")];
    assert!(relay_outputs(&core, &held).await.is_empty());
    assert_eq!(core.pending_media.read().await.len(), 1);
}

fn gifv_embed(url: &str) -> serenity::all::Embed {
    serde_json::from_value(serde_json::json!({
        "type": "gifv",
        "url": url,
        "video": { "url": format!("{url}.webm") }
    }))
    .unwrap()
}

#[tokio::test]
async fn a_gif_blocked_after_an_edit_is_logged_like_any_block() {
    let (_directory, core) = relay_core(AppConfig {
        privacy_auto_delete_blocked_messages: false,
        privacy_concepts: vec![privacy::ForbiddenConcept {
            canonical: "blockedname".into(),
            aliases: Vec::new(),
            regexes: vec!["blockedname".into()],
        }],
        ..relay_config()
    })
    .await;
    // The GIF embed arrives with a message update.
    let mut message = relay_message(1_000_000_000_000_000_270, "blockedname");
    message.embeds = vec![gifv_embed("https://relay.invalid/dance")];
    assert!(submit_embedded_gifs(&core, &Http::new(""), &message, false).await);
    let (entries, _) = core.moderation_entries();
    assert!(
        entries.iter().any(|entry| {
            entry.action == crate::moderation::log::LogAction::Blocked
                && entry.lane == Some(crate::moderation::Lane::Media)
                && entry.reason == "forbidden_regex"
                && entry.author_id.as_deref() == Some("223456789012345678")
        }),
        "{entries:?}"
    );
}

#[tokio::test]
async fn a_gif_dropped_for_an_unknown_member_age_is_logged_once() {
    let mut config = relay_config();
    config.moderation.min_member_age_days = 3;
    let (_directory, core) = relay_core(config).await;
    // Discord sent the update without the member's join date.
    let mut message = relay_message(1_000_000_000_000_000_280, "look");
    message.embeds = vec![gifv_embed("https://relay.invalid/dance")];
    for _ in 0..2 {
        assert!(!submit_embedded_gifs(&core, &Http::new(""), &message, false).await);
    }
    let (entries, _) = core.moderation_entries();
    let logged = entries
        .iter()
        .filter(|entry| {
            entry.action == crate::moderation::log::LogAction::Ignored
                && entry.lane == Some(crate::moderation::Lane::Media)
                && entry.reason == "member_age_unknown"
                && entry.author_id.as_deref() == Some("223456789012345678")
        })
        .count();
    assert_eq!(logged, 1, "{entries:?}");
}

#[tokio::test]
async fn a_failed_action_error_clears_when_the_same_action_succeeds() {
    let (_directory, core) = relay_core(relay_config()).await;
    let error = || async { core.bot_status.read().await.error.clone() };
    report_action_result(&core, true, PRIVACY_DELETION_FAILED).await;
    assert_eq!(error().await.as_deref(), Some(PRIVACY_DELETION_FAILED));
    // Another action's success says nothing about this one.
    report_action_result(&core, false, AI_IMAGE_DELETION_FAILED).await;
    assert_eq!(error().await.as_deref(), Some(PRIVACY_DELETION_FAILED));
    report_action_result(&core, false, PRIVACY_DELETION_FAILED).await;
    assert_eq!(error().await, None);
    // A warning that is still true stays.
    core.bot_status.write().await.error = Some(MISSING_CHANNEL_WARNING.into());
    report_action_result(&core, false, AUTOMATIC_TIMEOUT_FAILED).await;
    assert_eq!(error().await.as_deref(), Some(MISSING_CHANNEL_WARNING));
}

#[tokio::test]
async fn the_former_message_channel_shows_the_same_text_as_the_relay_channel() {
    let (_directory, core) = relay_core(AppConfig {
        former_message_channel_id: FORMER_MESSAGE_CHANNEL.into(),
        ..relay_config()
    })
    .await;
    let former = |id: u64, content: &str| {
        let mut message = relay_message(id, content);
        message.channel_id = ChannelId::new(FORMER_MESSAGE_CHANNEL.parse().unwrap());
        message
    };
    assert!(
        relay_outputs(&core, &former(1_000_000_000_000_000_271, "/relay status"))
            .await
            .is_empty()
    );
    assert!(
        relay_outputs(
            &core,
            &former(
                1_000_000_000_000_000_272,
                "https://tenor.com/view/dance-gif-123"
            )
        )
        .await
        .is_empty()
    );
    assert_eq!(
        relay_outputs(
            &core,
            &former(
                1_000_000_000_000_000_273,
                "lol https://tenor.com/view/dance-gif-123"
            )
        )
        .await,
        vec![Output::Notification("lol".into())]
    );
    let mut joined = former(1_000_000_000_000_000_274, "welcome to the server");
    joined.kind = MessageType::MemberJoin;
    assert!(relay_outputs(&core, &joined).await.is_empty());
}

#[tokio::test]
async fn a_link_preview_never_makes_a_new_text_a_duplicate() {
    use crate::moderation::Lane;
    let mut config = relay_config();
    config.moderation.block_duplicates = true;
    let (_directory, core) = relay_core(config.clone()).await;
    let lanes = [Lane::Notifications, Lane::Media];
    let link: serenity::all::Embed = serde_json::from_value(serde_json::json!({
        "type": "link",
        "url": "https://example.com/page",
        "title": "A page"
    }))
    .unwrap();
    let send = |id: u64, content: &str, embed: &serenity::all::Embed| {
        let mut message = relay_message(id, content);
        message.embeds = vec![embed.clone()];
        apply_moderation_gate(&core, &message, &[], false, &lanes, &config)
    };
    assert!(send(
        1_000_000_000_000_000_275,
        "read https://example.com/page",
        &link
    ));
    assert!(send(
        1_000_000_000_000_000_276,
        "and again https://example.com/page",
        &link
    ));
    // The same GIF twice is still a repeated media.
    let gif = gifv_embed("https://relay.invalid/dance");
    assert!(send(1_000_000_000_000_000_277, "", &gif));
    assert!(!send(1_000_000_000_000_000_278, "", &gif));
}

#[tokio::test]
async fn an_older_message_waiting_for_a_media_slot_keeps_its_place_on_stage() {
    let (_directory, core) = relay_core(relay_config()).await;
    let mut events = core.relay_tx.subscribe();
    let busy = core.media_ingest_slots.acquire_many(2).await.unwrap();
    let route = |message: Message| {
        let core = core.clone();
        async move {
            let config = core.config.read().await.clone();
            route_relay_message(&core, &Http::new(""), &message, &config, &[], false).await;
        }
    };
    let mut older = relay_message(1_000_000_000_000_000_279, "first");
    older.attachments = vec![attachment(279, "wave.webm", "video/webm")];
    let older = tokio::spawn(route(older));
    // The older message reserves its places, then waits for a media slot.
    for _ in 0..50 {
        if !core.stage_scheduler.queue_snapshot().await.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    route(relay_message(1_000_000_000_000_000_280, "second")).await;
    assert!(events.try_recv().is_err(), "a later text showed first");
    drop(busy);
    older.await.unwrap();
    let mut outputs = Vec::new();
    while let Ok(event) = events.try_recv() {
        outputs.push(match event {
            crate::model::RelayEvent::Notification(event) => Output::Notification(event.text),
            crate::model::RelayEvent::Media(event) => {
                Output::Media(media_kind_name(event.kind), event.text)
            }
            _ => continue,
        });
        core.stage_scheduler.skip_active().await;
    }
    assert_eq!(
        outputs,
        vec![
            Output::Notification("first".into()),
            Output::Media("video", None),
            Output::Notification("second".into()),
        ]
    );
}

/// A track chosen while Relay is paused stays with the member: once the pause
/// ends, the same buttons or custom range still start it.
#[tokio::test]
async fn a_selection_refused_during_a_pause_can_start_after_it() {
    let directory = tempfile::tempdir().unwrap();
    let core = AppCore::load(directory.path().join("config.json")).unwrap();
    let strings = music_locale(&core).await;
    let selection = MusicSelection {
        owner_id: 7,
        owner_name: "owner".into(),
        channel_id: 9,
        track: crate::youtube::YouTubeTrack {
            video_id: "aaaaaaaaaaa".into(),
            title: "Track".into(),
            channel_title: "Channel".into(),
            thumbnail: "https://relay.invalid/thumb.jpg".into(),
            duration_seconds: 180,
        },
    };
    core.music.lock().await.restore_selection("s1", selection);
    core.stage_scheduler.set_paused(true);
    assert_eq!(
        take_selection_to_start(&core, "s1", 7, &strings)
            .await
            .unwrap_err(),
        strings.relay_paused
    );
    core.stage_scheduler.set_paused(false);
    let taken = take_selection_to_start(&core, "s1", 7, &strings).await;
    assert_eq!(taken.unwrap().track.video_id, "aaaaaaaaaaa");
    assert_eq!(
        take_selection_to_start(&core, "s1", 7, &strings)
            .await
            .unwrap_err(),
        strings.selection_expired
    );
}
