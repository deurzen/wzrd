use crate::change::Change;
use crate::change::Direction;
use crate::identify::Index;

use winsys::input::Button;
use winsys::input::CodeMap;
use winsys::input::KeyCode;
use winsys::input::Modifier;
use winsys::input::MouseShortcut;

use std::cmp::Ord;
use std::hash::BuildHasher;
use std::hash::Hasher;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Sub;
use std::ops::SubAssign;
use std::process::Command;
use std::process::Stdio;

use x11rb::protocol::xproto::ModMask;

#[derive(Default)]
pub struct IdHasher {
    state: u64,
}

impl Hasher for IdHasher {
    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    ) {
        for &byte in bytes {
            self.state = self.state.rotate_left(8) + u64::from(byte);
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }
}

#[derive(Default, Clone)]
pub struct BuildIdHasher;

impl BuildHasher for BuildIdHasher {
    type Hasher = IdHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        Self::Hasher {
            state: 0,
        }
    }
}

pub struct Util;

impl Util {
    #[inline]
    pub fn last_index(iter: impl ExactSizeIterator) -> Index {
        if iter.len() != 0 {
            iter.len() - 1
        } else {
            0
        }
    }

    #[inline]
    pub fn next_index(
        iter: impl ExactSizeIterator,
        index: Index,
        dir: Direction,
    ) -> Index {
        match dir {
            Direction::Forward => (index + 1) % iter.len(),
            Direction::Backward => {
                if index == 0 {
                    iter.len() - 1
                } else {
                    index - 1
                }
            },
        }
    }

    #[inline]
    pub fn change_within_range<T>(
        min: T,
        max: T,
        mut base: T,
        change: Change<T>,
    ) -> T
    where
        T: Ord
            + Add<Output = T>
            + AddAssign
            + Mul<Output = T>
            + MulAssign
            + Sub<Output = T>
            + SubAssign
            + Copy,
    {
        match change {
            Change::Inc(delta) => {
                base += delta;
                if base > max {
                    max
                } else {
                    base
                }
            },
            Change::Dec(delta) => {
                if base >= min + delta {
                    base - delta
                } else {
                    min
                }
            },
        }
    }

    pub fn spawn<S: Into<String>>(cmd: S) {
        let cmd = cmd.into();
        let args: Vec<&str> = cmd.split_whitespace().collect();

        if args.len() > 1 {
            Command::new(args[0])
                .args(&args[1..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok();
        } else {
            Command::new(args[0])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok();
        };
    }

    pub fn spawn_shell<S: Into<String>>(cmd: S) {
        let cmd = cmd.into();

        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    }

    pub fn system_keycodes() -> CodeMap {
        match Command::new("xmodmap").arg("-pke").output() {
            Err(e) => panic!("unable to fetch keycodes via xmodmap: {}", e),
            Ok(o) => match String::from_utf8(o.stdout) {
                Err(e) => panic!("invalid utf8 from xmodmap: {}", e),
                Ok(s) => s
                    .lines()
                    .flat_map(|l| {
                        let mut words = l.split_whitespace();
                        let key_code: u8 = words.nth(1).unwrap().parse().unwrap();

                        words.skip(1).map(move |name| (name.into(), key_code))
                    })
                    .collect::<CodeMap>(),
            },
        }
    }

    pub fn parse_key_binding(
        key_binding: impl Into<String>,
        keycodes: &CodeMap,
    ) -> Option<KeyCode> {
        let s = key_binding.into();
        let mut constituents: Vec<&str> = s.split('-').collect();

        match keycodes.get(constituents.remove(constituents.len() - 1)) {
            Some(&code) => {
                let mask = constituents
                    .iter()
                    .map(|&modifier| match modifier {
                        "A" | "Alt" | "Meta" => u16::from(ModMask::M1),
                        "M" | "Super" => u16::from(ModMask::M4),
                        "S" | "Shift" => u16::from(ModMask::SHIFT),
                        "C" | "Ctrl" | "Control" => u16::from(ModMask::CONTROL),
                        "1" | "Mod" => u16::from(if cfg!(debug_assertions) {
                            ModMask::M1
                        } else {
                            ModMask::M4
                        }),
                        "2" | "Sec" => u16::from(if cfg!(debug_assertions) {
                            ModMask::M4
                        } else {
                            ModMask::M1
                        }),
                        _ => panic!("invalid modifier: {}", s),
                    })
                    .fold(0, |acc, modifier| acc | modifier);

                Some(KeyCode {
                    mask,
                    code,
                })
            },
            None => None,
        }
    }

    pub fn parse_mouse_binding(mouse_binding: impl Into<String>) -> Option<MouseShortcut> {
        let s = mouse_binding.into();
        let mut constituents: Vec<&str> = s.split('-').collect();

        let button = match constituents.remove(constituents.len() - 1) {
            "1" | "Left" => Button::Left,
            "2" | "Middle" => Button::Middle,
            "3" | "Right" => Button::Right,
            "4" | "ScrollUp" => Button::ScrollUp,
            "5" | "ScrollDown" => Button::ScrollDown,
            "8" | "Backward" => Button::Backward,
            "9" | "Forward" => Button::Forward,
            s => panic!("invalid button: {}", s),
        };

        let mut modifiers = constituents
            .iter()
            .map(|&modifier| match modifier {
                "A" | "Alt" | "Meta" => Modifier::Alt,
                "AGr" | "AltGr" => Modifier::AltGr,
                "M" | "Super" => Modifier::Super,
                "S" | "Shift" => Modifier::Shift,
                "C" | "Ctrl" | "Control" => Modifier::Ctrl,
                "N" | "NumLock" => Modifier::NumLock,
                "L" | "ScrollLock" => Modifier::ScrollLock,
                "1" | "Mod" => {
                    if cfg!(debug_assertions) {
                        Modifier::Alt
                    } else {
                        Modifier::Super
                    }
                },
                "2" | "Sec" => {
                    if cfg!(debug_assertions) {
                        Modifier::Super
                    } else {
                        Modifier::Alt
                    }
                },
                _ => panic!("invalid modifier: {}", s),
            })
            .collect::<Vec<Modifier>>();

        modifiers.sort();

        Some(MouseShortcut {
            button,
            modifiers,
        })
    }
}
