# Fahare Horizon 0.3.0 — experimental native rollover

Unofficial Rust adaptation of [Fahare](https://github.com/qixils/fahare/tree/dd4250a5ce3879e06070a6f72400e1a00909cc97), by Lexi Larkin (qixils), MIT.

**Experimental: the engine integration has not been compiled or game-tested. Use a disposable dedicated server and independent backups.**

## Install
Add https://github.com/gurumumu/Fahare-Horizon to Horizon's GitHub mod linker. Targets Horizon 26.3 source commit de8b4e2526df9d7f611515b78123a26a3914176f. Exact SDK availability and mixin compatibility must be checked in the launcher's build log.

## What changed in 0.3
- World-local, UUID-keyed append-only death journal, synced to disk on every death. The counter survives an ordinary restart; a new save starts a new journal.
- A corrupted/truncated/full journal, failed write, or invalid configuration disables resetting rather than guessing. /fahare status reports the reason.
- Spectator mode is requested on respawn and reconnect for players who have exhausted their lives.
- The candidate save must match the canonical active world captured at startup. Rollover still waits for a successful clean dedicated-server shutdown, after the engine releases its storage lock.
- Dependency-free tests cover round rules, journal replay, truncated journals and archive behavior.

## How the reset works
When the last online player loses their final life (or an operator runs /fahare reset), the mod requests normal server shutdown through native mixins. After a successful shutdown it renames the entire active save into a uniquely named sibling backup directory. All dimensions, player data and the journal are retained there. On the next normal server start Horizon should generate a new world. Set level-seed blank for random seeds.

The server still needs to be started again by an administrator or supervisor. Players reconnect. This is not a live reset. Run only one server process per save; the archive is a same-volume rename, not an independent backup. Never use the mod on important saves before testing.

## Settings
Optional file fahare.conf in the dedicated engine's working directory:
```
lives=1
auto-reset=true
any-death=false
backup=true
```
Comments beginning with # and blank lines are accepted. Invalid values disable resetting. Backups cannot be disabled. For hardcore set difficulty=hard and hardcore=true in server.properties. For multiple lives, use hardcore=false so the vanilla one-life rule does not override the mod's counter.

Commands: /fahare status (everyone), /fahare reset (permission level 4). If a journal is corrupt, stop the server, keep a copy of the file and repair or restore it before restarting. To restore a world, stop the server, move the current save to another backup location, and move the archived save back to the original path. Never merge two world folders.

## Testing
Run the standalone core tests from the repository root with a Rust compiler supporting edition 2024:
```
rustc --edition=2024 --test tests/core.rs -o core-tests
./core-tests
```
On Windows run core-tests.exe instead. This does not compile or test Horizon hooks/mixins. In a disposable dedicated server test: two players dying in succession, reconnecting as spectator, restarting between deaths, failure of a journal write, and successful regeneration after clean shutdown. Verify archive contents before restarting.

## Remaining limitations
No verified native engine build, no automated server relaunch, no live in-session reset, no integrated/LAN rollover, no automatic transfer of datapacks or world-local configurations to the new save, no restore command. No external downloads, telemetry or subprocesses in the mod code. The journal uses synchronous disk writes only on deaths, so slow storage may add a brief tick delay.

## Source references
- [Horizon clean server shutdown](https://github.com/arismoko/horizon/blob/de8b4e2526df9d7f611515b78123a26a3914176f/crates/horizon-client/src/game/world/server_lifecycle/mod.rs)
- [Normal stop path](https://github.com/arismoko/horizon/blob/de8b4e2526df9d7f611515b78123a26a3914176f/crates/horizon-client/src/game/interface/dedicated/transfer.rs)
- [Native mixins](https://github.com/arismoko/horizon/blob/de8b4e2526df9d7f611515b78123a26a3914176f/docs/modding/src/concepts/native-route/2-mixins.md)
