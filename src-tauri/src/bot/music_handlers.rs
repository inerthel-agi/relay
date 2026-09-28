use super::*;

const MUSIC_MOVE_UP_PREFIX: &str = "relay:music:up:";
const MUSIC_MOVE_DOWN_PREFIX: &str = "relay:music:down:";
const MUSIC_REMOVE_PREFIX: &str = "relay:music:remove:";

pub(super) async fn music_notice(
    core: &Arc<AppCore>,
    http: &Arc<Http>,
    channel: ChannelId,
    text: &str,
) {
    if let Ok(reply) = channel.say(http, text).await {
        crate::music_cleanup::expire_message(core, http, channel.get(), reply.id.get()).await;
    }
}

pub(super) async fn handle_music_message(
    core: &Arc<AppCore>,
    http: &Arc<Http>,
    message: &Message,
    scoped_config: &AppConfig,
) {
    let query = message.content.trim();
    if query.is_empty() || query.chars().any(char::is_control) {
        return;
    }
    let text_report = classify_message_privacy(message, scoped_config);
    if block_and_delete_message_if_needed(core, http, message, &text_report, scoped_config).await {
        return;
    }
    if privacy::privacy_rules_enabled(scoped_config)
        && matches!(
            privacy::action_for(&text_report, scoped_config),
            privacy::PrivacyAction::Review
        )
    {
        return;
    }

    let strings = music_locale(core).await;
    let api_key = match load_youtube_api_key() {
        Ok(Some(api_key)) => api_key,
        Ok(None) => {
            music_notice(core, http, message.channel_id, strings.not_configured).await;
            return;
        }
        Err(_) => {
            core.bot_status.write().await.error =
                Some("Unable to read the saved YouTube API key.".into());
            music_notice(core, http, message.channel_id, strings.search_unavailable).await;
            return;
        }
    };

    let user_id = message.author.id.get();
    {
        let mut music = core.music.lock().await;
        let now = Instant::now();
        if let Some(remaining) = music.search_cooldown_remaining(user_id, now) {
            let seconds = cooldown_wait_seconds(remaining).to_string();
            let reply = music_i18n::fill(strings.search_cooldown, &[("seconds", &seconds)]);
            music_notice(core, http, message.channel_id, &reply).await;
            return;
        }
        // Mark before the API call so failed/retried spam still burns the cooldown.
        music.mark_search_attempt(user_id, now);
    }

    let mut results = match youtube::search(query, &api_key).await {
        Ok(results) => results,
        Err(error) => {
            let detail = error.to_string();
            core.bot_status.write().await.error = Some(detail.clone());
            let reply = if detail.to_ascii_lowercase().contains("quota") {
                "YouTube API quota exceeded for today. Try again after the daily reset, or raise the quota in Google Cloud."
            } else {
                strings.search_unavailable
            };
            music_notice(core, http, message.channel_id, reply).await;
            return;
        }
    };
    // Titles and channel names end up on the music card: keep only clean results.
    if scoped_config.moderation.filter_music_titles {
        results.retain(|track| {
            !privacy::filter_words_match(
                &format!(
                    "{}
{}",
                    track.title, track.channel_title
                ),
                scoped_config,
            )
        });
    }
    if results.is_empty() {
        music_notice(core, http, message.channel_id, strings.no_results).await;
        return;
    }

    let query = query.chars().take(200).collect::<String>();
    let search_id = core.music.lock().await.insert_search(
        message.author.id.get(),
        message.channel_id.get(),
        query.clone(),
        results.clone(),
    );
    let mut embed = CreateEmbed::new()
        .title(strings.results_title)
        .description(music_i18n::fill(
            strings.choose_title,
            &[("query", &truncate_text(&query, 180))],
        ));
    let mut options = Vec::with_capacity(results.len());
    for (index, track) in results.iter().enumerate() {
        embed = embed.field(
            format!("{}. {}", index + 1, truncate_text(&track.title, 180)),
            format!(
                "{} · {}",
                truncate_text(&track.channel_title, 80),
                format_duration(track.duration_seconds)
            ),
            false,
        );
        options.push(
            CreateSelectMenuOption::new(truncate_text(&track.title, 100), track.video_id.clone())
                .description(format!(
                    "{} · {}",
                    truncate_text(&track.channel_title, 80),
                    format_duration(track.duration_seconds)
                )),
        );
    }
    let message_builder = CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{MUSIC_SEARCH_PREFIX}{search_id}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(strings.choose_placeholder),
        )])
        .allowed_mentions(CreateAllowedMentions::new());
    match message.channel_id.send_message(http, message_builder).await {
        Ok(reply) => {
            crate::music_cleanup::expire_message(core, http, reply.channel_id.get(), reply.id.get())
                .await
        }
        Err(_) => {
            core.bot_status.write().await.error =
                Some("Discord rejected the YouTube search results.".into())
        }
    }
}

pub(super) async fn handle_music_component(
    core: &Arc<AppCore>,
    context: &Context,
    component: &ComponentInteraction,
) {
    let strings = music_locale(core).await;
    let custom_id = component.data.custom_id.as_str();
    if let Some(search_id) = custom_id.strip_prefix(MUSIC_SEARCH_PREFIX) {
        let Some(video_id) = (match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
            _ => None,
        }) else {
            respond_music_component(
                core,
                context,
                component,
                strings.nothing_selected,
                Vec::new(),
            )
            .await;
            return;
        };
        let selection = core.music.lock().await.select_search(
            search_id,
            component.user.id.get(),
            &music_requester_name(&component.user),
            &video_id,
        );
        match selection {
            SearchSelection::Selected(selection_id) => {
                let duration_seconds = core
                    .music
                    .lock()
                    .await
                    .selection_duration_seconds(&selection_id)
                    .unwrap_or(0);
                let preview_seconds = duration_seconds.min(30);
                let preview_end = format_duration(preview_seconds);
                let full_end = format_duration(duration_seconds);
                let content = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    strings.choose_mode,
                    music_i18n::fill(strings.preview_bullet, &[("end", &preview_end)]),
                    music_i18n::fill(strings.full_bullet, &[("end", &full_end)]),
                    strings.custom_bullet,
                    strings.owner_only_action,
                );
                respond_music_component(
                    core,
                    context,
                    component,
                    &content,
                    music_mode_components(&selection_id, duration_seconds, &strings),
                )
                .await;
                crate::music_cleanup::delete(
                    core,
                    &context.http,
                    component.channel_id.get(),
                    component.message.id.get(),
                )
                .await;
            }
            SearchSelection::NotOwner => {
                respond_music_component(
                    core,
                    context,
                    component,
                    strings.search_not_owner,
                    Vec::new(),
                )
                .await;
            }
            SearchSelection::NotFound | SearchSelection::InvalidVideo => {
                respond_music_component(
                    core,
                    context,
                    component,
                    strings.search_expired,
                    Vec::new(),
                )
                .await;
            }
        }
        return;
    }

    if let Some(rest) = custom_id.strip_prefix(MUSIC_MODE_PREFIX) {
        let Some((selection_id, mode_name)) = rest.rsplit_once(':') else {
            return;
        };
        if mode_name == "cancel" {
            let result = core
                .music
                .lock()
                .await
                .cancel_selection(selection_id, component.user.id.get());
            let content = match result {
                SelectionTake::Taken(_) => strings.selection_cancelled,
                SelectionTake::NotOwner => strings.selection_not_owner,
                SelectionTake::NotFound => strings.selection_expired,
            };
            respond_music_component(core, context, component, content, Vec::new()).await;
            return;
        }
        if mode_name == "custom" {
            let access = core
                .music
                .lock()
                .await
                .peek_selection(selection_id, component.user.id.get());
            match access {
                SelectionTake::Taken(_) => {
                    let _ = core
                        .music
                        .lock()
                        .await
                        .touch_selection(selection_id, component.user.id.get());
                    let modal = CreateModal::new(
                        format!("{MUSIC_CUSTOM_PREFIX}{selection_id}"),
                        truncate_text(strings.custom_modal_title, 45),
                    )
                    .components(vec![
                        CreateActionRow::InputText(
                            CreateInputText::new(
                                InputTextStyle::Short,
                                truncate_text(strings.custom_start_label, 45),
                                MUSIC_CUSTOM_START_ID,
                            )
                            .placeholder(truncate_text(strings.custom_start_placeholder, 100))
                            .required(true)
                            .max_length(12),
                        ),
                        CreateActionRow::InputText(
                            CreateInputText::new(
                                InputTextStyle::Short,
                                truncate_text(strings.custom_end_label, 45),
                                MUSIC_CUSTOM_END_ID,
                            )
                            .placeholder(truncate_text(strings.custom_end_placeholder, 100))
                            .required(true)
                            .max_length(12),
                        ),
                    ]);
                    if component
                        .create_response(&context.http, CreateInteractionResponse::Modal(modal))
                        .await
                        .is_err()
                    {
                        core.bot_status.write().await.error =
                            Some("Discord rejected the custom clip modal.".into());
                    }
                }
                SelectionTake::NotOwner => {
                    respond_music_component(
                        core,
                        context,
                        component,
                        strings.selection_not_owner,
                        Vec::new(),
                    )
                    .await;
                }
                SelectionTake::NotFound => {
                    respond_music_component(
                        core,
                        context,
                        component,
                        strings.selection_expired,
                        Vec::new(),
                    )
                    .await;
                }
            }
            return;
        }
        let mode = match mode_name {
            "preview" => MusicPlaybackMode::Preview,
            "full" => MusicPlaybackMode::Full,
            _ => return,
        };
        let selection = match core
            .music
            .lock()
            .await
            .take_selection(selection_id, component.user.id.get())
        {
            SelectionTake::Taken(selection) => selection,
            SelectionTake::NotOwner => {
                respond_music_component(
                    core,
                    context,
                    component,
                    strings.selection_not_owner,
                    Vec::new(),
                )
                .await;
                return;
            }
            SelectionTake::NotFound => {
                respond_music_component(
                    core,
                    context,
                    component,
                    strings.selection_expired,
                    Vec::new(),
                )
                .await;
                return;
            }
        };
        if core.outputs_paused() {
            core.music
                .lock()
                .await
                .restore_selection(selection_id, selection.clone());
            respond_music_component(core, context, component, strings.relay_paused, Vec::new())
                .await;
            return;
        }
        let result = core
            .start_music(selection.clone(), mode, now_ms(), &component.id.to_string())
            .await;
        match &result {
            MusicStartResult::QueueFull
            | MusicStartResult::UserQueueFull { .. }
            | MusicStartResult::DuplicatePending { .. } => {
                core.music
                    .lock()
                    .await
                    .restore_selection(selection_id, selection.clone());
                let content = music_start_rejection_message(&strings, &result);
                respond_music_component(core, context, component, &content, Vec::new()).await;
            }
            MusicStartResult::Started(_) | MusicStartResult::Queued { .. } => {
                announce_music_playback(
                    core,
                    context,
                    &selection,
                    &result,
                    &strings,
                    Some(component),
                )
                .await;
            }
        }
        return;
    }

    if let Some(control_id) = custom_id.strip_prefix(MUSIC_MOVE_UP_PREFIX) {
        handle_pending_music_move(
            core,
            context,
            component,
            control_id,
            crate::music::MusicQueueDirection::Up,
            &strings,
        )
        .await;
        return;
    }
    if let Some(control_id) = custom_id.strip_prefix(MUSIC_MOVE_DOWN_PREFIX) {
        handle_pending_music_move(
            core,
            context,
            component,
            control_id,
            crate::music::MusicQueueDirection::Down,
            &strings,
        )
        .await;
        return;
    }

    if let Some(control_id) = custom_id.strip_prefix(MUSIC_REMOVE_PREFIX) {
        let access = core
            .music
            .lock()
            .await
            .control_playback_id(control_id, component.user.id.get());
        if access.is_err() {
            respond_music_component(core, context, component, strings.skip_not_owner, Vec::new())
                .await;
            return;
        }
        if core.remove_pending_music(control_id).await {
            let _ = component
                .create_response(&context.http, CreateInteractionResponse::Acknowledge)
                .await;
        } else {
            respond_music_component(
                core,
                context,
                component,
                strings.selection_expired,
                Vec::new(),
            )
            .await;
        }
        return;
    }

    if let Some(control_id) = custom_id.strip_prefix(MUSIC_LOOP_PREFIX) {
        let result = core
            .music
            .lock()
            .await
            .toggle_loop(control_id, component.user.id.get());
        match result {
            Ok(looping) => {
                if !looping {
                    let waiting = core.music.lock().await.waiting_repeat_id(control_id);
                    if let Some(id) = waiting {
                        let _ = component
                            .create_response(&context.http, CreateInteractionResponse::Acknowledge)
                            .await;
                        core.cancel_pending_music(&id).await;
                        return;
                    }
                }
                let pending = core.music.lock().await.is_pending(control_id);
                let response = CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().components(if pending {
                        pending_music_control_components(control_id, looping, &strings)
                    } else {
                        music_control_components(control_id, looping, &strings)
                    }),
                );
                if component
                    .create_response(&context.http, response)
                    .await
                    .is_err()
                {
                    core.bot_status.write().await.error =
                        Some("Unable to update the Loop button.".into());
                }
            }
            Err(_) => {
                respond_music_component(
                    core,
                    context,
                    component,
                    strings.skip_not_owner,
                    Vec::new(),
                )
                .await
            }
        }
        return;
    }
    if let Some(control_id) = custom_id.strip_prefix(MUSIC_SKIP_PREFIX) {
        let result = core
            .music
            .lock()
            .await
            .control_playback_id(control_id, component.user.id.get());
        let playback_id = match result {
            Ok(id) => id,
            Err(_) => {
                respond_music_component(
                    core,
                    context,
                    component,
                    strings.skip_not_owner,
                    Vec::new(),
                )
                .await;
                return;
            }
        };
        let _ = component
            .create_response(&context.http, CreateInteractionResponse::Acknowledge)
            .await;
        if core.stop_music_if_current(&playback_id).await.is_none() {
            core.cancel_pending_music(&playback_id).await;
        }
    }
}

pub(super) async fn respond_music_component(
    core: &Arc<AppCore>,
    context: &Context,
    component: &ComponentInteraction,
    content: &str,
    components: Vec<CreateActionRow>,
) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true)
            .components(components)
            .allowed_mentions(CreateAllowedMentions::new()),
    );
    if component
        .create_response(&context.http, response)
        .await
        .is_err()
    {
        core.bot_status.write().await.error =
            Some("Discord rejected the music interaction.".into());
    }
}

fn music_start_rejection_message(strings: &MusicStrings, result: &MusicStartResult) -> String {
    match result {
        MusicStartResult::QueueFull => strings.queue_full.to_owned(),
        MusicStartResult::UserQueueFull { limit } => {
            music_i18n::fill(strings.user_queue_full, &[("limit", &limit.to_string())])
        }
        MusicStartResult::DuplicatePending { .. } => strings.duplicate_pending.to_owned(),
        MusicStartResult::Started(_) | MusicStartResult::Queued { .. } => String::new(),
    }
}

async fn handle_pending_music_move(
    core: &Arc<AppCore>,
    context: &Context,
    component: &ComponentInteraction,
    control_id: &str,
    direction: crate::music::MusicQueueDirection,
    strings: &MusicStrings,
) {
    let access = core
        .music
        .lock()
        .await
        .control_playback_id(control_id, component.user.id.get());
    if access.is_err() {
        respond_music_component(core, context, component, strings.skip_not_owner, Vec::new()).await;
        return;
    }

    match core.move_pending_music(control_id, direction).await {
        crate::music::MusicQueueMove::Moved { .. } | crate::music::MusicQueueMove::AtBoundary => {
            let _ = component
                .create_response(&context.http, CreateInteractionResponse::Acknowledge)
                .await;
            refresh_pending_music_cards(core).await;
        }
        crate::music::MusicQueueMove::NotFound | crate::music::MusicQueueMove::Current => {
            respond_music_component(
                core,
                context,
                component,
                strings.selection_expired,
                Vec::new(),
            )
            .await;
        }
        crate::music::MusicQueueMove::SchedulerUnavailable => {
            respond_music_component(core, context, component, strings.queue_full, Vec::new()).await;
        }
    }
}

pub(super) async fn music_locale(core: &AppCore) -> MusicStrings {
    let language = core.interface_preferences.read().await.language.clone();
    music_i18n::music_strings_for_language(&language)
}

pub(super) fn music_mode_components(
    selection_id: &str,
    duration_seconds: u64,
    strings: &MusicStrings,
) -> Vec<CreateActionRow> {
    let preview_seconds = duration_seconds.min(30);
    let preview_end = format_duration(preview_seconds);
    let full_duration = format_duration(duration_seconds);
    let preview_label = music_i18n::fill(strings.preview_button, &[("end", &preview_end)]);
    let full_label = music_i18n::fill(strings.full_button, &[("duration", &full_duration)]);
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{MUSIC_MODE_PREFIX}{selection_id}:preview"))
            .label(truncate_text(&preview_label, 80))
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{MUSIC_MODE_PREFIX}{selection_id}:full"))
            .label(truncate_text(&full_label, 80))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{MUSIC_MODE_PREFIX}{selection_id}:custom"))
            .label(truncate_text(strings.custom_button, 80))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{MUSIC_MODE_PREFIX}{selection_id}:cancel"))
            .label(truncate_text(strings.cancel, 80))
            .style(ButtonStyle::Danger),
    ])]
}

pub(super) async fn handle_music_custom_modal(
    core: &Arc<AppCore>,
    context: &Context,
    modal: &ModalInteraction,
) {
    let strings = music_locale(core).await;
    let Some(selection_id) = modal.data.custom_id.strip_prefix(MUSIC_CUSTOM_PREFIX) else {
        return;
    };
    let start_raw = modal_input_value(modal, MUSIC_CUSTOM_START_ID).unwrap_or_default();
    let end_raw = modal_input_value(modal, MUSIC_CUSTOM_END_ID).unwrap_or_default();
    let (Some(start_seconds), Some(end_seconds)) =
        (parse_timestamp(&start_raw), parse_timestamp(&end_raw))
    else {
        respond_music_modal(core, context, modal, strings.custom_invalid).await;
        return;
    };

    let selection = match core
        .music
        .lock()
        .await
        .take_selection(selection_id, modal.user.id.get())
    {
        SelectionTake::Taken(selection) => selection,
        SelectionTake::NotOwner => {
            respond_music_modal(core, context, modal, strings.selection_not_owner).await;
            return;
        }
        SelectionTake::NotFound => {
            respond_music_modal(core, context, modal, strings.selection_expired).await;
            return;
        }
    };

    if core.outputs_paused() {
        respond_music_modal(core, context, modal, strings.relay_paused).await;
        return;
    }
    let result = match core
        .start_music_custom(
            selection.clone(),
            start_seconds,
            end_seconds,
            now_ms(),
            &modal.id.to_string(),
        )
        .await
    {
        Ok(result) => result,
        Err(_) => {
            core.music
                .lock()
                .await
                .restore_selection(selection_id, selection);
            respond_music_modal(core, context, modal, strings.custom_invalid).await;
            return;
        }
    };

    match &result {
        MusicStartResult::QueueFull
        | MusicStartResult::UserQueueFull { .. }
        | MusicStartResult::DuplicatePending { .. } => {
            core.music
                .lock()
                .await
                .restore_selection(selection_id, selection.clone());
            let content = music_start_rejection_message(&strings, &result);
            respond_music_modal(core, context, modal, &content).await;
        }
        MusicStartResult::Started(playback) | MusicStartResult::Queued { playback, .. } => {
            announce_music_playback(core, context, &selection, &result, &strings, None).await;
            let range = music_playback_range_label(playback);
            let started_title = truncate_text(&playback.title, 140);
            let user = truncate_text(&playback.requested_by, 40);
            let content = match &result {
                MusicStartResult::Queued { position, .. } => music_i18n::fill(
                    strings.playback_queued,
                    &[
                        ("title", &started_title),
                        ("position", &position.to_string()),
                        ("user", &user),
                    ],
                ),
                _ => music_i18n::fill(
                    strings.playback_started,
                    &[
                        ("title", &started_title),
                        ("range", &range),
                        ("user", &user),
                    ],
                ),
            };
            respond_music_modal(core, context, modal, &content).await;
        }
    }
}

pub(super) fn modal_input_value(modal: &ModalInteraction, custom_id: &str) -> Option<String> {
    for row in &modal.data.components {
        for component in &row.components {
            if let ActionRowComponent::InputText(input) = component
                && input.custom_id == custom_id
            {
                return input.value.clone();
            }
        }
    }
    None
}

pub(super) async fn announce_music_playback(
    core: &Arc<AppCore>,
    context: &Context,
    selection: &MusicSelection,
    result: &MusicStartResult,
    strings: &MusicStrings,
    component: Option<&ComponentInteraction>,
) {
    let (playback, queued_position) = match result {
        MusicStartResult::Started(playback) => (playback, None),
        MusicStartResult::Queued { playback, position } => (playback, Some(*position)),
        MusicStartResult::QueueFull
        | MusicStartResult::UserQueueFull { .. }
        | MusicStartResult::DuplicatePending { .. } => return,
    };
    let range = music_playback_range_label(playback);
    let title = truncate_text(&playback.title, 160);
    let channel = truncate_text(&playback.channel_title, 60);
    let user = truncate_text(&playback.requested_by, 40);

    let content = if let Some(position) = queued_position {
        music_i18n::fill(
            strings.playback_queued,
            &[
                ("title", &title),
                ("position", &position.to_string()),
                ("user", &user),
            ],
        )
    } else {
        music_i18n::fill(
            strings.now_playing,
            &[
                ("title", &title),
                ("channel", &channel),
                ("range", &range),
                ("user", &user),
            ],
        )
    };
    let controls = if queued_position.is_some() {
        pending_music_control_components(&playback.playback_id, false, strings)
    } else {
        music_control_components(&playback.playback_id, false, strings)
    };
    let now_playing = CreateMessage::new()
        .content(content)
        .components(controls)
        .allowed_mentions(CreateAllowedMentions::new());
    if let Ok(now_playing_message) = ChannelId::new(selection.channel_id)
        .send_message(&context.http, now_playing)
        .await
    {
        let attached = core
            .music
            .lock()
            .await
            .set_now_playing_message_id(&playback.playback_id, now_playing_message.id.get());
        if !attached {
            core.delete_now_playing_message(
                selection.channel_id,
                Some(now_playing_message.id.get()),
            )
            .await;
        }
    }
    if let Some(component) = component {
        let started_title = truncate_text(&playback.title, 140);
        respond_music_component(
            core,
            context,
            component,
            &music_i18n::fill(
                strings.playback_started,
                &[
                    ("title", &started_title),
                    ("range", &range),
                    ("user", &user),
                ],
            ),
            Vec::new(),
        )
        .await;
    }
}

pub(crate) async fn refresh_music_card(core: &AppCore) {
    let Some(card) = core.music.lock().await.current_card() else {
        return;
    };
    let Some(http) = core.discord_http().await else {
        return;
    };
    let strings = music_locale(core).await;
    let content = music_i18n::fill(
        strings.now_playing,
        &[
            ("title", &truncate_text(&card.playback.title, 160)),
            ("channel", &truncate_text(&card.playback.channel_title, 60)),
            ("range", &music_playback_range_label(&card.playback)),
            ("user", &truncate_text(&card.playback.requested_by, 40)),
        ],
    );
    let _ = ChannelId::new(card.channel_id)
        .edit_message(
            &http,
            MessageId::new(card.message_id),
            EditMessage::new()
                .content(content)
                .components(music_control_components(
                    &card.control_id,
                    card.looping,
                    &strings,
                ))
                .allowed_mentions(CreateAllowedMentions::new()),
        )
        .await;
}

pub(super) async fn respond_music_modal(
    core: &Arc<AppCore>,
    context: &Context,
    modal: &ModalInteraction,
    content: &str,
) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true)
            .allowed_mentions(CreateAllowedMentions::new()),
    );
    if modal
        .create_response(&context.http, response)
        .await
        .is_err()
    {
        core.bot_status.write().await.error =
            Some("Discord rejected the music modal response.".into());
    }
}

pub(super) fn music_requester_name(user: &User) -> String {
    user.global_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| user.name.clone())
}

pub(super) fn music_playback_range_label(playback: &MusicPlaybackEvent) -> String {
    let start = format_duration(playback.start_seconds);
    let end = playback
        .end_seconds
        .map(format_duration)
        .unwrap_or_else(|| format_duration(playback.duration_seconds));
    format!("{start}→{end}")
}

pub(super) fn music_control_components(
    control_id: &str,
    looping: bool,
    strings: &MusicStrings,
) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{MUSIC_SKIP_PREFIX}{control_id}"))
            .label(truncate_text(strings.skip, 80))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{MUSIC_LOOP_PREFIX}{control_id}"))
            .label(if looping { "Loop: ON" } else { "Loop: OFF" })
            .style(if looping {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            }),
    ])]
}

fn pending_music_control_components(
    control_id: &str,
    looping: bool,
    strings: &MusicStrings,
) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{MUSIC_MOVE_UP_PREFIX}{control_id}"))
            .label("↑")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{MUSIC_MOVE_DOWN_PREFIX}{control_id}"))
            .label("↓")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{MUSIC_REMOVE_PREFIX}{control_id}"))
            .label("✕")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("{MUSIC_SKIP_PREFIX}{control_id}"))
            .label(truncate_text(strings.skip, 80))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{MUSIC_LOOP_PREFIX}{control_id}"))
            .label(if looping { "Loop: ON" } else { "Loop: OFF" })
            .style(if looping {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            }),
    ])]
}

pub(crate) async fn refresh_pending_music_cards(core: &AppCore) {
    let cards = core.music.lock().await.pending_cards();
    let Some(http) = core.discord_http().await else {
        return;
    };
    let strings = music_locale(core).await;
    for card in cards {
        let content = music_i18n::fill(
            strings.playback_queued,
            &[
                ("title", &truncate_text(&card.playback.title, 160)),
                ("position", &card.position.to_string()),
                ("user", &truncate_text(&card.playback.requested_by, 40)),
            ],
        );
        let _ = ChannelId::new(card.channel_id)
            .edit_message(
                &http,
                MessageId::new(card.message_id),
                EditMessage::new()
                    .content(content)
                    .components(pending_music_control_components(
                        &card.control_id,
                        card.looping,
                        &strings,
                    ))
                    .allowed_mentions(CreateAllowedMentions::new()),
            )
            .await;
    }
}

pub(super) fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut value = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() == max_chars && value.chars().count() < value.len() {
        value.push('…');
    }
    value
}

pub(super) fn format_duration(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
