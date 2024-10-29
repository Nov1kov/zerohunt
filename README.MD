# ZeroHunt
ZeroHunt is a Rust-based tool designed to generate Ethereum wallets with a specific focus on finding addresses that start with a maximum number of leading zeros. This project utilizes multi-threading with Tokio to efficiently scan through potential wallet addresses.

## Usage
By default, the program will continuously generate Ethereum wallets and output addresses with the maximum number of leading zeros found. The results will be logged in a file named `scanned_keys.txt`.

This script also identifies and saves addresses with **consecutive repeating characters** after leading zeros. For example:
`0x0000...bbbb...`

### build
```bash
git clone https://github.com/Nov1kov/zerohunt.git
cd zerohunt
cargo build
```

### Find EOA with leading zeros
```bash
cargo run --release eoa 9
```
a single argument: the **target number of leading zeros**. If not provided, it defaults to **8**.

### Find salt to create smart contract with leading zeros via create3
```bash
cargo run --release create3 10 0x0000093849ce47cd01ad266e2d083690601898c9
```
two arguments: the **target number of leading zeros** and the **executor EOA** to find the salt for.

### Running with Docker
```bash
docker build -t zero_hunt .
docker run -d --name zerohunt -v $(pwd):/app zerohunt eoa 9
```

run to find the salt for address with 10 leading zeros 
```bash
docker run -d --name zerohunt -v $(pwd):/app zerohunt create3 10 0x0000093849ce47cd01ad266e2d083690601898c9
```

### Example output

```
New best address with 5 leading zeros and 2 repeating characters: 0x0000093849ce47cd01ad266e2d083690601898c9
New best address with 5 leading zeros and 3 repeating characters: 0x00000dccc200ac0550fc22cf7ad1821d1ce65b6f
New best address with 5 leading zeros and 3 repeating characters: 0x00000a65bbd9ab3f1e4ac747dfd9836051666427
Address generation rate: 207941.45 address/sec
New best address with 6 leading zeros and 2 repeating characters: 0x000000161056f1a577193819fa1d57727b9b6aba
New best address with 7 leading zeros and 2 repeating characters: 0x0000000fa18ba5fb3516b94187059bae8ffc9675
Address generation rate: 207941.45 address/sec
```
