use crate::client::Client;
use crate::util::BuildIdHasher;

use winsys::connection::Pid;
use winsys::window::Window;

use std::collections::HashMap;
use std::fs;

fn get_parent_pid(pid: Pid) -> Option<Pid> {
    if let Ok(stat) = fs::read_to_string(format!("/proc/{}/stat", pid)) {
        let stat = stat.split(" ").collect::<Vec<&str>>();
        return stat.get(3).and_then(|ppid| ppid.parse::<Pid>().ok());
    }

    None
}

pub fn get_spawner_pid(
    pid: Pid,
    wm_pid: Pid,
    pid_map: &HashMap<Pid, Window>,
    client_map: &HashMap<Window, Client, BuildIdHasher>,
) -> Option<Pid> {
    let mut ppid = get_parent_pid(pid);

    while ppid.is_some() {
        let ppid_new = get_parent_pid(ppid.unwrap());

        let mut is_consumer = false;
        if let Some(ppid_new) = ppid_new {
            if let Some(window) = pid_map.get(&ppid_new) {
                if let Some(client) = client_map.get(&window) {
                    is_consumer = client.is_consuming();
                }
            }
        };

        if is_consumer {
            return ppid_new;
        }

        if ppid_new == Some(wm_pid) {
            return if ppid == Some(pid) { None } else { ppid };
        }

        ppid = ppid_new;
    }

    None
}
