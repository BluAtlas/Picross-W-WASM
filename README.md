# Picross w: WASM

The web-assembly portion of [Picross W](https://github.com/BluAtlas/Picross-W)

To build yourself, run the following two commands:

```sh
cargo build --release --target wasm32-unknown-unknown

wasm-bindgen --out-dir ./out --target web ./target/wasm32-unknown-unknown/release/picross_w.wasm
```

Clone [Picross W](https://github.com/BluAtlas/Picross-W), Then place the contents of `/out` and `/assets` into your local [Picross W](https://github.com/BluAtlas/Picross-W) repo at `/public/out` and `/public/assets` respectively.
