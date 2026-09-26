APP := `basename $(pwd)`
profile := env_var_or_default('PROFILE', 'debug')
features := env_var_or_default('FEATURES', '')

# Show available targets
help:
    @just --list

# Install development tools (cargo plugins)
setup:
    @echo "Installing development tools..."
    @cargo install --locked prek
    @prek install
    @cargo install --locked cargo-llvm-cov
    @cargo install --locked cargo-nextest
    @cargo install --locked cargo-deny
    @cargo install --locked cargo-shear
    @cargo install --locked typos-cli
    @echo "✓ Development tools installed"

# Rust
# Build application binary
build *opts="":
    @echo "Building {{APP}} ({{profile}} profile{{ if features != "" { ", features=" + features } else { "" } }})"
    @cargo build {{ if profile == "release" { "--release" } else { "" } }} {{ if features != "" { "--features " + features } else { "" } }} {{opts}}

# Install application into ~/.cargo/bin
install *opts="":
    @echo "Installing {{APP}}"
    @cargo install --path . {{opts}}

# Run tests
test *opts="--workspace":
    @cargo nextest run {{opts}}

# Run integration tests
test-integration filter="":
    @cargo nextest run --workspace -E 'binary(/integration_/){{ if filter != "" { " & test(/" + filter + "/)" } else { "" } }}' --run-ignored all

# Generate code coverage report (requires: cargo install cargo-llvm-cov)
test-coverage *opts="--workspace":
    @cargo llvm-cov nextest {{opts}}

# Run documentation tests
test-doc *opts="--workspace":
    @cargo test --doc {{opts}}

# Lint code
lint *opts="":
    cargo clippy --workspace --fix --allow-dirty --allow-staged --no-deps --all-targets --all-features {{opts}} -- -D warnings
    @cargo fmt --all -- --check

# Format code
fmt:
    @cargo fmt --all

# Check code for typos
typos:
    @typos --write-changes

# Tidy dependencies
tidy:
    @cargo update

# Download dependencies
deps:
    @cargo fetch

# Flags after the recipe reach the criterion harness:
#   --save-baseline NAME   save this run as a named baseline to diff against later
#   --baseline NAME        compare this run against a saved baseline
#   --profile-time N       profile for N s -> target/criterion/*/profile/profile.pb
#   --sample-size N        override the per-benchmark sample count
#   --list                 list benchmark ids without running
#   <regex>                run only matching benchmark ids, e.g. `just bench fibonacci`
# Run benchmarks.
bench *opts="":
    @cargo bench -- {{opts}}

# Run application.
[positional-arguments]
run +args="--help":
    @cargo run {{ if profile == "release" { "--release" } else { "" } }} {{ if features != "" { "--features " + features } else { "" } }} -- "$@"
