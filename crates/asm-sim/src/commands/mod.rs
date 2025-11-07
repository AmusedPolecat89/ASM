#![allow(
    clippy::ptr_arg,
    clippy::vec_init_then_push,
    clippy::field_reassign_with_default
)]
pub mod ablation;
pub mod assert;
pub mod assert_batch;
pub mod deform;
pub mod demo;
pub mod doctor;
pub mod extract;
pub mod fit_couplings;
pub mod fit_running;
pub mod gaps;
pub mod gauge;
pub mod gauge_batch;
pub mod gauge_compare;
pub mod interact;
pub mod interact_batch;
pub mod landscape;
pub mod paper_pack;
pub mod plugin;
pub mod publish;
pub mod report;
pub mod rg;
pub mod rg_covariance;
pub mod spectrum;
pub mod spectrum_batch;
pub mod submit;
pub mod sweep;
pub mod verify;
pub mod version;
pub mod web;
