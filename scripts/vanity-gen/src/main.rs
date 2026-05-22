use coins_bip39::{English, Mnemonic};
use coins_bip32::prelude::*;
use k256::ecdsa::SigningKey;
use sha3::{Keccak256, Digest};
use rand::thread_rng;
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use std::time::{Instant, Duration};
use std::sync::Mutex;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let targets: Vec<String> = if args.len() > 1 {
        args[1].split(',').map(|s| s.to_lowercase()).collect()
    } else {
        vec![
            "cafe".to_string(),
            "babe".to_string(),
            "face".to_string(),
            "fade".to_string(),
            "feed".to_string(),
            "aced".to_string(),
            "beef".to_string(),
            "deed".to_string(),
            "abba".to_string(),
            "dace".to_string(),
        ]
    };

    let duration_secs: u64 = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(300); // 5 минут по умолчанию

    let valid_hex: &str = "0123456789abcdef";
    for t in &targets {
        let invalid: Vec<char> = t.chars().filter(|c| !valid_hex.contains(*c)).collect();
        if !invalid.is_empty() {
            eprintln!("❌ ERROR: target '{}' contains non-hex characters: {:?}", t, invalid);
            eprintln!("   Ethereum addresses use only: 0-9, a-f");
            std::process::exit(1);
        }
    }

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    println!("🎯 Searching for {} vanity word(s): {:?}", targets.len(), targets);
    println!("   Duration: {} seconds | Threads: {}", duration_secs, num_threads);
    println!();

    let found_results: Mutex<Vec<(String, String, String, u64)>> = Mutex::new(Vec::new());
    let found_flags: Vec<AtomicBool> = targets.iter().map(|_| AtomicBool::new(false)).collect();
    let deadline = Instant::now() + Duration::from_secs(duration_secs);

    let total_attempts = AtomicBool::new(false); // we track per-thread

    (0..num_threads)
        .into_par_iter()
        .for_each(|thread_id| {
            let mut rng = thread_rng();
            let mut attempts: u64 = 0;

            loop {
                if Instant::now() > deadline {
                    return;
                }

                attempts += 1;

                let mnemonic = match Mnemonic::<English>::new_with_count(&mut rng, 12) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let seed = match mnemonic.to_seed(Some("")) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let xprv = match XPriv::root_from_seed(&seed, None) {
                    Ok(x) => x,
                    Err(_) => continue,
                };

                let derived = match xprv.derive_path("m/44'/60'/0'/0/0") {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                let signing_key: &SigningKey = derived.as_ref();
                let priv_key = signing_key.to_bytes();

                let verifying_key = signing_key.verifying_key();
                let encoded = verifying_key.to_encoded_point(false);
                let pubkey_bytes = encoded.as_bytes();

                let hash = Keccak256::digest(&pubkey_bytes[1..]);
                let address = format!("0x{}", hex::encode(&hash[12..]));

                for (i, target) in targets.iter().enumerate() {
                    if found_flags[i].load(Ordering::Relaxed) {
                        continue;
                    }
                    if address.contains(target) {
                        found_flags[i].store(true, Ordering::Relaxed);
                        let phrase = mnemonic.to_phrase();
                        println!("\n========================================");
                        println!("🎉 FOUND '{}': {}", target, address);
                        println!("========================================");
                        println!("Mnemonic:    {}", phrase);
                        println!("Private Key: 0x{}", hex::encode(&priv_key));
                        println!("========================================");
                        println!("⚠️  WRITE DOWN THE MNEMONIC ON PAPER NOW!");
                        println!("   Never share the private key or mnemonic.");
                        std::io::stdout().flush().unwrap();
                    }
                }

                if attempts % 500_000 == 0 && thread_id == 0 {
                    let remaining = deadline.duration_since(Instant::now()).as_secs();
                    print!(". ({}s left)", remaining);
                    std::io::stdout().flush().unwrap();
                }
            }
        });

    let results = found_results.lock().unwrap();
    println!("\n\n========================================");
    println!("🏁 Search complete! Found {} address(es):", results.len());
    println!("========================================");
    for (i, (address, phrase, priv_key, thread_id)) in results.iter().enumerate() {
        println!("\n[{}] Thread {}", i + 1, thread_id);
        println!("Address:     {}", address);
        println!("Mnemonic:    {}", phrase);
        println!("Private Key: 0x{}", priv_key);
    }
    println!("\n⚠️  WRITE DOWN THE MNEMONIC ON PAPER NOW!");
    println!("   Never share the private key or mnemonic.");
}
