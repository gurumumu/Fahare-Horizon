// Dependency-free core test harness. Run: rustc --edition=2024 --test tests/core.rs -o core-tests
#[path = "../src/round.rs"] mod round;
#[path = "../src/journal.rs"] mod journal;
#[path = "../src/rollover.rs"] mod rollover;
