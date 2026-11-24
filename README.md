# Fahare Horizon — experimental preview

Unofficial Rust adaptation of [Fahare](https://github.com/qixils/fahare/tree/dd4250a5ce3879e06070a6f72400e1a00909cc97), originally by Lexi Larkin (qixils), under MIT.

**Not a complete or tested replacement for Fahare.** The launcher format targets Horizon 26.3, using exact SDK pins taken from [a native mod example](https://github.com/Jorungandr/horizon-rust-mods/tree/main/mods/native-v2/probe). Compilation and in-game behavior have not been verified.

Current implementation:
- Rust round tracker, keyed by player UUID, notified by the player-respawn hook (not immediate death).
- Defaults to one life and announces when all online players have exhausted their lives.
- State is in memory, reset on server startup.
- /fahare status reports round state; /fahare reset is operator-only and deliberately refuses destructive actions.
- No file deletion, network requests, external processes, inventory clearing or world modification by this code.

Missing: actual vanilla world regeneration, seed changes, backups, persistent death tracking, configurable lives and spectator enforcement. The documented runtime-level API only permits mod-owned dimensions and does not provide vanilla noise generation; do not substitute a flat/void world for a real hardcore reset.

Install for experimental testing via Horizon's Add from GitHub using this repository URL. Use a disposable game instance, and check the native build log. This preview is not suitable if automatic world regeneration is required.
