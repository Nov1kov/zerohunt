use std::env;
use ethers::utils::{hex, keccak256, secret_key_to_address};
use rand::rngs::OsRng;
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::task;
use std::time::{Duration, Instant};
use ethers::abi::Address;
use ethers::core::k256::ecdsa::SigningKey;
use rand::Rng;


const DEFAULT_FACTORY_ADDRESS: &str = "0x9fBB3DF7C40Da2e5A0dE984fFE2CCB7C47cd0ABf";
const DEFAULT_PROXY_BYTECODE: &str = "67363d3d37363d34f03d5260086018f3";

fn parse_arguments() -> (usize, Arc<dyn AddressGenerator>) {
    // Получаем и разбираем аргументы командной строки
    let args: Vec<String> = env::args().collect();
    let target_zeros: usize = args.get(2)
        .unwrap_or(&"8".to_string())
        .parse()
        .expect("Invalid number of target zeros");

    match args.get(1).map(|s| s.as_str()) {
        Some("eoa") => return (target_zeros, Arc::new(EoaGenerator {})),
        Some("create3") => {
            let deployer_address = args.get(3)
                .expect("Deployer address is required for CREATE3 mode")
                .parse()
                .expect("Invalid deployer address format");

            let factory_address = args.get(4)
                .map(|addr| addr.parse().expect("Invalid factory address format"))
                .unwrap_or_else(|| Address::from_str(DEFAULT_FACTORY_ADDRESS).unwrap());

            let proxy_byte_code = args.get(5)
                .map(|code| hex::decode(code).expect("Invalid proxy bytecode"))
                .unwrap_or_else(|| hex::decode(DEFAULT_PROXY_BYTECODE).expect("Invalid default proxy bytecode"));

            return (target_zeros, Arc::new(Create3Generator::new(deployer_address, factory_address, proxy_byte_code)));
        }
        _ => {
            eprintln!("Usage: zerohung <mode> <target_zeros> [deployer_address] [create3_factory_address] [proxy_byte_code]");
            eprintln!("Modes:");
            eprintln!("  eoa      - Generate an EOA address with leading zeros.");
            eprintln!("  create3  - Generate a CREATE3 contract address with leading zeros.");
            panic!("Wrong arguments");
        }
    }
}


enum GenerationResult {
    EOA { signer: SigningKey },
    Create3 { salt: Vec<u8> },
}


trait AddressGenerator: Send + Sync {
    fn generate_address(&self) -> (Address, GenerationResult);
    fn generate_private_or_salt(&self, data: GenerationResult) -> String;
}

struct EoaGenerator {}

impl AddressGenerator for EoaGenerator {
    fn generate_address(&self) -> (Address, GenerationResult) {
        let signer = SigningKey::random(&mut OsRng);
        (secret_key_to_address(&signer), GenerationResult::EOA { signer })
    }

    fn generate_private_or_salt(&self, data: GenerationResult) -> String {
        match data {
            GenerationResult::EOA { signer } => hex::encode(signer.to_bytes()),
            _ => panic!("Invalid generation result"),
        }
    }
}

struct Create3Generator {
    deployer_address: Address,
    factory_address: Address,
    proxy_byte_code: Vec<u8>,
}

impl Create3Generator {
    pub fn new(deployer_address: Address, factory_address: Address, proxy_byte_code: Vec<u8>) -> Self {
        Create3Generator {
            deployer_address,
            factory_address,
            proxy_byte_code,
        }
    }
}

impl AddressGenerator for Create3Generator {
    fn generate_address(&self) -> (Address, GenerationResult) {
        let salt: [u8; 32] = OsRng.gen();
        let hashed_salt = keccak256(&[self.deployer_address.as_bytes(), &salt].concat());

        let proxy_bytecode_hash = keccak256(&[
            b"\xff",
            self.factory_address.as_bytes(),
            &hashed_salt,
            &keccak256(&self.proxy_byte_code),
        ].concat());

        let proxy_address = &proxy_bytecode_hash[12..32];


        let deployed_address_hash = keccak256(&[b"\xd6\x94", proxy_address, &[0x01]].concat());
        let deployed_address = Address::from_slice(&deployed_address_hash[12..32]);
        (deployed_address, GenerationResult::Create3 { salt: salt.to_vec() })
    }

    fn generate_private_or_salt(&self, data: GenerationResult) -> String {
        match data {
            GenerationResult::Create3 { salt } => hex::encode(salt),
            _ => panic!("Invalid generation result"),
        }
    }
}

#[tokio::main]
async fn main() {
    let (target_zeros, generator) = parse_arguments();
    let num_threads = num_cpus::get();
    println!("Number of threads: {}\nfinding first wallet with {} leading zeros", num_threads, target_zeros);
    let max_zero_count = Arc::new(AtomicUsize::new(0));
    let max_order_chars = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    let start_time = Instant::now();
    let total_generated = Arc::new(AtomicUsize::new(0));

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = Arc::clone(&stop_signal);
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        stop_signal_clone.store(true, Ordering::SeqCst);
        println!("Received Ctrl+C. Stopping...");
    });

    let file = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("scanned_keys.txt")
            .expect("Unable to open file"),
    ));

    for _ in 0..num_threads {
        let max_zero_count = Arc::clone(&max_zero_count);
        let max_order_chars = Arc::clone(&max_order_chars);
        let total_generated = Arc::clone(&total_generated);
        let stop_signal = Arc::clone(&stop_signal);
        let file = Arc::clone(&file);
        let generator = generator.clone();

        let handle = task::spawn_blocking(move || {
            loop {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                let (address, result) = generator.generate_address();

                let max_zero_count_value = max_zero_count.load(Ordering::Relaxed);
                let mut zero_count = 0;
                let address_bytes = address.as_bytes();
                for &byte in &address_bytes[0..] {
                    if byte == 0 {
                        zero_count += 2;
                    } else {
                        zero_count += byte.leading_zeros() as usize / 4;
                        break;
                    }
                }

                if zero_count < max_zero_count_value {
                    total_generated.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                let address_str = format!("{:?}", address);
                let chars_in_order = address_str
                    .chars()
                    .skip(zero_count + 2)
                    .fold((None, 0, 0), |(prev_char, max_count, current_count), c| {
                        if Some(c) == prev_char {
                            (prev_char, max_count.max(current_count + 1), current_count + 1)
                        } else {
                            (Some(c), max_count.max(current_count), 1)
                        }
                    }).1;

                let mex_chars_in_order_value = max_order_chars.load(Ordering::Relaxed);
                if chars_in_order < mex_chars_in_order_value && zero_count == max_zero_count_value {
                    total_generated.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                max_zero_count.store(zero_count, Ordering::SeqCst);
                // ignore simple addresses
                if zero_count < 3 {
                    total_generated.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                if chars_in_order > mex_chars_in_order_value {
                    max_order_chars.store(chars_in_order, Ordering::SeqCst);
                }

                let private_key = generator.generate_private_or_salt(result);

                {
                    let mut file = file.lock().unwrap();
                    writeln!(
                        file,
                        "{}\t{}\t{}\t{}",
                        total_generated.load(Ordering::Relaxed),
                        address_str,
                        zero_count,
                        private_key
                    )
                        .expect("Unable to write data to file");
                }

                println!(
                    "New best address with {} leading zeros and {} repeating characters: {}",
                    zero_count, chars_in_order, address_str
                );
                total_generated.fetch_add(1, Ordering::Relaxed);

                if zero_count >= target_zeros {
                    break;
                }
            }
        });

        handles.push(handle);
    }

    let rate_handle = {
        let total_generated = Arc::clone(&total_generated);
        let start_time = start_time.clone();

        task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(20)).await;
                let count = total_generated.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = count as f64 / elapsed.max(1.0);

                println!("Address generation rate: {:.2} wallets/sec", rate);
            }
        })
    };

    for handle in handles {
        let _ = handle.await;
    }

    rate_handle.abort();
    println!("Found address with the most leading zeros:");
}