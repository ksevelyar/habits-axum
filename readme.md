# Habits
[![habits-vue](habits-vue.png)](https://github.com/ksevelyar/habits-vue)

## Documentation
* [Chains](doc/chains.md)
* [Charts](doc/charts.md)
* [Devices](doc/devices.md)
* [Errors](doc/errors.md)
* [Metrics](doc/metrics.md)
* [Nginx](doc/nginx.md)
* [Notifications](doc/notifications.md)
* [Sessions](doc/sessions.md)
* [Tasks](doc/tasks.md)
* [Users](doc/users.md)

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
