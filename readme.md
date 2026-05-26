# Habits

## Run
```
nix develop
cargo watch -x run
```

## Database
```
psql -U postgres -c "CREATE DATABASE habits_axum;"
```

regenerate sqlx cache:
```
cargo sqlx prepare
```
