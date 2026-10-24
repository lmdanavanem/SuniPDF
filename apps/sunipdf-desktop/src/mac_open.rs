//! macOS : PDF ouverts depuis le Finder (double-clic, « Ouvrir avec », dépôt sur l'icône du Dock).
//! Ils n'arrivent jamais par argv mais par `application:openURLs:` du délégué de l'application.
//! Ce délégué appartient à winit (le remplacer le fait paniquer) : la méthode est ajoutée à sa
//! classe au début du lancement, avant l'événement d'ouverture du fichier qui a lancé l'app.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::{define_class, ffi, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationWillFinishLaunchingNotification};
use objc2_foundation::{
    NSArray, NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSURL,
};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static OPENED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

type OpenUrls = unsafe extern "C-unwind" fn(&AnyObject, Sel, &NSApplication, &NSArray<NSURL>);

/// `-[delegate application:openURLs:]` ; ne panique jamais (appelé depuis AppKit).
unsafe extern "C-unwind" fn open_urls(
    _this: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
    urls: &NSArray<NSURL>,
) {
    let paths = urls.iter().filter_map(|url| url.to_file_path());
    OPENED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .extend(paths);
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; `Observer` does not implement Drop.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "SuniPDFLaunchObserver"]
    struct Observer;

    // SAFETY: NSObjectProtocol has no safety requirements.
    unsafe impl NSObjectProtocol for Observer {}

    impl Observer {
        // SAFETY: notification observers receive exactly one NSNotification.
        #[unsafe(method(willFinishLaunching:))]
        fn will_finish_launching(&self, _notification: &NSNotification) {
            let app = NSApplication::sharedApplication(self.mtm());
            let Some(delegate) = app.delegate() else {
                return;
            };
            let object: &AnyObject = (*delegate).as_ref();
            let class: *const AnyClass = object.class();
            let imp: OpenUrls = open_urls;
            // SAFETY: winit's delegate class does not define this selector, and "v@:@@" is the
            // encoding of `open_urls` (void return, self, _cmd, two objects).
            unsafe {
                ffi::class_addMethod(
                    class.cast_mut(),
                    sel!(application:openURLs:),
                    std::mem::transmute::<OpenUrls, Imp>(imp),
                    c"v@:@@".as_ptr(),
                );
            }
            // AppKit mémorise les méthodes du délégué à son affectation : le réaffecter.
            app.setDelegate(Some(&delegate));
        }
    }
);

/// À appeler avant `eframe::run_native` : la boucle d'événements crée l'application ensuite.
pub fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let this = Observer::alloc(mtm).set_ivars(());
    // SAFETY: NSObject's `init` has this signature.
    let observer: Retained<Observer> = unsafe { msg_send![super(this), init] };
    // SAFETY: `willFinishLaunching:` is defined above and takes one NSNotification.
    unsafe {
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &observer,
            sel!(willFinishLaunching:),
            Some(NSApplicationWillFinishLaunchingNotification),
            None,
        );
    }
    // Vit autant que l'application : le centre de notifications ne le retient pas.
    std::mem::forget(observer);
}

/// Contexte à réveiller quand un fichier arrive alors que l'application est inactive.
pub fn set_context(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}

/// Fichiers reçus depuis le dernier appel.
pub fn take() -> Vec<PathBuf> {
    std::mem::take(&mut *OPENED.lock().unwrap_or_else(|e| e.into_inner()))
}
