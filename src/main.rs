use std::env;
use ethers::signers::{Signer, Wallet};
use ethers::utils::{hex, secret_key_to_address};
use rand::rngs::OsRng;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::task;
use std::time::{Duration, Instant};
use ethers::core::k256::ecdsa::SigningKey;

#[tokio::main]
async fn main() {
    let max_zeros: usize = env::args().nth(1).unwrap_or("8".to_string()).parse().expect("Invalid number");
    let num_threads = num_cpus::get();
    println!("Number of threads: {}\nfinding first wallet with {} leading zeros", num_threads, max_zeros);
    let max_zero_count = Arc::new(AtomicUsize::new(0));
    let max_order_chars = Arc::new(AtomicUsize::new(0));
    let best_wallet = Arc::new(Mutex::new(None));

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
        let best_wallet = Arc::clone(&best_wallet);
        let total_generated = Arc::clone(&total_generated);
        let stop_signal = Arc::clone(&stop_signal);
        let file = Arc::clone(&file);

        let handle = task::spawn_blocking(move || {
            loop {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                let signer = SigningKey::random(&mut OsRng);
                let address = secret_key_to_address(&signer);

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

                let wallet = Wallet::new_with_signer(signer, address, 1);
                let private_key = hex::encode(wallet.signer().to_bytes());

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

                {
                    let mut best_wallet_lock = best_wallet.lock().unwrap();
                    *best_wallet_lock = Some(wallet);
                }

                println!(
                    "New best address with {} leading zeros and {} chars in order: {}",
                    zero_count, chars_in_order, address_str
                );
                total_generated.fetch_add(1, Ordering::Relaxed);

                if zero_count >= max_zeros {
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

                println!("Count rate: {:.2} wallets/sec", rate);
            }
        })
    };

    for handle in handles {
        let _ = handle.await;
    }

    rate_handle.abort();

    let best_wallet_lock = best_wallet.lock().unwrap();
    if let Some(wallet) = &*best_wallet_lock {
        println!("Found wallet with the most leading zeros:");
        println!("Address: {:?}", wallet.address());
        println!("Private Key: {}", hex::encode(wallet.signer().to_bytes()));
    } else {
        println!("No wallet found.");
    }
}