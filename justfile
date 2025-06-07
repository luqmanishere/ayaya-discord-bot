sqlite-url := "sqlite://dev/stats.sqlite?mode=rwc"

default:
    just -l

# create a fresh sqlite db and generate entities 
refresh-sqlite: fresh-sqlite generate-sqlite

# refresh dev sqlite db
fresh-sqlite:
    sea-orm-cli migrate fresh -d migration-sqlite -u {{sqlite-url}}

# generate entities for sqlite db
generate-sqlite:
    sea-orm-cli generate entity --date-time-crate time -o entity-sqlite/src -u "sqlite://dev/stats.sqlite?mode=rwc" -l --with-prelude all

# generate a new migration with NAME
generate-migration NAME:
    sea-orm-cli migrate generate -d migration-sqlite -u {{sqlite-url}} {{NAME}}

db-up:
    sea-orm-cli migrate up -d migration-sqlite -u {{sqlite-url}}

db-down:
    sea-orm-cli migrate down -d migration-sqlite -u {{sqlite-url}}

bump-minor:
    git cliff --bump minor -o CHANGELOG.md
    cargo set-version --bump minor
