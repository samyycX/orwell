pub struct Notifier {}

impl Notifier {
    pub fn is_focused() -> bool {
        #[cfg(target_os = "windows")]
        unsafe {
            use winapi::um::wincon::GetConsoleWindow;
            use winapi::um::winuser::{GetForegroundWindow, GetParent, GetWindowThreadProcessId};

            let hwnd = GetForegroundWindow();
            let mut focus_process_id = 0;
            // name
            GetWindowThreadProcessId(hwnd, &mut focus_process_id);
            let this_process_hwnd = GetConsoleWindow();
            let parent_hwnd = GetParent(this_process_hwnd);
            let mut parent_process_id = 0;
            GetWindowThreadProcessId(parent_hwnd, &mut parent_process_id);

            focus_process_id == parent_process_id
        }
        #[cfg(not(target_os = "windows"))]
        {
            true // Assume focused on non-Windows platforms
        }
    }
    pub fn notify_message(username: &str, message: &str) {
        #[cfg(target_os = "windows")]
        unsafe {
            use winapi::um::wincon::GetConsoleWindow;
            use winapi::um::winuser::{
                FlashWindowEx, GetForegroundWindow, GetParent, GetWindowThreadProcessId,
                FLASHWINFO, FLASHW_TIMERNOFG, FLASHW_TRAY,
            };

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
