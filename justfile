check:
  cargo test --all
  cargo fmt
  cargo clippy --all-targets --all-features -- -D warnings

