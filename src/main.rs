extern crate rug;

use clap::clap_app;
use rug::ops::Pow;
use rug::Integer;
use std::cell::RefCell;
use std::io::prelude::*;
use std::path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

thread_local! {
    pub static NUM_THREADS: RefCell<u32> = RefCell::new(8);
}

fn main() {
    let matches = clap_app!(myapp =>
        (version: "1.0")
        (author: "Blake N. <blakeanedved@gmail.com>")
        (about: "Mersenne Prime Finder")
        (@arg threads: -t --threads [num] "Number of threads to run")
        (@arg verbose: -v --verbose "Print test information verbosely")
    )
    .get_matches();

    if matches.is_present("threads") {
        NUM_THREADS.with(|nt| {
            *nt.borrow_mut() = matches.value_of("threads").unwrap().parse::<u32>().unwrap()
        });
    }

    NUM_THREADS.with(|nt| println!("running with NUM_THREADS={}", *nt.borrow()));

    let primes_exist = path::Path::new("primes.dat").exists();

    if !primes_exist {
        generate_primes();
    }

    let primes = std::fs::read_to_string("primes.dat")
        .unwrap()
        .split_whitespace()
        .map(|s| s.parse::<u32>().unwrap())
        .collect::<Vec<_>>();

    print!("What p (2^p-1) would you like to start at? >> ");
    std::io::stdout().flush().unwrap();

    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    let start = buf.trim().parse::<u32>().unwrap();

    start_mersenne_prime_search(&primes, start);
}

fn prime_sieve(p: u32) -> bool {
    if p % 2 == 0 {
        return false;
    }
    let max = (p as f64).sqrt().floor() as u32;
    let mut a = 3;

    while a <= max {
        if p % a == 0 {
            return false;
        }

        a += 2;
    }

    return true;
}

fn generate_primes() {
    let (tx, rx) = mpsc::channel::<u32>();
    let mut threads = Vec::new();

    NUM_THREADS.with(|nt| {
        for i in 0..*nt.borrow() {
            let thread_tx = tx.clone();
            let num_threads: u32 = nt.borrow().clone();

            threads.push(thread::spawn(move || {
                let mut x: u32 = 3;
                let mut num_primes: u32 = 0;
                while x < 25000000 {
                    if x % num_threads == i {
                        if prime_sieve(x) {
                            thread_tx.send(x).unwrap();
                            num_primes += 1;
                        }
                    }
                    x += 1;
                }
                num_primes
            }));
        }
    });

    let mut primes = Vec::new();
    primes.push(2);

    for t in threads {
        let num_primes = t.join().unwrap();

        primes.append(&mut (0..num_primes).map(|_| rx.recv().unwrap()).collect());
    }

    primes.sort();

    std::fs::write(
        "primes.dat",
        primes
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .expect("failed to write primes file");
}

fn start_mersenne_prime_search(primes: &Vec<u32>, start: u32) {
    let (tx, rx) = mpsc::channel::<u32>();
    let number_lock = Arc::new(Mutex::new(rx));
    let done_lock = Arc::new(Mutex::new(false));

    for p in primes.iter().skip_while(|x| x < &&start) {
        tx.send(*p).unwrap();
    }

    let mut threads = Vec::new();

    NUM_THREADS.with(|nt| {
        for _ in 0..*nt.borrow() {
            let thread_done = Arc::clone(&done_lock);
            let thread_rx = Arc::clone(&number_lock);
            threads.push(thread::spawn(move || loop {
                if *thread_done.lock().unwrap() {
                    return;
                }

                let l = thread_rx.lock().unwrap();
                let val = l.recv();
                drop(l);

                match val {
                    Ok(x) => {
                        let prime = test_mersenne(x);

                        if prime {
                            println!("Found mersenne prime: 2^{}-1", x);
                            *thread_done.lock().unwrap() = true;
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }));
        }
    });

    for t in threads {
        t.join().unwrap();
    }
}

fn test_mersenne(p: u32) -> bool {
    let m: Integer = Integer::from(2).pow(p) - 1;
    let mut l = Integer::from(4);

    for _ in 1..(p - 1) {
        l = (l.pow(2) - 2) % &m;
    }

    l % m == 0
}
