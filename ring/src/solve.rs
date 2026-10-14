//! The SSZ hash tree root, with SHA-256 from `ring`.
//!
//! The same loop over pairs, through BoringSSL's lineage: `ring` picks its
//! implementation at run time and uses the SHA extensions where the machine
//! has them. One digest per pair either way — what differs from the portable
//! row is the compression function, not the shape of the tree.
//!
//! Nothing is allocated in the measured call. Two buffers are taken in `init`
//! and the layers ping-pong between them, because allocating sixteen
//! megabytes per case would be most of what a fast hash saves.
use ring::digest::{digest, SHA256};

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
    // The first layer out of N leaves is N/2 digests, and every layer after
    // it is smaller; one buffer of N·16 bytes holds any of them.
    let half = (p.N as usize / 2).max(1) * 32;
    State { a: vec![0u8; half], b: vec![0u8; half] }
}

/// One layer: `pairs` blocks of 64 bytes in, one digest each out.
fn layer(out: &mut [u8], input: &[u8], pairs: usize) {
    for at in 0..pairs {
        let hashed = digest(&SHA256, &input[at * 64..(at + 1) * 64]);
        out[at * 32..(at + 1) * 32].copy_from_slice(hashed.as_ref());
    }
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
