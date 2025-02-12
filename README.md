# Ayaya

[![Rust CI](https://github.com/luqmanishere/ayaya-discord-bot/actions/workflows/checks.yaml/badge.svg?branch=main)](https://github.com/luqmanishere/ayaya-discord-bot/actions/workflows/checks.yaml)[![Shuttle Deploy](https://github.com/luqmanishere/ayaya-discord-bot/actions/workflows/shuttle-deploy.yaml/badge.svg)](https://github.com/luqmanishere/ayaya-discord-bot/actions/workflows/shuttle-deploy.yaml)

A Discord music bot (for now)

Built upon [serenity-rs/serenity](https://github.com/serenity-rs/serenity) and [serenity-rs/songbird](https://github.com/serenity-rs/songbird)

### TODO

- [x] Detect inactivity and leave automatically
- [x] Queue management
- [x] Youtube playlists
- [ ] Minigames

## Docker usage

Example docker command with the required env vars:

```sh
docker run -e DISCORD_TOKEN="Discord bot token" \
    -e DATABASE_URL="mysql database url" \
    -e AGE_SECRET_KEY="age secret key" \
    luqmanishere/ayayadc
```

We also support reading from files:

```sh
# TODO
docker run -e DISCORD_TOKEN="Discord bot token" \
    -e DATABASE_URL="mysql database url" \
    -e AGE_SECRET_KEY="age secret key" \
    luqmanishere/ayayadc

```

Excuse the bad design while we migrate from shuttle.
