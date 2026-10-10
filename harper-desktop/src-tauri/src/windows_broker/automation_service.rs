use std::iter::once;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};
use std::thread::sleep;
use std::time::Duration;

use super::win32_edit;
use crate::rect::Rect;
use crate::windows_broker::get_focused_monitor_scale;
use harper_core::{Span, linting::Suggestion};
use is_macro::Is;
use uiautomation::types::{
    ControlType, Handle, TextPatternRangeEndpoint, TextUnit, TreeScope, UIProperty,
};
use uiautomation::variants::Variant;
use uiautomation::{
    UIAutomation, UIElement,
    patterns::{UITextPattern, UITextRange, UIValuePattern},
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::IUIAutomationTextRange;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY, VK_DELETE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

/// Information about a worker thread.
struct WorkerData {
    sender: SyncSender<(WorkerJob, Vec<JobArgument>)>,
    receiver: Receiver<JobResult>,
}

#[derive(Debug)]
struct ApplySuggestionRequest {
    window: isize,
    expected_text: String,
    span: Span<char>,
    suggestion: Suggestion,
}

#[derive(Debug, Is)]
enum JobArgument {
    Span(Span<char>),
    Window(isize),
    Text(String),
    ApplySuggestion(ApplySuggestionRequest),
}

/// The result of a job run by the worker thread.
#[derive(Debug, Is)]
enum JobResult {
    None,
    String(String),
    GroupedRects(Vec<Vec<Rect>>),
    Err,
}

/// An actual function pointer to be run by the worker thread.
type WorkerJob = fn(&UIAutomation, Vec<JobArgument>) -> JobResult;

/// Runs and communicates with a worker thread to interact with the Win32 Automation API to query the accessibility tree.
/// Necessary because the API has very specific thread setting requirements to work.
pub struct AutomationService {
    worker_data: Option<WorkerData>,
    // Needed to redirect focus to the last focused window when the focus arrives on the Harper highlighter window
    last_focused_window: Option<isize>,
}

impl AutomationService {
    pub fn create_and_start() -> Self {
        let mut output = Self {
            last_focused_window: None,
            worker_data: None,
        };

        output.start_worker_thread();

        output
    }

    /// Starts the worker thread if it is not already running.
    /// Does nothing if the worker thread is already running.
    fn start_worker_thread(&mut self) {
        let (job_sender, job_receiver) = sync_channel::<(WorkerJob, Vec<JobArgument>)>(1);
        let (result_sender, result_receiver) = sync_channel(1);

        std::thread::spawn(move || {
            let automation = UIAutomation::new().unwrap();

            loop {
                // Stop the thread if the other side of the channel has been closed (or dropped).
                let job = match job_receiver.try_recv() {
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => None,
                    Ok(job) => Some(job),
                };

                if let Some((job, arguments)) = job {
                    let result = job(&automation, arguments);

                    // Stop the thread if the other side of the channel has been closed (or dropped).
                    if let Err(err) = result_sender.try_send(result) {
                        if let TrySendError::Disconnected(_) = err {
                            break;
                        }
                    }
                }

                sleep(Duration::from_millis(16));
            }
        });

        self.worker_data = Some(WorkerData {
            receiver: result_receiver,
            sender: job_sender,
        });
    }

    /// Stops the worker thread if it is running. This method does nothing if it is not running.
    fn stop_worker_thread(&mut self) {
        // This drops the inner fields, which closes the channel, which signals to the worker to stop running.
        self.worker_data = None;
    }

    /// Attempts to run a worker job on the worker thread. Returns `None` if the worker thread does not exist.
    fn run_worker_job(&self, job: WorkerJob, arguments: Vec<JobArgument>) -> Option<JobResult> {
        let worker_data = self.worker_data.as_ref()?;
        worker_data.sender.send((job, arguments)).unwrap();
        Some(worker_data.receiver.recv().unwrap())
    }

    /// Grab text from the worker.
    /// Attempts to get the most up-to-date information possible.
    /// Returns `None` if the worker is not running.
    pub fn get_text(&mut self) -> Option<String> {
        let window = self.resolve_focused_window()?;
        let result = self.run_worker_job(get_text_job, vec![JobArgument::Window(window)])?;

        match result {
            JobResult::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn apply_suggestion(
        &mut self,
        expected_text: String,
        span: Span<char>,
        suggestion: Suggestion,
    ) {
        let Some(window) = self.resolve_focused_window() else {
            return;
        };

        let request = ApplySuggestionRequest {
            window,
            expected_text,
            span,
            suggestion,
        };

        let _ = self.run_worker_job(
            apply_suggestion_job,
            vec![JobArgument::ApplySuggestion(request)],
        );
    }

    /// Pass a collection of text spans to the worker and have it compute the associated bounding boxes for each span.
    /// Each span may have multiple bounding boxes.
    /// Input spans share the same index as their output bounding box.
    pub fn get_bounding_boxes(
        &mut self,
        text: &str,
        spans: impl IntoIterator<Item = Span<char>>,
    ) -> Option<Vec<Vec<Rect>>> {
        let window = self.resolve_focused_window()?;

        let result = self.run_worker_job(
            get_bounding_rect_job,
            once(JobArgument::Window(window))
                .chain(once(JobArgument::Text(text.to_string())))
                .chain(spans.into_iter().map(JobArgument::Span))
                .collect(),
        )?;

        match result {
            JobResult::GroupedRects(rects) => Some(rects),
            _ => None,
        }
    }

    /// Returns the foreground source window, retaining the last external window while Harper's
    /// overlay owns focus.
    pub fn resolve_focused_window(&mut self) -> Option<isize> {
        let (focused_window, focused_process_id) = focused_window()?;

        if focused_process_id == std::process::id() {
            return self.last_focused_window;
        }

        self.last_focused_window = Some(focused_window);
        Some(focused_window)
    }
}

impl Drop for AutomationService {
    fn drop(&mut self) {
        self.stop_worker_thread();
    }
}

fn apply_suggestion_job(automation: &UIAutomation, mut arguments: Vec<JobArgument>) -> JobResult {
    let Some(JobArgument::ApplySuggestion(request)) = arguments.pop() else {
        return JobResult::Err;
    };

    if !arguments.is_empty() {
        return JobResult::Err;
    }

    let Ok(element) =
        text_element_for_window(automation, request.window, Some(&request.expected_text))
    else {
        if let Some(edit) = win32_edit::focused_edit(automation, request.window)
            && win32_edit::get_text(edit).as_deref() == Some(request.expected_text.as_str())
        {
            let (span, replacement) = match &request.suggestion {
                Suggestion::ReplaceWith(with) => (request.span, with.iter().collect()),
                Suggestion::InsertAfter(with) => (
                    Span::new(request.span.end, request.span.end),
                    with.iter().collect(),
                ),
                Suggestion::Remove => (request.span, String::new()),
            };
            win32_edit::replace(edit, &request.expected_text, span, &replacement);
            return JobResult::None;
        }
        eprintln!(
            "Unable to apply Windows suggestion: the source text element is no longer available"
        );
        return JobResult::None;
    };

    let Ok(current_text) = get_text(&element) else {
        eprintln!("Unable to apply Windows suggestion: the source text can no longer be read");
        return JobResult::None;
    };

    let updated_text = match apply_suggestion_to_text(
        &current_text,
        &request.expected_text,
        request.span,
        &request.suggestion,
    ) {
        Ok(updated_text) => updated_text,
        Err(error) => {
            eprintln!("Unable to apply Windows suggestion: {error}");
            return JobResult::None;
        }
    };

    // Prefer selecting the misspelled range and typing the fix, the way a person would. Unlike
    // replacing the whole value, this works in Chrome and Electron editors (which often lack a
    // writable value pattern), keeps formatting, and leaves the change on the app's undo stack.
    match apply_suggestion_by_typing(
        &element,
        request.window,
        &current_text,
        request.span,
        &request.suggestion,
    ) {
        Ok(()) => return JobResult::None,
        Err(error) => {
            eprintln!(
                "Typing the Windows suggestion failed, falling back to setting the value: {error}"
            );
        }
    }

    let Ok(value_pattern) = element.get_pattern::<UIValuePattern>() else {
        eprintln!(
            "Unable to apply Windows suggestion: the text element has no writable value pattern"
        );
        return JobResult::None;
    };

    match value_pattern.is_readonly() {
        Ok(true) => {
            eprintln!("Unable to apply Windows suggestion: the text element is read-only");
        }
        Ok(false) => {
            if let Err(error) = value_pattern.set_value(&updated_text) {
                eprintln!("Unable to apply Windows suggestion: {error}");
            }
        }
        Err(error) => {
            eprintln!("Unable to determine whether the Windows text element is writable: {error}");
        }
    }

    JobResult::None
}

fn apply_suggestion_to_text(
    current_text: &str,
    expected_text: &str,
    span: Span<char>,
    suggestion: &Suggestion,
) -> std::result::Result<String, &'static str> {
    if current_text != expected_text {
        return Err("the source text changed after linting");
    }

    let mut chars = current_text.chars().collect::<Vec<_>>();
    if span.end > chars.len() {
        return Err("the lint span is outside the source text");
    }

    suggestion.apply(span, &mut chars);
    Ok(chars.into_iter().collect())
}

fn get_text(element: &UIElement) -> uiautomation::Result<String> {
    let pattern: UITextPattern = element.get_pattern()?;
    let range = pattern.get_document_range()?;
    range.get_text(-1)
}

/// Finds the focused text element in `window`.
///
/// Asks Windows for the element with keyboard focus first, which works for classic Win32 edit
/// controls (Notepad) as well as browsers and Electron apps, then falls back to searching the
/// window. When `expected_text` is provided, unrelated text providers are excluded.
fn text_element_for_window(
    automation: &UIAutomation,
    window: isize,
    expected_text: Option<&str>,
) -> uiautomation::Result<UIElement> {
    let text_condition = automation.create_property_condition(
        UIProperty::IsTextPatternAvailable,
        Variant::from(true),
        None,
    )?;

    let mut candidates = Vec::new();
    if let Ok(focused) = automation.get_focused_element()
        && belongs_to_window(&focused, window)
    {
        // Some editors give focus to a container around the text box.
        let inner = focused.find_first(TreeScope::Descendants, &text_condition);
        candidates.push(focused);
        candidates.extend(inner);
        // Trust what Windows says has focus. Searching the window for a text box that claims
        // focus can find one the user already left (Chrome is slow to update that flag), which
        // kept its underlines on screen after clicking elsewhere on the page.
        return candidates
            .into_iter()
            .filter(is_editable_text)
            .find(|element| {
                get_text(element)
                    .is_ok_and(|text| expected_text.is_none_or(|expected| expected == text))
            })
            .ok_or_else(|| {
                Error::new(uiautomation::errors::ERR_NOTFOUND, "no text element found")
            });
    }

    let root = automation.element_from_handle(Handle::from(window))?;
    let keyboard_condition = automation.create_property_condition(
        UIProperty::HasKeyboardFocus,
        Variant::from(true),
        None,
    )?;
    let condition = automation.create_and_condition(text_condition, keyboard_condition)?;
    candidates.extend(
        root.find_all(TreeScope::Subtree, &condition)
            .unwrap_or_default(),
    );

    for element in candidates {
        if !is_editable_text(&element) {
            continue;
        }
        let Ok(text) = get_text(&element) else {
            continue;
        };
        if expected_text.is_some_and(|expected| expected != text) {
            continue;
        }
        return Ok(element);
    }

    Err(Error::new(
        uiautomation::errors::ERR_NOTFOUND,
        "no text element found",
    ))
}

/// Whether `element` is a box the user types into, rather than a whole page or read-only text.
///
/// Browsers and Electron apps (Chrome, Edge, the Claude app, Firefox) expose every web page as a
/// readable document, so without this check a page with nothing focused gets every button and
/// label underlined.
fn is_editable_text(element: &UIElement) -> bool {
    // Never read password boxes.
    if element.is_password().unwrap_or(true) {
        return false;
    }
    // Browser address bars hold web addresses, not writing.
    if element
        .get_classname()
        .is_ok_and(|class| class.starts_with("Omnibox"))
    {
        return false;
    }
    if let Ok(value) = element.get_pattern::<UIValuePattern>()
        && let Ok(read_only) = value.is_readonly()
    {
        return !read_only;
    }
    match element.get_control_type() {
        Ok(ControlType::Edit) => true,
        Ok(ControlType::Document) => !matches!(
            element.get_framework_id().unwrap_or_default().as_str(),
            "Chrome" | "Gecko"
        ),
        _ => false,
    }
}

/// Whether two windows belong to the same process.
pub(super) fn same_process(a: isize, b: isize) -> bool {
    let (mut pa, mut pb) = (0u32, 0u32);
    unsafe {
        GetWindowThreadProcessId(HWND(a as *mut c_void), Some(&mut pa));
        GetWindowThreadProcessId(HWND(b as *mut c_void), Some(&mut pb));
    }
    pa != 0 && pa == pb
}

/// Whether `element` belongs to the same process as `window`.
fn belongs_to_window(element: &UIElement, window: isize) -> bool {
    let mut window_process = 0u32;
    unsafe {
        GetWindowThreadProcessId(HWND(window as *mut c_void), Some(&mut window_process));
    }
    element
        .get_process_id()
        .is_ok_and(|pid| pid == window_process && pid != 0)
}

/// Describes the focused element, to explain in the log why its text can't be read.
fn describe_focused_element(automation: &UIAutomation) -> String {
    let Ok(focused) = automation.get_focused_element() else {
        return "Windows reports no focused element".to_string();
    };
    // Web apps can have class lists hundreds of characters long.
    let class: String = focused
        .get_classname()
        .unwrap_or_default()
        .chars()
        .take(60)
        .collect();
    format!(
        "focused element is a {:?} (class {class:?}, framework {:?}); text pattern: {}; editable: {}",
        focused.get_control_type().ok(),
        focused.get_framework_id().unwrap_or_default(),
        match focused.get_pattern::<UITextPattern>() {
            Ok(_) => "yes".to_string(),
            Err(error) => format!("no ({error})"),
        },
        if is_editable_text(&focused) {
            "yes"
        } else {
            "no"
        }
    )
}

fn get_text_job(automation: &UIAutomation, args: Vec<JobArgument>) -> JobResult {
    let Some(JobArgument::Window(window)) = args.first() else {
        return JobResult::Err;
    };
    let Ok(element) = text_element_for_window(automation, *window, None) else {
        if let Some(edit) = win32_edit::focused_edit(automation, *window)
            && let Some(text) = win32_edit::get_text(edit)
        {
            crate::logging::note_change("text element", "classic Win32 edit box".to_string());
            return JobResult::String(text);
        }
        crate::logging::note_change("text element", describe_focused_element(automation));
        return JobResult::Err;
    };

    get_text(&element).map_or(JobResult::Err, JobResult::String)
}

use std::{ffi::c_void, mem::size_of};

use uiautomation::{Error, Result};
use windows::Win32::System::{
    Com::SAFEARRAY,
    Ole::{
        SafeArrayDestroy, SafeArrayGetDim, SafeArrayGetElement, SafeArrayGetElemsize,
        SafeArrayGetLBound, SafeArrayGetUBound,
    },
};

struct OwnedSafeArray(*mut SAFEARRAY);

impl Drop for OwnedSafeArray {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = SafeArrayDestroy(self.0);
            }
        }
    }
}

/// Returns the text range covering `len` characters starting at `start`.
fn range_for_span(pattern: &UITextPattern, start: i32, len: i32) -> Result<UITextRange> {
    let range = pattern.get_document_range()?;

    range.move_endpoint_by_range(
        TextPatternRangeEndpoint::End,
        &range,
        TextPatternRangeEndpoint::Start,
    )?;

    range.move_endpoint_by_unit(TextPatternRangeEndpoint::Start, TextUnit::Character, start)?;

    range.move_endpoint_by_range(
        TextPatternRangeEndpoint::End,
        &range,
        TextPatternRangeEndpoint::Start,
    )?;

    range.move_endpoint_by_unit(TextPatternRangeEndpoint::End, TextUnit::Character, len)?;

    Ok(range)
}

/// Selects the lint's text in the source control and types the suggestion over it.
fn apply_suggestion_by_typing(
    element: &UIElement,
    window: isize,
    current_text: &str,
    span: Span<char>,
    suggestion: &Suggestion,
) -> std::result::Result<(), String> {
    let chars: Vec<char> = current_text.chars().collect();
    if span.end > chars.len() {
        return Err("the lint span is outside the source text".into());
    }

    let (start, len, replacement) = match suggestion {
        Suggestion::ReplaceWith(with) => (span.start, span.len(), with.iter().collect::<String>()),
        Suggestion::InsertAfter(with) => (span.end, 0, with.iter().collect::<String>()),
        Suggestion::Remove => (span.start, span.len(), String::new()),
    };

    let pattern: UITextPattern = element.get_pattern().map_err(|e| e.to_string())?;
    let range = range_for_span(&pattern, start as i32, len as i32).map_err(|e| e.to_string())?;

    // Apps count characters differently (some use UTF-16 units, some collapse line breaks), so
    // make sure the range we are about to overwrite really holds the text we linted.
    let expected: String = chars[start..start + len].iter().collect();
    let found = range.get_text(-1).map_err(|e| e.to_string())?;
    if normalize_line_breaks(&found) != normalize_line_breaks(&expected) {
        return Err(format!(
            "the selected range holds {found:?} instead of {expected:?}"
        ));
    }

    unsafe {
        let _ = SetForegroundWindow(HWND(window as *mut std::ffi::c_void));
    }
    let _ = element.set_focus();
    range.select().map_err(|e| e.to_string())?;

    if replacement.is_empty() {
        send_virtual_key(VK_DELETE)
    } else {
        send_unicode_text(&replacement)
    }
}

fn normalize_line_breaks(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Types `text` into the focused control as Unicode keystrokes, independent of keyboard layout.
fn send_unicode_text(text: &str) -> std::result::Result<(), String> {
    let mut inputs = Vec::with_capacity(text.len() * 4);
    for unit in text.encode_utf16() {
        for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
    }
    send_inputs(&inputs)
}

fn send_virtual_key(key: VIRTUAL_KEY) -> std::result::Result<(), String> {
    let inputs = [KEYBD_EVENT_FLAGS(0), KEYEVENTF_KEYUP].map(|flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
    send_inputs(&inputs)
}

fn send_inputs(inputs: &[INPUT]) -> std::result::Result<(), String> {
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(format!(
            "only {sent} of {} keystrokes were delivered (the app may be running as administrator)",
            inputs.len()
        ));
    }
    Ok(())
}

fn bounding_rectangles_for_span(
    element: &UIElement,
    start: i32,
    len: i32,
) -> Result<Vec<(f64, f64, f64, f64)>> {
    if start < 0 || len < 0 {
        return Err(Error::new(
            uiautomation::errors::ERR_INVALID_ARG,
            "start and len must be non-negative",
        ));
    }

    let pattern: UITextPattern = element.get_pattern()?;
    let range = range_for_span(&pattern, start, len)?;

    let raw: &IUIAutomationTextRange = range.as_ref();
    let array = OwnedSafeArray(unsafe { raw.GetBoundingRectangles()? });

    if array.0.is_null() {
        return Ok(Vec::new());
    }

    let dim = unsafe { SafeArrayGetDim(array.0) };

    if dim != 1 {
        return Err(Error::new(
            uiautomation::errors::ERR_FORMAT,
            "bounding rectangles SAFEARRAY is not one-dimensional",
        ));
    }

    let elem_size = unsafe { SafeArrayGetElemsize(array.0) };

    if elem_size as usize != size_of::<f64>() {
        return Err(Error::new(
            uiautomation::errors::ERR_FORMAT,
            "bounding rectangles SAFEARRAY does not contain f64-sized elements",
        ));
    }

    let lower = unsafe { SafeArrayGetLBound(array.0, 1)? };
    let upper = unsafe { SafeArrayGetUBound(array.0, 1)? };

    if upper < lower {
        return Ok(Vec::new());
    }

    let count = usize::try_from(i64::from(upper) - i64::from(lower) + 1)
        .map_err(|_| Error::new(uiautomation::errors::ERR_FORMAT, "SAFEARRAY is too large"))?;

    if count % 4 != 0 {
        return Err(Error::new(
            uiautomation::errors::ERR_FORMAT,
            "bounding rectangles SAFEARRAY length is not divisible by four",
        ));
    }

    let mut result = Vec::with_capacity(count / 4);

    for rect in 0..count / 4 {
        let mut values = [0.0_f64; 4];

        for (component, value) in values.iter_mut().enumerate() {
            let offset = rect
                .checked_mul(4)
                .and_then(|n| n.checked_add(component))
                .ok_or_else(|| {
                    Error::new(uiautomation::errors::ERR_FORMAT, "SAFEARRAY index overflow")
                })?;

            let index = i64::from(lower)
                .checked_add(i64::try_from(offset).map_err(|_| {
                    Error::new(uiautomation::errors::ERR_FORMAT, "SAFEARRAY index overflow")
                })?)
                .and_then(|n| i32::try_from(n).ok())
                .ok_or_else(|| {
                    Error::new(uiautomation::errors::ERR_FORMAT, "SAFEARRAY index overflow")
                })?;

            unsafe {
                SafeArrayGetElement(array.0, &index, value as *mut f64 as *mut c_void)?;
            }
        }

        result.push((values[0], values[1], values[2], values[3]));
    }

    Ok(result)
}

fn get_bounding_rect_job(automation: &UIAutomation, arguments: Vec<JobArgument>) -> JobResult {
    let Some(JobArgument::Window(window)) = arguments.first() else {
        return JobResult::Err;
    };
    let Some(JobArgument::Text(expected_text)) = arguments.get(1) else {
        return JobResult::Err;
    };
    let effective_monitor_scale = get_focused_monitor_scale();
    let scale = |(x, y, w, h): &(f64, f64, f64, f64)| {
        Rect::new(
            *x / effective_monitor_scale,
            *y / effective_monitor_scale,
            *w / effective_monitor_scale,
            *h / effective_monitor_scale,
        )
    };

    let Ok(text_element) = text_element_for_window(automation, *window, Some(expected_text)) else {
        let Some(edit) = win32_edit::focused_edit(automation, *window) else {
            return JobResult::Err;
        };
        if win32_edit::get_text(edit).as_deref() != Some(expected_text.as_str()) {
            return JobResult::Err;
        }
        return JobResult::GroupedRects(
            arguments
                .iter()
                .skip(2)
                .filter_map(|argument| match argument {
                    JobArgument::Span(span) => Some(
                        win32_edit::span_rects(edit, expected_text, *span)
                            .iter()
                            .map(scale)
                            .collect(),
                    ),
                    _ => None,
                })
                .collect(),
        );
    };

    let mut rects = Vec::with_capacity(arguments.len().saturating_sub(2));

    for span in arguments.into_iter().skip(2) {
        let span = span.expect_span();

        let Ok(found_rects) =
            bounding_rectangles_for_span(&text_element, span.start as i32, span.len() as i32)
        else {
            return JobResult::Err;
        };

        rects.push(found_rects.iter().map(scale).collect());
    }

    JobResult::GroupedRects(rects)
}

fn focused_window() -> Option<(isize, u32)> {
    let hwnd: HWND = unsafe { GetForegroundWindow() };

    if hwnd.0.is_null() {
        return None;
    }

    let mut process_id = 0;

    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }

    Some((hwnd.0 as isize, process_id))
}
