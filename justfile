# List all recipes
list:
    just --list --unsorted

################################################################################
# Build
# Build for production.
build:
    cargo build --release

# Build for development.
build-dev:
    cargo build

################################################################################
# Releases
# Build the binary for a given tag.
release tag:
    git switch --detach {{ tag }}
    just build

# Publish a tag to Cargo.
publish tag:
    git switch --detach {{ tag }}
    cargo publish

################################################################################
# Tests
# Run all tests.
test-all:
    cargo test

# Test all binaries.
test-bins:
    cargo test --bins

# Test all lib-docs.
test-docs:
    cargo test --doc

# Test all library unit tests.
test-lib:
    cargo test --lib

# Test all tests in tests folder.
test-units:
    cargo test --tests

################################################################################
# Miscellaneous
# Switch to the latest tag.
[private]
latest:
    git switch --detach $(git describe --tags $(git rev-list --tags --max-count=1))
