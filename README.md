# Fahare Horizon 0.2.0 — experimental native rollover

Unofficial Rust port of [Fahare](https://github.com/qixils/fahare/tree/dd4250a5ce3879e06070a6f72400e1a00909cc97) by Lexi Larkin (qixils), MIT.

**Experimental, not build- or game-tested. Use a disposable dedicated server first.**

## Installation
Add https://github.com/gurumumu/Fahare-Horizon in Horizon's GitHub mod linker. Targets Horizon 26.3 source commit de8b4e2526df9d7f611515b78123a26a3914176f. The exact SDK must be available to the launcher. Mixins are version-specific.

## Alternative to Paper live resets
Horizon runtime dimensions do not support vanilla noise terrain or replacing vanilla dimensions. This version uses the dedicated engine's native clean stop and shutdown boundary instead. On final death or an operator's /fahare reset, it requests the normal stop path; after successful teardown and final save it attempts to rename the entire active world to a unique sibling backup folder. The next normal server start should generate a new world according to server.properties (blank level-seed for random seeds). The server must be started again manually or by the administrator's supervisor. Players must reconnect. Do not run multiple server processes against one save.

All world-local player data and dimensions are archived together; nothing is recursively deleted. A failed save/archive keeps the original save, and this mod does not intentionally archive on crash, integrated server, or ordinary shutdown. The backup is a filesystem rename, not an independent off-device copy. Keep separate backups.

## Settings
Optional plain-text file fahare.conf in the dedicated engine's working directory:
```
lives=1
auto-reset=true
any-death=false
```
For hardcore rules set difficulty=hard and hardcore=true in server.properties. Always archive: no destructive backup=false option. /fahare status shows state; /fahare reset requires permission level 4.

## Limitations
No instant in-session regeneration, no automatic server relaunch, no LAN/single-player rollover. The death counter is in memory and is cleared at server start; spectator enforcement is attempted on respawn, and has not been verified. Existing world-local datapacks/configs are archived rather than automatically copied to the new world. The engine mixins and native SDK compatibility have not been compiled or tested. Do not use on important saves before testing. No external downloads, telemetry or subprocesses by this mod.

## Source references
- [Horizon clean server shutdown](https://github.com/arismoko/horizon/blob/de8b4e2526df9d7f611515b78123a26a3914176f/crates/horizon-client/src/game/world/server_lifecycle/mod.rs)
- [Normal stop path](https://github.com/arismoko/horizon/blob/de8b4e2526df9d7f611515b78123a26a3914176f/crates/horizon-client/src/game/interface/dedicated/transfer.rs)
- [Native mixins](https://github.com/arismoko/horizon/blob/de8b4e2526df9d7f611515b78123a26a3914176f/docs/modding/src/concepts/native-route/2-mixins.md)
