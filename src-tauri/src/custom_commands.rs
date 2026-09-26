use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, bail};
use rand::{RngCore, rng};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ButtonStyle, Channel, ChannelId, ChannelType, CommandDataOption, CommandDataOptionValue,
    CommandInteraction, CommandOptionType, ComponentInteraction, Context as DiscordContext,
    CreateActionRow, CreateAllowedMentions, CreateButton, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
    EditMember, GuildId, Member, PartialGuild, Permissions, Role, RoleId, Timestamp, UserId,
};

use crate::state::AppCore;

pub const MAX_CUSTOM_COMMANDS: usize = 16;
pub const DEFAULT_COMMAND_NAMES: [&str; 10] = [
    "channel",
    "url",
    "show",
    "status",
    "test",
    "regenerate",
    "clear",
    "nuke",
    "lock",
    "changelog",
];
const MAX_ACCESS_IDS: usize = 100;
const MAX_REASON_CHARS: usize = 512;
const MAX_REPLY_CHARS: usize = 1_900;
const MAX_TIMEOUT_MINUTES: u32 = 28 * 24 * 60;
const CONFIRMATION_TTL: Duration = Duration::from_secs(60);
const MAX_PENDING_CONFIRMATIONS: usize = 64;
const CONFIRM_COMPONENT_PREFIX: &str = "relay-custom:confirm:";
const CANCEL_COMPONENT_PREFIX: &str = "relay-custom:cancel:";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterMode {
    Required,
    #[default]
    Optional,
    Fixed,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TextParameter {
    pub mode: ParameterMode,
    pub fixed_value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IntegerParameter {
    pub mode: ParameterMode,
    pub fixed_value: u32,
}

impl Default for IntegerParameter {
    fn default() -> Self {
        Self {
            mode: ParameterMode::Fixed,
            fixed_value: 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EntityParameter {
    pub mode: ParameterMode,
    pub fixed_value: String,
}

impl Default for EntityParameter {
    fn default() -> Self {
        Self {
            mode: ParameterMode::Required,
            fixed_value: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CustomPermission {
    Administrator,
    ManageGuild,
    ManageMessages,
    ManageRoles,
    BanMembers,
    KickMembers,
    ModerateMembers,
}

impl CustomPermission {
    pub fn bits(self) -> Permissions {
        match self {
            Self::Administrator => Permissions::ADMINISTRATOR,
            Self::ManageGuild => Permissions::MANAGE_GUILD,
            Self::ManageMessages => Permissions::MANAGE_MESSAGES,
            Self::ManageRoles => Permissions::MANAGE_ROLES,
            Self::BanMembers => Permissions::BAN_MEMBERS,
            Self::KickMembers => Permissions::KICK_MEMBERS,
            Self::ModerateMembers => Permissions::MODERATE_MEMBERS,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomCommandAccess {
    pub administrator_only: bool,
    pub required_permissions: Vec<CustomPermission>,
    pub allowed_user_ids: Vec<String>,
    pub allowed_role_ids: Vec<String>,
    pub allowed_channel_ids: Vec<String>,
}

impl Default for CustomCommandAccess {
    fn default() -> Self {
        Self {
            administrator_only: true,
            required_permissions: Vec::new(),
            allowed_user_ids: Vec::new(),
            allowed_role_ids: Vec::new(),
            allowed_channel_ids: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CustomCommandAction {
    Ban {
        reason: TextParameter,
        #[serde(alias = "delete_message_days")]
        delete_message_days: IntegerParameter,
    },
    Unban {
        reason: TextParameter,
    },
    Kick {
        reason: TextParameter,
    },
    Timeout {
        #[serde(alias = "duration_minutes")]
        duration_minutes: IntegerParameter,
        reason: TextParameter,
    },
    RemoveTimeout {
        reason: TextParameter,
    },
    ClearMessages {
        channel: EntityParameter,
        count: IntegerParameter,
    },
    AddRole {
        role: EntityParameter,
        reason: TextParameter,
    },
    RemoveRole {
        role: EntityParameter,
        reason: TextParameter,
    },
    Reply {
        text: String,
        ephemeral: bool,
    },
}

impl CustomCommandAction {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Ban { .. } => "BAN",
            Self::Unban { .. } => "UNBAN",
            Self::Kick { .. } => "KICK",
            Self::Timeout { .. } => "TIMEOUT",
            Self::RemoveTimeout { .. } => "REMOVE_TIMEOUT",
            Self::ClearMessages { .. } => "CLEAR_MESSAGES",
            Self::AddRole { .. } => "ADD_ROLE",
            Self::RemoveRole { .. } => "REMOVE_ROLE",
            Self::Reply { .. } => "REPLY",
        }
    }

    pub fn minimum_permission(&self) -> Permissions {
        match self {
            Self::Ban { .. } | Self::Unban { .. } => Permissions::BAN_MEMBERS,
            Self::Kick { .. } => Permissions::KICK_MEMBERS,
            Self::Timeout { .. } | Self::RemoveTimeout { .. } => Permissions::MODERATE_MEMBERS,
            Self::ClearMessages { .. } => Permissions::MANAGE_MESSAGES,
            Self::AddRole { .. } | Self::RemoveRole { .. } => Permissions::MANAGE_ROLES,
            Self::Reply { .. } => Permissions::empty(),
        }
    }

    pub fn requires_confirmation(&self) -> bool {
        !matches!(self, Self::Reply { .. })
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Ban {
                reason,
                delete_message_days,
            } => {
                validate_text_parameter(reason, "ban reason")?;
                validate_integer_parameter(delete_message_days, 0, 7, "ban message deletion window")
            }
            Self::Unban { reason } | Self::Kick { reason } | Self::RemoveTimeout { reason } => {
                validate_text_parameter(reason, "reason")
            }
            Self::Timeout {
                duration_minutes,
                reason,
            } => {
                validate_integer_parameter(
                    duration_minutes,
                    1,
                    MAX_TIMEOUT_MINUTES,
                    "timeout duration",
                )?;
                validate_text_parameter(reason, "timeout reason")
            }
            Self::ClearMessages { channel, count } => {
                validate_entity_parameter(channel, "clear channel", true)?;
                validate_integer_parameter(count, 1, 1_000, "message count")
            }
            Self::AddRole { role, reason } | Self::RemoveRole { role, reason } => {
                validate_entity_parameter(role, "role", false)?;
                validate_text_parameter(reason, "role reason")
            }
            Self::Reply { text, .. } => {
                let length = text.chars().count();
                if !(1..=MAX_REPLY_CHARS).contains(&length)
                    || contains_disallowed_control(text, true)
                {
                    bail!(
                        "A predefined reply must contain 1 to {MAX_REPLY_CHARS} printable characters."
                    );
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomCommandDefinition {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub action: CustomCommandAction,
    pub access: CustomCommandAccess,
}

impl Default for CustomCommandDefinition {
    fn default() -> Self {
        Self {
            name: "reply".into(),
            description: "Send a predefined Relay reply".into(),
            enabled: true,
            action: CustomCommandAction::Reply {
                text: "Relay".into(),
                ephemeral: true,
            },
            access: CustomCommandAccess::default(),
        }
    }
}

impl CustomCommandDefinition {
    fn validate(&self) -> Result<()> {
        let name_length = self.name.chars().count();
        if !(1..=32).contains(&name_length)
            || !self.name.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '-' | '_')
            })
        {
            bail!(
                "Custom command names must contain 1 to 32 lowercase letters, digits, hyphens, or underscores."
            );
        }
        if DEFAULT_COMMAND_NAMES.contains(&self.name.as_str()) || self.name == "reaction" {
            bail!("A custom command name conflicts with a default Relay command.");
        }
        let description_length = self.description.chars().count();
        if !(1..=100).contains(&description_length)
            || contains_disallowed_control(&self.description, false)
        {
            bail!("Custom command descriptions must contain 1 to 100 printable characters.");
        }
        self.action.validate()?;
        self.access.validate()
    }

    pub fn command_option(&self) -> CreateCommandOption {
        let mut option = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            self.name.clone(),
            self.description.clone(),
        );
        let mut parameters = Vec::new();
        match &self.action {
            CustomCommandAction::Ban {
                reason,
                delete_message_days,
            } => {
                parameters.push((
                    false,
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "member",
                        "Current member to ban; use either member or user_id",
                    )
                    .required(false),
                ));
                parameters.push((
                    false,
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "user_id",
                        "Discord user ID to ban before they join",
                    )
                    .min_length(17)
                    .max_length(24)
                    .required(false),
                ));
                push_text_option(&mut parameters, reason, "reason", "Audit log reason");
                push_integer_option(
                    &mut parameters,
                    delete_message_days,
                    "delete_days",
                    "Days of recent messages to delete",
                    0,
                    7,
                );
            }
            CustomCommandAction::Unban { reason } => {
                parameters.push(required_option(
                    CommandOptionType::String,
                    "user_id",
                    "Discord user ID to unban",
                ));
                push_text_option(&mut parameters, reason, "reason", "Audit log reason");
            }
            CustomCommandAction::Kick { reason }
            | CustomCommandAction::RemoveTimeout { reason } => {
                parameters.push(required_option(
                    CommandOptionType::User,
                    "member",
                    "Target member",
                ));
                push_text_option(&mut parameters, reason, "reason", "Audit log reason");
            }
            CustomCommandAction::Timeout {
                duration_minutes,
                reason,
            } => {
                parameters.push(required_option(
                    CommandOptionType::User,
                    "member",
                    "Member to time out",
                ));
                push_integer_option(
                    &mut parameters,
                    duration_minutes,
                    "duration_minutes",
                    "Timeout duration in minutes",
                    1,
                    u64::from(MAX_TIMEOUT_MINUTES),
                );
                push_text_option(&mut parameters, reason, "reason", "Audit log reason");
            }
            CustomCommandAction::ClearMessages { channel, count } => {
                push_entity_option(
                    &mut parameters,
                    channel,
                    CommandOptionType::Channel,
                    "channel",
                    "Channel to clear; defaults to the current channel",
                );
                push_integer_option(
                    &mut parameters,
                    count,
                    "count",
                    "Number of messages to delete",
                    1,
                    1_000,
                );
            }
            CustomCommandAction::AddRole { role, reason }
            | CustomCommandAction::RemoveRole { role, reason } => {
                parameters.push(required_option(
                    CommandOptionType::User,
                    "member",
                    "Target member",
                ));
                push_entity_option(
                    &mut parameters,
                    role,
                    CommandOptionType::Role,
                    "role",
                    "Role to update",
                );
                push_text_option(&mut parameters, reason, "reason", "Audit log reason");
            }
            CustomCommandAction::Reply { .. } => {}
        }
        parameters.sort_by_key(|(required, _)| !*required);
        for (_, parameter) in parameters {
            option = option.add_sub_option(parameter);
        }
        option
    }
}

impl CustomCommandAccess {
    fn validate(&self) -> Result<()> {
        let mut permissions = HashSet::new();
        for permission in &self.required_permissions {
            if !permissions.insert(*permission) {
                bail!("Additional custom command permissions must be unique.");
            }
        }
        validate_id_list(&self.allowed_user_ids, "allowed user")?;
        validate_id_list(&self.allowed_role_ids, "allowed role")?;
        validate_id_list(&self.allowed_channel_ids, "allowed channel")
    }

    pub fn required_permissions(&self, action: &CustomCommandAction) -> Permissions {
        let mut permissions = action.minimum_permission();
        if self.administrator_only {
            permissions |= Permissions::ADMINISTRATOR;
        }
        for permission in &self.required_permissions {
            permissions |= permission.bits();
        }
        permissions
    }
}

pub fn validate_custom_commands(commands: &[CustomCommandDefinition]) -> Result<()> {
    if commands.len() > MAX_CUSTOM_COMMANDS {
        bail!("At most {MAX_CUSTOM_COMMANDS} custom Relay commands may be configured.");
    }
    let mut names = HashSet::new();
    for command in commands {
        command.validate()?;
        if !names.insert(command.name.as_str()) {
            bail!("Custom command names must be unique.");
        }
    }
    Ok(())
}

pub fn required_bot_permissions(commands: &[CustomCommandDefinition]) -> Permissions {
    commands
        .iter()
        .filter(|command| command.enabled)
        .fold(Permissions::empty(), |permissions, command| {
            permissions | command.action.minimum_permission()
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BanTargetKind {
    Member,
    ExternalUserId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PreparedAction {
    Ban {
        guild_id: GuildId,
        target_id: UserId,
        target_kind: BanTargetKind,
        delete_message_days: u8,
        reason: String,
    },
    Unban {
        guild_id: GuildId,
        target_id: UserId,
        reason: String,
    },
    Kick {
        guild_id: GuildId,
        target_id: UserId,
        reason: String,
    },
    Timeout {
        guild_id: GuildId,
        target_id: UserId,
        duration_minutes: u32,
        reason: String,
    },
    RemoveTimeout {
        guild_id: GuildId,
        target_id: UserId,
        reason: String,
    },
    ClearMessages {
        guild_id: GuildId,
        channel_id: ChannelId,
        count: usize,
    },
    AddRole {
        guild_id: GuildId,
        target_id: UserId,
        role_id: RoleId,
        reason: String,
    },
    RemoveRole {
        guild_id: GuildId,
        target_id: UserId,
        role_id: RoleId,
        reason: String,
    },
}

impl PreparedAction {
    fn guild_id(&self) -> GuildId {
        match self {
            Self::Ban { guild_id, .. }
            | Self::Unban { guild_id, .. }
            | Self::Kick { guild_id, .. }
            | Self::Timeout { guild_id, .. }
            | Self::RemoveTimeout { guild_id, .. }
            | Self::ClearMessages { guild_id, .. }
            | Self::AddRole { guild_id, .. }
            | Self::RemoveRole { guild_id, .. } => *guild_id,
        }
    }
}

#[derive(Clone, Debug)]
struct PendingConfirmation {
    user_id: UserId,
    guild_id: GuildId,
    expires_at: Instant,
    definition: CustomCommandDefinition,
    action: PreparedAction,
}

#[derive(Debug, Eq, PartialEq)]
enum ConfirmationLookup {
    Missing,
    Mismatched,
    Expired,
}

#[derive(Default)]
pub struct CustomCommandConfirmations {
    entries: HashMap<String, PendingConfirmation>,
}

impl CustomCommandConfirmations {
    fn insert(&mut self, pending: PendingConfirmation) -> String {
        let now = Instant::now();
        self.entries.retain(|_, item| item.expires_at > now);
        if self.entries.len() >= MAX_PENDING_CONFIRMATIONS
            && let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, item)| item.expires_at)
                .map(|(token, _)| token.clone())
        {
            self.entries.remove(&oldest);
        }
        let token = confirmation_token();
        self.entries.insert(token.clone(), pending);
        token
    }

    fn take_for(
        &mut self,
        token: &str,
        user_id: UserId,
        guild_id: GuildId,
        now: Instant,
    ) -> std::result::Result<PendingConfirmation, ConfirmationLookup> {
        let Some(pending) = self.entries.get(token) else {
            return Err(ConfirmationLookup::Missing);
        };
        if pending.user_id != user_id || pending.guild_id != guild_id {
            return Err(ConfirmationLookup::Mismatched);
        }
        let pending = self
            .entries
            .remove(token)
            .expect("the pending confirmation was checked above");
        if pending.expires_at <= now {
            return Err(ConfirmationLookup::Expired);
        }
        Ok(pending)
    }

    fn remove_if_expired(&mut self, token: &str, now: Instant) {
        if self
            .entries
            .get(token)
            .is_some_and(|pending| pending.expires_at <= now)
        {
            self.entries.remove(token);
        }
    }
}

pub struct CustomCommandResponse {
    pub content: String,
    pub ephemeral: bool,
    pub components: Vec<CreateActionRow>,
}

impl CustomCommandResponse {
    pub fn into_message(self) -> CreateInteractionResponseMessage {
        CreateInteractionResponseMessage::new()
            .content(self.content)
            .ephemeral(self.ephemeral)
            .allowed_mentions(CreateAllowedMentions::new())
            .components(self.components)
    }

    fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ephemeral: true,
            components: Vec::new(),
        }
    }
}

fn confirmation_token() -> String {
    let mut bytes = [0_u8; 16];
    rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub async fn handle_custom_command(
    core: &std::sync::Arc<AppCore>,
    context: &DiscordContext,
    command: &CommandInteraction,
) -> Option<CustomCommandResponse> {
    let option = command.data.options.first()?;
    let arguments = match &option.value {
        CommandDataOptionValue::SubCommand(arguments) => arguments,
        _ => return None,
    };
    let definition = core
        .config
        .read()
        .await
        .custom_commands
        .iter()
        .find(|definition| definition.enabled && definition.name == option.name)
        .cloned()?;
    let Some(guild_id) = command.guild_id else {
        log_custom_outcome(&definition, "DENIED");
        return Some(CustomCommandResponse::error(
            "Custom Relay commands can only run inside a Discord server.",
        ));
    };
    let Some(member) = command.member.as_deref() else {
        log_custom_outcome(&definition, "DENIED");
        return Some(CustomCommandResponse::error(
            "Discord member context is unavailable for this command.",
        ));
    };
    if let Err(message) = authorize_invocation(
        &definition,
        member,
        command.user.id,
        command.channel_id,
        command.app_permissions,
    ) {
        log_custom_outcome(&definition, "DENIED");
        return Some(CustomCommandResponse::error(message));
    }

    if let CustomCommandAction::Reply { text, ephemeral } = &definition.action {
        log_custom_outcome(&definition, "SUCCEEDED");
        return Some(CustomCommandResponse {
            content: text.clone(),
            ephemeral: *ephemeral,
            components: Vec::new(),
        });
    }
    debug_assert!(definition.action.requires_confirmation());

    let action = match prepare_action(&definition.action, arguments, guild_id, command.channel_id) {
        Ok(action) => action,
        Err(message) => {
            log_custom_outcome(&definition, "INVALID_INPUT");
            return Some(CustomCommandResponse::error(message));
        }
    };
    if let Err(message) = validate_prepared_action(context, member, command.user.id, &action).await
    {
        log_custom_outcome(&definition, "DENIED");
        return Some(CustomCommandResponse::error(message));
    }

    let pending = PendingConfirmation {
        user_id: command.user.id,
        guild_id,
        expires_at: Instant::now() + CONFIRMATION_TTL,
        definition: definition.clone(),
        action,
    };
    let token = core
        .custom_command_confirmations
        .lock()
        .await
        .insert(pending);
    let cleanup_core = std::sync::Arc::clone(core);
    let cleanup_token = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(CONFIRMATION_TTL).await;
        cleanup_core
            .custom_command_confirmations
            .lock()
            .await
            .remove_if_expired(&cleanup_token, Instant::now());
    });
    log_custom_outcome(&definition, "AWAITING_CONFIRMATION");
    Some(CustomCommandResponse {
        content: format!(
            "Confirm the `{}` action within 60 seconds. Relay will recheck permissions before executing it.",
            definition.action.code()
        ),
        ephemeral: true,
        components: vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{CONFIRM_COMPONENT_PREFIX}{token}"))
                .label("Confirm")
                .style(ButtonStyle::Danger),
            CreateButton::new(format!("{CANCEL_COMPONENT_PREFIX}{token}"))
                .label("Cancel")
                .style(ButtonStyle::Secondary),
        ])],
    })
}

fn authorize_invocation(
    definition: &CustomCommandDefinition,
    member: &Member,
    user_id: UserId,
    channel_id: ChannelId,
    app_permissions: Option<Permissions>,
) -> std::result::Result<(), &'static str> {
    let member_permissions = member.permissions.unwrap_or_default();
    let required = definition.access.required_permissions(&definition.action);
    if !has_permissions(member_permissions, required) {
        return Err("You do not have the permissions required by this Relay command.");
    }
    let bot_permissions = app_permissions.unwrap_or_default();
    if !has_permissions(bot_permissions, definition.action.minimum_permission()) {
        return Err("Relay is missing the Discord permission required by this action.");
    }
    if !definition.access.allowed_user_ids.is_empty()
        && !definition
            .access
            .allowed_user_ids
            .iter()
            .any(|allowed| allowed == &user_id.to_string())
    {
        return Err("This Relay command is not available to your Discord account.");
    }
    if !definition.access.allowed_role_ids.is_empty()
        && !member.roles.iter().any(|role_id| {
            definition
                .access
                .allowed_role_ids
                .iter()
                .any(|allowed| allowed == &role_id.to_string())
        })
    {
        return Err("This Relay command is restricted to configured Discord roles.");
    }
    if !definition.access.allowed_channel_ids.is_empty()
        && !definition
            .access
            .allowed_channel_ids
            .iter()
            .any(|allowed| allowed == &channel_id.to_string())
    {
        return Err("This Relay command is not available in this Discord channel.");
    }
    Ok(())
}

fn has_permissions(actual: Permissions, required: Permissions) -> bool {
    required.is_empty() || actual.contains(Permissions::ADMINISTRATOR) || actual.contains(required)
}

fn prepare_action(
    action: &CustomCommandAction,
    arguments: &[CommandDataOption],
    guild_id: GuildId,
    current_channel_id: ChannelId,
) -> std::result::Result<PreparedAction, &'static str> {
    match action {
        CustomCommandAction::Ban {
            reason,
            delete_message_days,
        } => {
            let (target_id, target_kind) = resolve_ban_target(arguments)?;
            Ok(PreparedAction::Ban {
                guild_id,
                target_id,
                target_kind,
                delete_message_days: u8::try_from(resolve_integer(
                    delete_message_days,
                    arguments,
                    "delete_days",
                    0,
                    7,
                )?)
                .map_err(|_| "The message deletion window is invalid.")?,
                reason: resolve_text(reason, arguments, "reason")?,
            })
        }
        CustomCommandAction::Unban { reason } => Ok(PreparedAction::Unban {
            guild_id,
            target_id: parse_user_id(
                string_argument(arguments, "user_id").ok_or("A Discord user ID is required.")?,
            )?,
            reason: resolve_text(reason, arguments, "reason")?,
        }),
        CustomCommandAction::Kick { reason } => Ok(PreparedAction::Kick {
            guild_id,
            target_id: required_user(arguments, "member")?,
            reason: resolve_text(reason, arguments, "reason")?,
        }),
        CustomCommandAction::Timeout {
            duration_minutes,
            reason,
        } => Ok(PreparedAction::Timeout {
            guild_id,
            target_id: required_user(arguments, "member")?,
            duration_minutes: resolve_integer(
                duration_minutes,
                arguments,
                "duration_minutes",
                1,
                MAX_TIMEOUT_MINUTES,
            )?,
            reason: resolve_text(reason, arguments, "reason")?,
        }),
        CustomCommandAction::RemoveTimeout { reason } => Ok(PreparedAction::RemoveTimeout {
            guild_id,
            target_id: required_user(arguments, "member")?,
            reason: resolve_text(reason, arguments, "reason")?,
        }),
        CustomCommandAction::ClearMessages { channel, count } => {
            Ok(PreparedAction::ClearMessages {
                guild_id,
                channel_id: resolve_channel(channel, arguments, current_channel_id)?,
                count: usize::try_from(resolve_integer(count, arguments, "count", 1, 1_000)?)
                    .map_err(|_| "The message count is invalid.")?,
            })
        }
        CustomCommandAction::AddRole { role, reason } => Ok(PreparedAction::AddRole {
            guild_id,
            target_id: required_user(arguments, "member")?,
            role_id: resolve_role(role, arguments)?,
            reason: resolve_text(reason, arguments, "reason")?,
        }),
        CustomCommandAction::RemoveRole { role, reason } => Ok(PreparedAction::RemoveRole {
            guild_id,
            target_id: required_user(arguments, "member")?,
            role_id: resolve_role(role, arguments)?,
            reason: resolve_text(reason, arguments, "reason")?,
        }),
        CustomCommandAction::Reply { .. } => Err("This action does not require preparation."),
    }
}

fn required_user(
    arguments: &[CommandDataOption],
    name: &str,
) -> std::result::Result<UserId, &'static str> {
    user_argument(arguments, name).ok_or("A target Discord member is required.")
}

fn user_argument(arguments: &[CommandDataOption], name: &str) -> Option<UserId> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .and_then(|argument| argument.value.as_user_id())
}

fn resolve_ban_target(
    arguments: &[CommandDataOption],
) -> std::result::Result<(UserId, BanTargetKind), &'static str> {
    let member = user_argument(arguments, "member");
    let user_id = string_argument(arguments, "user_id");
    match (member, user_id) {
        (Some(target_id), None) => Ok((target_id, BanTargetKind::Member)),
        (None, Some(value)) => Ok((parse_user_id(value)?, BanTargetKind::ExternalUserId)),
        (None, None) => Err("Select a current member or provide a Discord user ID to ban."),
        (Some(_), Some(_)) => Err("Select either a current member or a Discord user ID, not both."),
    }
}

fn string_argument<'a>(arguments: &'a [CommandDataOption], name: &str) -> Option<&'a str> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .and_then(|argument| argument.value.as_str())
}

fn integer_argument(arguments: &[CommandDataOption], name: &str) -> Option<i64> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .and_then(|argument| argument.value.as_i64())
}

fn resolve_text(
    parameter: &TextParameter,
    arguments: &[CommandDataOption],
    name: &str,
) -> std::result::Result<String, &'static str> {
    let supplied = string_argument(arguments, name);
    let value = match parameter.mode {
        ParameterMode::Required => supplied.ok_or("A required text parameter is missing.")?,
        ParameterMode::Optional => supplied.unwrap_or(&parameter.fixed_value),
        ParameterMode::Fixed => &parameter.fixed_value,
    };
    if value.chars().count() > MAX_REASON_CHARS || contains_disallowed_control(value, false) {
        return Err("A text parameter is invalid.");
    }
    Ok(value.to_owned())
}

fn resolve_integer(
    parameter: &IntegerParameter,
    arguments: &[CommandDataOption],
    name: &str,
    minimum: u32,
    maximum: u32,
) -> std::result::Result<u32, &'static str> {
    let supplied = integer_argument(arguments, name).and_then(|value| u32::try_from(value).ok());
    let value = match parameter.mode {
        ParameterMode::Required => supplied.ok_or("A required number parameter is missing.")?,
        ParameterMode::Optional => supplied.unwrap_or(parameter.fixed_value),
        ParameterMode::Fixed => parameter.fixed_value,
    };
    if !(minimum..=maximum).contains(&value) {
        return Err("A number parameter is outside the supported range.");
    }
    Ok(value)
}

fn resolve_channel(
    parameter: &EntityParameter,
    arguments: &[CommandDataOption],
    current_channel_id: ChannelId,
) -> std::result::Result<ChannelId, &'static str> {
    let supplied = arguments
        .iter()
        .find(|argument| argument.name == "channel")
        .and_then(|argument| argument.value.as_channel_id());
    match parameter.mode {
        ParameterMode::Required => supplied.ok_or("A Discord channel is required."),
        ParameterMode::Optional => supplied
            .or_else(|| parse_channel_id(&parameter.fixed_value))
            .or(Some(current_channel_id))
            .ok_or("The configured Discord channel is invalid."),
        ParameterMode::Fixed => parse_channel_id(&parameter.fixed_value)
            .ok_or("The configured Discord channel is invalid."),
    }
}

fn resolve_role(
    parameter: &EntityParameter,
    arguments: &[CommandDataOption],
) -> std::result::Result<RoleId, &'static str> {
    let supplied = arguments
        .iter()
        .find(|argument| argument.name == "role")
        .and_then(|argument| argument.value.as_role_id());
    match parameter.mode {
        ParameterMode::Required => supplied.ok_or("A Discord role is required."),
        ParameterMode::Optional => supplied
            .or_else(|| parse_role_id(&parameter.fixed_value))
            .ok_or("The configured Discord role is invalid."),
        ParameterMode::Fixed => {
            parse_role_id(&parameter.fixed_value).ok_or("The configured Discord role is invalid.")
        }
    }
}

fn parse_user_id(value: &str) -> std::result::Result<UserId, &'static str> {
    parse_snowflake_input(value)
        .map(UserId::new)
        .ok_or("The Discord user ID is invalid.")
}

fn parse_channel_id(value: &str) -> Option<ChannelId> {
    parse_snowflake_input(value).map(ChannelId::new)
}

fn parse_role_id(value: &str) -> Option<RoleId> {
    parse_snowflake_input(value).map(RoleId::new)
}

fn parse_snowflake_input(value: &str) -> Option<u64> {
    let value = value.trim();
    let digits = if value.starts_with("<@&") && value.ends_with('>') {
        &value[3..value.len() - 1]
    } else if value.starts_with("<@") && value.ends_with('>') {
        value[2..value.len() - 1].trim_start_matches('!')
    } else if value.starts_with("<#") && value.ends_with('>') {
        &value[2..value.len() - 1]
    } else {
        value
    };
    if !(17..=20).contains(&digits.len())
        || !digits.chars().all(|character| character.is_ascii_digit())
    {
        return None;
    }
    digits.parse::<u64>().ok().filter(|id| *id != 0)
}

async fn validate_prepared_action(
    context: &DiscordContext,
    invoking_member: &Member,
    invoking_user_id: UserId,
    action: &PreparedAction,
) -> std::result::Result<(), &'static str> {
    match action {
        PreparedAction::Unban { .. } => Ok(()),
        PreparedAction::ClearMessages {
            guild_id,
            channel_id,
            ..
        } => {
            validate_clear_channel(
                context,
                invoking_member,
                invoking_user_id,
                *guild_id,
                *channel_id,
            )
            .await
        }
        PreparedAction::AddRole {
            guild_id,
            target_id,
            role_id,
            ..
        }
        | PreparedAction::RemoveRole {
            guild_id,
            target_id,
            role_id,
            ..
        } => {
            let (guild, bot_member, target_member) = validate_member_target(
                context,
                invoking_member,
                invoking_user_id,
                *guild_id,
                *target_id,
            )
            .await?;
            validate_role_hierarchy(
                &guild,
                invoking_member,
                &bot_member,
                &target_member,
                *role_id,
            )
        }
        PreparedAction::Ban {
            guild_id,
            target_id,
            target_kind,
            ..
        } => match target_kind {
            BanTargetKind::Member => {
                validate_member_target(
                    context,
                    invoking_member,
                    invoking_user_id,
                    *guild_id,
                    *target_id,
                )
                .await?;
                Ok(())
            }
            BanTargetKind::ExternalUserId => {
                validate_external_ban_target(context, invoking_user_id, *guild_id, *target_id).await
            }
        },
        PreparedAction::Kick {
            guild_id,
            target_id,
            ..
        }
        | PreparedAction::Timeout {
            guild_id,
            target_id,
            ..
        }
        | PreparedAction::RemoveTimeout {
            guild_id,
            target_id,
            ..
        } => {
            validate_member_target(
                context,
                invoking_member,
                invoking_user_id,
                *guild_id,
                *target_id,
            )
            .await?;
            Ok(())
        }
    }
}

async fn validate_member_target(
    context: &DiscordContext,
    invoking_member: &Member,
    invoking_user_id: UserId,
    guild_id: GuildId,
    target_id: UserId,
) -> std::result::Result<(PartialGuild, Member, Member), &'static str> {
    if target_id == invoking_user_id {
        return Err("Relay does not allow this command to target its invoker.");
    }
    let bot_id = context.cache.current_user().id;
    if target_id == bot_id {
        return Err("Relay does not allow moderation actions against itself.");
    }
    let guild = guild_id
        .to_partial_guild(&context.http)
        .await
        .map_err(|_| "Relay could not verify the Discord server hierarchy.")?;
    if target_id == guild.owner_id {
        return Err("The Discord server owner cannot be targeted by this command.");
    }
    let target_member = guild_id
        .member(context, target_id)
        .await
        .map_err(|_| "The selected target is not an available server member.")?;
    if target_member.user.bot {
        return Err("Relay custom moderation commands cannot target bot accounts.");
    }
    let bot_member = guild_id
        .member(context, bot_id)
        .await
        .map_err(|_| "Relay could not verify its Discord role hierarchy.")?;
    if invoking_user_id != guild.owner_id
        && !member_outranks(&guild.roles, invoking_member, &target_member)
    {
        return Err("Your highest Discord role must be above the target member.");
    }
    if bot_id != guild.owner_id && !member_outranks(&guild.roles, &bot_member, &target_member) {
        return Err("Relay's highest Discord role must be above the target member.");
    }
    Ok((guild, bot_member, target_member))
}

async fn validate_external_ban_target(
    context: &DiscordContext,
    invoking_user_id: UserId,
    guild_id: GuildId,
    target_id: UserId,
) -> std::result::Result<(), &'static str> {
    if target_id == invoking_user_id {
        return Err("Relay does not allow this command to target its invoker.");
    }
    let bot_id = context.cache.current_user().id;
    if target_id == bot_id {
        return Err("Relay does not allow moderation actions against itself.");
    }
    let guild = guild_id
        .to_partial_guild(&context.http)
        .await
        .map_err(|_| "Relay could not verify the Discord server before this ban.")?;
    if target_id == guild.owner_id {
        return Err("The Discord server owner cannot be targeted by this command.");
    }

    match context.http.get_member(guild_id, target_id).await {
        Ok(_) => {
            return Err(
                "This user is currently a server member. Use the member option so Relay can enforce role hierarchy.",
            );
        }
        Err(error) if is_unknown_member_error(&error) => {}
        Err(_) => {
            return Err("Relay could not verify that this user is outside the Discord server.");
        }
    }

    let user = context
        .http
        .get_user(target_id)
        .await
        .map_err(|_| "Relay could not verify the supplied Discord user ID.")?;
    if user.bot {
        return Err("Relay custom moderation commands cannot target bot accounts.");
    }
    Ok(())
}

fn is_unknown_member_error(error: &serenity::Error) -> bool {
    matches!(
        error,
        serenity::Error::Http(serenity::http::HttpError::UnsuccessfulRequest(response))
            if response.error.code == 10_007
    )
}

async fn validate_clear_channel(
    context: &DiscordContext,
    invoking_member: &Member,
    invoking_user_id: UserId,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> std::result::Result<(), &'static str> {
    let channel = channel_id
        .to_channel(&context.http)
        .await
        .map_err(|_| "Relay could not access the selected Discord channel.")?;
    let Channel::Guild(channel) = channel else {
        return Err("Custom clear commands only support server text channels.");
    };
    if channel.guild_id != guild_id
        || !matches!(channel.kind, ChannelType::Text | ChannelType::News)
    {
        return Err("The selected channel is not a text channel in this Discord server.");
    }
    let guild = guild_id
        .to_partial_guild(&context.http)
        .await
        .map_err(|_| "Relay could not verify channel permissions.")?;
    let bot_id = context.cache.current_user().id;
    let bot_member = guild_id
        .member(context, bot_id)
        .await
        .map_err(|_| "Relay could not verify its Discord permissions.")?;
    let invoking_permissions = guild.user_permissions_in(&channel, invoking_member);
    if invoking_user_id != guild.owner_id
        && !has_permissions(invoking_permissions, Permissions::MANAGE_MESSAGES)
    {
        return Err("You cannot manage messages in the selected Discord channel.");
    }
    let bot_permissions = guild.user_permissions_in(&channel, &bot_member);
    if !has_permissions(bot_permissions, Permissions::MANAGE_MESSAGES) {
        return Err("Relay cannot manage messages in the selected Discord channel.");
    }
    Ok(())
}

fn validate_role_hierarchy(
    guild: &PartialGuild,
    invoking_member: &Member,
    bot_member: &Member,
    _target_member: &Member,
    role_id: RoleId,
) -> std::result::Result<(), &'static str> {
    let role = guild
        .roles
        .get(&role_id)
        .ok_or("The selected Discord role no longer exists.")?;
    if role_id == guild.id.everyone_role() || role.managed {
        return Err("The selected Discord role cannot be managed by Relay.");
    }
    if invoking_member.user.id != guild.owner_id
        && !member_outranks_role(&guild.roles, invoking_member, role)
    {
        return Err("Your highest Discord role must be above the selected role.");
    }
    if bot_member.user.id != guild.owner_id && !member_outranks_role(&guild.roles, bot_member, role)
    {
        return Err("Relay's highest Discord role must be above the selected role.");
    }
    Ok(())
}

fn member_outranks(roles: &HashMap<RoleId, Role>, left: &Member, right: &Member) -> bool {
    match (
        highest_member_role(roles, left),
        highest_member_role(roles, right),
    ) {
        (Some(left), Some(right)) => role_outranks(left, right),
        (Some(left), None) => left.position > 0,
        (None, Some(_)) | (None, None) => false,
    }
}

fn member_outranks_role(roles: &HashMap<RoleId, Role>, member: &Member, role: &Role) -> bool {
    highest_member_role(roles, member).is_some_and(|highest| role_outranks(highest, role))
}

fn highest_member_role<'a>(roles: &'a HashMap<RoleId, Role>, member: &Member) -> Option<&'a Role> {
    member
        .roles
        .iter()
        .filter_map(|role_id| roles.get(role_id))
        .max_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then_with(|| right.id.get().cmp(&left.id.get()))
        })
}

fn role_outranks(left: &Role, right: &Role) -> bool {
    left.position > right.position
        || (left.position == right.position && left.id.get() < right.id.get())
}

pub async fn handle_custom_component(
    core: &std::sync::Arc<AppCore>,
    context: &DiscordContext,
    component: &ComponentInteraction,
) -> bool {
    let Some((confirm, token)) = parse_component_action(&component.data.custom_id) else {
        return false;
    };
    if component
        .create_response(&context.http, CreateInteractionResponse::Acknowledge)
        .await
        .is_err()
    {
        core.bot_status.write().await.error =
            Some("Discord did not acknowledge a custom command confirmation.".into());
        return true;
    }

    let response = finish_custom_component(core, context, component, confirm, token).await;
    let edit = EditInteractionResponse::new()
        .content(response.content)
        .allowed_mentions(CreateAllowedMentions::new())
        .components(Vec::new());
    if component.edit_response(&context.http, edit).await.is_err() {
        core.bot_status.write().await.error =
            Some("Discord did not update a custom command confirmation response.".into());
    }
    true
}

async fn finish_custom_component(
    core: &std::sync::Arc<AppCore>,
    context: &DiscordContext,
    component: &ComponentInteraction,
    confirm: bool,
    token: &str,
) -> CustomCommandResponse {
    let Some(guild_id) = component.guild_id else {
        return CustomCommandResponse::error("This confirmation is no longer valid.");
    };
    let pending = core.custom_command_confirmations.lock().await.take_for(
        token,
        component.user.id,
        guild_id,
        Instant::now(),
    );
    let pending = match pending {
        Ok(pending) => pending,
        Err(ConfirmationLookup::Expired) => {
            return CustomCommandResponse::error(
                "This confirmation expired. Run the Relay command again.",
            );
        }
        Err(ConfirmationLookup::Missing | ConfirmationLookup::Mismatched) => {
            return CustomCommandResponse::error(
                "This confirmation is invalid or has already been used.",
            );
        }
    };
    if !confirm {
        log_custom_outcome(&pending.definition, "CANCELLED");
        return CustomCommandResponse::error("Custom Relay action cancelled.");
    }

    let current = core
        .config
        .read()
        .await
        .custom_commands
        .iter()
        .find(|definition| {
            definition.enabled
                && definition.name == pending.definition.name
                && **definition == pending.definition
        })
        .cloned();
    let Some(definition) = current else {
        log_custom_outcome(&pending.definition, "CONFIG_CHANGED");
        return CustomCommandResponse::error(
            "This command changed after confirmation was requested. Run it again.",
        );
    };
    let Some(member) = component.member.as_ref() else {
        log_custom_outcome(&definition, "DENIED");
        return CustomCommandResponse::error(
            "Discord member context is unavailable for this confirmation.",
        );
    };
    if pending.action.guild_id() != guild_id
        || authorize_invocation(
            &definition,
            member,
            component.user.id,
            component.channel_id,
            component.app_permissions,
        )
        .is_err()
        || validate_prepared_action(context, member, component.user.id, &pending.action)
            .await
            .is_err()
    {
        log_custom_outcome(&definition, "DENIED");
        return CustomCommandResponse::error(
            "Permissions or Discord hierarchy changed. The action was not executed.",
        );
    }

    match execute_prepared_action(core, context, &pending.action).await {
        Ok(content) => {
            log_custom_outcome(&definition, "SUCCEEDED");
            CustomCommandResponse::error(content)
        }
        Err(_) => {
            log_custom_outcome(&definition, "FAILED");
            CustomCommandResponse::error(
                "Discord rejected the action. Check Relay permissions and role hierarchy.",
            )
        }
    }
}

fn parse_component_action(custom_id: &str) -> Option<(bool, &str)> {
    let (confirm, token) = if let Some(token) = custom_id.strip_prefix(CONFIRM_COMPONENT_PREFIX) {
        (true, token)
    } else {
        let token = custom_id.strip_prefix(CANCEL_COMPONENT_PREFIX)?;
        (false, token)
    };
    if token.len() != 32 || !token.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    Some((confirm, token))
}

async fn execute_prepared_action(
    core: &AppCore,
    context: &DiscordContext,
    action: &PreparedAction,
) -> Result<&'static str> {
    match action {
        PreparedAction::Ban {
            guild_id,
            target_id,
            delete_message_days,
            reason,
            ..
        } => {
            if reason.is_empty() {
                guild_id
                    .ban(&context.http, *target_id, *delete_message_days)
                    .await?;
            } else {
                guild_id
                    .ban_with_reason(&context.http, *target_id, *delete_message_days, reason)
                    .await?;
            }
            Ok("Discord user banned.")
        }
        PreparedAction::Unban {
            guild_id,
            target_id,
            reason,
        } => {
            context
                .http
                .remove_ban(*guild_id, *target_id, audit_reason(reason))
                .await?;
            Ok("Discord user unbanned.")
        }
        PreparedAction::Kick {
            guild_id,
            target_id,
            reason,
        } => {
            if reason.is_empty() {
                guild_id.kick(&context.http, *target_id).await?;
            } else {
                guild_id
                    .kick_with_reason(&context.http, *target_id, reason)
                    .await?;
            }
            Ok("Discord member kicked.")
        }
        PreparedAction::Timeout {
            guild_id,
            target_id,
            duration_minutes,
            reason,
        } => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system clock is before the Unix epoch")?
                .as_secs();
            let expires = now
                .checked_add(u64::from(*duration_minutes) * 60)
                .context("timeout expiry overflow")?;
            let expires = Timestamp::from_unix_timestamp(i64::try_from(expires)?)?;
            let mut edit = EditMember::new().disable_communication_until_datetime(expires);
            if !reason.is_empty() {
                edit = edit.audit_log_reason(reason);
            }
            guild_id
                .edit_member(&context.http, *target_id, edit)
                .await?;
            Ok("Discord member timed out.")
        }
        PreparedAction::RemoveTimeout {
            guild_id,
            target_id,
            reason,
        } => {
            let mut edit = EditMember::new().enable_communication();
            if !reason.is_empty() {
                edit = edit.audit_log_reason(reason);
            }
            guild_id
                .edit_member(&context.http, *target_id, edit)
                .await?;
            Ok("Discord member timeout removed.")
        }
        PreparedAction::ClearMessages {
            channel_id, count, ..
        } => {
            crate::bot::clear_selected_channel(core, &context.http, *channel_id, *count).await?;
            Ok("Discord messages cleared.")
        }
        PreparedAction::AddRole {
            guild_id,
            target_id,
            role_id,
            reason,
        } => {
            context
                .http
                .add_member_role(*guild_id, *target_id, *role_id, audit_reason(reason))
                .await?;
            Ok("Discord role added.")
        }
        PreparedAction::RemoveRole {
            guild_id,
            target_id,
            role_id,
            reason,
        } => {
            context
                .http
                .remove_member_role(*guild_id, *target_id, *role_id, audit_reason(reason))
                .await?;
            Ok("Discord role removed.")
        }
    }
}

fn audit_reason(reason: &str) -> Option<&str> {
    (!reason.is_empty()).then_some(reason)
}

fn log_custom_outcome(definition: &CustomCommandDefinition, outcome: &str) {
    eprintln!(
        "Custom command: {} Action: {} Outcome: {}",
        definition.name,
        definition.action.code(),
        outcome
    );
}

fn validate_text_parameter(parameter: &TextParameter, label: &str) -> Result<()> {
    if parameter.fixed_value.chars().count() > MAX_REASON_CHARS
        || contains_disallowed_control(&parameter.fixed_value, false)
    {
        bail!("The {label} must contain at most {MAX_REASON_CHARS} printable characters.");
    }
    Ok(())
}

fn validate_integer_parameter(
    parameter: &IntegerParameter,
    minimum: u32,
    maximum: u32,
    label: &str,
) -> Result<()> {
    if !(minimum..=maximum).contains(&parameter.fixed_value) {
        bail!("The configured {label} is outside the supported range.");
    }
    Ok(())
}

fn validate_entity_parameter(
    parameter: &EntityParameter,
    label: &str,
    optional_empty_allowed: bool,
) -> Result<()> {
    match parameter.mode {
        ParameterMode::Required if parameter.fixed_value.is_empty() => Ok(()),
        ParameterMode::Required => validate_snowflake_id(&parameter.fixed_value, label),
        ParameterMode::Optional if optional_empty_allowed && parameter.fixed_value.is_empty() => {
            Ok(())
        }
        ParameterMode::Optional | ParameterMode::Fixed => {
            validate_snowflake_id(&parameter.fixed_value, label)
        }
    }
}

fn validate_id_list(values: &[String], label: &str) -> Result<()> {
    if values.len() > MAX_ACCESS_IDS {
        bail!("At most {MAX_ACCESS_IDS} {label} IDs may be configured.");
    }
    let mut unique = HashSet::new();
    for value in values {
        validate_snowflake_id(value, label)?;
        if !unique.insert(value) {
            bail!("The {label} ID list contains duplicates.");
        }
    }
    Ok(())
}

fn validate_snowflake_id(value: &str, label: &str) -> Result<()> {
    if !(17..=20).contains(&value.len())
        || !value.chars().all(|character| character.is_ascii_digit())
        || value.parse::<u64>().map_or(true, |id| id == 0)
    {
        bail!("The {label} ID is invalid.");
    }
    Ok(())
}

fn contains_disallowed_control(value: &str, allow_lines: bool) -> bool {
    value.chars().any(|character| {
        character.is_control() && !(allow_lines && matches!(character, '\n' | '\r' | '\t'))
    })
}

fn required_option(
    kind: CommandOptionType,
    name: &str,
    description: &str,
) -> (bool, CreateCommandOption) {
    (
        true,
        CreateCommandOption::new(kind, name, description).required(true),
    )
}

fn push_text_option(
    options: &mut Vec<(bool, CreateCommandOption)>,
    parameter: &TextParameter,
    name: &str,
    description: &str,
) {
    if matches!(parameter.mode, ParameterMode::Fixed) {
        return;
    }
    let required = matches!(parameter.mode, ParameterMode::Required);
    options.push((
        required,
        CreateCommandOption::new(CommandOptionType::String, name, description)
            .max_length(MAX_REASON_CHARS as u16)
            .required(required),
    ));
}

fn push_integer_option(
    options: &mut Vec<(bool, CreateCommandOption)>,
    parameter: &IntegerParameter,
    name: &str,
    description: &str,
    minimum: u64,
    maximum: u64,
) {
    if matches!(parameter.mode, ParameterMode::Fixed) {
        return;
    }
    let required = matches!(parameter.mode, ParameterMode::Required);
    options.push((
        required,
        CreateCommandOption::new(CommandOptionType::Integer, name, description)
            .min_int_value(minimum)
            .max_int_value(maximum)
            .required(required),
    ));
}

fn push_entity_option(
    options: &mut Vec<(bool, CreateCommandOption)>,
    parameter: &EntityParameter,
    kind: CommandOptionType,
    name: &str,
    description: &str,
) {
    if matches!(parameter.mode, ParameterMode::Fixed) {
        return;
    }
    let required = matches!(parameter.mode, ParameterMode::Required);
    let mut option = CreateCommandOption::new(kind, name, description).required(required);
    if matches!(kind, CommandOptionType::Channel) {
        option = option.channel_types(vec![ChannelType::Text, ChannelType::News]);
    }
    options.push((required, option));
}

#[cfg(test)]
mod tests;
