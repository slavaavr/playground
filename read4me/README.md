## Read4Me
app that syntheses text to speech
 
### Compilation
```bash
cargo install cross --git https://github.com/cross-rs/cross
```
```bash
CROSS_CONFIG=./Cross.toml cross build -r
``` 

### Environment
on empty linux server run:
```bash
apt-get update -y
apt-get install vim -y
apt-get install mc -y
apt-get install screen -y
``` 
```bash
apt-get install snapd -y && snap install --classic certbot && ln -s /snap/bin/certbot /usr/bin/certbot
certbot certonly --standalone
``` 

### Run
Environment variables must be provided, see `example.env`.
```bash
source example.env && ./read4me
```