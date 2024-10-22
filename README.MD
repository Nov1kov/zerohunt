# ZeroHunt
ZeroHunt is a Rust-based tool designed to generate Ethereum wallets with a specific focus on finding addresses that start with a maximum number of leading zeros. This project utilizes multi-threading with Tokio to efficiently scan through potential wallet addresses.

## Usage
By default, the program will continuously generate Ethereum wallets and output addresses with the maximum number of leading zeros found. The results will be logged in a file named scanned_keys.txt.

```bash
git clone https://github.com/yourusername/ZeroHunt.git
cd ZeroHunt
cargo build
cargo run
```

## Running with Docker
```bash
docker build -t zero_hunt .
docker run zero_hunt
```
