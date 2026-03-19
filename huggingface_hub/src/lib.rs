//! # huggingface-hub
//!
//! Async Rust client for the [Hugging Face Hub API](https://huggingface.co/docs/hub/api).
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use huggingface_hub::{HfApi, ModelInfoParams};
//!
//! #[tokio::main]
//! async fn main() -> huggingface_hub::Result<()> {
//!     let api = HfApi::new()?;
//!     let info = api.model_info(
//!         &ModelInfoParams::builder().repo_id("gpt2").build()
//!     ).await?;
//!     println!("Model: {}", info.id);
//!     Ok(())
//! }
//! ```

pub mod api;
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod client;
pub mod constants;
pub mod error;
pub mod pagination;
pub mod types;
#[cfg(feature = "xet")]
pub mod xet;

#[cfg(feature = "blocking")]
pub use blocking::HfApiSync;
pub use client::{HfApi, HfApiBuilder};
pub use error::{HfError, Result};
pub use types::*;
