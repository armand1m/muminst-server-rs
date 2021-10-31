# muminst-server-rs

This is the muminst server implementation in rust.

This is currently a work in progress, and many features are still unstable. Follow up on the feature compatibility list to get familiar with ongoing progress:

- [x] dotenv 
- [x] Logger
- [-] Storage
- [x] Modularized
- [-] HTTP Server
    - [ ] GET /sounds
    - [x] GET /assets
    - [ ] GET /download-sounds
    - [-] POST /play-sound
    - [ ] POST /upload
    - [ ] POST /add-tags/:id
- [ ] Websocket Server
    - [ ] /ws route
        - [ ] Notifies locked state to clients
        - [ ] Manages connections correctly
- [x] Discord Client
    - [x] Enable consumers to play audio outside of a command function. _(e.g.: from an endpoint handler)_