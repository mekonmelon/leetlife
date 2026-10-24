//! Native NSStatusBarButton action. Keeping the click target on the button
//! avoids animated-image replacement interfering with the tray overlay's hit area.
use objc2::{define_class, msg_send, sel, DeclaredClass, MainThreadMarker, MainThreadOnly};
use objc2::rc::Retained;
use objc2_foundation::{NSObject, NSObjectProtocol, NSString, NSUserDefaults};
use std::cell::RefCell;
use tauri::AppHandle;

struct TargetState { app: AppHandle }
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LeetLifeSpiritStatusTarget"]
    #[ivars = TargetState]
    struct SpiritStatusTarget;
    unsafe impl NSObjectProtocol for SpiritStatusTarget {}
    impl SpiritStatusTarget {
        #[unsafe(method(visitSpirit:))]
        fn visit_spirit(&self, _sender: Option<&NSObject>) {
            super::toggle_summary(&self.ivars().app);
        }
    }
);
thread_local! {
    // NSControl's target is weak. Retain it on its owning main thread.
    static TARGET: RefCell<Option<Retained<SpiritStatusTarget>>> = const { RefCell::new(None) };
}

pub fn install(tray: &tauri::tray::TrayIcon, app: AppHandle) -> tauri::Result<()> {
    tray.with_inner_tray_icon(move |icon| {
        let mtm = MainThreadMarker::new().expect("status button must be configured on the main thread");
        if let Some(item) = icon.ns_status_item() {
            unsafe {
                item.setMenu(None);
                // New items otherwise land at the far left of a crowded menu
                // bar, which can be behind a MacBook notch. Seed only our own
                // initial slot; macOS retains later user rearrangements.
                let preferences = NSUserDefaults::standardUserDefaults();
                let position_key = NSString::from_str("NSStatusItem Preferred Position LeetLifeSpirit");
                if preferences.objectForKey(&position_key).is_none() {
                    preferences.setInteger_forKey(0, &position_key);
                }
                item.setAutosaveName(Some(&NSString::from_str("LeetLifeSpirit")));
                if let Some(button) = item.button(mtm) {
                    for child in button.subviews() { child.removeFromSuperview(); }
                    let target = mtm.alloc().set_ivars(TargetState { app });
                    let target: Retained<SpiritStatusTarget> = msg_send![super(target), init];
                    button.setTarget(Some(&target));
                    button.setAction(Some(sel!(visitSpirit:)));
                    TARGET.with(|slot| { *slot.borrow_mut() = Some(target); });
                }
            }
        }
    })?;
    Ok(())
}
