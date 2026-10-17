//! macOS : barre de menus du système, construite depuis le registre de commandes du bouton
//! « Menu ». Une commande choisie passe par `SuniApp::execute`, comme dans l'application.
//! Seuls les raccourcis sûrs pendant une saisie y sont posés (les autres restent à egui : ⌘⌫ dans
//! un champ efface du texte, pas une page).

use egui::{Key, Modifiers};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::{Command, Menu};
use sunipdf_i18n::{tr, Language};

/// Menus du document, dans l'ordre du bouton « Menu », avec leurs séparateurs.
const MENUS: &[(&str, Menu, &[Command])] = &[
    ("menu.file", Menu::File, &[Command::Save]),
    ("menu.tools", Menu::Tools, &[]),
    ("menu.edit", Menu::Edit, &[]),
    ("menu.view", Menu::View, &[Command::FirstPage]),
    ("menu.pages", Menu::Pages, &[]),
    (
        "menu.forms",
        Menu::Forms,
        &[Command::ClearForm, Command::ExportFormData],
    ),
    ("menu.convert", Menu::Convert, &[]),
    ("menu.security", Menu::Security, &[]),
    ("menu.help", Menu::Help, &[]),
];

/// Étiquette d'une entrée (`tag`) → commande et besoin d'un document ouvert.
static ITEMS: Mutex<Vec<(Command, bool)>> = Mutex::new(Vec::new());
static CHOSEN: Mutex<Vec<Command>> = Mutex::new(Vec::new());
static HAS_DOCUMENT: AtomicBool = AtomicBool::new(false);
static BLOCKED: AtomicBool = AtomicBool::new(false);
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

thread_local! {
    static BUILT: RefCell<Option<(Language, Retained<Target>)>> = const { RefCell::new(None) };
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; `Target` does not implement Drop.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "SuniPDFMenuTarget"]
    struct Target;

    // SAFETY: NSObjectProtocol has no safety requirements.
    unsafe impl NSObjectProtocol for Target {}

    impl Target {
        // SAFETY: menu actions receive their NSMenuItem as sender.
        #[unsafe(method(run:))]
        fn run(&self, item: &NSMenuItem) {
            if let Some(&(command, _)) = lock(&ITEMS).get(item.tag() as usize) {
                lock(&CHOSEN).push(command);
                if let Some(ctx) = CONTEXT.get() {
                    ctx.request_repaint();
                }
            }
        }

        // SAFETY: AppKit's informal NSMenuItemValidation signature.
        #[unsafe(method(validateMenuItem:))]
        fn validate(&self, item: &NSMenuItem) -> bool {
            let entry = lock(&ITEMS).get(item.tag() as usize).copied();
            entry.is_some_and(|(command, needs_document)| {
                command == Command::Quit
                    || (!BLOCKED.load(Ordering::Relaxed)
                        && (!needs_document || HAS_DOCUMENT.load(Ordering::Relaxed)))
            })
        }
    }
);

/// À chaque image : état des entrées, et reconstruction si la langue a changé.
pub fn sync(
    ctx: &egui::Context,
    lang: Language,
    has_document: bool,
    blocked: bool,
    shortcuts: &[(Modifiers, Key, Command)],
) {
    HAS_DOCUMENT.store(has_document, Ordering::Relaxed);
    BLOCKED.store(blocked, Ordering::Relaxed);
    let _ = CONTEXT.set(ctx.clone());
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    BUILT.with_borrow_mut(|built| {
        if built.as_ref().is_some_and(|(l, _)| *l == lang) {
            return;
        }
        let target = match built.take() {
            Some((_, target)) => target,
            None => {
                let this = Target::alloc(mtm).set_ivars(());
                // SAFETY: NSObject's `init` has this signature.
                unsafe { msg_send![super(this), init] }
            }
        };
        build(mtm, lang, shortcuts, &target);
        *built = Some((lang, target));
    });
}

/// Commandes choisies dans la barre de menus depuis le dernier appel.
pub fn take() -> Vec<Command> {
    std::mem::take(&mut *lock(&CHOSEN))
}

fn build(
    mtm: MainThreadMarker,
    lang: Language,
    shortcuts: &[(Modifiers, Key, Command)],
    target: &Target,
) {
    let mut items = Vec::new();
    let main = NSMenu::new(mtm);
    let mut command = |menu: &NSMenu, c: Command, key: &str, flags: NSEventModifierFlags| {
        let info = c.info();
        let item = menu_item(mtm, tr(lang, info.label_key), Some(sel!(run:)), key, flags);
        item.setTag(items.len() as isize);
        // SAFETY: `Target` implements `run:` and `validateMenuItem:`, and outlives the menu.
        unsafe { item.setTarget(Some(target as &AnyObject)) };
        menu.addItem(&item);
        items.push((c, info.needs_document));
    };

    let app = submenu(mtm, &main, "SuniPDF");
    command(&app, Command::About, "", NSEventModifierFlags::empty());
    app.addItem(&NSMenuItem::separatorItem(mtm));
    let system = |menu: &NSMenu, key: &str, action: Sel, equivalent: &str, flags| {
        menu.addItem(&menu_item(
            mtm,
            tr(lang, key),
            Some(action),
            equivalent,
            flags,
        ));
    };
    system(
        &app,
        "menu.app.hide",
        sel!(hide:),
        "h",
        NSEventModifierFlags::Command,
    );
    system(
        &app,
        "menu.app.hide_others",
        sel!(hideOtherApplications:),
        "h",
        NSEventModifierFlags::Command | NSEventModifierFlags::Option,
    );
    system(
        &app,
        "menu.app.show_all",
        sel!(unhideAllApplications:),
        "",
        NSEventModifierFlags::empty(),
    );
    app.addItem(&NSMenuItem::separatorItem(mtm));
    command(&app, Command::Quit, "q", NSEventModifierFlags::Command);

    for &(title_key, menu, separators) in MENUS {
        let sub = submenu(mtm, &main, tr(lang, title_key));
        for info in Command::in_menu(menu) {
            if matches!(info.command, Command::About | Command::Quit) {
                continue;
            }
            if separators.contains(&info.command) {
                sub.addItem(&NSMenuItem::separatorItem(mtm));
            }
            let (key, flags) = key_equivalent(shortcuts, info.command);
            command(&sub, info.command, &key, flags);
        }
    }

    let window = submenu(mtm, &main, tr(lang, "menu.window"));
    system(
        &window,
        "menu.window.minimize",
        sel!(performMiniaturize:),
        "m",
        NSEventModifierFlags::Command,
    );
    system(
        &window,
        "menu.window.zoom",
        sel!(performZoom:),
        "",
        NSEventModifierFlags::empty(),
    );

    *lock(&ITEMS) = items;
    let ns_app = NSApplication::sharedApplication(mtm);
    ns_app.setMainMenu(Some(&main));
    ns_app.setWindowsMenu(Some(&window));
}

/// Raccourci ⌘ d'une commande (lettre seule, Maj éventuel), sinon aucun.
fn key_equivalent(
    shortcuts: &[(Modifiers, Key, Command)],
    command: Command,
) -> (String, NSEventModifierFlags) {
    shortcuts
        .iter()
        .find(|(m, k, c)| *c == command && m.command && k.name().len() == 1)
        .map_or(
            (String::new(), NSEventModifierFlags::empty()),
            |(m, k, _)| {
                let mut flags = NSEventModifierFlags::Command;
                if m.shift {
                    flags |= NSEventModifierFlags::Shift;
                }
                (k.name().to_lowercase(), flags)
            },
        )
}

fn menu_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<Sel>,
    key: &str,
    flags: NSEventModifierFlags,
) -> Retained<NSMenuItem> {
    // SAFETY: `action` is either one of `Target`'s selectors or a standard NSApplication/NSWindow
    // action resolved through the responder chain.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key),
        )
    };
    item.setKeyEquivalentModifierMask(flags);
    item
}

fn submenu(mtm: MainThreadMarker, main: &NSMenu, title: &str) -> Retained<NSMenu> {
    let title = NSString::from_str(title);
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);
    let item = NSMenuItem::new(mtm);
    item.setTitle(&title);
    item.setSubmenu(Some(&menu));
    main.addItem(&item);
    menu
}
