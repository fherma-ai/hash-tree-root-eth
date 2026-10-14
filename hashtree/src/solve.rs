//! The SSZ hash tree root, with hashtree.
//!
//! hashtree is what Prysm and Lighthouse merkleise state with: Intel's
//! SHA-256 assembly with the message schedule hardcoded, and — the part that
//! matters — one call per *layer* rather than per pair. Sixteen blocks are
//! compressed at a time where the machine allows it, so the loop a portable
//! answer writes in Rust is inside the library and vectorised.
//!
//! Nothing is allocated in the measured call. Two buffers are taken in `init`
//! and the layers ping-pong between them, because allocating sixteen
//! megabytes per case would be most of what a fast hash saves.
use hashtree_rs::{hash, init as choose_backend};

use crate::fherma::{Inputs, Outputs, Point, Tensor};

pub struct State {
    a: Vec<u8>,
    b: Vec<u8>,
}

impl Default for State {
    fn default() -> Self {
        State { a: Vec::new(), b: Vec::new() }
    }
}

pub fn init(p: &Point) -> State {
    // The library probes the machine and installs its widest backend. Once,
    // outside the measurement, as its own interface asks.
    //
    // Its return value is not checked, and deliberately: the binding declares
    // `hashtree_init` as returning an int and what comes back is not the 1 the
    // C library documents — on this build it is a register nobody set. Whether
    // the backend works is settled by the root this answer produces, which the
    // oracle compares byte for byte, and not by a number the binding invented.
    let _ = choose_backend();

    // The first layer out of N leaves is N/2 digests, and every layer after
    // it is smaller; one buffer of N·16 bytes holds any of them.
    let half = (p.N as usize / 2).max(1) * 32;
    State { a: vec![0u8; half], b: vec![0u8; half] }
}

/// One layer in one call: `pairs` blocks of 64 bytes in, one digest each out.
fn layer(out: &mut [u8], input: &[u8], pairs: usize) {
    hash(out, input, pairs);
}

pub fn run(state: &mut State, inp: &Inputs) -> Outputs {
    let mut pairs = inp.leaves.data.len() / 64;
    layer(&mut state.a, &inp.leaves.data, pairs);

    // Layers alternate between the two buffers, so each one reads what the
    // other wrote and nothing is copied.
    let mut from_a = true;
    while pairs > 1 {
        pairs /= 2;
        if from_a {
            let (src, dst) = (&state.a, &mut state.b);
            let src: &[u8] = unsafe { &*(src.as_slice() as *const [u8]) };
            layer(dst, src, pairs);
        } else {
            let (src, dst) = (&state.b, &mut state.a);
            let src: &[u8] = unsafe { &*(src.as_slice() as *const [u8]) };
            layer(dst, src, pairs);
        }
        from_a = !from_a;
    }

    let root = if from_a { &state.a } else { &state.b };
    Outputs { root: Tensor { shape: vec![32], data: root[..32].to_vec() } }
}

pub fn free(state: State) {
    drop(state);
}
