# muminst-server-rs

This is the muminst server implementation in rust.

This is currently a work in progress, and many features are still unstable.

Follow up on the feature compatibility list to get familiar with ongoing progress:

- [x] dotenv 
- [x] Logger
    - [x] uses env_logger
    - [x] uses [actix logger middleware](https://actix.rs/actix-web/actix_web/middleware/struct.Logger.html)
    - [x] discord client connection event 
    - [x] discord client reconnection event
    - [x] discord client startup
    - [x] http server startup
    - [x] http server requests
- [x] Storage
    - [x] sqlite3 database
    - [x] diesel orm setup
    - [x] run pending migrations when `RUN_PENDING_MIGRATIONS` env var is set to `true`
- [x] Modularized
- [x] Docker Image
    - [x] debian version
    - [ ] alpine version _(lots of problems with musl)_
- [x] Kubernetes manifests
    - [x] PersistentVolumeClaim
    - [x] Deployment
    - [x] Service
    - [x] VirtualService
    - [x] DestinationRule 
    - [x] Kustomize
- [x] CI
    - [x] Setup github actions
    - [x] Build docker image
    - [x] Deploy application to kubernetes
- [ ] Observability
    - [ ] [Setup sentry](https://docs.sentry.io/platforms/rust/guides/actix-web/)
- [ ] HTTP Server
    - [x] GET /sounds
    - [x] GET /assets
    - [ ] GET /download-sounds
    - [x] POST /play-sound
    - [x] POST /upload
        - [x] Checks for supported file types
            - [x] mp3
            - [x] webm
            - [x] ogg
            - [ ] wav
        - [x] Checks if sound already exists in the database
        - [x] Uploads sound to disk
        - [x] Inserts sound record in the database
        - [x] Inserts given tags
    - [x] PUT /add-tags/:sound_id
- [x] Websocket Server
    - [x] actix websocket setup 
    - [x] /ws route
        - [x] Notifies locked state to clients
        - [x] Manages connections correctly
- [x] Discord Client
    - [x] Reconnects in case of disconnect events from the discord server
    - [x] Enable consumers to play audio outside of a command function. _(e.g.: from an endpoint handler)_
      - Audio can be played through messaging to the Discord Actor address, which is available in Actix Web Data context in case you need access from a middleware or an endpoint handler.
- [ ] Telegram Client
    - [ ] Sends audio to telegram in case the `POST /play-sound` endpoint receives `telegram` as a client
- [x] Thread management
    - [x] Supports multiple worker threads
    - [x] Terminates the entire process and child threads in case one gets terminated.

