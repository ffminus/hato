# list all available commands
help:
    @just --list

# lint and type check code with static analyzers
check:
    cargo fmt --check
    cargo clippy --tests --benches -- --deny warnings
    cargo check  --tests --benches
    RUSTDOCFLAGS='--deny warnings' cargo doc

# run code quality and logic checks
ci: check
    cargo      test -- --test-threads 1
    cargo miri test -- --test-threads 1
