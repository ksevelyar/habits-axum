# Notifications

## Goals
* One scheduler per user
* Multiple client connections (web, mobile, esp32) receive the same notifications from the user scheduler
* Tasks are loaded from pg only when the user is connected

## Connect
```
websocat ws://localhost:3003/websocket/notifications --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3ODA5MzY2NzcsImVtYWlsIjoia3NldmVseWFyQGdtYWlsLmNvbSIsImRldmljZV9pZCI6bnVsbCwiZGV2aWNlX25hbWUiOm51bGx9.ZDylN8H7MV0rV8Ya0ZYV-Iq0ny5NcI-MSkwaI3UER4A"
```
