use std::ffi::CString;

pub const ANY_BROWSER: usize = 1;

unsafe extern "C" {
    fn webui_new_window() -> usize;
    fn webui_set_config(option: u32, status: bool);
    fn webui_show_browser(window: usize, content: *const std::ffi::c_char, browser: usize) -> bool;
    fn webui_show_wv(window: usize, content: *const std::ffi::c_char) -> bool;
    fn webui_set_size(window: usize, width: u32, height: u32);
    fn webui_set_minimum_size(window: usize, width: u32, height: u32);
    fn webui_set_resizable(window: usize, status: bool);
    fn webui_set_frameless(window: usize, status: bool);
    fn webui_set_icon_file(window: usize, path: *const std::ffi::c_char);
    fn webui_minimize(window: usize);
    fn webui_maximize(window: usize);
    fn webui_close(window: usize);
    fn webui_clean();
}

#[derive(Clone, Copy)]
pub struct Window(pub usize);

impl Window {
    pub fn create(icon: &std::path::Path) -> Self {
        let window = unsafe { webui_new_window() };
        unsafe {
            webui_set_size(window, 1180, 780);
            webui_set_minimum_size(window, 720, 520);
            webui_set_resizable(window, true);
            webui_set_frameless(window, true);
        }
        if let Some(icon) = icon.to_str().and_then(|path| CString::new(path).ok()) {
            unsafe { webui_set_icon_file(window, icon.as_ptr()) };
        }
        Self(window)
    }

    pub fn show(self, url: &str) -> bool {
        let Ok(url) = CString::new(url) else {
            return false;
        };
        unsafe {
            // Browser launch should not wait for the first renderer connection:
            // a browser can take several seconds to construct its private profile.
            webui_set_config(0, false);
            webui_show_browser(self.0, url.as_ptr(), ANY_BROWSER)
        }
    }

    pub fn show_webview_fallback(self, url: &str) -> bool {
        let Ok(url) = CString::new(url) else {
            return false;
        };
        unsafe {
            webui_set_config(0, false);
            webui_show_wv(self.0, url.as_ptr())
        }
    }

    pub fn minimize(self) {
        unsafe { webui_minimize(self.0) };
    }

    pub fn maximize(self) {
        unsafe { webui_maximize(self.0) };
    }

    pub fn close(self) {
        unsafe { webui_close(self.0) };
    }
}

pub fn clean() {
    unsafe { webui_clean() };
}
