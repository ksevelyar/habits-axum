# Habits
[![habits-vue](habits-vue.png)](https://github.com/ksevelyar/habits-vue)

## Documentation
* [Features](doc/features/)
* [Boundaries](doc/boundaries.md)

## Run
```
nix develop
cargo sqlx database create
cargo watch -x run
```

## Build
```
cargo sqlx prepare
nix build
```
