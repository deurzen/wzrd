use crate::client::Client;

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
        client: &mut Client,
    ) {
        if let Some(float) = self.float {
            client.set_floating(float);
        }

        if let Some(fullscreen) = self.fullscreen {
            client.set_fullscreen(fullscreen);
        }

        if let Some(workspace) = self.workspace {
            client.set_workspace(workspace);
        }

        if let Some(context) = self.context {
            client.set_context(context);
        }
    }

    pub fn float(
        &self,
        must_float: &mut bool,
    ) -> bool {
        if let Some(float) = self.float {
            *must_float = float;
            return float;
        }

        false
    }

    pub fn center(
        &self,
        must_center: &mut bool,
    ) -> bool {
        if let Some(center) = self.center {
            *must_center = center;
            return center;
        }

        false
    }

    pub fn fullscreen(
        &self,
        must_fullscreen: &mut bool,
    ) -> bool {
        if let Some(fullscreen) = self.fullscreen {
            *must_fullscreen = fullscreen;
            return fullscreen;
        }

        false
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
