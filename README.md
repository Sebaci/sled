# sled

Fast pipelines for Advent of Code.

Sled is an experimental DSL and interpreter written in Rust. The language is
early and intentionally implementation-guided.

## Current CLI

Run a one-line program against stdin:

```sh
cargo run -- --expr 'input lines map len sum' < input.txt
```

Short aliases are available for the highest-frequency transforms:

```sh
cargo run -- --expr 'input L m i sum' < input.txt
```

Or run a program file:

```sh
cargo run -- program.sled --input input.txt
```
