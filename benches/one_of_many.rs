use criterion::{criterion_group, criterion_main, Criterion};
use magic_pake::ot::one_of_many::*;
use magic_pake::util::SECURITY_PARAM;

fn log2(x: u32) -> u32 {
    ((std::mem::size_of::<u32>() * 8) as u32 - (x - 1).leading_zeros()) as u32
}

fn one_of_many_local(n: u32, domain: u32, messages: Vec<Vec<u8>>) {
    let choice = n / 2;

    let (s1, r1) = ductile::new_local_channel();
    let (s2, r2) = ductile::new_local_channel();
    let ch1 = (s1, r2);
    let ch2 = (s2, r1);

    use std::thread;
    let h1 = thread::Builder::new()
        .name("Sender".to_string())
        .spawn(move || {
            let sender = ManyOTSender {
                interal_sender: magic_pake::ot::chou_orlandi::OTSender,
            };
            sender.exchange(&messages, domain, &ch1).unwrap();
        });

    let h2 = thread::Builder::new()
        .name("Receiver".to_string())
        .spawn(move || {
            let receiver = ManyOTReceiver {
                internal_receiver: magic_pake::ot::chou_orlandi::OTReceiver,
            };
            receiver.exchange(choice, domain, &ch2).unwrap()
        });

    h1.unwrap().join().unwrap();
    h2.unwrap().join().unwrap();
}

fn bench_1_of_n_ot(c: &mut Criterion) {
    let mut group = c.benchmark_group("1-to-n OT");
    group.sample_size(10);

    // Local
    for i in 1..=16u32 {
        let n = 1 << i;
        let domain = log2(n);

        let mut messages = Vec::with_capacity(n as usize);
        for _ in 0..n {
            messages.push(vec![0u8; 2048 * (SECURITY_PARAM / 8)]);
        }

        group.bench_function(&format!("Local 1-to-{}", n), |b| {
            b.iter(|| {
                one_of_many_local(n, domain, messages.clone());
            })
        });
    }

    // TODO: LAN
    // TODO: WAN

    group.finish();
}

criterion_group!(benches, bench_1_of_n_ot,);
criterion_main!(benches);
