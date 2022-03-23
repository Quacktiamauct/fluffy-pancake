// https://eprint.iacr.org/2015/546.pdf

use crate::util::*;

use crate::ot::bitmatrix::*;
use crate::ot::common::*;
use crate::ot::polynomial::*;
use bitvec::prelude::*;
use itertools::izip;

/// The computational security paramter (k)
const COMP_SEC: usize = 256;
/// The statistical security paramter (s)
const STAT_SEC: usize = 128;

pub struct Sender {
    pub bootstrap: Box<dyn ObliviousReceiver>,
}

pub struct Receiver {
    pub bootstrap: Box<dyn ObliviousSender>,
}

impl ObliviousSender for Sender {
    fn exchange(&self, msg: &Message, channel: &Channel<Vec<u8>>) -> Result<(), Error> {
        assert!(
            msg.len() >= BLOCK_SIZE,
            "Message length must be larger than {BLOCK_SIZE}"
        );
        assert!(
            msg.len() % BLOCK_SIZE == 0,
            "Message length must be a multiple of {BLOCK_SIZE}"
        );
        let pb = TransactionProperties{msg_size: msg.len()};
        validate_properties(&pb, channel)?;

        let l = msg.len(); // 8 bits stored in a byte.

        // The parameter kappa.
        const K: usize = COMP_SEC;
        const S: usize = STAT_SEC;

        let l = l + K + S;

        // COTe
        use rand::Rng;
        use rand::SeedableRng;

        // receiver:
        // sample k pairs of k-bit seeds.
        use rand_chacha::ChaCha20Rng;
        let mut rng = ChaCha20Rng::from_entropy();

        // INITIALIZATION
        let delta: [u8; K / 8] = rng.gen();

        // do OT.
        let payload = self
            .bootstrap
            .exchange(&u8_vec_to_bool_vec(&delta), channel)?;
        let mut seed = [[0u8; K / 8]; K];
        for (i, p) in payload.iter().enumerate() {
            seed[i].copy_from_slice(p);
        }

        let delta : BitVec<Block> = BitVec::from_vec(delta.to_vec());
        // EXTENSION
        let t: BitMatrix = seed
            .iter()
            .map(|&s| {
                let mut prg = ChaCha20Rng::from_seed(s);
                let v = (0..l / BLOCK_SIZE).map(|_| prg.gen::<Block>()).collect();
                BitVec::from_vec(v)
            })
            .collect();

        let (_, r) = channel;
        let u: BitMatrix = bincode::deserialize(&r.recv()?)?;

        let mut q = Vec::with_capacity(K);
        for i in 0..K {
            if delta[i] {
                q.push(u[i].clone() ^ t[i].clone());
            } else {
                q.push(t[i].clone());
            }
        }
        let q = BitMatrix::new(q);

        // Sender outputs `q_j`
        let q = q.transpose();

        // -- Check correlation --
        let chi: BitMatrix = bincode::deserialize(&r.recv()?)?;
        let vector_len = chi[0].len();
        let mut q_sum = polynomial_new(vector_len);
        let mut q_acc = polynomial_new(vector_len);
        for (q, chi) in izip!(&q, &chi) {
            // We would like to work in the finite field F_(2^k) in order to achieve this we will
            // work on polynomials modulo x^k with coefficients in F_2. The coefficients can be
            // represented directly as strings of bits and the sum of two of these polynomials will
            // be the xor of these bitstrings (as dictated by the operations on the underlying field
            // to which the coefficients belong). The product of two elements will be the standard
            // polynomial products modulo x^k.

            // TODO: We could technically do this in one go, but lets keep it simple for now...
            polynomial_mul(&mut q_acc, q, chi);
            polynomial_acc(&mut q_sum, &q_acc);

            // TODO: Depending on the performance of the bitvector it might be faster to add a check
            //       here, so we avoid doing unnecessary work the last iteration. (This depends
            //       greatly on the underlying implementation and the performance of the branch
            //       predictor)
            polynomial_zero(&mut q_acc);
        }

        // TODO: *Maybe* doesn't work
        {
            let x_sum = bincode::deserialize(&r.recv()?)?;
            let t_sum = bincode::deserialize(&r.recv()?)?;

            let mut acc = polynomial_new(q[0].len());
            polynomial_mul(&mut acc, &x_sum, &delta);
            polynomial_acc(&mut acc, &q_sum);

            if !polynomial_eq(&t_sum, &acc) {
                return Err(Box::new(OTError::PolychromaticInput()));
            }
        }



        // -- Randomize --
        let (v0, v1): (Vec<Vec<u8>>, Vec<Vec<u8>>) = q[..msg.len()]
            .iter()
            .enumerate()
            .map(|(j, q)| {
                let v0 = hash!(j.to_be_bytes(), q.as_raw_slice()).to_vec();
                let q = q.clone() ^ &delta;
                let v1 = hash!(j.to_be_bytes(), q.as_raw_slice()).to_vec();
                (v0, v1)
            })
            .unzip();

        // -- DeROT --
        use aes_gcm::aead::{Aead, NewAead};
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        let (d0, d1): (Vec<Vec<u8>>, Vec<Vec<u8>>) = izip!(&msg.0, v0, v1)
            .map(|([m0, m1], v0, v1)| {
                // encrypt the messages.
                let nonce = Nonce::from_slice(b"unique nonce");
                let cipher = Aes256Gcm::new(Key::from_slice(&v0));
                let c0 = cipher.encrypt(nonce, m0.as_slice()).unwrap();
                let cipher = Aes256Gcm::new(Key::from_slice(&v1));
                let c1 = cipher.encrypt(nonce, m1.as_slice()).unwrap();
                (c0, c1) // TODO: Proper error handling.
            })
            .unzip();

        let (s, _) = channel;
        let d0 = bincode::serialize(&d0)?;
        let d1 = bincode::serialize(&d1)?;
        s.send(d0)?;
        s.send(d1)?;

        Ok(())
    }
}

impl ObliviousReceiver for Receiver {
    fn exchange(&self, choices: &[bool], channel: &Channel<Vec<u8>>) -> Result<Payload, Error> {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;
        let mut rng = ChaCha20Rng::from_entropy();

        let pb = TransactionProperties{msg_size: choices.len()};
        validate_properties(&pb, channel)?;
        assert!(
            choices.len() >= BLOCK_SIZE,
            "Message length must be larger than {BLOCK_SIZE}"
        );
        assert!(
            choices.len() % BLOCK_SIZE == 0,
            "Message length must be a multiple of {BLOCK_SIZE}"
        );
        const K: usize = COMP_SEC;
        const S: usize = STAT_SEC;
        let l = choices.len();
        let l = l + K + S;
        let bonus: [bool; K+S] = rng.gen();

        // COTe

        // receiver:
        // sample k pairs of k-bit seeds.

        // INITIALIZATION
        let seed0: [u8; K * K / 8] = rng.gen();
        let seed0: [[u8; K / 8]; K] = unsafe { std::mem::transmute(seed0) };
        let seed1: [u8; K * K / 8] = rng.gen();
        let seed1: [[u8; K / 8]; K] = unsafe { std::mem::transmute(seed1) };

        let msg = Message::new(&seed0, &seed1);
        self.bootstrap.exchange(&msg, channel)?;

        // EXTENSION

        let padded_choices = [choices, &bonus].concat();
        let x: BitMatrix = padded_choices
            .iter()
            .map(|b| {
                if !*b {
                    vec![0x00u8; K / 8]
                } else {
                    vec![0xFFu8; K / 8]
                }
            })
            .map(BitVec::from_vec)
            .collect();
        let x = x.transpose();

        let t0: BitMatrix = seed0
            .iter()
            .map(|&s| {
                let mut prg = ChaCha20Rng::from_seed(s);
                let v = (0..l / BLOCK_SIZE).map(|_| prg.gen::<Block>()).collect();
                BitVec::from_vec(v)
            })
            .collect();

        let t1: BitMatrix = seed1
            .iter()
            .map(|&s| {
                let mut prg = ChaCha20Rng::from_seed(s);
                let v = (0..l / BLOCK_SIZE).map(|_| prg.gen::<Block>()).collect();
                BitVec::from_vec(v)
            })
            .collect();

        let t = t0.transpose(); // saving this for later

        let u: BitMatrix = izip!(x, t0, t1)
            .map(|(x, t0, t1)| {
                let mut u = x;
                u ^= &t0;
                u ^= &t1;
                u
            })
            .collect();

        let (s, _) = channel;
        let u = bincode::serialize(&u)?;
        s.send(u)?;

        // Receiver outputs `t_j`


        // -- Check correlation --
        let k_blocks = K / 8;
        let chi : BitMatrix = (0..l).map(|_| {
            let v = (0..k_blocks).map(|_| rng.gen::<Block>()).collect();
            BitVec::from_vec(v)
        }).collect();
        s.send(bincode::serialize(&chi)?)?;

        let vector_len = chi[0].len();
        let mut x_sum = polynomial_new(vector_len);
        let mut t_sum = polynomial_new(vector_len);
        let mut t_acc = polynomial_new(vector_len);
        for (x, t, chi) in izip!(padded_choices, &t, &chi) {
            if x {
                polynomial_acc(&mut x_sum, &chi);
            }

            polynomial_mul(&mut t_acc, &t, &chi);
            polynomial_acc(&mut t_sum, &t_acc);

            polynomial_zero(&mut t_acc);
        }
        s.send(bincode::serialize(&x_sum)?)?;
        s.send(bincode::serialize(&t_sum)?)?;


        // -- Randomize --
        let v: Vec<Vec<u8>> = t
            .into_iter()
            .enumerate()
            .map(|(j, t)| hash!(j.to_be_bytes(), t.as_raw_slice()).to_vec())
            .collect();

        // -- DeROT --
        use aes_gcm::aead::{Aead, NewAead};
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        let (_, r) = channel;
        let d0: Vec<Vec<u8>> = bincode::deserialize(&r.recv()?)?;
        let d1: Vec<Vec<u8>> = bincode::deserialize(&r.recv()?)?;
        let y = izip!(v, choices, d0, d1)
            .map(|(v, c, d0, d1)| {
                let nonce = Nonce::from_slice(b"unique nonce");
                let cipher = Aes256Gcm::new(Key::from_slice(&v));
                let d = if *c { d1 } else { d0 };
                let c = cipher.decrypt(nonce, d.as_slice()).unwrap();
                c // TODO: Proper error handling.
            })
            .collect();
        Ok(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ot_receiver() {
        use crate::ot::chou_orlandi::{OTReceiver, OTSender};
        let (s1, r1) = ductile::new_local_channel();
        let (s2, r2) = ductile::new_local_channel();
        let ch1 = (s1, r2);
        let ch2 = (s2, r1);

        use std::thread;
        let h1 = thread::Builder::new()
            .name("Sender".to_string())
            .spawn(move || {
                let sender = Sender {
                    bootstrap: Box::new(OTReceiver),
                };
                let msg = Message::new(&[b"Hello"; 8 << 2], &[b"World"; 8 << 2]);
                sender.exchange(&msg, &ch1).unwrap();
            });

        let h2 = thread::Builder::new()
            .name("Receiver".to_string())
            .spawn(move || {
                let receiver = Receiver {
                    bootstrap: Box::new(OTSender),
                };
                let choices = [true; 8 << 2];
                let msg = receiver.exchange(&choices, &ch2).unwrap();
                assert_eq!(msg[0], b"World");
            });

        h1.unwrap().join().unwrap();
        h2.unwrap().join().unwrap();
    }
}
