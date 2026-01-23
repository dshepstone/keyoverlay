use std::sync::mpsc::Sender;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

use anyhow::{anyhow, Context, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, KBDLLHOOKSTRUCT, VK_BACK, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_LWIN,
    VK_MENU, VK_RIGHT, VK_RWIN, VK_RETURN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP, WH_KEYBOARD_LL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, MSG, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

static SENDER: OnceLock<Mutex<Option<Sender<KeyEvent>>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub win: bool,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub timestamp: SystemTime,
    pub key_name: String,
    pub modifiers: Modifiers,
}

#[derive(Debug)]
pub struct KeyboardHook {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

struct HookState {
    thread_id: u32,
}

impl KeyboardHook {
    pub fn start(tx: Sender<KeyEvent>) -> Result<Self> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<HookState>>();

        let thread = thread::spawn(move || {
            setup_hook_thread(tx, ready_tx);
        });

        match ready_rx.recv() {
            Ok(Ok(state)) => Ok(Self {
                thread_id: state.thread_id,
                thread: Some(thread),
            }),
            Ok(Err(err)) => {
                let _ = thread.join();
                Err(err)
            }
            Err(err) => {
                let _ = thread.join();
                Err(anyhow!("hook thread did not respond: {err}"))
            }
        }
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn setup_hook_thread(tx: Sender<KeyEvent>, ready_tx: Sender<Result<HookState>>) {
    let sender_slot = SENDER.get_or_init(|| Mutex::new(None));
    {
        let mut guard = sender_slot.lock().expect("sender mutex poisoned");
        *guard = Some(tx);
    }

    let thread_id = unsafe { GetCurrentThreadId() };
    let result = unsafe { GetModuleHandleW(PCWSTR::null()) }
        .context("failed to get module handle")
        .and_then(|module| {
            let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0) };
            if hook.0 == 0 {
                return Err(anyhow!("SetWindowsHookExW failed"));
            }

            let _ = ready_tx.send(Ok(HookState { thread_id }));
            run_message_loop(hook);
            Ok(())
        });

    if let Err(err) = result {
        let _ = ready_tx.send(Err(err));
    }

    {
        let mut guard = sender_slot.lock().expect("sender mutex poisoned");
        *guard = None;
    }
}

fn run_message_loop(hook: HHOOK) {
    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, HWND(0), 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let _ = UnhookWindowsHookEx(hook);
    }
}

extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && (wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN) {
        let hook_data = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let key_name = key_name_from_vk(hook_data.vkCode);
        let modifiers = Modifiers {
            shift: is_vk_down(VK_SHIFT.0 as i32),
            ctrl: is_vk_down(VK_CONTROL.0 as i32),
            alt: is_vk_down(VK_MENU.0 as i32),
            win: is_vk_down(VK_LWIN.0 as i32) || is_vk_down(VK_RWIN.0 as i32),
        };

        let event = KeyEvent {
            timestamp: SystemTime::now(),
            key_name,
            modifiers,
        };

        if let Some(sender) = SENDER.get() {
            if let Ok(guard) = sender.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(event);
                }
            }
        }
    }

    unsafe { CallNextHookEx(HHOOK(0), code, wparam, lparam) }
}

fn is_vk_down(vk: i32) -> bool {
    unsafe { (GetKeyState(vk) as u16 & 0x8000) != 0 }
}

fn key_name_from_vk(vk: u32) -> String {
    match vk {
        0x30..=0x39 => ((b'0' + (vk as u8 - 0x30)) as char).to_string(),
        0x41..=0x5A => ((b'A' + (vk as u8 - 0x41)) as char).to_string(),
        vk if vk == VK_SPACE.0 => "Space".to_string(),
        vk if vk == VK_RETURN.0 => "Enter".to_string(),
        vk if vk == VK_BACK.0 => "Backspace".to_string(),
        vk if vk == VK_TAB.0 => "Tab".to_string(),
        vk if vk == VK_ESCAPE.0 => "Escape".to_string(),
        vk if vk == VK_UP.0 => "ArrowUp".to_string(),
        vk if vk == VK_DOWN.0 => "ArrowDown".to_string(),
        vk if vk == VK_LEFT.0 => "ArrowLeft".to_string(),
        vk if vk == VK_RIGHT.0 => "ArrowRight".to_string(),
        vk if vk == VK_SHIFT.0 => "Shift".to_string(),
        vk if vk == VK_CONTROL.0 => "Ctrl".to_string(),
        vk if vk == VK_MENU.0 => "Alt".to_string(),
        vk if vk == VK_LWIN.0 || vk == VK_RWIN.0 => "Win".to_string(),
        _ => format!("VK_{vk:02X}"),
    }
}
