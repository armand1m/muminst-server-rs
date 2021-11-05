# muminst-server-rs

This is the muminst server implementation in rust.

This is currently a work in progress, and many features are still unstable. Follow up on the feature compatibility list to get familiar with ongoing progress:

- [x] dotenv 
- [ ] Logger
    - [x] discord client connection event 
    - [ ] http server startup
    - [ ] http server requests
- [x] Storage
    - [x] sqlite3 database
    - [x] diesel orm setup
    - [x] auto-migration
- [x] Modularized
- [x] Docker Image
    - [x] debian version
    - [ ] alpine version
- [ ] Kubernetes manifests
    - [ ] PersistentVolumeClaim
    - [ ] VolumeSnapshot
    - [ ] Deployment
    - [ ] Service
    - [ ] VirtualService
    - [ ] DestinationRule 
    - [ ] Kustomize
- [ ] CI
    - [ ] Setup github actions
    - [ ] Build docker image
    - [ ] Deploy application to kubernetes
- [ ] Observability
    - [ ] [Setup sentry](https://docs.sentry.io/platforms/rust/guides/actix-web/)
- [ ] HTTP Server
    - [x] GET /sounds
    - [x] GET /assets
    - [ ] GET /download-sounds
    - [x] POST /play-sound
    - [ ] POST /upload
        - [x] Checks for supported file types
        - [x] Checks if sound already exists in the database
        - [x] Uploads sound to disk
        - [x] Inserts sound record in the database
        - [ ] Inserts given tags
    - [ ] POST /add-tags/:sound_id
- [ ] Websocket Server
    - [ ] /ws route
        - [ ] Notifies locked state to clients
        - [ ] Manages connections correctly
- [x] Discord Client
    - [x] Reconnects in case of disconnect events from the discord server
    - [x] Enable consumers to play audio outside of a command function. _(e.g.: from an endpoint handler)_
- [ ] Telegram Client
    - [ ] Sends audio to telegram in case the /play-sound endpoint receives `telegram` as a client
- [x] Thread management
    - [x] Supports multiple worker threads
    - [x] Terminates the entire process and child threads in case one gets terminated.