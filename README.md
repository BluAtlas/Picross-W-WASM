# Picross w: WASM

The web-assembly portion of [Picross W](github.com) <!-- add full link -->

To build yourself, run the following two commands:

```sh
cargo build --release --target wasm32-unknown-unknown

wasm-bindgen --out-dir ./out --target web ./target/wasm32-unknown-unknown/release/picross_w.wasm
```

Then place the contents of `/out` and `/assets` into [Picross W](github.com) at `/public/out` and `/public/assets` respectively.
