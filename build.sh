# Prerequisites (one-time)
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.114   # exact version from bevy/Cargo.lock

# Build
cd /home/ubuntu/workspace/fps_walkthrough
cargo build --release --target wasm32-unknown-unknown

# Generate JS bindings
mkdir -p dist
wasm-bindgen \
  --out-name fps_walkthrough \
  --out-dir dist \
  --target web \
  target/wasm32-unknown-unknown/release/fps_walkthrough.wasm

# Copy HTML and serve
cp index.html dist/
python3 -m http.server 8080 --directory dist
# Open http://localhost:8080