//! The nettai app in a browser: what the page calls (web/index.html).
//!
//! The page, before the app starts: it puts the files it keeps (the data folder: the settings, the builds, the
//! replays, the packs) in the app's files with [`preload`], and fetches the content's archive for [`start`]. After:
//! it takes what changed in the data folder ([`changes`]) now and then, and writes it to its storage (the origin
//! private file system). Built with Wasm's atomics, so the app's threads are Web Workers on its memory
//! (nettai-thread); build.sh builds it and the page, and serve.py serves them as a page with threads must be.

/// Where the data folder is in the app's files (`nettai::paths::data`, in the browser).
pub const DATA: &str = "/data";

/// Where the content is (`nettai_content::index::content`, in the browser).
pub const CONTENT: &str = "/content";

/// The files of the content's archive (a tar of content/), each where it goes in the app's files.
pub fn content_files(archive: &[u8]) -> std::io::Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
    let mut files = Vec::new();
    for entry in tar::Archive::new(archive).entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = std::path::Path::new(CONTENT).join(entry.path()?);
        let mut data = Vec::with_capacity(entry.size() as usize);
        std::io::Read::read_to_end(&mut entry, &mut data)?;
        files.push((path, data));
    }
    Ok(files)
}

#[cfg(target_arch = "wasm32")]
mod page {
    use wasm_bindgen::prelude::*;

    /// A file the page keeps, put in the data folder (`path` within it) before the app starts.
    #[wasm_bindgen]
    pub fn preload(path: &str, data: Vec<u8>) {
        nettai_fs::preload(std::path::Path::new(super::DATA).join(path.trim_start_matches('/')), data);
    }

    /// The app started, on the content's archive (`content`). A panic is said on the console. It returns once the
    /// window's loop is running on the page's; an error if the app couldn't start.
    #[wasm_bindgen]
    pub fn start(content: &[u8]) -> Result<(), JsError> {
        console_error_panic_hook::set_once();
        for (path, data) in super::content_files(content).map_err(|e| JsError::new(&format!("the content's archive: {e}")))? {
            nettai_fs::preload(path, data);
        }
        nettai_fs::watch(super::DATA);
        nettai::run().map_err(|e| JsError::new(&e.to_string()))
    }

    /// What changed in the data folder since the last call, in order: each `["write", path, bytes]` or
    /// `["remove", path]` (a file or a folder and all in it), `path` within the data folder; none in a folder whose
    /// name starts with a dot.
    #[wasm_bindgen]
    pub fn changes() -> js_sys::Array {
        let out = js_sys::Array::new();
        for change in nettai_fs::changes() {
            let (kind, path, data) = match change {
                nettai_fs::Change::Write(path, data) => ("write", path, Some(data)),
                nettai_fs::Change::Remove(path) => ("remove", path, None),
            };
            let Ok(path) = path.strip_prefix(super::DATA) else { continue };
            // (A folder whose name starts with a dot is the app's scratch, as a pack is made in packs/.new and
            // moved into place: what is kept is the pack, once, as it lands.)
            if path.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
                continue;
            }
            let entry = js_sys::Array::of2(&kind.into(), &path.to_string_lossy().as_ref().into());
            if let Some(data) = data {
                entry.push(&js_sys::Uint8Array::from(data.as_slice()));
            }
            out.push(&entry);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    /// The content's archive is unpacked under the content's folder, its folders' entries passed over.
    #[test]
    fn the_content_is_unpacked_where_the_app_reads_it() {
        let mut archive = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(6);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        archive.append_data(&mut header, "exe6/init.luau", &b"return"[..]).unwrap();
        let archive = archive.into_inner().unwrap();
        let files = super::content_files(&archive).unwrap();
        assert_eq!(files, [(std::path::PathBuf::from("/content/exe6/init.luau"), b"return".to_vec())]);
    }
}
