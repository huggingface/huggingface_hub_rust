//! # hf-hub
//!
//! Async Rust client for the [Hugging Face Hub API](https://huggingface.co/docs/hub/api).
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hf_hub::{HfApi, ModelInfoParams};
//!
//! #[tokio::main]
//! async fn main() -> hf_hub::Result<()> {
//!     let api = HfApi::new()?;
//!     let info = api.model_info(
//!         &ModelInfoParams::builder().repo_id("gpt2").build()
//!     ).await?;
//!     println!("Model: {}", info.id);
//!     Ok(())
//! }
//! ```

pub mod constants;
pub mod error;
pub mod types;
pub mod client;
pub mod pagination;
pub mod api;
pub(crate) mod xet;

pub use client::{HfApi, HfApiBuilder};
pub use error::{HfError, Result};
pub use types::*;
