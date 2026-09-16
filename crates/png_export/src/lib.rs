use std::path::{Path, PathBuf};

pub const SIZES: [u32; 3] = [256, 512, 1024];
pub const MAX_FILES: usize = 10_000;

pub fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            renderer::SUPPORTED_EXTENSIONS
                .iter()
                .any(|e| ext.eq_ignore_ascii_case(e))
        })
}

pub fn selection(mut files: Vec<PathBuf>) -> anyhow::Result<Vec<PathBuf>> {
    anyhow::ensure!(
        !files.is_empty() && files.len() <= MAX_FILES,
        "Select between 1 and {MAX_FILES} supported files"
    );
    for file in &mut files {
        anyhow::ensure!(supported(file), "Unsupported file: {}", file.display());
        *file = std::path::absolute(&*file)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(windows)]
pub mod batch;
#[cfg(windows)]
pub mod job;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_is_supported_sorted_and_deduplicated() {
        let files = selection(vec!["z.OBJ".into(), "a.fbx".into(), "z.OBJ".into()]).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].file_name().unwrap(), "a.fbx");
        assert!(selection(vec!["a.txt".into()]).is_err());
        assert!(selection(vec![]).is_err());
    }
}
