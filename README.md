# playground 
space to create

## Delivery

### Env
```bash
ssh root@78.40.219.186 mkdir /root/playground/
```

### Apps
read4me
```bash
scp ./target/x86_64-unknown-linux-musl/release/read4me root@78.40.219.186:/root/playground/
```
sub4usd
```bash
scp ./target/x86_64-unknown-linux-musl/release/sub4usd root@78.40.219.186:/root/playground/
```

### Configs
read4me
```bash
scp ./read4me/example.env root@78.40.219.186:/root/playground/read4me.env
```
sub4usd
```bash
scp ./sub4usd/example.env root@78.40.219.186:/root/playground/sub4usd.env
```

### Backup
download
```bash
scp -r root@78.40.219.186:/root/playground ~/Desktop/
```
upload
```bash
scp -r ~/Desktop/playground root@78.40.219.186:/root/
```