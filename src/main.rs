use std::env;
use ethers::signers::{LocalWallet, Signer};
use ethers::utils::hex;
use rand::rngs::OsRng;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::task;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    let max_zeros: usize = env::args().nth(1).unwrap_or("8".to_string()).parse().expect("Invalid number");
    let num_threads = num_cpus::get();
    println!("Number of threads: {}\nfinding first wallet with {} leading zeros", num_threads, max_zeros);
    let scanned_count = Arc::new(Mutex::new(0)); // Счетчик для отслеживания обработанных кошельков
    let max_zero_count = Arc::new(Mutex::new(0)); // Для отслеживания максимального количества нулей
    let best_wallet = Arc::new(Mutex::new(None)); // Для сохранения лучшего кошелька

    let mut handles = Vec::new();

    let start_time = Instant::now();
    let total_generated = Arc::new(Mutex::new(0));

    let stop_signal = Arc::new(Mutex::new(false)); // Флаг для остановки
    let stop_signal_clone = Arc::clone(&stop_signal);
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        let mut stop_signal = stop_signal_clone.lock().unwrap();
        *stop_signal = true; // Устанавливаем флаг остановки
        println!("Received Ctrl+C. Stopping...");
    });


    for _ in 0..num_threads {
        let scanned_count = Arc::clone(&scanned_count);
        let max_zero_count = Arc::clone(&max_zero_count);
        let best_wallet = Arc::clone(&best_wallet);
        let total_generated = Arc::clone(&total_generated);
        let stop_signal = Arc::clone(&stop_signal);

        let handle = task::spawn_blocking(move || {
            loop {
                if *stop_signal.lock().unwrap() {
                    break;
                }

                let wallet = LocalWallet::new(&mut OsRng);
                let address = format!("{:?}", wallet.address());
                let private_key = hex::encode(wallet.signer().to_bytes());

                // Убираем префикс "0x" и подсчитываем количество нулей
                let address_without_prefix = &address[2..];
                let zero_count = address_without_prefix.chars().take_while(|&c| c == '0').count();

                let mut max_zero_count_lock = max_zero_count.lock().unwrap();
                if *max_zero_count_lock > max_zeros {
                    break;
                }
                if zero_count > *max_zero_count_lock {
                    *max_zero_count_lock = zero_count;

                    let mut best_wallet_lock = best_wallet.lock().unwrap();
                    *best_wallet_lock = Some(wallet.clone());

                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("scanned_keys.txt")
                        .expect("Unable to open file");

                    writeln!(
                        file,
                        "{}\t{}\t{}\t{}",
                        *scanned_count.lock().unwrap(),
                        address,
                        zero_count,
                        private_key
                    )
                        .expect("Unable to write data to file");

                    println!(
                        "New best address with {} leading zeros: {}",
                        zero_count, address
                    );
                }

                *scanned_count.lock().unwrap() += 1;
                *total_generated.lock().unwrap() += 1; // Увеличиваем общий счетчик
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
                let count = *total_generated.lock().unwrap();
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = count as f64 / elapsed.max(1.0); // Избегаем деления на ноль

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
