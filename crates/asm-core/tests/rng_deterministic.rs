use asm_core::rng::RngHandle;
use rand::RngCore;

#[test]
fn rng_emits_reproducible_sequence() {
    let mut rng_a = RngHandle::from_seed(1234);
    let mut rng_b = RngHandle::from_seed(1234);

    let seq_a: Vec<u64> = (0..100).map(|_| rng_a.next_u64()).collect();
    let seq_b: Vec<u64> = (0..100).map(|_| rng_b.next_u64()).collect();

    assert_eq!(seq_a, seq_b);
}
