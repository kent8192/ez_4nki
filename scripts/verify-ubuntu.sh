#!/usr/bin/env bash
# Run inside scripts/Dockerfile.ubuntu, with /source read-only and /out writable.
set -euo pipefail
tar -C /source --exclude=node_modules --exclude=target --exclude=dist --exclude=artifacts --exclude=.git --exclude=src-tauri/runtime -cf - . | tar -C /work -xf -
cd /work
npm ci
npm test
npm run build
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p kotoba-core --example portability_fixture -- check /out/macos.age
cargo run --locked -p kotoba-core --example portability_fixture -- create /out/linux.age
npm run tauri -- build --debug --bundles deb
cp target/debug/bundle/deb/*.deb /out/
