use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    fs::File,
    io::{self, Read},
    path::Path,
    rc::Rc,
};

use crate::MAX_MODEL_BYTES;
use openusd::ar::{self, Asset, DefaultResolver, ResolvedPath, Resolver};

#[derive(Clone)]
pub(super) struct Assets(Rc<State>);
struct State {
    resolver: DefaultResolver,
    seen: RefCell<HashSet<String>>,
    used: Cell<u64>,
}

impl Assets {
    pub fn new(parent: &Path) -> Self {
        Self(Rc::new(State {
            resolver: DefaultResolver::with_search_paths([parent]),
            seen: RefCell::new(HashSet::new()),
            used: Cell::new(0),
        }))
    }

    fn charge(&self, key: String, bytes: u64) -> io::Result<()> {
        let mut seen = self.0.seen.borrow_mut();
        if seen.contains(&key) {
            return Ok(());
        }
        let used = self
            .0
            .used
            .get()
            .checked_add(bytes)
            .ok_or_else(|| io::Error::other("USD asset size overflow"))?;
        if used > MAX_MODEL_BYTES || seen.len() >= 1024 {
            return Err(io::Error::other(
                "USD assets exceed the 300 MiB / 1024-asset limit",
            ));
        }
        seen.insert(key);
        self.0.used.set(used);
        Ok(())
    }

    fn package(&self, path: &Path) -> io::Result<()> {
        let key = format!("package:{}", path.display());
        if self.0.seen.borrow().contains(&key) {
            return Ok(());
        }
        let file = File::open(path)?;
        if file.metadata()?.len() > MAX_MODEL_BYTES {
            return Err(io::Error::other("USDZ exceeds 300 MiB"));
        }
        let mut archive = zip::ZipArchive::new(file).map_err(io::Error::other)?;
        if archive.len() > 20_000 {
            return Err(io::Error::other("too many USDZ entries"));
        }
        let mut bytes = 0u64;
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(io::Error::other)?;
            if entry.enclosed_name().is_none()
                || entry.compression() != zip::CompressionMethod::Stored
            {
                return Err(io::Error::other(
                    "USDZ requires contained, uncompressed entries",
                ));
            }
            bytes = bytes
                .checked_add(entry.size())
                .ok_or_else(|| io::Error::other("USDZ size overflow"))?;
            if bytes > MAX_MODEL_BYTES {
                return Err(io::Error::other("USDZ contents exceed 300 MiB"));
            }
        }
        self.charge(key, bytes)
    }

    pub fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let resolved = self
            .resolve(path)
            .ok_or_else(|| io::Error::other("USD asset not found"))?;
        let mut data = Vec::new();
        self.open_asset(&resolved)?
            .take(MAX_MODEL_BYTES + 1)
            .read_to_end(&mut data)?;
        if data.len() as u64 > MAX_MODEL_BYTES {
            return Err(io::Error::other("USD asset exceeds 300 MiB"));
        }
        Ok(data)
    }
}

impl Resolver for Assets {
    fn create_identifier(&self, path: &str, anchor: Option<&ResolvedPath>) -> String {
        self.0.resolver.create_identifier(path, anchor)
    }
    fn resolve(&self, path: &str) -> Option<ResolvedPath> {
        if path.contains("://") || path.starts_with("data:") {
            return None;
        }
        self.0.resolver.resolve(path)
    }
    fn resolve_for_new_asset(&self, path: &str) -> Option<ResolvedPath> {
        self.0.resolver.resolve_for_new_asset(path)
    }
    fn identity(&self) -> String {
        format!("meshthumbs:{}", self.0.resolver.identity())
    }
    fn open_asset(&self, path: &ResolvedPath) -> io::Result<Box<dyn Asset>> {
        let name = path.to_string();
        if let Some((package, inner)) = ar::split_package_relative_path_outer(&name) {
            if ar::is_package_relative_path(&inner) {
                return Err(io::Error::other("nested USDZ packages are unsupported"));
            }
            self.package(Path::new(&package))?;
            let mut archive =
                zip::ZipArchive::new(File::open(package)?).map_err(io::Error::other)?;
            let entry = archive.by_name(&inner).map_err(io::Error::other)?;
            let mut bytes = Vec::new();
            entry.take(MAX_MODEL_BYTES + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_MODEL_BYTES {
                return Err(io::Error::other("USDZ entry exceeds 300 MiB"));
            }
            return Ok(Box::new(io::Cursor::new(bytes)));
        }
        let file = File::open(&**path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::other("USD assets must be regular files"));
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("usdz"))
        {
            self.package(path)?;
        } else {
            self.charge(name, metadata.len())?;
        }
        Ok(Box::new(file))
    }
}
