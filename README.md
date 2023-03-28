# Picross w: WASM

The web-assembly portion of [Picross W](https://github.com/BluAtlas/Picross-W)

## Building

To build yourself, install [Rust](https://www.rust-lang.org/).

Next install `wasm32-unknown-unknown` with the following command:

```sh
rustup target add wasm32-unknown-unknown
```

Then build the project with:

```sh
cargo build --release --target wasm32-unknown-unknown

wasm-bindgen --out-dir ./out --target web ./target/wasm32-unknown-unknown/release/picross_w.wasm
```

Clone [Picross W](https://github.com/BluAtlas/Picross-W) and place the contents of `/out` and `/assets` into your local [Picross W](https://github.com/BluAtlas/Picross-W) repo at `/public/out` and `/public/assets` respectively.
