# Changelog

All notable changes to this project starting from version 0.3 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.5.0 - 2024-11-15

### Added

- Introduction of database to track persistent data.
- Keep track of command call count per user per server.
- Log command calls to database.
- Add commands to show stored stats.
- Preliminary work to serve a dashboard alongside the bot

### Changed

- Changed help generation to a prettier one
- Changed weightage for the meme gay command

### Removed

- Removed external youtube oauth plugin

## [0.4.0] - 2024-10-22

### Changed

- Make the commands under `music` available without the music namespace. These are commonly used.
- Updated `anyhow` and `thiserror` to their latest version
- Changed `dotenv` to `dotenvy` due to unmaintained

## [0.3.0] - 2024-10-20

### Added

- Add new `CHANGELOG` file
- Meme command category. The only available command is `gay`.
- New dependency: crate `rand`.

[0.4.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/luqmanishere/ayaya-discord-bot/compare/v0.2.0...v0.3.0
