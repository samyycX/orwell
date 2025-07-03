pub struct Notifier {}

impl Notifier {
    pub fn notify_message(username: &str, message: &str) {
        #[cfg(target_os = "windows")]
        unsafe {
            use winapi::um::{
                processthreadsapi::GetCurrentProcessId,
                wincon::GetConsoleWindow,
                winuser::{
                    FlashWindowEx, GetFocus, GetForegroundWindow, GetParent,
                    GetWindowThreadProcessId, FLASHWINFO, FLASHW_TIMERNOFG, FLASHW_TRAY,
                },
            };

            use crate::message::{add_debug_message, MessageLevel};

            let hwnd = GetForegroundWindow();
            let mut focus_process_id = 0;
            // name
            GetWindowThreadProcessId(hwnd, &mut focus_process_id);
            let this_process_hwnd = GetConsoleWindow();
            let parent_hwnd = GetParent(this_process_hwnd);
            let mut parent_process_id = 0;
            GetWindowThreadProcessId(parent_hwnd, &mut parent_process_id);

            if focus_process_id != parent_process_id {
                use notify_rust::Notification;

                Notification::new()
                    .appname("Orwell")
                    .summary(username)
                    .body(message)
                    .show()
                    .unwrap();

                let mut pfwi = FLASHWINFO {
                    cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                    hwnd: parent_hwnd,
                    dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
                    uCount: u32::MAX,
                    dwTimeout: 100,
                };

                FlashWindowEx(&mut pfwi);
            }
        }
    }
}
