// Dependency-free core test harness. Run: rustc --edition=2024 --test src/core_tests.rs -o core-tests
#[path = "round.rs"] mod round;
#[path = "journal.rs"] mod journal;
#[path = "rollover.rs"] mod rollover;
