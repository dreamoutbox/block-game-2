# TODO

- [x] build WASM and play in web. create new branch.

- [x] camera mode. first person with flying.

- [x] add block place and break.

- [x] add player tools bar player gui. currently show grass block, dirt block.

- [x] fix dig below dirt. see empty block. should add stone texture (assets/minecraft/textures/block/stone.png).

- [x] make flying camera not passthrough block. unless press `C`.

- [x] move world generation code to separate module. no generate every game open.

- [x] create new world create function. save to files with `bincode` crate.

- [x] add bevy_egui. create main memu. pause(esc) menu. world create/load/save gui. make it most simple for dev now.

- [ ] add ./assets and resource pack zip to .gitignore. remove those from github.

- [x] fix error in WASM egui `No fonts available until first call to Context::run()`.

- [ ] web WASM. save data in IndexedDB.

- [ ] multiplayer early code implementation. create new git branch. play in browser (WASM). use websocket. backend server with tokio + websocket.

