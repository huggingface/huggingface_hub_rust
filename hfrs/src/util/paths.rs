use std::path::PathBuf;

// Duplicated from huggingface_hub/src/client.rs to avoid exposing internal library functions.
// Keep in sync with the library's resolve_cache_dir() and dirs_or_home().

fn dirs_or_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

pub fn hf_home() -> PathBuf {
    if let Ok(path) = std::env::var("HF_HOME") {
        return PathBuf::from(path);
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("huggingface");
    }
    let home = dirs_or_home();
    PathBuf::from(format!("{home}/.cache/huggingface"))
}

pub fn resolve_cache_dir() -> PathBuf {
    if let Ok(cache) = std::env::var("HF_HUB_CACHE") {
        return PathBuf::from(cache);
    }
    if let Ok(cache) = std::env::var("HUGGINGFACE_HUB_CACHE") {
        return PathBuf::from(cache);
    }
    hf_home().join("hub")
}
