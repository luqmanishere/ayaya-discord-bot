# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Calendar Versioning](https://calver.org/).

## [2025.11.3-5] - 2025-11-03

### Added

- Add new justfile commands for jj release

## [2025.11.3-2] - 2025-11-03

### Fixed

- Unwrap_or_default when converting format_id

## [2025.11.3-1] - 2025-11-03

### Changed

- Push latest tag to docker

## [2025.11.3-0] - 2025-11-03

### Added

- Add release recipe to automate version releases

### Fixed

- Fix date tags again
- Filter out m3u8 streams and update ytdlp for nsig
- Use CARGO_MANIFEST_DIR for test fixture paths in tracker
- More workflow matching fixes

## [2025.10.30-0] - 2025-10-29

### Changed

- Styling: run rustfmt

### Fixed

- Fix some clippy warnings
- Fix(ci): versioning does not use regex

## [2025.10.27-0] - 2025-10-27

### Changed

- Change versioning to calver
- Error instead of panic during cookies decryption
- Migrate from miette and thiserror to snafu
- Update flake.lock 20251025

### Fixed

- Log the type of url used
- Use serenity with all codecs
- Update runner to trixie

### Removed

- Remove some expects

## [0.15.0] - 2025-10-20

### Changed

- Example show stats command
- Initial tracker implementation
- Update nix locks
- Update cargo locks
- Update flake.lock
- Update gitignore
- Update nix inputs
- Update to rust 1.88

### Removed

- Remove unused file
- Removed some unused code paths

## [0.14.0] - 2025-06-21

### Added

- Add repeat and rename soundboard feature

## [0.13.1] - 2025-06-11

### Fixed

- Rename Reason to Status
- Old database configurations should not be valid

## [0.13.0] - 2025-06-11

### Added

- Add soundboard functionality and linger

### Changed

- Migrate to local database

### Fixed

- Cfg! breaking tests
- Logic error when deciding inactivity
- Disallow unwraps

## [0.12.0] - 2025-05-20

### Added

- Add command play_next

### Changed

- Make subcrates use workspace version
- Broadcast playlist info
- Ping command now shows api ping
- Move items in queue command

### Fixed

- Clippy lints

### Removed

- Remove unneeded dependencies

## [0.11.0] - 2025-05-16

### Changed

- Better, more resilient seek implementation
- Filter urls, remove tracking links
- Use the dyn Any track data from songbird 0.5, show requester in embed
- Queued songs stats & play autocomplete
- Server command call ranking
- Factored out permissions into its own handler

### Fixed

- Impl Default for Metrics
- Fix the caching
- Fix clippy errors

## [0.10.1] - 2025-02-12

### Fixed

- Fix for some music dropping in the middle

## [0.10.0] - 2025-02-12

### Added

- Add our cookies to containers in general
- Add `dep_version` command

### Changed

- Use github new arm builder
- Update shuttle & use the new platform
- Update the about command with build details

### Fixed

- Abstract some code into functions
- Make sure pip always upgrades its dependencies

## [0.9.2] - 2024-12-15

### Changed

- Move workspace dependencies to the top
- Reorganize code into workspaces

### Fixed

- Change the player name

## [0.9.1] - 2024-12-08

### Fixed

- "secure" endpoint with http basic auth

## [0.9.0] - 2024-12-08

### Added

- Add metrics v1

## [0.8.0] - 2024-12-04

### Added

- Add cache implementation

### Fixed

- Fix command logging on each call taking time

## [0.7.4] - 2024-12-02

### Fixed

- Fix redundant empty track code
- Fix driver lock being held for too long

## [0.7.3] - 2024-12-02

### Added

- Add command `shuffle_play`

### Fixed

- Zombie processes lint

## [0.7.2] - 2024-11-30

### Added

- Add shuffle command

### Fixed

- Stop sqlx logging from pollutitng the logs

## [0.7.1] - 2024-11-19

### Added

- Add activity

### Changed

- Wtf
- V0.7.1

### Fixed

- Fix clippy and fmt

## [0.7.0] - 2024-11-18

### Changed

- Try to fix ytdlp cookies
- Update changelog
- Move database calls into data manager

## [0.6.0] - 2024-11-16

### Added

- Add command restriction features

### Changed

- V0.6.0

### Fixed

- Fix lint

## [0.5.1] - 2024-11-14

### Fixed

- Fix oauth

## [0.5.0] - 2024-11-14

### Added

- Add command_user_allow_table

### Changed

- V0.5.0
- Update
- Alot of things
- Serve webpage alongside bot

### Fixed

- Weightage on memes
- Fix wrong version

## [0.4.1] - 2024-10-22

### Added

- Add missing help texts

## [0.4.0] - 2024-10-22

### Changed

- Update changelog and formatting
- Copy commands out of subcommands
- Update dependencies
- Update badges
- Update badge
- Ci name
- Deploy only on version tags

### Fixed

- Run rust checks only if rust is changed
- Fix diff link

### Removed

- Remove unused comment

## [0.3.0] - 2024-10-20

### Added

- Add gay command

### Changed

- Bump to 0.3.0

## [0.2.0] - 2024-10-19

### Added

- Add loop command
- Add 45s timeout
- Add pagination to queue
- Add youtube-dl deps
- Add new skip command
- Add docker build to gha
- Add missing pub
- Added the delete command
- Added nowplaying command
- Added inactivity auto kick
- Add README and fix formatting
- Add Github CI
- Added join guard

### Changed

- Todos
- Improve playlist handling so youtube likes us
- Ordering is now correct with playlists
- Move to our own implementation that uses youtube-dl-rs
- Update Cargo.lock
- Modularize audio commands
- Allow running in nix env
- Pretty embeds in discord
- Deploy to shuttle
- Filter out unnecessary
- Push to loki
- Starting diagnostics and error messages
- Use sane errors in code
- Leave when alone in channel
- Shuttle.rs integration
- Implement playlist support
- Restructuring
- Swap some plain messages for embeds
- Normalize files
- Implement notifications when a track starts playing
- Implement search command
- Cargo update
- Temp fix clippy
- Attempting to fix
- Updated bot to latest serenity and songbird versions
- Small improvements to the code
- Fix clippy warnings
- Implement search in play option
- Initiial code for feature `search`
- Fix the inactive counter behaviour
- Updated README
- Minor quality of life changes
- Publish to github
- Fixed project name
- Initial Commit

### Fixed

- Clippy
- Fix clippy
- Reply when the queue is empty instead of erroring on emptyness
- Cleanup some code that should be unused
- Make crate only internal stuff
- Have shuttle apt update first before installing packages
- Clippy
- Fix queue
- Deploy should only run on code changes
- Clippy errors
- Youtube playback now uses oauth
- Show help when music is called without subcommands
- Fix error handling in play command
- Use futures crate stream instead
- Limit spawned tasks
- Update to latest version
- Flesh out more errors
- Flesh out more errors
- Move deps install to main for visibility
- Log in localtime or +8 or UTC
- Fix attempt 2
- Fixed prefix

### Removed

- Delete from queue
- Remove unused workflow

[2025.11.3-5]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v2025.11.3-2..v2025.11.3-5
[2025.11.3-2]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v2025.11.3-1..v2025.11.3-2
[2025.11.3-1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v2025.11.3-0..v2025.11.3-1
[2025.11.3-0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v2025.10.30-0..v2025.11.3-0
[2025.10.30-0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v2025.10.27-0..v2025.10.30-0
[2025.10.27-0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.15.0..v2025.10.27-0
[0.15.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.14.0..v0.15.0
[0.14.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.13.1..v0.14.0
[0.13.1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.13.0..v0.13.1
[0.13.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.12.0..v0.13.0
[0.12.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.11.0..v0.12.0
[0.11.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.10.1..v0.11.0
[0.10.1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.10.0..v0.10.1
[0.10.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.9.2..v0.10.0
[0.9.2]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.9.1..v0.9.2
[0.9.1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.9.0..v0.9.1
[0.9.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.8.0..v0.9.0
[0.8.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.7.4..v0.8.0
[0.7.4]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.7.3..v0.7.4
[0.7.3]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.7.2..v0.7.3
[0.7.2]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.7.1..v0.7.2
[0.7.1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.7.0..v0.7.1
[0.7.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.6.0..v0.7.0
[0.6.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.5.1..v0.6.0
[0.5.1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.5.0..v0.5.1
[0.5.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.4.1..v0.5.0
[0.4.1]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.4.0..v0.4.1
[0.4.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.3.0..v0.4.0
[0.3.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.2.0..v0.3.0

<!-- generated by git-cliff -->
