//! Deterministic RNG wrapper and seed-derivation helpers.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use siphasher::sip::SipHasher13;
use std::hash::Hasher;

/// Deterministic RNG handle exposed to ASM consumers.
///
/// The handle is a thin wrapper around `StdRng` that documents the seeding
/// policy used throughout the project. A master `seed: u64` must be provided by
/// the caller. Substreams are derived by hashing `(master_seed, substream_id)`
/// with SipHash-1-3 configured with fixed zero keys. This rule is stable across
/// platforms and must be used whenever deterministic branching is required.
#[derive(Debug, Clone)]
pub struct RngHandle {
    rng: StdRng,
}

impl RngHandle {
    /// Creates a new RNG handle from a master seed.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Returns a mutable reference to the underlying RNG for advanced usage.
    pub fn inner_mut(&mut self) -> &mut StdRng {
        &mut self.rng
    }
}

impl RngCore for RngHandle {
    fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.rng.try_fill_bytes(dest)
    }
}

/// Derives the deterministic seed for a specific substream.
pub fn derive_substream_seed(master_seed: u64, substream: u64) -> u64 {
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    hasher.write_u64(master_seed);
    hasher.write_u64(substream);
    hasher.finish()
}
