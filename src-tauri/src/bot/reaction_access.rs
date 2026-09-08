use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serenity::all::{ChannelType, Guild, GuildChannel, GuildId, Role, UserId};

use crate::state::AppCore;

const BOT_NOT_RUNNING: &str = "The Discord bot is not running. Start Relay and try again.";
const BOT_NOT_CONNECTED: &str = "The Discord bot is not connected. Reconnect Relay and try again.";
const GUILDS_UNAVAILABLE: &str =
    "Discord server data is unavailable. Reconnect Relay and try again.";
const NO_GUILDS: &str = "Relay is connected but has not joined a Discord server.";

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionAccessOptions {
    pub channels: Vec<ReactionAccessChannel>,
    pub roles: Vec<ReactionAccessRole>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionAccessChannel {
    pub id: String,
    pub name: String,
    pub guild_id: String,
    pub guild_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionAccessRole {
    pub id: String,
    pub name: String,
    pub guild_id: String,
    pub guild_name: String,
}

/// Build the reaction access catalog from the connected Discord cache.
///
/// Guild and role data are sent in gateway events and kept in Serenity's cache. Using those
/// entries avoids one REST request per guild and means a refresh cannot accidentally replace a
/// configured ID with an empty list when Discord is temporarily unavailable.
pub(crate) async fn get_reaction_access_options(
    core: &Arc<AppCore>,
) -> Result<ReactionAccessOptions> {
    let cache = {
        let runtime = core.bot_runtime.lock().await;
        let runtime = runtime.as_ref().context(BOT_NOT_RUNNING)?;
        runtime.cache.clone()
    };

    if !core.bot_status.read().await.connected {
        bail!(BOT_NOT_CONNECTED);
    }

    let bot_id = cache.current_user().id;
    let guild_ids = cache.guilds();
    if guild_ids.is_empty() {
        bail!(NO_GUILDS);
    }

    let mut result = ReactionAccessOptions::default();
    let mut missing_guild = false;
    for guild_id in guild_ids {
        let Some(guild) = cache.guild(guild_id) else {
            // Cache::guilds also includes unavailable guilds. Do not return a partial catalog:
            // callers must retain their configured IDs and explain that a refresh is needed.
            missing_guild = true;
            continue;
        };
        let (channels, roles) = catalog_for_guild(&guild, bot_id);
        result.channels.extend(channels);
        result.roles.extend(roles);
    }

    if missing_guild {
        bail!(GUILDS_UNAVAILABLE);
    }

    sort_catalog(&mut result);
    Ok(result)
}

fn selectable_channel(channel: &GuildChannel, guild: &Guild, bot_id: UserId) -> bool {
    matches!(channel.kind, ChannelType::Text | ChannelType::News)
        && guild
            .members
            .get(&bot_id)
            .map(|member| {
                guild
                    .user_permissions_in(channel, member)
                    .contains(serenity::all::Permissions::VIEW_CHANNEL)
            })
            // Keep the existing routing fallback: if the member is not in the cache yet, do not
            // hide an otherwise valid channel until the gateway fills the member entry.
            .unwrap_or(true)
}

fn selectable_role(role: &Role, guild_id: GuildId) -> bool {
    role.id != guild_id.everyone_role()
}

fn catalog_for_guild(
    guild: &Guild,
    bot_id: UserId,
) -> (Vec<ReactionAccessChannel>, Vec<ReactionAccessRole>) {
    let guild_id = guild.id;
    let guild_id_text = guild_id.to_string();
    let guild_name = guild.name.clone();
    let channels = guild
        .channels
        .values()
        .filter(|channel| selectable_channel(channel, guild, bot_id))
        .map(|channel| ReactionAccessChannel {
            id: channel.id.to_string(),
            name: channel.name.clone(),
            guild_id: guild_id_text.clone(),
            guild_name: guild_name.clone(),
        })
        .collect();
    let roles = guild
        .roles
        .values()
        .filter(|role| selectable_role(role, guild_id))
        .map(|role| ReactionAccessRole {
            id: role.id.to_string(),
            name: role.name.clone(),
            guild_id: guild_id_text.clone(),
            guild_name: guild_name.clone(),
        })
        .collect();
    (channels, roles)
}

fn sort_catalog(options: &mut ReactionAccessOptions) {
    options.channels.sort_by(|left, right| {
        left.guild_name
            .cmp(&right.guild_name)
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
    options.roles.sort_by(|left, right| {
        left.guild_name
            .cmp(&right.guild_name)
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_channel_and_role_catalog_with_guild_context() {
        let mut options = ReactionAccessOptions {
            channels: vec![ReactionAccessChannel {
                id: "20".into(),
                name: "clips".into(),
                guild_id: "2".into(),
                guild_name: "Relay server".into(),
            }],
            roles: vec![ReactionAccessRole {
                id: "30".into(),
                name: "Member".into(),
                guild_id: "2".into(),
                guild_name: "Relay server".into(),
            }],
        };
        sort_catalog(&mut options);

        let value = serde_json::to_value(options).expect("catalog should serialize");
        assert_eq!(
            value,
            serde_json::json!({
                "channels": [{
                    "id": "20",
                    "name": "clips",
                    "guildId": "2",
                    "guildName": "Relay server"
                }],
                "roles": [{
                    "id": "30",
                    "name": "Member",
                    "guildId": "2",
                    "guildName": "Relay server"
                }]
            })
        );
    }

    #[test]
    fn maps_only_visible_text_channels_and_non_everyone_roles() {
        use serenity::all::{
            ChannelId, Member, PermissionOverwrite, PermissionOverwriteType, Permissions,
        };

        let guild_id = GuildId::new(42);
        let mut guild = Guild::default();
        guild.id = guild_id;
        guild.name = "Relay server".into();
        guild.owner_id = UserId::new(2);

        let bot_id = UserId::new(1);
        let mut bot_member = Member::default();
        bot_member.user.id = bot_id;
        guild.members.insert(bot_id, bot_member);

        let mut everyone = Role::default();
        everyone.id = guild_id.everyone_role();
        everyone.permissions = Permissions::VIEW_CHANNEL;
        guild.roles.insert(everyone.id, everyone);

        let mut visible_channel = GuildChannel::default();
        visible_channel.id = ChannelId::new(20);
        visible_channel.guild_id = guild_id;
        visible_channel.kind = ChannelType::Text;
        visible_channel.name = "clips".into();
        guild.channels.insert(visible_channel.id, visible_channel);

        let mut private_channel = GuildChannel::default();
        private_channel.id = ChannelId::new(21);
        private_channel.guild_id = guild_id;
        private_channel.kind = ChannelType::Text;
        private_channel.name = "private".into();
        private_channel
            .permission_overwrites
            .push(PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
            });
        guild.channels.insert(private_channel.id, private_channel);

        let mut member = Role::default();
        member.id = serenity::all::RoleId::new(30);
        member.name = "Member".into();
        guild.roles.insert(member.id, member);

        let (channels, roles) = catalog_for_guild(&guild, bot_id);
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, "20");
        assert_eq!(channels[0].guild_id, "42");
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].id, "30");
        assert_eq!(roles[0].guild_name, "Relay server");
    }

    #[test]
    fn only_text_and_news_channels_are_selectable() {
        let guild = Guild::default();
        let bot_id = UserId::new(1);
        for kind in [ChannelType::Text, ChannelType::News] {
            let mut channel = GuildChannel::default();
            channel.kind = kind;
            assert!(selectable_channel(&channel, &guild, bot_id));
        }
        for kind in [
            ChannelType::Voice,
            ChannelType::Category,
            ChannelType::Forum,
            ChannelType::Stage,
        ] {
            let mut channel = GuildChannel::default();
            channel.kind = kind;
            assert!(!selectable_channel(&channel, &guild, bot_id));
        }
    }

    #[test]
    fn everyone_role_is_not_selectable_but_other_roles_are() {
        use serenity::all::RoleId;

        let guild_id = GuildId::new(42);
        let mut everyone = Role::default();
        everyone.id = guild_id.everyone_role();
        let mut member = Role::default();
        member.id = RoleId::new(43);
        assert!(!selectable_role(&everyone, guild_id));
        assert!(selectable_role(&member, guild_id));
    }
}
