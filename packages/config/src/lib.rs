pub mod config;
pub mod environment;
pub mod one_or_many;
pub mod types;

// Re-export the trait and the singleton getter as they're used in lib.rs
pub use config::ConfigTrait;
pub use one_or_many::OneOrMany;
