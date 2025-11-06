use asm_core::rng::RngHandle;

use asm_mcmc::tempering;

#[test]
fn exchange_probabilities_land_in_target_band() {
    let acceptance = tempering::exchange_acceptance(10.0, 1.0, 12.0, 2.0);
    assert!(
        acceptance > 0.2 && acceptance < 0.5,
        "unexpected acceptance {acceptance}"
    );

    let mut rng = RngHandle::from_seed(0xDEADBEEF);
    let (_accepted, prob) = tempering::attempt_exchange(10.0, 1.0, 12.0, 2.0, &mut rng);
    assert!((prob - acceptance).abs() < 1e-12);
}
