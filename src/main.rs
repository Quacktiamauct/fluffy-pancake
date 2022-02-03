#![allow(dead_code)]
#![allow(unused_variables)]

// -------------------------------------------------------------------------------------------------
// Circuit stuff
#[derive(PartialEq)]
enum GateKind {
    // Unary
    NOT,

    // Binary
    AND,
    OR,
    XOR,

}

struct Gate {
    kind: GateKind,
    output: usize,
    inputs: Vec<usize>,
}

struct Circuit {
    gates: Vec<Gate>,
    num_inputs: usize,
    num_outputs: usize,
    num_wires: usize,
}

impl Circuit {
    fn evaluate(&self, input: Vec<bool>) -> Vec<bool> {
        let mut wires = vec![false; self.num_wires];

        for i in 0..input.len() {
            wires[i] = input[i];
        }

        // TODO: Support magic several input gates
        for gate in &self.gates {
            wires[gate.output] = match gate.kind {
                GateKind::NOT => !wires[gate.inputs[0]],
                GateKind::AND => wires[gate.inputs[0]] && wires[gate.inputs[1]],
                GateKind::XOR => wires[gate.inputs[0]] ^ wires[gate.inputs[1]],
                GateKind::OR => wires[gate.inputs[0]] || wires[gate.inputs[1]],
                _ => false,
            };
        }

        return wires[(wires.len() - self.num_outputs)..wires.len()].to_vec();
    }
}

// -------------------------------------------------------------------------------------------------
// Yao stuff

const SECURITY_BYTES: usize = 16;
type Primitive = [u8; SECURITY_BYTES];
type Primitives = [Primitive; 2];

fn xor(left: Primitive, right: Primitive) -> Primitive {
    let mut result = [0u8; SECURITY_BYTES];
    for i in 0..SECURITY_BYTES {
        result[i] = left[i] ^ right[i];
    }

    return result;
}

fn eq(left: Primitive, right: Primitive) -> bool {
    for i in 0..SECURITY_BYTES {
        if left[i] != right[i] {
            return false;
        }
    }

    return true;
}

fn zero(p: Primitive) -> bool {
    for i in 0..SECURITY_BYTES {
        if p[i] != 0x00 {
            return false;
        }
    }

    return true;
}

use std::vec;
use rand::Rng;
use sha2ni::Digest;
fn prf(left: Primitive, right: Primitive, index: usize) -> (Primitive, Primitive) {
    // TODO(frm): This is probably not the best way to do it!
    let mut sha = sha2ni::Sha256::new();
    sha.input(left);
    sha.input(right);

    // TODO(frm): This is super not nice :(
    use std::mem::transmute;
    let index: [u8; 4] = unsafe { transmute((index as u32).to_be()) };
    sha.input(index);

    // TODO(frm): What if the sizes are out of bounds?
    let digest = sha.result();
    let mut l_result = [0u8; SECURITY_BYTES];
    let mut r_result = [0u8; SECURITY_BYTES];
    for i in 0..SECURITY_BYTES {
        l_result[i] = digest[i];
        r_result[i] = digest[16 - i];
    }

    return (l_result, r_result);
}

use ring::rand::{SecureRandom, SystemRandom};
fn random_primitives() -> [Primitive; 2] {
    let random = SystemRandom::new();

    let mut left = [0u8; SECURITY_BYTES];
    let _ = random.fill(&mut left);

    let mut right = [0u8; SECURITY_BYTES];
    let _ = random.fill(&mut right);

    return [left, right];
}

fn yao_garble(circuit: &Circuit) -> (Vec<Primitives>, Vec<Primitives>, Vec<[Primitives; 4]>) {
    let mut k: Vec<Primitives> = vec![[[0; SECURITY_BYTES]; 2]; circuit.num_wires];

    // 1. Pick key pairs for the inputs wires
    for i in 0..circuit.num_wires {
        k[i] = random_primitives();
    }
    let e = k[..circuit.num_inputs].to_vec();

    // 2. Gooble garble
    let rand = SystemRandom::new();
    let mut f: Vec<[Primitives; 4]> = vec![[[[0; SECURITY_BYTES]; 2]; 4]; circuit.num_wires];
    for gate in &circuit.gates {
        if gate.kind == GateKind::NOT {
            k[gate.output] = [k[gate.inputs[0]][1], k[gate.inputs[0]][0]];
            continue;
        }

        // Binary gates
        // TODO(frm): Magic many input gates?
        let mut c: [Primitives; 4] = [[[0u8; SECURITY_BYTES]; 2]; 4];
        let permutation = rand::thread_rng().gen_range(0..4);
        let combinations = [(false, false), (false, true), (true, false), (true, true)];
        for j in 0..combinations.len() {
            let (left, right) = combinations[j];
            let gate_value = match gate.kind {
                GateKind::NOT => !left,
                GateKind::AND => left && right,
                GateKind::XOR => left ^ right,
                GateKind::OR => left || right,
                _ => false,
            };
            let garbled_value = k[gate.output as usize][gate_value as usize];
            let (g_left, g_right) = prf(
                k[gate.inputs[0]][left as usize],
                k[gate.inputs[1]][right as usize],
                gate.output
            );
            c[(j + permutation) % 4] = [xor(g_left, garbled_value), g_right];
        }

        f[gate.output] = c;
    }

    // 3. Decoding information
    let d = k[(circuit.num_wires - circuit.num_outputs)..].to_vec();
    return (e, d, f);
}

fn yao_encode(circuit: &Circuit, e: &Vec<Primitives>, x: &Vec<bool>) -> Vec<Primitive> {
    assert_eq!(x.len(), circuit.num_inputs);
    assert_eq!(e.len(), circuit.num_inputs);

    let mut z: Vec<Primitive> = vec![[0; SECURITY_BYTES]; circuit.num_inputs];
    for i in 0..circuit.num_inputs {
        z[i] = e[i][if x[i] { 1 } else { 0 }];
    }

    return z;
}

fn yao_evaluate(circuit: &Circuit, f: &Vec<[Primitives; 4]>, x: &Vec<Primitive>) -> Vec<Primitive> {
    assert_eq!(x.len(), circuit.num_inputs);

    // 1. Set the inputs
    let mut wire: Vec<Primitive> = vec![[0; SECURITY_BYTES]; circuit.num_wires];
    for i in 0..circuit.num_inputs {
        wire[i] = x[i];
    }

    // 2. Compute gates
    for gate in &circuit.gates {
        if gate.kind == GateKind::NOT {
            wire[gate.output] = wire[gate.inputs[0]];
            continue;
        }

        // TODO(frm): What about NOT gates?
        let (gate_left, gate_right) = prf(wire[gate.inputs[0]], wire[gate.inputs[1]], gate.output);

        let mut found = false;
        for j in 0..4 {
            let c = f[gate.output][j];
            let (c_left, c_right) = (c[0], c[1]);

            let k = xor(gate_left, c_left);
            let t = xor(gate_right, c_right);
            if zero(t) {
                wire[gate.output] = k;
                found = true;
                break;
            }
        }

        if !found {
            eprintln!("Cannot find solution for gate {}, no match with table!", gate.output);
        }
    }

    // 3. Result
    let z = wire[(circuit.num_wires - circuit.num_outputs)..].to_vec();
    return z;
}

fn yao_decode(circuit: &Circuit, d: &Vec<Primitives>, z: &Vec<Primitive>) -> (bool, Vec<bool>) {
    assert_eq!(z.len(), circuit.num_outputs);
    assert_eq!(d.len(), circuit.num_outputs);

    let mut success = true;
    let mut y: Vec<bool> = vec![false; circuit.num_outputs];
    for i in 0..circuit.num_outputs {
        if eq(d[i][0], z[i]) {
            y[i] = false;
        }
        else if eq(d[i][1], z[i]) {
            y[i] = true;
        }
        else {
            eprintln!("Error decoding output {}, no match with decoding information!", i);
            success = false;
        }
    }

    return (success, y);
}

// -------------------------------------------------------------------------------------------------
//
//
struct NewCircuit {
    wiredomains: Vec<u64>,
    inputdomains : Vec<u64>,
    num_inputs : usize,
    gates : Vec<NewGate>
}

#[derive(PartialEq)]
enum NewGateKind {
    ADD,
    MULT(u64),
    PROJ(u64, fn(u64) -> u64),
}

struct NewGate {
    kind: NewGateKind,
    output: usize,
    inputs: Vec<usize>,
}


fn log2(x : u64) -> u64 {
    (std::mem::size_of::<u64>() as u64) * 8 - (x.leading_zeros() as u64)
}

fn hash(a : u64, b : u64) -> u64 {
    // This is super nice 😎
    use ring::digest::SHA256;
    use ring::digest::Context;
    let mut digest = Context::new(&SHA256);
    digest.update(&a.to_be_bytes());
    digest.update(&b.to_be_bytes());
    return u64::from_be_bytes(digest.finish().as_ref().try_into().unwrap());
}

fn garble(k : u64, circuit : NewCircuit) {
    fn rng(max : u64) -> u64 {
        rand::thread_rng().gen_range(0..4)
    }
    fn lsb(a : u64) -> u64 {
        (a & 1 == 1) as u64
    }
    let mut lambda = Vec::new();
    let mut delta = Vec::new();
    for (i,&m) in circuit.wiredomains.iter().enumerate() {
        lambda.push(k / log2(m));
        delta.push(rng(lambda[i] + 1) | 0b1 );
    }
    let mut domains = Vec::new();
    let mut wires = Vec::new();
    for (i,dom) in circuit.inputdomains.iter().enumerate() {
        domains.push(dom);
        wires.push(rng(lambda[i] + 1)); // 5 is randomly chosen
    }
    let encoding = (&wires[..circuit.num_inputs], &delta[..circuit.num_inputs]);
    for (i, gate) in circuit.gates.iter().enumerate() {
        let domain = 999; //gate.domain;
        match gate.kind {
            NewGateKind::ADD => {
                wires[i] = gate.inputs.iter()
                    .map(|&x| wires[x])
                    .fold(0, |acc, x| acc + x % domain);
            },
            NewGateKind::MULT(c) => {
                let a = gate.inputs[0];
                wires[i] = c * wires[a];
            },
            _ => {}
            // NewGateKind::PROJ(range, phi) => {
            //     let a = gate.inputs[0];
            //     let tau = lsb(wires[a]);
            //     wires[i] -= hash(i as u64, wires[a] + (tau * delta[i]));
            //     wires[i] -= phi( -(tau as i64) as u64)*delta[a];
            //     for x 
            // }
        }
    }
}


// -------------------------------------------------------------------------------------------------
// fun times ahead
fn main() {
    /*
    let circuit = Circuit {
        gates: vec![Gate {
            kind: GateKind::NOT,
            inputs: vec![0],
            output: 1,
        }],
        num_inputs: 1,
        num_outputs: 1,
        num_wires: 2,
    };

    let (e, d, f) = yao_garble(&circuit);
    test_circuit_with_input(&circuit, vec![false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![ true], &e, &d, &f);
    */

    /*
    let circuit = Circuit {
        gates: vec![Gate {
            kind: GateKind::XOR,
            inputs: vec![0, 1],
            output: 2,
        }],
        num_inputs: 2,
        num_outputs: 1,
        num_wires: 3,
    };

    let (e, d, f) = yao_garble(&circuit);

    test_circuit_with_input(&circuit, vec![false, false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![false,  true], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![ true, false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![ true,  true], &e, &d, &f);
    */

    let circuit = Circuit {
        gates: vec![Gate {
            kind: GateKind::XOR,
            inputs: vec![0, 1],
            output: 3,
        }, Gate {
            kind: GateKind::XOR,
            inputs: vec![2, 3],
            output: 4,
        }],
        num_inputs: 3,
        num_outputs: 1,
        num_wires: 5,
    };

    let (e, d, f) = yao_garble(&circuit);

    test_circuit_with_input(&circuit, vec![false, false, false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![false, false,  true], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![false,  true, false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![false,  true,  true], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![true,  false, false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![true,  false,  true], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![true,   true, false], &e, &d, &f);
    test_circuit_with_input(&circuit, vec![true,   true,  true], &e, &d, &f);
}



fn test_circuit_with_input(circuit: &Circuit, input: Vec<bool>, e: &Vec<Primitives>, d: &Vec<Primitives>, f: &Vec<[Primitives; 4]>) {
    let x = yao_encode(&circuit, e, &input);
    let z = yao_evaluate(&circuit, f, &x);
    let (success, y) = yao_decode(&circuit, d, &z);
    if !success {
        println!("\x1b[31mError decoding, no match found!\x1b[0m");
    }

    let expected = circuit.evaluate(input);

    assert_eq!(y.len(), expected.len());
    let mut success = true;
    for i in 0..y.len() {
        let matches = y[i] == expected[i];
        success &= matches;
        println!("Output {}> {} ?= {} => {}{}\x1b[0m", i, y[i], expected[i], if matches {"\x1b[32m"} else {"\x1b[31m"}, matches);
    }

    if success {
        println!("\x1b[32mTest passed :)\x1b[0m");
    }
    else {
        println!("\x1b[31mTest failed :)\x1b[0m");
    }
}
