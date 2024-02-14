## advtm
app that helps to live this life

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
source example.env && ./advtm
```