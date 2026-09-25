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
                assert_eq!(text.contains("Kick or ban failed"), action_failed);
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
    assert!(honeypot_notice(HoneypotAction::Kick).contains("kicked from the server"));
    assert!(honeypot_notice(HoneypotAction::Ban).contains("banned from the server"));
    for notice in [
        honeypot_notice(HoneypotAction::Kick),
        honeypot_notice(HoneypotAction::Ban),
    ] {
        assert!(notice.contains("token-grabbing"));
        assert!(notice.contains("Change your Discord password"));
        assert!(notice.contains("two-factor authentication"));
    }
}

#[test]
fn deferred_embed_updates_fetch_roles_before_using_the_partial_fallback() {
    let source = include_str!("../bot.rs");
    let handler = source
        .split("async fn message_update(")
        .nth(1)
        .and_then(|value| value.split("async fn interaction_create(").next())
        .expect("message update handler");
    let fetch = handler
        .find("get_message(event.channel_id, event.id)")
        .unwrap();
    let partial = handler.find("role_ids: Vec::new()").unwrap();
    assert!(fetch < partial);
}

#[test]
fn extracts_only_enabled_discord_guild_tags() {
    let tagged_user: User = serde_json::from_value(serde_json::json!({
        "id": "123456789012345678",
        "username": "Stealthy.",
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
        "username": "Stealthy.",
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
        audio_overlay_url(&config),
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
        "https://discord.com/oauth2/authorize?client_id=123456789012345678&permissions=268510224&scope=bot%20applications.commands"
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
        tts_channel_id: "223456789012345678".into(),
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

    let status = format_relay_status(&config, &bot, &server, 3, 2);

    assert!(status.contains("Bot: connected as Relay"));
    assert!(status.contains("Media channel: <#123456789012345678>"));
    assert!(status.contains("Moderation: enabled (3 pending)"));
    assert!(status.contains("Messages preparing: 2"));
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
        tts_channel_id: "1".into(),
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
    assert_eq!(config.tts_channel_id, "3");
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
fn prepares_plain_tts_messages_with_an_optional_unicode_limit() {
    assert_eq!(
        prepare_tts_text("  Bonjour Relay  ", 0).as_deref(),
        Some("Bonjour Relay")
    );
    assert_eq!(
        prepare_tts_text("\u{e9}l\u{e9}phant", 3).as_deref(),
        Some("\u{e9}l\u{e9}")
    );
    assert!(prepare_tts_text("   ", 0).is_none());
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
