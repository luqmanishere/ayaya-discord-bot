default:
    just -l

# create a fresh sqlite db and generate entities 
refresh-sqlite: fresh-sqlite generate-sqlite

# refresh dev sqlite db
fresh-sqlite:
    sea-orm-cli migrate fresh -d migration-sqlite -u "sqlite://dev/stats.sqlite?mode=rwc"

# generate entities for sqlite db
generate-sqlite:
    sea-orm-cli generate entity --date-time-crate time -o entity-sqlite/src -u "sqlite://dev/stats.sqlite?mode=rwc" -l --with-prelude all

bump-minor:
    git cliff --bump minor -o CHANGELOG.md
    cargo set-version --bump minor
