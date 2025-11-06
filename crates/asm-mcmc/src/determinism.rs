use asm_core::derive_substream_seed;

/// Derives the deterministic seed used for a specific replica.
pub fn replica_seed(master_seed: u64, replica_index: usize) -> u64 {
    derive_substream_seed(master_seed, replica_index as u64)
}

/// Derives the deterministic seed for a move proposal executed during a sweep.
pub fn move_seed(master_seed: u64, replica_index: usize, sweep: usize, move_slot: usize) -> u64 {
    let intermediate =
        derive_substream_seed(master_seed, (replica_index as u64) << 32 | sweep as u64);
    derive_substream_seed(intermediate, move_slot as u64)
}

/// Deterministic identifier for exchange proposals between replicas.
pub fn exchange_seed(master_seed: u64, sweep: usize, pair_index: usize) -> u64 {
    derive_substream_seed(
        master_seed ^ 0xA5A5_A5A5_A5A5_A5A5,
        (sweep as u64) << 16 | pair_index as u64,
    )
}
