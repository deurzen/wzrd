use super::super::input::Result;

use std::collections::HashMap;
use std::process::Command;

pub type KeyMap = HashMap<String, u8>;
pub type CodeMap = HashMap<u8, String>;

pub struct Util;
impl Util {
    pub fn system_keymap() -> KeyMap {
        match Command::new("xmodmap").arg("-pke").output() {
            Err(err) => panic!("unable to fetch keycodes via xmodmap: {}", err),
            Ok(out) => match String::from_utf8(out.stdout) {
                Err(err) => panic!("invalid UTF8 from xmodmap: {}", err),
                Ok(out) => out
                    .lines()
                    .flat_map(|l| {
                        let mut words = l.split_whitespace();
                        let key_code: u8 = words.nth(1).unwrap().parse().unwrap();

                        words.skip(1).map(move |name| (name.into(), key_code))
                    })
                    .collect::<KeyMap>(),
            },
        }
    }

    pub fn system_codemap() -> CodeMap {
        match Command::new("xmodmap").arg("-pke").output() {
            Err(err) => panic!("unable to fetch keycodes via xmodmap: {}", err),
            Ok(out) => match String::from_utf8(out.stdout) {
                Err(err) => panic!("invalid UTF8 from xmodmap: {}", err),
                Ok(out) => out
                    .lines()
                    .flat_map(|l| {
                        let mut words = l.split_whitespace();
                        let key_code: u8 = words.nth(1).unwrap().parse().unwrap();

                        words.skip(1).map(move |name| (key_code, name.into()))
                    })
                    .collect::<CodeMap>(),
            },
        }
    }
}
