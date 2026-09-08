use super::*;

fn command(name: &str, action: CustomCommandAction) -> CustomCommandDefinition {
    CustomCommandDefinition {
        name: name.into(),
        description: format!("Run the {name} action"),
        action,
        ..CustomCommandDefinition::default()
    }
}

#[test]
fn rejects_reserved_duplicate_and_oversized_lists() {
    let reserved = command(
        "clear",
        CustomCommandAction::Reply {
            text: "ok".into(),
            ephemeral: true,
        },
    );
    assert!(validate_custom_commands(&[reserved]).is_err());
    let nuke = command(
        "nuke",
        CustomCommandAction::Reply {
            text: "ok".into(),
            ephemeral: true,
        },
    );
    assert!(validate_custom_commands(&[nuke]).is_err());

    let duplicate = command(
        "moderate",
        CustomCommandAction::Reply {
            text: "ok".into(),
            ephemeral: true,
        },
    );
    assert!(validate_custom_commands(&[duplicate.clone(), duplicate]).is_err());

    let oversized = (0..=MAX_CUSTOM_COMMANDS)
        .map(|index| {
            command(
                &format!("custom-{index}"),
                CustomCommandAction::Reply {
                    text: "ok".into(),
                    ephemeral: true,
                },
            )
        })
        .collect::<Vec<_>>();
    assert!(validate_custom_commands(&oversized).is_err());
}

#[test]
fn ban_schema_omits_fixed_values_and_exposes_both_target_modes() {
    let definition = command(
        "secure-ban",
        CustomCommandAction::Ban {
            reason: TextParameter {
                mode: ParameterMode::Optional,
                fixed_value: String::new(),
            },
            delete_message_days: IntegerParameter {
                mode: ParameterMode::Fixed,
                fixed_value: 1,
            },
        },
    );
    let value = serde_json::to_value(definition.command_option()).unwrap();
    let parameters = value["options"].as_array().unwrap();
    assert_eq!(parameters.len(), 3);
    assert_eq!(parameters[0]["name"], "member");
    assert_eq!(parameters[0]["required"], false);
    assert_eq!(parameters[1]["name"], "user_id");
    assert_eq!(parameters[1]["required"], false);
    assert_eq!(parameters[2]["name"], "reason");
    assert_eq!(parameters[2]["required"], false);
    assert!(!value.to_string().contains("delete_days"));
}

#[test]
fn ban_requires_exactly_one_member_or_external_user_id() {
    let action = CustomCommandAction::Ban {
        reason: TextParameter::default(),
        delete_message_days: IntegerParameter {
            mode: ParameterMode::Fixed,
            fixed_value: 0,
        },
    };
    let guild_id = GuildId::new(123_456_789_012_345_678);
    let channel_id = ChannelId::new(223_456_789_012_345_678);
    let prepare = |value| {
        let arguments: Vec<CommandDataOption> = serde_json::from_value(value).unwrap();
        prepare_action(&action, &arguments, guild_id, channel_id)
    };

    let member = prepare(serde_json::json!([{
        "name": "member",
        "type": 6,
        "value": "323456789012345678"
    }]))
    .unwrap();
    assert!(matches!(
        member,
        PreparedAction::Ban {
            target_id,
            target_kind: BanTargetKind::Member,
            ..
        } if target_id == UserId::new(323_456_789_012_345_678)
    ));

    let external = prepare(serde_json::json!([{
        "name": "user_id",
        "type": 3,
        "value": "423456789012345678"
    }]))
    .unwrap();
    assert!(matches!(
        external,
        PreparedAction::Ban {
            target_id,
            target_kind: BanTargetKind::ExternalUserId,
            ..
        } if target_id == UserId::new(423_456_789_012_345_678)
    ));

    assert!(prepare(serde_json::json!([])).is_err());
    assert!(
        prepare(serde_json::json!([
            {
                "name": "member",
                "type": 6,
                "value": "323456789012345678"
            },
            {
                "name": "user_id",
                "type": 3,
                "value": "423456789012345678"
            }
        ]))
        .is_err()
    );
}

#[test]
fn derives_the_action_permission_without_weakening_it() {
    let definition = command(
        "ban-user",
        CustomCommandAction::Ban {
            reason: TextParameter::default(),
            delete_message_days: IntegerParameter {
                mode: ParameterMode::Fixed,
                fixed_value: 0,
            },
        },
    );
    let permissions = definition.access.required_permissions(&definition.action);
    assert!(permissions.contains(Permissions::BAN_MEMBERS));
    assert!(permissions.contains(Permissions::ADMINISTRATOR));
}

#[test]
fn public_reply_accepts_lines_but_rejects_hidden_controls() {
    let valid = command(
        "rules",
        CustomCommandAction::Reply {
            text: "Line one\nLine two".into(),
            ephemeral: false,
        },
    );
    assert!(validate_custom_commands(&[valid]).is_ok());

    let invalid = command(
        "hidden",
        CustomCommandAction::Reply {
            text: "visible\u{0007}".into(),
            ephemeral: true,
        },
    );
    assert!(validate_custom_commands(&[invalid]).is_err());
}

#[test]
fn access_policy_combines_permissions_users_roles_and_channels() {
    let user_id = UserId::new(123_456_789_012_345_678);
    let role_id = RoleId::new(223_456_789_012_345_678);
    let channel_id = ChannelId::new(323_456_789_012_345_678);
    let mut definition = command(
        "staff-reply",
        CustomCommandAction::Reply {
            text: "ok".into(),
            ephemeral: true,
        },
    );
    definition.access = CustomCommandAccess {
        administrator_only: false,
        required_permissions: vec![CustomPermission::ManageMessages],
        allowed_user_ids: vec![user_id.to_string()],
        allowed_role_ids: vec![role_id.to_string()],
        allowed_channel_ids: vec![channel_id.to_string()],
    };
    let mut member = Member::default();
    member.permissions = Some(Permissions::MANAGE_MESSAGES);
    member.roles = vec![role_id];

    assert!(
        authorize_invocation(
            &definition,
            &member,
            user_id,
            channel_id,
            Some(Permissions::empty()),
        )
        .is_ok()
    );
    assert!(
        authorize_invocation(
            &definition,
            &member,
            UserId::new(423_456_789_012_345_678),
            channel_id,
            Some(Permissions::empty()),
        )
        .is_err()
    );
}

#[test]
fn confirmations_are_owned_expiring_and_single_use() {
    let user_id = UserId::new(123_456_789_012_345_678);
    let guild_id = GuildId::new(223_456_789_012_345_678);
    let definition = command(
        "kick-user",
        CustomCommandAction::Kick {
            reason: TextParameter::default(),
        },
    );
    let action = PreparedAction::Kick {
        guild_id,
        target_id: UserId::new(323_456_789_012_345_678),
        reason: String::new(),
    };
    let mut store = CustomCommandConfirmations::default();
    let token = store.insert(PendingConfirmation {
        user_id,
        guild_id,
        expires_at: Instant::now() + Duration::from_secs(5),
        definition: definition.clone(),
        action: action.clone(),
    });
    assert!(matches!(
        store.take_for(
            &token,
            UserId::new(423_456_789_012_345_678),
            guild_id,
            Instant::now(),
        ),
        Err(ConfirmationLookup::Mismatched)
    ));
    assert_eq!(
        store
            .take_for(&token, user_id, guild_id, Instant::now())
            .unwrap()
            .action,
        action
    );
    assert!(matches!(
        store.take_for(&token, user_id, guild_id, Instant::now()),
        Err(ConfirmationLookup::Missing)
    ));

    let expired = store.insert(PendingConfirmation {
        user_id,
        guild_id,
        expires_at: Instant::now() - Duration::from_secs(1),
        definition,
        action: PreparedAction::Kick {
            guild_id,
            target_id: UserId::new(323_456_789_012_345_678),
            reason: String::new(),
        },
    });
    assert!(matches!(
        store.take_for(&expired, user_id, guild_id, Instant::now()),
        Err(ConfirmationLookup::Expired)
    ));
}

#[test]
fn hierarchy_uses_position_then_the_lower_role_id() {
    let high_id = RoleId::new(123_456_789_012_345_678);
    let same_position_lower_id = RoleId::new(123_456_789_012_345_677);
    let low_id = RoleId::new(223_456_789_012_345_678);
    let mut high = Role::default();
    high.id = high_id;
    high.position = 10;
    let mut same_position = Role::default();
    same_position.id = same_position_lower_id;
    same_position.position = 10;
    let mut low = Role::default();
    low.id = low_id;
    low.position = 2;
    let roles = HashMap::from([
        (high_id, high),
        (same_position_lower_id, same_position),
        (low_id, low),
    ]);
    let mut left = Member::default();
    left.roles = vec![same_position_lower_id];
    let mut right = Member::default();
    right.roles = vec![high_id, low_id];
    assert!(member_outranks(&roles, &left, &right));
    assert!(!member_outranks(&roles, &right, &left));
}

#[test]
fn parses_ids_and_supported_discord_mentions_only() {
    assert_eq!(
        parse_snowflake_input("<@!123456789012345678>"),
        Some(123_456_789_012_345_678)
    );
    assert_eq!(
        parse_snowflake_input("<@&223456789012345678>"),
        Some(223_456_789_012_345_678)
    );
    assert_eq!(
        parse_snowflake_input("<#323456789012345678>"),
        Some(323_456_789_012_345_678)
    );
    assert_eq!(parse_snowflake_input("user 123456789012345678"), None);
}

#[test]
fn builds_a_valid_typed_schema_for_every_supported_action() {
    let reason = TextParameter::default();
    let role = EntityParameter::default();
    let commands = vec![
        command(
            "custom-ban",
            CustomCommandAction::Ban {
                reason: reason.clone(),
                delete_message_days: IntegerParameter {
                    mode: ParameterMode::Fixed,
                    fixed_value: 0,
                },
            },
        ),
        command(
            "custom-unban",
            CustomCommandAction::Unban {
                reason: reason.clone(),
            },
        ),
        command(
            "custom-kick",
            CustomCommandAction::Kick {
                reason: reason.clone(),
            },
        ),
        command(
            "custom-timeout",
            CustomCommandAction::Timeout {
                duration_minutes: IntegerParameter {
                    mode: ParameterMode::Required,
                    fixed_value: 60,
                },
                reason: reason.clone(),
            },
        ),
        command(
            "custom-untimeout",
            CustomCommandAction::RemoveTimeout {
                reason: reason.clone(),
            },
        ),
        command(
            "custom-clear",
            CustomCommandAction::ClearMessages {
                channel: EntityParameter {
                    mode: ParameterMode::Optional,
                    fixed_value: String::new(),
                },
                count: IntegerParameter {
                    mode: ParameterMode::Fixed,
                    fixed_value: 10,
                },
            },
        ),
        command(
            "custom-add-role",
            CustomCommandAction::AddRole {
                role: role.clone(),
                reason: reason.clone(),
            },
        ),
        command(
            "custom-remove-role",
            CustomCommandAction::RemoveRole { role, reason },
        ),
        command(
            "custom-reply",
            CustomCommandAction::Reply {
                text: "Configured reply".into(),
                ephemeral: false,
            },
        ),
    ];
    validate_custom_commands(&commands).unwrap();
    for definition in commands {
        let value = serde_json::to_value(definition.command_option()).unwrap();
        assert_eq!(value["type"], serde_json::json!(1));
        assert_eq!(value["name"], definition.name);
    }
}

#[test]
fn interaction_responses_disable_every_kind_of_mass_mention() {
    let value = serde_json::to_value(
        CustomCommandResponse {
            content: "@everyone <@123456789012345678>".into(),
            ephemeral: false,
            components: Vec::new(),
        }
        .into_message(),
    )
    .unwrap();
    assert_eq!(value["allowed_mentions"]["parse"], serde_json::json!([]));
    assert_eq!(value["allowed_mentions"]["users"], serde_json::json!([]));
    assert_eq!(value["allowed_mentions"]["roles"], serde_json::json!([]));
}

#[test]
fn frontend_camel_case_action_payloads_round_trip_with_legacy_aliases() {
    let commands: Vec<CustomCommandDefinition> = serde_json::from_value(serde_json::json!([
        {
            "name": "custom-ban",
            "description": "Ban a selected member",
            "enabled": true,
            "action": {
                "type": "ban",
                "reason": { "mode": "optional", "fixedValue": "" },
                "deleteMessageDays": { "mode": "fixed", "fixedValue": 0 }
            }
        },
        {
            "name": "custom-timeout",
            "description": "Timeout a selected member",
            "enabled": true,
            "action": {
                "type": "timeout",
                "durationMinutes": { "mode": "fixed", "fixedValue": 60 },
                "reason": { "mode": "optional", "fixedValue": "" }
            }
        }
    ]))
    .unwrap();
    validate_custom_commands(&commands).unwrap();

    let serialized = serde_json::to_value(&commands).unwrap();
    assert!(serialized[0]["action"].get("deleteMessageDays").is_some());
    assert!(serialized[0]["action"].get("delete_message_days").is_none());
    assert!(serialized[1]["action"].get("durationMinutes").is_some());
    assert!(serialized[1]["action"].get("duration_minutes").is_none());

    let legacy: CustomCommandAction = serde_json::from_value(serde_json::json!({
        "type": "ban",
        "reason": { "mode": "optional", "fixedValue": "" },
        "delete_message_days": { "mode": "fixed", "fixedValue": 1 }
    }))
    .unwrap();
    assert!(matches!(legacy, CustomCommandAction::Ban { .. }));
}
