# The SSZ hash tree root, three ways

Three answers to `hash-tree-root/sha256@1.0.0` on FHERMA. The kernel is what
a beacon node spends its state processing on: N chunks of 32 bytes merkleised
with SHA-256, pairwise, until one node is left.

| directory | the hash |
| --- | --- |
| `sha2` | SHA-256 in portable Rust, one digest per pair |
| `ring` | SHA-256 from BoringSSL's lineage, which uses the machine's SHA extensions where it has them |
| `hashtree` | what Prysm and Lighthouse merkleise with: Intel's assembly with the message schedule hardcoded, and **one call per layer** rather than per pair |

The third is the interesting one, and not only for its compression function.
`hashtree_hash(out, in, count)` takes a whole layer and compresses sixteen
blocks at a time where the machine allows it, so the loop the other two write
in Rust is inside the library and vectorised.

None of the three allocates in the measured call: two buffers are taken in
`init` and the layers ping-pong between them, because allocating sixteen
megabytes per case would be most of what a fast hash saves.

Checked against each other on a 16384-leaf case before any was submitted —
identical roots, and `hashtree` ahead even on an arm64 machine where it has
only the NEON path.

One note on the bindings: `hashtree_rs::init()` is declared as returning an
int and does not return the 1 the C library documents — on the build used
here it is a register nobody set. The answer calls it and ignores the value,
because whether the backend works is settled by the root the oracle compares
byte for byte.
