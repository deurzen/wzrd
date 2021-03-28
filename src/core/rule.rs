use crate::change::Toggle;
use crate::client::Client;

#[derive(Debug)]
pub struct Rules {
    pub float: Option<bool>,
    pub center: Option<bool>,
    pub fullscreen: Option<bool>,
    pub workspace: Option<usize>,
    pub context: Option<usize>,
}

impl Rules {
    pub fn propagate(
        &self,
        client: &Client,
    ) {
        if let Some(float) = self.float {
            client.set_floating(Toggle::from(float));
        }

        if let Some(fullscreen) = self.fullscreen {
            client.set_fullscreen(Toggle::from(fullscreen));
        }

        if let Some(workspace) = self.workspace {
            client.set_workspace(workspace);
        }

        if let Some(context) = self.context {
            client.set_context(context);
        }
    }

    pub fn float(&self) -> bool {
        self.float.map_or(false, |float| float)
    }

    pub fn center(&self) -> bool {
        self.center.map_or(false, |center| center)
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen.map_or(false, |fullscreen| fullscreen)
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            float: None,
            center: None,
            fullscreen: None,
            workspace: None,
            context: None,
        }
    }
}
