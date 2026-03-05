#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

pub mod api;
pub mod config;
pub mod domain;
pub mod infra;
pub mod module;

pub use module::ResourceGroupModule;
