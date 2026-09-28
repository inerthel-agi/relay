# Custom Relay Commands Design

Status: implemented on 2026-08-14 and covered by automated tests.

## Understanding

- Keep the existing Relay subcommands under a **Default Commands** section.
- Add a separate **Custom Commands** section in the desktop application.
- A custom command uses the form `/relay <name>` and executes exactly one predefined action.
- Command definitions belong to the local Relay installation, not to the Discord user who edits them.
- The same custom command list is registered for every guild where the Relay bot is installed.
- Support up to 16 configured custom commands. Discord limits one command to 25 top-level options and Relay already has 9 default subcommands.
- Initial actions are ban, unban, kick, timeout, remove timeout, clear messages, add role, remove role, and predefined reply.

## Assumptions and non-functional requirements

- No arbitrary scripts, shell commands, webhooks, URLs, or multi-action workflows are supported.
- Command synchronization runs only when custom commands are saved.
- The implementation uses the existing Serenity client, Tauri command path, local configuration store, and frontend stack.
- No new privileged Discord gateway intent is required. User and member values come from command interactions; explicit IDs or mentions are accepted where enumeration is unavailable.
- No new dependency is required.
- Configuration and command registration failures preserve the previous active configuration.
- Local logs contain only the command name, action code, and outcome. They never contain a target, reason, predefined reply, or submitted parameter value.
- A live Discord smoke test must use a dedicated test guild and test accounts. Automated tests must not mutate a real guild.

## Selected approach

Build dynamic flat subcommands inside the existing `/relay` command.

Alternatives rejected:

- `/relay-custom <name>` isolates permissions but does not match the requested command form.
- `/relay run command:<name>` avoids schema synchronization but makes commands less discoverable and less convenient.

## Configuration model

`AppConfig` gains a bounded `custom_commands` list. Each entry contains:

- A unique normalized name and a Discord description.
- An enabled flag.
- One tagged `CustomCommandAction` variant.
- Action-specific parameter policies: required, optional, or fixed locally.
- An access policy with optional administrator-only mode, additional required member permissions, allowed user IDs, allowed role IDs, and allowed channel IDs.

Names must be valid lowercase Discord option names, must not collide with default Relay subcommands, and must be unique after normalization. Descriptions, IDs, counts, durations, and fixed text values are bounded and reject control characters.

Minimum permissions and mandatory confirmation are derived from the action and are not user-disableable. Extra restrictions may only narrow access.

## Action schemas

| Action | Discord parameters | Enforced checks |
|---|---|---|
| Ban | current member or external user ID, reason, message deletion window | Ban Members, guild hierarchy for current members, owner and bot protection, verified absence for external IDs |
| Unban | user ID, reason | Ban Members, valid banned user ID |
| Kick | member, reason | Kick Members, guild hierarchy, owner protection |
| Timeout | member, duration, reason | Moderate Members, bounded duration, guild hierarchy |
| Remove timeout | member, reason | Moderate Members, guild hierarchy |
| Clear messages | channel, count | Manage Messages, bounded count, existing bulk-delete safeguards |
| Add role | member, role, reason | Manage Roles, member and role hierarchy |
| Remove role | member, role, reason | Manage Roles, member and role hierarchy |
| Reply | fixed text and visibility | No mass mentions; public or ephemeral output |

Each parameter policy controls whether Discord prompts for a value, whether it is required, or whether Relay uses a locally fixed value. Fixed private values are never copied into logs or error messages.

## Registration and persistence flow

1. The panel submits the complete candidate custom-command list.
2. Rust validates the list and builds the candidate `/relay` schema.
3. Relay registers the candidate schema through the existing Discord HTTP client in `BotRuntime`.
4. Relay saves the candidate configuration atomically and swaps the in-memory configuration.
5. If local persistence fails, Relay attempts to restore the previous Discord schema and keeps the previous configuration active.

The existing static constructor becomes configuration-aware. It always adds the nine default subcommands and then adds enabled custom commands. Saving default command toggles does not change their schema. Saving custom commands performs a Discord synchronization without restarting the bot.

## Authorization and execution flow

The top-level `/relay` command can no longer rely on one Administrator default for every subcommand. Runtime authorization becomes mandatory:

- Existing default commands continue to require Administrator, preserving current behavior.
- Custom commands use the action minimum plus the configured access policy.
- Direct messages and interactions without a guild member context are rejected.
- Relay validates the invoking member, target member, target role, bot permissions, guild ownership, channel restrictions, and role hierarchy before preparing an action.
- Missing bot permissions are reported ephemerally. Relay never requests Administrator; the generated invite uses the union of permissions needed by enabled actions.

Destructive actions create a one-time in-memory confirmation bound to the invoking user, guild, command, and normalized parameters. It expires after 60 seconds. Authorization and hierarchy checks run again when the confirmation is accepted. Replayed, expired, or mismatched confirmations fail closed.

Predefined replies disable `@everyone`, `@here`, role mentions, and user mentions by default. The normal command result is ephemeral unless the Reply action explicitly selects public output.

## Desktop interface

The Commands page contains:

1. **Default Commands**: the existing nine switches and permission notes.
2. **Custom Commands**: a `0 / 16` counter, Add command button, and cards showing command name, action, enabled state, synchronization state, Edit, and Delete.

The custom-command editor has four sections:

1. Name and description with a live `/relay <name>` preview.
2. Predefined action selection.
3. Action parameters with required, optional, or fixed modes.
4. Access restrictions for administrators, extra permissions, users, roles, and channels.

Discord IDs and mentions can be pasted directly. Relay's existing discovered text channels are offered as choices. The editor displays required bot and member permissions as read-only requirements and prevents invalid commands from being submitted.

Save states are `Unsaved`, `Validating`, `Syncing`, `Active`, and `Error`. Errors expose no command parameter values.

## Error handling

- Discord registration errors keep the old schema and local configuration.
- API action errors produce a bounded ephemeral explanation and a sanitized local outcome code.
- Rate limits are delegated to Serenity's Discord HTTP client.
- Partial multi-action state is impossible because every custom command has one action.
- Confirmation state is memory-only, bounded, time-limited, and removed after success or failure.

## Test strategy

- Configuration round-trip, migration, bounds, duplicate and reserved names.
- Serialized Discord schema for every action and parameter mode.
- Default-command Administrator preservation after removing the top-level permission gate.
- Permission matrix for invoking members and bot permissions.
- User, role, and channel allowlists.
- Guild owner, self-target, bot-target, member hierarchy, and role hierarchy rejection.
- One-time confirmation ownership, expiry, replay, and authorization recheck.
- Pure action planning tests plus bounded HTTP-facing adapters.
- Frontend payload, editor validation, rendering, and synchronization-state tests.
- Existing frontend, Rust, Clippy, packaging, and portable smoke-test suites.

## Decision log

- Use one predefined action per custom command to keep execution atomic and auditable.
- Store one local command list per Relay installation and register it globally for that Discord application.
- Keep Default Commands and Custom Commands visually and structurally separate.
- Allow user, role, and channel restrictions in addition to derived Discord permissions.
- Use typed Discord parameters with required, optional, and fixed modes.
- Use flat dynamic `/relay <name>` subcommands despite the 16-command limit because it matches the requested workflow.
- Preserve Administrator-only access for existing commands through runtime checks.
- Require one-time confirmation and a second authorization check for destructive actions.
- Keep the action registry closed and reject arbitrary execution mechanisms.

## Implemented verification

- Rust tests cover configuration round-trips, bounds, reserved and duplicate names, every action schema, parameter ordering, permission derivation, access restrictions, role-order helpers, confirmation ownership, expiry and replay, mention suppression, and preservation of the default-command Administrator gate.
- Frontend tests cover the closed action list, Discord ID and mention normalization, separation from Default Commands, and the Tauri synchronization hook.
- The complete Rust and frontend suites, formatting checks, and strict Clippy pass locally.
- No automated test executes a Discord moderation action. Ban, kick, timeout, message deletion, role mutation, global propagation latency, and rollback against Discord still require a dedicated test guild before they can be considered live-smoke-tested.
