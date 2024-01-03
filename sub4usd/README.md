## sub4usd
app that provide subscription to usd price in RUB

### Compilation
```bash
cargo install cross --git https://github.com/cross-rs/cross
```
```bash
CROSS_CONFIG=./Cross.toml cross build -r
```

### Run
Environment variables must be provided, see `example.env`.
```bash
source example.env && ./sub4usd
```