//! iOS's pickers, share sheet and pasteboard (UIKit), on the main thread: the app's, whose frames call
//! [`super::poll`], from which each of these is called.
//!
//! The pickers are `UIDocumentPickerViewController`s, presented over the app's top view controller: files are
//! picked as copies (the system copies them into the app's container), which are moved into their lot; a folder
//! is picked as it is, its access held (it is security-scoped) while its wanted files are copied out on a thread of
//! their own, and let go once its answer is heard. What is handed over goes to the share sheet
//! (`UIActivityViewController`), from which the player saves it to Files, AirDrops it or sends it.

use super::{Answer, answer, lot};
use crate::dialog::wanted::wanted_in;
use crate::dialog::Filter;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{
    UIActivityViewController, UIApplication, UIDocumentPickerDelegate, UIDocumentPickerViewController, UIPasteboard, UIViewController,
    UIWindowScene,
};
use objc2_uniform_type_identifiers::UTType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

/// What a picker asked for: its request, and for a folder the paths wanted in it.
struct Asked {
    request: u64,
    folder: Option<Vec<String>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NettaiDocumentPicked"]
    #[ivars = Asked]
    struct Picked;

    unsafe impl NSObjectProtocol for Picked {}

    unsafe impl UIDocumentPickerDelegate for Picked {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn did_pick(&self, _picker: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
            picked(self.ivars(), urls);
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn was_cancelled(&self, _picker: &UIDocumentPickerViewController) {
            let asked = self.ivars();
            answer(asked.request, if asked.folder.is_some() { Answer::Folder(None) } else { Answer::Files(Vec::new()) });
        }
    }
);

impl Picked {
    fn new(main: MainThreadMarker, asked: Asked) -> Retained<Picked> {
        let this = Picked::alloc(main).set_ivars(asked);
        // SAFETY: NSObject's `init`, on a class of NSObject's.
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    /// The pickers' delegates while they are up (a picker holds its delegate weakly), by request.
    static DELEGATES: RefCell<HashMap<u64, Retained<Picked>>> = RefCell::new(HashMap::new());
    /// The folders whose access is held while their files are copied, by request.
    static HELD: RefCell<HashMap<u64, Retained<NSURL>>> = RefCell::new(HashMap::new());
}

/// Request `request`'s answer is heard: its delegate and its folder's access let go.
pub fn answered(request: u64) {
    DELEGATES.with(|d| d.borrow_mut().remove(&request));
    if let Some(folder) = HELD.with(|h| h.borrow_mut().remove(&request)) {
        // SAFETY: access to this URL was started when it was picked, and is stopped once.
        unsafe { folder.stopAccessingSecurityScopedResource() };
    }
}

pub fn open_files(request: u64, filter: Filter) {
    let Some(main) = MainThreadMarker::new() else { return refuse(request, false, "not on the main thread") };
    let types: Vec<Retained<UTType>> = filter
        .extensions
        .iter()
        .filter_map(|e| UTType::typeWithFilenameExtension(&NSString::from_str(e)))
        // SAFETY: the framework's constant.
        .chain(filter.extensions.is_empty().then(|| unsafe { objc2_uniform_type_identifiers::UTTypeData }.retain()))
        .collect();
    let picker = UIDocumentPickerViewController::initForOpeningContentTypes_asCopy(UIDocumentPickerViewController::alloc(main), &NSArray::from_retained_slice(&types), true);
    picker.setAllowsMultipleSelection(true);
    present(main, &picker, Asked { request, folder: None });
}

pub fn open_folder(request: u64, wanted: Vec<String>) {
    let Some(main) = MainThreadMarker::new() else { return refuse(request, true, "not on the main thread") };
    // SAFETY: the framework's constant.
    let folder = unsafe { objc2_uniform_type_identifiers::UTTypeFolder };
    let picker = UIDocumentPickerViewController::initForOpeningContentTypes(UIDocumentPickerViewController::alloc(main), &NSArray::from_slice(&[folder]));
    present(main, &picker, Asked { request, folder: Some(wanted) });
}

/// The picker over the app's top view controller, `asked` its delegate.
fn present(main: MainThreadMarker, picker: &UIDocumentPickerViewController, asked: Asked) {
    let (request, folder) = (asked.request, asked.folder.is_some());
    let Some(top) = top(main) else { return refuse(request, folder, "no view controller to present the picker over") };
    let delegate = Picked::new(main, asked);
    picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    DELEGATES.with(|d| d.borrow_mut().insert(request, delegate));
    top.presentViewController_animated_completion(picker, true, None);
}

/// Request `request` answered with nothing, and why logged.
fn refuse(request: u64, folder: bool, why: &str) {
    log::warn!("opening the picker: {why}");
    answer(request, if folder { Answer::Folder(None) } else { Answer::Files(Vec::new()) });
}

/// What was picked, read in: files (copies already) moved into their lot; a folder's wanted files copied out of it.
fn picked(asked: &Asked, urls: &NSArray<NSURL>) {
    let request = asked.request;
    let paths: Vec<PathBuf> = urls.iter().filter_map(|u| u.path()).map(|p| PathBuf::from(p.to_string())).collect();
    let Some(wanted) = &asked.folder else {
        let files = match lot(request) {
            Ok(dir) => paths
                .into_iter()
                .filter_map(|from| {
                    let to = dir.join(from.file_name()?);
                    std::fs::rename(&from, &to).map(|_| to).map_err(|e| log::warn!("reading in {}: {e}", from.display())).ok()
                })
                .collect(),
            Err(why) => {
                log::warn!("reading in the files picked: {why}");
                Vec::new()
            }
        };
        return answer(request, Answer::Files(files));
    };
    let (Some(url), Some(root)) = (urls.firstObject(), paths.into_iter().next()) else { return answer(request, Answer::Folder(None)) };
    // SAFETY: the URL the picker gave; its access is stopped once, as its answer is heard (`answered`).
    if unsafe { url.startAccessingSecurityScopedResource() } {
        HELD.with(|h| h.borrow_mut().insert(request, url));
    }
    let wanted = wanted.clone();
    let copying = std::thread::Builder::new().name("nettai-picked".into()).spawn(move || {
        let made = (|| -> Result<PathBuf, String> {
            let dir = lot(request)?;
            for file in wanted_in(&root, &wanted) {
                let to = dir.join(&file);
                if let Some(folder) = to.parent() {
                    std::fs::create_dir_all(folder).map_err(|e| format!("making {}: {e}", folder.display()))?;
                }
                std::fs::copy(root.join(&file), &to).map_err(|e| format!("copying {file}: {e}"))?;
            }
            Ok(dir)
        })();
        answer(request, Answer::Folder(made.map_err(|why| log::warn!("reading in the folder picked: {why}")).ok()));
    });
    if let Err(why) = copying {
        log::warn!("reading in the folder picked: no thread: {why}");
        answer(request, Answer::Folder(None));
    }
}

/// The file at `path` to the share sheet, over the app's top view controller (on an iPad a popover from the
/// screen's foot).
pub fn hand_over(path: PathBuf) {
    let Some(main) = MainThreadMarker::new() else { return log::warn!("handing over {}: not on the main thread", path.display()) };
    let Some(top) = top(main) else { return log::warn!("handing over {}: no view controller to present over", path.display()) };
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let item: &AnyObject = &url;
    // SAFETY: the items are objects the share sheet takes (a file URL).
    let sheet = unsafe { UIActivityViewController::initWithActivityItems_applicationActivities(UIActivityViewController::alloc(main), &NSArray::from_slice(&[item]), None) };
    if let (Some(popover), Some(view)) = (sheet.popoverPresentationController(), top.view()) {
        let bounds = view.bounds();
        popover.setSourceView(Some(&view));
        popover.setSourceRect(CGRect::new(CGPoint::new(bounds.size.width / 2.0, bounds.size.height - 1.0), CGSize::new(1.0, 1.0)));
    }
    top.presentViewController_animated_completion(&sheet, true, None);
}

pub fn copy(text: &str) {
    // SAFETY: a string, as the pasteboard takes.
    unsafe { UIPasteboard::generalPasteboard().setString(Some(&NSString::from_str(text))) };
}

/// The view controller on top: the app's window's root, or what it presents (a picker presented over another's).
fn top(main: MainThreadMarker) -> Option<Retained<UIViewController>> {
    let scenes = UIApplication::sharedApplication(main).connectedScenes().allObjects();
    let window = scenes.iter().filter_map(|s| s.downcast::<UIWindowScene>().ok()).find_map(|s| s.keyWindow().or_else(|| s.windows().firstObject()))?;
    let mut top = window.rootViewController()?;
    while let Some(over) = top.presentedViewController() {
        top = over;
    }
    Some(top)
}
