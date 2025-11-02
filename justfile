sqlite-url := "sqlite://dev/stats.sqlite?mode=rwc"

alias t := test

default:
    just -l

# create a fresh sqlite db and generate entities
refresh-sqlite-all: fresh-sqlite generate-sqlite-all

# refresh dev sqlite db
fresh-sqlite:
    sea-orm-cli migrate fresh -d migration-sqlite -u {{sqlite-url}}

# generate entities for sqlite db
generate-sqlite-all:
    sea-orm-cli generate entity --date-time-crate time -o entity-sqlite/src -u "sqlite://dev/stats.sqlite?mode=rwc" -l --with-prelude all

generate-sqlite-tables TABLES:
    sea-orm-cli generate entity --date-time-crate time -o entity-sqlite/src -u "sqlite://dev/stats.sqlite?mode=rwc" -l --with-prelude all --tables {{TABLES}}

# generate a new migration with NAME
generate-migration NAME:
    sea-orm-cli migrate generate -d migration-sqlite -u {{sqlite-url}} {{NAME}}

db-up:
    sea-orm-cli migrate up -d migration-sqlite -u {{sqlite-url}}

db-down:
    sea-orm-cli migrate down -d migration-sqlite -u {{sqlite-url}}

bump:
    #!/usr/bin/env bash
    DATE=$(date +%Y.%-m.%-d)
    # Find the highest pre-release number for today's date
    LATEST_TAG=$(git tag -l "v${DATE}-*" | sort -V | tail -n 1)
    if [ -z "$LATEST_TAG" ]; then
        PRERELEASE=0
    else
        PRERELEASE=$(echo "$LATEST_TAG" | sed "s/v${DATE}-//")
        PRERELEASE=$((PRERELEASE + 1))
    fi
    VERSION="${DATE}-${PRERELEASE}"
    git cliff --bump auto -o CHANGELOG.md --tag "v${VERSION}"
    cargo set-version "${VERSION}"

release:
    #!/usr/bin/env bash
    set -e
    just bump
    VERSION=$(grep '^version = ' Cargo.toml | head -n1 | sed 's/version = "\(.*\)"/\1/')
    git add Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore(release): release ${VERSION}" -m "changelog: ignore"
    git tag "v${VERSION}"
    echo "Tagged version ${VERSION}"

test:
    cargo nextest run

podman-build:
    podman build --tag "luqmanishere/ayayadc-dev" .

podman-run:
    podman run -v ./secrets:/secrets -v ./dev/local_share:/root/.local/share/ayayadc -e DISCORD_TOKEN_FILE=/secrets/dev-discordtoken -e AGE_SECRET_KEY_FILE=/secrets/dev-age -it localhost/luqmanishere/ayayadc-dev:latest
