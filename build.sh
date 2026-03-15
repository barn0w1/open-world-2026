# Prerequisites (one-time)
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.114   # exact version from bevy/Cargo.lock

# Build
cd /home/ubuntu/workspace/open-world-2026
cargo build --release --target wasm32-unknown-unknown

# Generate JS bindings
mkdir -p dist
wasm-bindgen \
  --out-name open-world-2026 \
  --out-dir dist \
  --target web \
  target/wasm32-unknown-unknown/release/open-world-2026.wasm

# Copy HTML and serve
cp index.html dist/
python3 -m http.server 8080 --directory dist
# Open http://localhost:8080