pub struct NewCircuit {
    num_wires: usize,
    num_inputs: usize,
    num_outputs: usize,
    gates: Vec<NewGate>,
    input_domains: Vec<u64>,
}

#[derive(PartialEq)]
enum NewGateKind {
    ADD,
    MUL(u64),
    PROJ(u64, fn(u64) -> u64),
}

struct NewGate {
    kind: NewGateKind,
    output: usize,
    inputs: Vec<usize>,
    domain: u64,
}


fn hash(a: u64, b: u64, w: &Wire) -> u64 {
    // This is super nice 😎
    use ring::digest::Context;
    use ring::digest::SHA256;

    let mut context = Context::new(&SHA256);
    context.update(&a.to_be_bytes());
    context.update(&b.to_be_bytes());
    context.update(&w.lambda.to_be_bytes());
    context.update(&w.domain.to_be_bytes());
    for v in &w.values {
        context.update(&v.to_be_bytes());
    }
    let digest = context.finish();
    let bytes = digest.as_ref();

    let num = u64::from_be_bytes(bytes[..8].try_into().unwrap());
    return num;
}

fn hash_wire(a: u64, b: u64, w: &Wire, target: &Wire) -> Wire {
    // This is super nice 😎
    use ring::digest::Context;
    use ring::digest::SHA256;

    // Compute the hash
    let mut context = Context::new(&SHA256);
    context.update(&a.to_be_bytes());
    context.update(&b.to_be_bytes());
    context.update(&w.lambda.to_be_bytes());
    context.update(&w.domain.to_be_bytes());
    for v in &w.values {
        context.update(&v.to_be_bytes());
    }
    let digest = context.finish();
    let bytes = digest.as_ref();

    // Turn them into a wire
    let mut v = Vec::with_capacity(target.lambda as usize);
    let bits_per_value = log2(target.domain);
    let truncated_bytes_per_value = bits_per_value / 8;
    let bytes_per_value = if (bits_per_value % 8) == 0 {
        truncated_bytes_per_value
    } else {
        truncated_bytes_per_value + 1
    };
    assert!(target.lambda * bytes_per_value <= 32);
    for i in (0..32).step_by(bytes_per_value as usize) {
        let mut value = 0u64;
        for j in 0..bytes_per_value {
            value |= (bytes[i + j as usize] as u64) << (8 * j);
        }
        v.push(value % target.domain);

        if v.len() == (target.lambda as usize) {
            break;
        }
    }

    assert_eq!(v.len(), target.lambda as usize);
    assert!(
        v.iter().all(|v| v < &target.domain),
        "value not under domain"
    );
    return Wire {
        domain: target.domain,
        lambda: target.lambda,
        values: v,
    };
}

#[derive(Clone, Debug)]
pub struct Wire {
    lambda: u64,
    values: Vec<u64>,
    domain: u64,
}

use core::ops;
use std::iter;

impl ops::Add<&Wire> for &Wire {
    type Output = Wire;
    fn add(self, rhs: &Wire) -> Wire {
        assert_eq!(self.lambda, rhs.lambda, "Lambdas doesn't match.");
        assert_eq!(self.domain, rhs.domain, "Domain not matching");
        let domain = self.domain;
        let lambda = self.lambda;
        let values = self
            .values
            .iter()
            .zip(rhs.values.iter())
            .map(|(a, b)| (a + b) % domain)
            .collect();
        return Wire {
            domain,
            values,
            lambda,
        };
    }
}

impl ops::Sub<&Wire> for &Wire {
    type Output = Wire;
    fn sub(self, rhs: &Wire) -> Self::Output {
        assert_eq!(self.lambda, rhs.lambda, "Lambdas doesn't match.");
        assert_eq!(self.domain, rhs.domain, "Domain not matching");
        let domain = self.domain;
        let lambda = self.lambda;
        let values = self
            .values
            .iter()
            .zip(rhs.values.iter())
            .map(|(a, b)| (a + (domain - b)) % domain)
            .collect();
        return Wire {
            domain,
            values,
            lambda,
        };
    }
}

impl ops::Neg for &Wire {
    type Output = Wire;
    fn neg(self) -> Wire {
        return Wire {
            domain: self.domain,
            lambda: self.lambda, // this probably works
            values: self.values.iter().map(|x| self.domain - x).collect(),
        };
    }
}

impl ops::Mul<u64> for &Wire {
    type Output = Wire;
    #[inline]
    fn mul(self, rhs: u64) -> Wire {
        let domain = self.domain;
        let lambda = self.lambda;
        let values = self.values.iter().map(|x| (x * rhs) % domain).collect();
        return Wire {
            domain,
            values,
            lambda,
        };
    }
}

impl iter::Sum for Wire {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let init = iter.next().unwrap();
        iter.fold(init, |acc: Wire, w: Wire| &acc + &w)
    }
}

impl Wire {
    fn empty() -> Wire {
        return Wire {
            domain: 0,
            lambda: 0,
            values: Vec::new()
        }
    }

    fn new(domain: u64, lambda: u64) -> Wire {
        let mut values = vec![0u64; (lambda+1) as usize];
        for i in 0..=lambda {
            values[i as usize] = rng(domain + 1);
        }
        
        return Wire {
            values,
            lambda,
            domain,
        };
    }

    fn delta(domain: u64, lambda: u64) -> Wire {
        let mut values = vec![0u64; (lambda+1) as usize];
        for i in 0..lambda {
            values[i as usize] =  rng(domain + 1);
        }
        values[lambda as usize] = 1;
        return Wire {
            values,
            lambda,
            domain,
        };
    }
}

// -------------------------------------------------------------------------------------------------
// Start of stuff ...

use itertools::Itertools;
use rand::Rng;

fn rng(max: u64) -> u64 {
    rand::thread_rng().gen_range(0..max)
}

#[inline]
fn lsb2(a: u64) -> u64 {
    (a & 1 == 1) as u64
}

#[inline]
fn tau(w : &Wire) -> u64 {
    w.values[w.lambda as usize]
}

#[inline]
fn log2(x: u64) -> u64 {
    (std::mem::size_of::<u64>() as u64) * 8 - (x.leading_zeros() as u64)
}

pub struct Encoding {
    wires: Vec<Wire>,
    delta: HashMap<u64, Wire>,
}

pub struct Decoding {
    map: Vec<Vec<u64>>,
    ids: Vec<usize>,
    domains: Vec<u64>,
}

use std::collections::HashMap;
use std::mem;

fn garble(circuit: &NewCircuit, k: u64) -> (HashMap<usize, Vec<Wire>>, Encoding, Decoding) {
    // 1. For each domain
    let mut domains: Vec<u64> = circuit.gates.iter().map(|g| g.domain).unique().collect();
    domains.extend(
        circuit
            .gates
            .iter()
            .filter_map(|g| match g.kind {
                NewGateKind::PROJ(range, _) => Some(range),
                _ => None,
            })
            .unique(),
    );

    let lambda: HashMap<_, _> = domains
        .iter()
        .map(|&m| (m, (k + log2(m) - 1) / log2(m)))
        .collect();

    let delta: HashMap<_, _> = domains
        .iter()
        .map(|&m| (m, Wire::delta(m, lambda[&m])))
        .collect();

    // 2. For each input
    let inputs = 0..circuit.num_inputs;
    let mut wires = Vec::with_capacity(circuit.num_wires);
    for i in inputs {
        let m = circuit.input_domains[i];
        wires.push(Wire::new(m, lambda[&m]));
    }

    // 3. Encoding
    let encoding = Encoding {
        wires: wires[..circuit.num_inputs].to_vec(),
        delta: delta.clone(),
    };

    // 4. For each gate
    let mut f = HashMap::new();
    for gate in &circuit.gates {
        let w = match gate.kind {
            NewGateKind::ADD => gate.inputs.iter().map(|&x| wires[x].clone()).sum(),
            NewGateKind::MUL(c) => &wires[gate.inputs[0]] * c,
            NewGateKind::PROJ(range, phi) => {
                let a = gate.inputs[0];
                let i = gate.output as u64;
                let domain = gate.domain;
                let delta_m = &delta[&domain];
                let delta_n = &delta[&range];
                let tau = tau(&wires[a]);
                let hw = hash_wire(i, 0, &(&wires[a] - &(delta_m * tau)), &delta_n);
                let w = &hw + &(delta_n * phi(domain - tau));
                let w = -&w;

                let mut g: Vec<Wire> = vec![Wire::empty(); gate.domain as usize];
                for x in 0..domain {
                    let hw = hash_wire(i, 0, &(&wires[a] + &(delta_m * x)), &w);
                    let wx = &(&hw + &w) + &(delta_n * phi(x));

                    g[((x + tau) % domain) as usize] = wx;
                }

                let tx = wires[a].values[(wires[a].lambda - 1) as usize];
                let ty = w.values[(w.lambda - 1) as usize];

                println!("Color input: {}", tx);
                println!("Color output: {}", ty);

                f.insert(i as usize, g);
                w
            }
        };
        wires.push(w);
    }

    // 5. Decoding / outputs
    let outputs = (circuit.num_wires - circuit.num_outputs)..circuit.num_wires;
    let outputs: Vec<&NewGate> = circuit
        .gates
        .iter()
        .filter(|g| outputs.contains(&g.output))
        .collect();

    let mut d = Vec::with_capacity(circuit.num_outputs);
    let mut ids = Vec::with_capacity(circuit.num_outputs);
    let mut domains = Vec::with_capacity(circuit.num_outputs);
    for gate in outputs {
        let id = gate.output;
        let domain = match gate.kind {
            NewGateKind::PROJ(range, _) => range,
            _ => gate.domain,
        };
        let mut values = vec![0; domain as usize];
        for k in 0..domain {
            let hash = hash(id as u64, k as u64, &(&wires[id] + &(&delta[&domain] * k)));
            values[k as usize] = hash;
        }
        d.push(values);
        ids.push(id);
        domains.push(domain);
    }
    let decoding = Decoding {
        map: d,
        ids,
        domains,
    };
    return (f, encoding, decoding);
}

fn evaluate(circuit: &NewCircuit, f: &HashMap<usize, Vec<Wire>>, x: &Vec<Wire>) -> Vec<Wire> {
    assert_eq!(x.len(), circuit.num_inputs, "input length mismatch");
    use std::mem::{transmute, MaybeUninit};
    let mut wires: Vec<MaybeUninit<Wire>> = Vec::with_capacity(circuit.num_wires);
    unsafe {
        wires.set_len(circuit.num_wires);
    }
    for i in 0..circuit.num_inputs {
        wires[i].write(x[i].clone());
    }
    for gate in &circuit.gates {
        let w: Wire = match gate.kind {
            NewGateKind::ADD => gate
                .inputs
                .iter()
                .map(|&x| unsafe { wires[x].assume_init_ref() }.clone())
                .sum::<Wire>(),
            NewGateKind::MUL(c) => unsafe { wires[gate.inputs[0]].assume_init_ref() * c },
            NewGateKind::PROJ(_, _) => {
                let wire = unsafe { wires[gate.inputs[0]].assume_init_ref() };
                let tau = tau(wire);
                let cipher = &f[&gate.output][tau as usize];
                let hw = hash_wire(gate.output as u64, 0, wire, cipher);
                cipher - &hw
            }
        };
        wires[gate.output].write(w);
    }
    let wires: Vec<Wire> = unsafe { transmute(wires) };
    return wires[(circuit.num_wires - circuit.num_outputs)..circuit.num_wires].to_vec();
}

pub fn encode(e: &Encoding, x: &Vec<u64>) -> Vec<Wire> {
    let wires = &e.wires;
    let delta = &e.delta;
    assert_eq!(
        wires.len(),
        x.len(),
        "Wire and input vector lengths do not match"
    );
    let mut z = Vec::with_capacity(wires.len());
    for (w, &x) in wires.iter().zip(x) {
        let domain = w.domain;
        z.push(w + &(&delta[&domain] * x));
    }
    return z;
}

use std::error::Error;
use std::fmt;
use std::mem::MaybeUninit;
use std::ptr::null;
use ring::io::der::Tag::Null;

#[derive(Debug)]
pub struct DecodeError {}
impl Error for DecodeError {}
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error decoding result")
    }
}

pub fn decode(decoding: &Decoding, z: &Vec<Wire>) -> Result<Vec<u64>, DecodeError> {
    let d = &decoding.map;
    let ids = &decoding.ids;
    let domains = &decoding.domains;
    assert_eq!(d.len(), z.len());
    assert_eq!(d.len(), ids.len());
    let mut y = vec![0u64; d.len()];
    for i in 0..d.len() {
        let mut success = false;
        let id = ids[i];
        let h = &d[i];
        for k in 0..domains[i] {
            let hash = hash(id as u64, k, &z[i]);
            if hash == h[k as usize] {
                y[i] = k;
                success = true;
                break;
            }
        }
        if !success {
            return Err(DecodeError {});
        }
    }
    Ok(y)
}

#[cfg(test)]
mod tests {
    use crate::arith::{decode, encode, evaluate, garble, hash, Decoding, Encoding, Wire};
    use std::collections::HashMap;

    use super::{NewCircuit, NewGate, NewGateKind};

    fn garble_encode_eval_decode(c: &NewCircuit, x: &Vec<u64>) -> Vec<u64> {
        const SECURITY: u64 = 128;
        let (f, e, d) = garble(&c, SECURITY);
        let x = encode(&e, x);
        let z = evaluate(c, &f, &x);
        return decode(&d, &z).unwrap();
    }

    #[test]
    fn encode_decode() {
        let id: usize = 0;
        let lambda = 8;
        let domain = 128;
        let mut map = HashMap::new();
        let delta = Wire::delta(domain, lambda);
        map.insert(domain, delta.clone());
        let wire = Wire::new(domain, lambda);
        let e = Encoding {
            wires: vec![wire.clone()],
            delta: map,
        };
        let hashes: Vec<u64> = (0..domain)
            .map(|k| hash(id as u64, k, &(&wire + &(&delta * k))))
            .collect();
        let d = Decoding {
            domains: vec![domain],
            ids: vec![id],
            map: vec![hashes],
        };
        let input = vec![69];
        let x = encode(&e, &input);
        let y = decode(&d, &x).unwrap();
        assert_eq!(input, y)
    }

    #[test]
    fn sum_circuit() {
        let domain = 128;
        let circuit = NewCircuit {
            gates: vec![NewGate {
                kind: NewGateKind::ADD,
                inputs: vec![0, 1],
                output: 2,
                domain: domain,
            }],
            num_inputs: 2,
            num_outputs: 1,
            num_wires: 3,
            input_domains: vec![domain, domain],
        };
        let inputs = vec![33, 66];
        let outputs = garble_encode_eval_decode(&circuit, &inputs);
        assert_eq!(outputs[0], 99);
    }

    #[test]
    fn sum_multiple_circuit() {
        let domain = 128;
        let circuit = NewCircuit {
            gates: vec![NewGate {
                kind: NewGateKind::ADD,
                inputs: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                output: 10,
                domain: domain,
            }],
            num_inputs: 10,
            num_outputs: 1,
            num_wires: 11,
            input_domains: vec![domain; 10],
        };
        let inputs = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let outputs = garble_encode_eval_decode(&circuit, &inputs);
        assert_eq!(outputs[0], 10, "Wrong result");
    }

    #[test]
    fn mult_circuit() {
        let domain = 600;
        let circuit = NewCircuit {
            gates: vec![NewGate {
                kind: NewGateKind::MUL(9),
                inputs: vec![0],
                output: 1,
                domain: domain,
            }],
            num_inputs: 1,
            num_outputs: 1,
            num_wires: 2,
            input_domains: vec![domain, domain],
        };
        let inputs = vec![57];
        let outputs = garble_encode_eval_decode(&circuit, &inputs);
        assert_eq!(outputs[0], 9 * 57);
    }

    #[test]
    fn proj_circuit_identity() {
        let target_domain = 8;
        let source_domain = 16;
        let phi = |x| x;
        let circuit = NewCircuit {
            gates: vec![NewGate {
                kind: NewGateKind::PROJ(target_domain, phi),
                inputs: vec![0],
                output: 1,
                domain: source_domain,
            }],
            num_inputs: 1,
            num_outputs: 1,
            num_wires: 2,
            input_domains: vec![source_domain],
        };
        let input = vec![7];
        let output = garble_encode_eval_decode(&circuit, &input);
        assert_eq!(output[0], input[0]);
    }

    #[test]
    fn proj_circuit_shl() {
        let target_domain = 64;
        let source_domain = 64;
        let phi = |x| (x * 2);
        let circuit = NewCircuit {
            gates: vec![NewGate {
                kind: NewGateKind::PROJ(target_domain, phi),
                inputs: vec![0],
                output: 1,
                domain: source_domain,
            }],
            num_inputs: 1,
            num_outputs: 1,
            num_wires: 2,
            input_domains: vec![source_domain],
        };
        let input = vec![7];
        let output = garble_encode_eval_decode(&circuit, &input);
        assert_eq!(output[0], phi(input[0]));
    }

    #[test]
    fn proj_circuit_shr() {
        let target_domain = 64;
        let source_domain = 64;
        let phi = |x| (x / 2);
        let circuit = NewCircuit {
            gates: vec![NewGate {
                kind: NewGateKind::PROJ(target_domain, phi),
                inputs: vec![0],
                output: 1,
                domain: source_domain,
            }],
            num_inputs: 1,
            num_outputs: 1,
            num_wires: 2,
            input_domains: vec![source_domain],
        };
        let input = vec![7];
        let output = garble_encode_eval_decode(&circuit, &input);
        assert_eq!(output[0], phi(input[0]));
    }
}
