use crate::client::Client;
use crate::decoration::ColorScheme;
use crate::decoration::Decoration;
use crate::decoration::Frame;
use crate::layout::Layout;
use crate::zone::Zone;

use winsys::geometry::Dim;
use winsys::geometry::Extents;
use winsys::geometry::Padding;

#[macro_export]
macro_rules! WM_NAME (
    () => { "wzrd" };
);

impl Client {
    pub const MIN_CLIENT_DIM: Dim = Dim {
        w: 75,
        h: 50,
    };

    pub const PREFERRED_CLIENT_DIM: Dim = Dim {
        w: 480,
        h: 260,
    };
}

impl Decoration {
    pub const NO_DECORATION: Self = Self {
        border: None,
        frame: None,
    };

    pub const FREE_DECORATION: Self = Self {
        border: None,
        frame: Some(Frame {
            extents: Extents {
                left: 3,
                right: 1,
                top: 1,
                bottom: 1,
            },
            colors: ColorScheme::DEFAULT,
        }),
    };
}

impl Layout {
    pub const MAX_MAIN_COUNT: u32 = 15;
    pub const MAX_GAP_SIZE: u32 = 300;
    pub const MAX_MARGIN: Padding = Padding {
        left: 700,
        right: 700,
        top: 400,
        bottom: 400,
    };
}

impl Zone {
    pub const MIN_ZONE_DIM: Dim = Dim {
        w: 25,
        h: 25,
    };
}
