use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B, VK_BACK, VK_C,
    VK_CONTROL, VK_D, VK_DOWN, VK_E, VK_ESCAPE, VK_F, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L,
    VK_LEFT, VK_LWIN, VK_M, VK_MENU, VK_N, VK_O, VK_P, VK_Q, VK_R, VK_RETURN, VK_RIGHT,
    VK_RWIN, VK_S, VK_SHIFT, VK_SPACE, VK_T, VK_TAB, VK_U, VK_UP, VK_V, VK_W, VK_X, VK_Y,
    VK_Z,
};

use crate::Key;

pub fn vk_to_key(vk_code: u32) -> Option<Key> {
    let vk_code = vk_code as u16;

    let key = match vk_code {
        VK_A => Key::A,
        VK_B => Key::B,
        VK_C => Key::C,
        VK_D => Key::D,
        VK_E => Key::E,
        VK_F => Key::F,
        VK_G => Key::G,
        VK_H => Key::H,
        VK_I => Key::I,
        VK_J => Key::J,
        VK_K => Key::K,
        VK_L => Key::L,
        VK_M => Key::M,
        VK_N => Key::N,
        VK_O => Key::O,
        VK_P => Key::P,
        VK_Q => Key::Q,
        VK_R => Key::R,
        VK_S => Key::S,
        VK_T => Key::T,
        VK_U => Key::U,
        VK_V => Key::V,
        VK_W => Key::W,
        VK_X => Key::X,
        VK_Y => Key::Y,
        VK_Z => Key::Z,
        VK_0 => Key::Digit0,
        VK_1 => Key::Digit1,
        VK_2 => Key::Digit2,
        VK_3 => Key::Digit3,
        VK_4 => Key::Digit4,
        VK_5 => Key::Digit5,
        VK_6 => Key::Digit6,
        VK_7 => Key::Digit7,
        VK_8 => Key::Digit8,
        VK_9 => Key::Digit9,
        VK_LEFT => Key::ArrowLeft,
        VK_RIGHT => Key::ArrowRight,
        VK_UP => Key::ArrowUp,
        VK_DOWN => Key::ArrowDown,
        VK_RETURN => Key::Enter,
        VK_ESCAPE => Key::Escape,
        VK_TAB => Key::Tab,
        VK_SPACE => Key::Space,
        VK_BACK => Key::Backspace,
        VK_SHIFT => Key::Shift,
        VK_CONTROL => Key::Ctrl,
        VK_MENU => Key::Alt,
        VK_LWIN | VK_RWIN => Key::Win,
        _ => return None,
    };

    Some(key)
}
