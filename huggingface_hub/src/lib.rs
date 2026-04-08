//! # huggingface-hub
//!
//! Async Rust client for the [Hugging Face Hub API](https://huggingface.co/docs/hub/api).
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use huggingface_hub::{HFClient, RepoInfoParams};
//!
//! #[tokio::main]
//! async fn main() -> huggingface_hub::Result<()> {
//!     let api = HFClient::new()?;
//!     let info = api.model("openai-community", "gpt2").info(&RepoInfoParams::default()).await?;
//!     println!("Repo: {:?}", info);
//!     Ok(())
//! }
//! ```

macro_rules! sync_api {
    (
        $(#[$impl_meta:meta])*
        impl HFClientSync {
            $(
                fn $name:ident(&self $(, $pname:ident : $ptype:ty)*) -> $ret:ty;
            )*
        }
    ) => {
        #[cfg(feature = "blocking")]
        $(#[$impl_meta])*
        impl $crate::blocking::HFClientSync {
            $(
                #[doc = concat!("Synchronous version of [`HFClient::", stringify!($name), "`].")]
                pub fn $name(&self $(, $pname : $ptype)*) -> $ret {
                    self.runtime.block_on(self.inner.$name($($pname),*))
                }
            )*
        }
    };
}

macro_rules! sync_api_stream {
    (
        $(#[$impl_meta:meta])*
        impl HFClientSync {
            $(
                fn $name:ident(&self $(, $pname:ident : $ptype:ty)*) -> $item:ty;
            )*
        }
    ) => {
        #[cfg(feature = "blocking")]
        $(#[$impl_meta])*
        impl $crate::blocking::HFClientSync {
            $(
                #[doc = concat!("Synchronous version of [`HFClient::", stringify!($name), "`]. Collects all items into a `Vec`.")]
                pub fn $name(&self $(, $pname : $ptype)*) -> $crate::error::Result<Vec<$item>> {
                    use futures::StreamExt;
                    self.runtime.block_on(async {
                        let stream = self.inner.$name($($pname),*)?;
                        futures::pin_mut!(stream);
                        let mut items = Vec::new();
                        while let Some(item) = stream.next().await {
                            items.push(item?);
                        }
                        Ok(items)
                    })
                }
            )*
        }
    };
}

pub mod api;
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod cache;
pub mod client;
pub(crate) mod constants;
pub mod error;
pub mod pagination;
pub mod repository;
pub mod types;
#[cfg(feature = "xet")]
pub mod xet;

#[cfg(feature = "blocking")]
pub use blocking::{HFBucketSync, HFClientSync, HFRepoSync, HFRepositorySync, HFSpaceSync};
pub use client::{HFClient, HFClientBuilder};
#[cfg(feature = "cli")]
#[doc(hidden)]
pub use constants::{hf_home, resolve_cache_dir};
pub use error::{HFError, Result};
pub use repository::*;
pub use types::*;
