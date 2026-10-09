//! Support for classic Win32 edit boxes (Notepad on older Windows, many dialogs and older apps).
//!
//! Windows UI Automation exposes these without the text pattern the rest of the checker relies
//! on, so their text, character positions and edits go through the edit control's own messages.

use std::ffi::c_void;

use harper_core::Span;
use uiautomation::{UIAutomation, UIElement};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetDC, GetTextMetricsW, HFONT, ReleaseDC, SelectObject, TEXTMETRICW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageW, WM_GETFONT, WM_GETTEXT, WM_GETTEXTLENGTH,
};

// Edit control messages, from WinUser.h.
const EM_SETSEL: u32 = 0x00B1;
const EM_LINEINDEX: u32 = 0x00BB;
const EM_REPLACESEL: u32 = 0x00C2;
const EM_POSFROMCHAR: u32 = 0x00D6;

/// The focused element, if it is a classic edit box inside `window`.
pub fn focused_edit(automation: &UIAutomation, window: isize) -> Option<HWND> {
    let focused: UIElement = automation.get_focused_element().ok()?;
    let class = focused.get_classname().ok()?;
    if !class.eq_ignore_ascii_case("Edit") || focused.is_password().unwrap_or(true) {
        return None;
    }
    let handle: isize = focused.get_native_window_handle().ok()?.into();
    if handle == 0 || !super::automation_service::same_process(handle, window) {
        return None;
    }
    Some(HWND(handle as *mut c_void))
}

fn send(edit: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    unsafe { SendMessageW(edit, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))).0 }
}

pub fn get_text(edit: HWND) -> Option<String> {
    let len = usize::try_from(send(edit, WM_GETTEXTLENGTH, 0, 0)).ok()?;
    let mut buffer = vec![0u16; len + 1];
    let copied = send(edit, WM_GETTEXT, buffer.len(), buffer.as_mut_ptr() as isize);
    let copied = usize::try_from(copied).ok()?.min(len);
    Some(String::from_utf16_lossy(&buffer[..copied]))
}

/// Edit controls count positions in UTF-16 code units; lints count characters.
fn utf16_index(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(char::len_utf16).sum()
}

/// The top-left corner of a character in screen pixels.
fn char_position(edit: HWND, utf16_index: usize) -> Option<(i32, i32)> {
    let result = send(edit, EM_POSFROMCHAR, utf16_index, 0);
    if result == -1 {
        return None;
    }
    let mut point = POINT {
        x: i32::from((result & 0xFFFF) as u16 as i16),
        y: i32::from(((result >> 16) & 0xFFFF) as u16 as i16),
    };
    unsafe {
        let _ = ClientToScreen(edit, &mut point);
    }
    Some((point.x, point.y))
}

/// The height of one line and the average character width, in pixels.
fn font_metrics(edit: HWND) -> (i32, i32) {
    let mut metrics = TEXTMETRICW::default();
    let ok = unsafe {
        let font = HFONT(send(edit, WM_GETFONT, 0, 0) as *mut c_void);
        let dc = GetDC(Some(edit));
        let previous = (!font.is_invalid()).then(|| SelectObject(dc, font.into()));
        let ok = GetTextMetricsW(dc, &mut metrics).as_bool();
        if let Some(previous) = previous {
            SelectObject(dc, previous);
        }
        ReleaseDC(Some(edit), dc);
        ok
    };
    let mut line_height = if ok { metrics.tmHeight } else { 16 };
    let char_width = if ok { metrics.tmAveCharWidth.max(1) } else { 8 };

    // With more than one line, the distance between them is exact, including any spacing.
    let second_line = send(edit, EM_LINEINDEX, 1, 0);
    if second_line > 0
        && let (Some((_, y0)), Some((_, y1))) = (
            char_position(edit, 0),
            char_position(edit, second_line as usize),
        )
        && y1 > y0
    {
        line_height = y1 - y0;
    }

    (line_height, char_width)
}

/// Screen rectangles (x, y, width, height) covering `span`, one per line it touches.
pub fn span_rects(edit: HWND, text: &str, span: Span<char>) -> Vec<(f64, f64, f64, f64)> {
    let (line_height, char_width) = font_metrics(edit);
    let chars: Vec<char> = text.chars().collect();
    let mut rects: Vec<(f64, f64, f64, f64)> = Vec::new();
    let mut line: Option<(i32, i32, i32)> = None; // (left, top, right)

    for index in span.start..span.end.min(chars.len()) {
        if chars[index] == '\r' || chars[index] == '\n' {
            continue;
        }
        let Some((x, y)) = char_position(edit, utf16_index(text, index)) else {
            continue;
        };
        let right = match char_position(edit, utf16_index(text, index + 1)) {
            Some((next_x, next_y)) if next_y == y && next_x > x => next_x,
            _ => x + char_width,
        };
        match &mut line {
            Some((_, top, line_right)) if *top == y => *line_right = right,
            _ => {
                if let Some((left, top, right)) = line.take() {
                    rects.push(rect(left, top, right, line_height));
                }
                line = Some((x, y, right));
            }
        }
    }
    if let Some((left, top, right)) = line {
        rects.push(rect(left, top, right, line_height));
    }
    rects
}

fn rect(left: i32, top: i32, right: i32, height: i32) -> (f64, f64, f64, f64) {
    (
        f64::from(left),
        f64::from(top),
        f64::from(right - left),
        f64::from(height),
    )
}

/// Replaces `span` with `replacement` the way typing would, so Ctrl+Z undoes it.
pub fn replace(edit: HWND, text: &str, span: Span<char>, replacement: &str) {
    let start = utf16_index(text, span.start);
    let end = utf16_index(text, span.end);
    let mut wide: Vec<u16> = replacement.encode_utf16().collect();
    wide.push(0);
    send(edit, EM_SETSEL, start, end as isize);
    // `wparam = 1` keeps the change on the edit box's undo stack.
    send(edit, EM_REPLACESEL, 1, wide.as_ptr() as isize);
}

#[cfg(test)]
mod tests {
    use super::utf16_index;

    #[test]
    fn counts_utf16_units() {
        assert_eq!(utf16_index("a😀b", 2), 3);
        assert_eq!(utf16_index("abc", 3), 3);
    }
}
