use crate::client::Client;
use crate::common::Change;
use crate::common::Ident;
use crate::common::Identify;
use crate::common::Placement;
use crate::common::FREE_EXTENTS;
use crate::common::MIN_WINDOW_DIM;
use crate::util::Util;

use winsys::common::Dim;
use winsys::common::Edge;
use winsys::common::Extents;
use winsys::common::Padding;
use winsys::common::Pos;
use winsys::common::Region;
use winsys::common::Window;

use std::default::Default;

pub type LayoutFn =
    fn(&[&Client], Option<Window>, &Region, &LayoutData) -> Vec<Placement>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutMethod {
    /// Does not inhibit free placement of clients
    Free,
    /// Arranges clients along a predefined layout
    Tile,
    /// Semi-adjustable tree-based layout
    Tree,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutKind {
    Center,
    Float,
    Monocle,
    Paper,
    PaperCenter,
    SStack,
    SingleFloat,
    Stack,
}

pub struct LayoutFactory {}

impl LayoutFactory {
    pub fn create_layout(kind: LayoutKind) -> Layout {
        match kind {
            LayoutKind::Center => CenterLayout::build(),
            LayoutKind::Float => FloatLayout::build(),
            LayoutKind::Monocle => MonocleLayout::build(),
            LayoutKind::Paper => PaperLayout::build(),
            LayoutKind::PaperCenter => PaperCenterLayout::build(),
            LayoutKind::SStack => SStackLayout::build(),
            LayoutKind::SingleFloat => SingleFloatLayout::build(),
            LayoutKind::Stack => StackLayout::build(),
        }
    }
}

pub trait LayoutApply {
    fn arrange(
        &self,
        clients: &[&Client],
        focus: Option<Window>,
        screen: &Region,
    ) -> Vec<Placement>;
}

#[derive(Clone)]
pub struct Layout {
    pub kind: LayoutKind,
    pub symbol: char,
    pub name: String,
    pub config: LayoutConfig,
    data: LayoutData,
    default_data: LayoutData,
    logic: LayoutFn,
}

impl Layout {
    const MAX_MAIN_COUNT: u32 = 15;
    const MAX_GAP_SIZE: u32 = 300;
    const MAX_MARGIN: Padding = Padding {
        left: 350,
        right: 350,
        top: 200,
        bottom: 200,
    };

    pub fn new(
        kind: LayoutKind,
        symbol: char,
        name: impl Into<String>,
        config: LayoutConfig,
        data: LayoutData,
        logic: LayoutFn,
    ) -> Self {
        Self {
            kind,
            symbol,
            name: name.into(),
            config,
            data,
            default_data: data,
            logic,
        }
    }

    pub fn change_gap_size(
        &mut self,
        change: Change,
        delta: u32,
    ) {
        if let LayoutData::Tile(ref mut data) = self.data {
            data.gap_size = Util::change_within_range(
                0,
                Self::MAX_GAP_SIZE,
                data.gap_size,
                change,
                delta,
            );
        }
    }

    pub fn reset_gap_size(&mut self) {
        match self.data {
            LayoutData::Tile(ref mut data) => {
                if let LayoutData::Tile(default_data) = self.default_data {
                    data.gap_size = default_data.gap_size;
                }
            },
            LayoutData::Tree(ref mut data) => {
                if let LayoutData::Tree(default_data) = self.default_data {
                    data.gap_size = default_data.gap_size;
                }
            },
            _ => {},
        };
    }

    pub fn main_count(&self) -> Option<u32> {
        if let LayoutData::Tile(data) = self.data {
            Some(data.main_count)
        } else {
            None
        }
    }

    pub fn change_main_count(
        &mut self,
        change: Change,
    ) {
        if let LayoutData::Tile(ref mut data) = self.data {
            data.main_count = Util::change_within_range(
                0,
                Self::MAX_MAIN_COUNT,
                data.main_count,
                change,
                1,
            );
        }
    }

    pub fn change_main_factor(
        &mut self,
        change: Change,
        delta: f32,
    ) {
        if let LayoutData::Tile(ref mut data) = self.data {
            match change {
                Change::Inc => data.main_factor += delta,
                Change::Dec => data.main_factor -= delta,
            }

            if data.main_factor < 0.05f32 {
                data.main_factor = 0.05f32;
            } else if data.main_factor > 0.95f32 {
                data.main_factor = 0.95f32;
            }
        }
    }

    pub fn change_margin(
        &mut self,
        edge: Edge,
        change: Change,
        delta: u32,
    ) {
        match self.data {
            LayoutData::Tile(ref mut data) => {
                if let LayoutData::Tile(default_data) = self.default_data {
                    if let Some(ref mut margin) = data.margin {
                        let default_margin =
                            if let Some(margin) = default_data.margin {
                                margin
                            } else {
                                Padding::default()
                            };

                        match edge {
                            Edge::Left => {
                                margin.left = Util::change_within_range(
                                    default_margin.left,
                                    Self::MAX_MARGIN.left,
                                    margin.left,
                                    change,
                                    delta,
                                );
                            },
                            Edge::Right => {
                                margin.right = Util::change_within_range(
                                    default_margin.right,
                                    Self::MAX_MARGIN.right,
                                    margin.right,
                                    change,
                                    delta,
                                );
                            },
                            Edge::Top => {
                                margin.top = Util::change_within_range(
                                    default_margin.top,
                                    Self::MAX_MARGIN.top,
                                    margin.top,
                                    change,
                                    delta,
                                );
                            },
                            Edge::Bottom => {
                                margin.bottom = Util::change_within_range(
                                    default_margin.bottom,
                                    Self::MAX_MARGIN.bottom,
                                    margin.bottom,
                                    change,
                                    delta,
                                );
                            },
                        }
                    }
                }
            },
            LayoutData::Tree(ref mut data) => {
                if let LayoutData::Tree(default_data) = self.default_data {
                    if let Some(ref mut margin) = data.margin {
                        let default_margin =
                            if let Some(margin) = default_data.margin {
                                margin
                            } else {
                                Padding::default()
                            };

                        match edge {
                            Edge::Left => {
                                margin.left = Util::change_within_range(
                                    default_margin.left,
                                    Self::MAX_MARGIN.left,
                                    margin.left,
                                    change,
                                    delta,
                                );
                            },
                            Edge::Right => {
                                margin.right = Util::change_within_range(
                                    default_margin.right,
                                    Self::MAX_MARGIN.right,
                                    margin.right,
                                    change,
                                    delta,
                                );
                            },
                            Edge::Top => {
                                margin.top = Util::change_within_range(
                                    default_margin.top,
                                    Self::MAX_MARGIN.top,
                                    margin.top,
                                    change,
                                    delta,
                                );
                            },
                            Edge::Bottom => {
                                margin.bottom = Util::change_within_range(
                                    default_margin.bottom,
                                    Self::MAX_MARGIN.bottom,
                                    margin.bottom,
                                    change,
                                    delta,
                                );
                            },
                        }
                    }
                }
            },
            _ => {},
        };
    }

    pub fn reset_margin(&mut self) {
        match self.data {
            LayoutData::Tile(ref mut data) => {
                if let LayoutData::Tile(default_data) = self.default_data {
                    data.margin = default_data.margin;
                }
            },
            LayoutData::Tree(ref mut data) => {
                if let LayoutData::Tree(default_data) = self.default_data {
                    data.margin = default_data.margin;
                }
            },
            _ => {},
        };
    }
    pub fn reset(&mut self) {
        self.data = self.default_data;
    }

    fn adjust_for_margin(
        region: Region,
        extents: &Extents,
    ) -> Region {
        Region {
            pos: Pos {
                x: region.pos.x + extents.left as i32,
                y: region.pos.y + extents.top as i32,
            },
            dim: Dim {
                w: region.dim.w - extents.left - extents.right,
                h: region.dim.h - extents.top - extents.bottom,
            },
        }
    }

    pub fn adjust_for_padding(
        mut placement: Placement,
        gap_size: u32,
    ) -> Placement {
        if let Some(ref mut region) = placement.region {
            let padding = 2 * gap_size;

            region.pos.x += gap_size as i32;
            region.pos.y += gap_size as i32;

            if region.dim.w >= padding + MIN_WINDOW_DIM.w {
                region.dim.w -= padding;
            } else {
                region.dim.w = MIN_WINDOW_DIM.w;
            }

            if region.dim.h >= padding + MIN_WINDOW_DIM.h {
                region.dim.h -= padding;
            } else {
                region.dim.h = MIN_WINDOW_DIM.h;
            }
        }

        placement
    }
}

impl LayoutApply for Layout {
    fn arrange(
        &self,
        clients: &[&Client],
        focus: Option<Window>,
        screen: &Region,
    ) -> Vec<Placement> {
        let screen = Layout::adjust_for_margin(
            *screen,
            &LayoutData::screen_margin(&self.data),
        );

        (self.logic)(clients, focus, &screen, &self.data)
            .iter_mut()
            .map(|p| {
                Layout::adjust_for_padding(
                    *p,
                    if self.config.gap {
                        LayoutData::gap_size(&self.data)
                    } else {
                        0
                    },
                )
            })
            .collect()
    }
}

impl std::cmp::PartialEq<Self> for Layout {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.kind == other.kind
            && self.symbol == other.symbol
            && self.name == other.name
            && self.config == other.config
            && self.data == other.data
    }
}

impl Identify for Layout {
    fn id(&self) -> Ident {
        self.kind as Ident
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct LayoutConfig {
    pub method: LayoutMethod,
    pub mirrorable: bool,
    pub gap: bool,
    pub persistent: bool,
    pub single: bool,
    pub wraps: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            method: LayoutMethod::Free,
            mirrorable: false,
            gap: false,
            persistent: false,
            single: false,
            wraps: true,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LayoutData {
    Free(FreeData),
    Tile(TileData),
    Tree(TreeData),
}

impl LayoutData {
    fn screen_margin(data: &LayoutData) -> Extents {
        let no_margin = Extents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        };

        match data {
            LayoutData::Free(_) => no_margin,
            LayoutData::Tile(data) => {
                if let Some(margin) = data.margin {
                    margin
                } else {
                    no_margin
                }
            },
            LayoutData::Tree(data) => {
                if let Some(margin) = data.margin {
                    margin
                } else {
                    no_margin
                }
            },
        }
    }

    fn gap_size(data: &LayoutData) -> u32 {
        match data {
            LayoutData::Free(_) => 0,
            LayoutData::Tile(data) => data.gap_size,
            LayoutData::Tree(data) => data.gap_size,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct FreeData {
    pub frame_extents: Option<Extents>,
}

impl Default for FreeData {
    fn default() -> Self {
        Self {
            frame_extents: Some(FREE_EXTENTS),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TileData {
    pub main_count: u32,
    pub gap_size: u32,
    pub main_factor: f32,
    pub mirrored: bool,
    pub margin: Option<Padding>,
    pub frame_extents: Option<Extents>,
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            main_count: 1,
            gap_size: 15,
            main_factor: 0.5,
            mirrored: false,
            margin: Some(Padding {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            }),
            frame_extents: Some(Extents {
                left: 0,
                right: 0,
                top: 3,
                bottom: 0,
            }),
        }
    }
}

impl TileData {
    pub fn stack_split<T>(
        clients: &[T],
        n_main: u32,
    ) -> (u32, u32) {
        let n = clients.len() as u32;
        if n <= n_main {
            (n, 0)
        } else {
            (n_main, n - n_main)
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TreeData {
    pub gap_size: u32,
    pub mirrored: bool,
    pub margin: Option<Padding>,
    pub frame_extents: Option<Extents>,
}

impl Default for TreeData {
    fn default() -> Self {
        Self {
            gap_size: 15,
            mirrored: false,
            margin: Some(Padding {
                left: 15,
                right: 15,
                top: 15,
                bottom: 15,
            }),
            frame_extents: Some(Extents {
                left: 0,
                right: 0,
                top: 3,
                bottom: 0,
            }),
        }
    }
}

trait LayoutBuilder {
    fn build() -> Layout;
}

trait LayoutFunc {
    fn func(
        clients: &[&Client],
        focus: Option<Window>,
        screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement>;
}

struct FloatLayout {}

impl LayoutBuilder for FloatLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Free,
            mirrorable: false,
            gap: false,
            persistent: false,
            single: false,
            wraps: true,
        };

        let data = FreeData::default();

        Layout {
            kind: LayoutKind::Float,
            symbol: 'F',
            name: "float".into(),
            config,
            data: LayoutData::Free(data),
            default_data: LayoutData::Free(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for FloatLayout {
    fn func(
        clients: &[&Client],
        _focus: Option<Window>,
        _screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement> {
        if let LayoutData::Free(data) = data {
            clients
                .iter()
                .map(|c| {
                    Placement::new(
                        c.window(),
                        Some(*c.free_region()),
                        data.frame_extents,
                    )
                })
                .collect()
        } else {
            Vec::with_capacity(0)
        }
    }
}

struct SingleFloatLayout {}

impl LayoutBuilder for SingleFloatLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Free,
            mirrorable: false,
            gap: false,
            persistent: true,
            single: true,
            wraps: true,
        };

        let data = FreeData::default();

        Layout {
            kind: LayoutKind::SingleFloat,
            symbol: 'Z',
            name: "singlefloat".into(),
            config,
            data: LayoutData::Free(data),
            default_data: LayoutData::Free(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for SingleFloatLayout {
    fn func(
        clients: &[&Client],
        focus: Option<Window>,
        _screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement> {
        if let LayoutData::Free(data) = data {
            if let Some(focus) = focus {
                clients
                    .iter()
                    .map(|c| {
                        let window = c.window();

                        if window == focus {
                            Placement::new(
                                window,
                                Some(*c.free_region()),
                                data.frame_extents,
                            )
                        } else {
                            Placement::new(window, None, None)
                        }
                    })
                    .collect()
            } else {
                Vec::with_capacity(0)
            }
        } else {
            Vec::with_capacity(0)
        }
    }
}

struct StackLayout {}

impl LayoutBuilder for StackLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Tile,
            mirrorable: true,
            gap: true,
            persistent: false,
            single: false,
            wraps: true,
        };

        let data = TileData::default();

        Layout {
            kind: LayoutKind::Stack,
            symbol: 'S',
            name: "stack".into(),
            config,
            data: LayoutData::Tile(data),
            default_data: LayoutData::Tile(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for StackLayout {
    fn func(
        clients: &[&Client],
        _focus: Option<Window>,
        screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement> {
        if let LayoutData::Tile(data) = data {
            let n = clients.len();
            let (screen_pos, screen_dim) = screen.values();

            if n == 1 {
                return vec![Placement::new(
                    clients[0].window(),
                    Some(*screen),
                    None,
                )];
            }

            let (n_main, n_stack) =
                TileData::stack_split(&clients, data.main_count);

            let h_stack = if n_stack > 0 {
                screen_dim.h / n_stack
            } else {
                0
            };

            let h_main = if n_main > 0 { screen_dim.h / n_main } else { 0 };

            let split = if data.main_count > 0 {
                (screen_dim.w as f32 * data.main_factor) as i32
            } else {
                0
            };

            clients
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let i = i as u32;

                    if i < data.main_count {
                        let w = if n_stack == 0 {
                            screen_dim.w
                        } else {
                            split as u32
                        };

                        Placement::new(
                            c.window(),
                            Some(Region::new(
                                screen_pos.x,
                                screen_pos.y + (i * h_main) as i32,
                                w,
                                h_main,
                            )),
                            data.frame_extents,
                        )
                    } else {
                        let sn = (i - data.main_count) as i32;

                        let region = Region::new(
                            screen_pos.x + split,
                            screen_pos.y + sn * h_stack as i32,
                            screen_dim.w - split as u32,
                            h_stack,
                        );

                        Placement::new(
                            c.window(),
                            Some(region),
                            data.frame_extents,
                        )
                    }
                })
                .collect()
        } else {
            Vec::with_capacity(0)
        }
    }
}

struct SStackLayout {}

impl LayoutBuilder for SStackLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Tile,
            mirrorable: true,
            gap: false,
            persistent: false,
            single: false,
            wraps: true,
        };

        let data = TileData::default();

        Layout {
            kind: LayoutKind::SStack,
            symbol: 'T',
            name: "sstack".into(),
            config,
            data: LayoutData::Tile(data),
            default_data: LayoutData::Tile(data),
            logic: StackLayout::func,
        }
    }
}

struct MonocleLayout {}

impl LayoutBuilder for MonocleLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Tile,
            mirrorable: false,
            gap: false,
            persistent: false,
            single: false,
            wraps: true,
        };

        let data = TileData {
            main_count: 0,
            gap_size: 0,
            main_factor: 0f32,
            mirrored: false,
            frame_extents: None,
            ..Default::default()
        };

        Layout {
            kind: LayoutKind::Monocle,
            symbol: 'M',
            name: "monocle".into(),
            config,
            data: LayoutData::Tile(data),
            default_data: LayoutData::Tile(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for MonocleLayout {
    fn func(
        clients: &[&Client],
        _focus: Option<Window>,
        screen: &Region,
        _data: &LayoutData,
    ) -> Vec<Placement> {
        let (screen_pos, screen_dim) = screen.values();

        clients
            .iter()
            .map(|c| {
                Placement::new(
                    c.window(),
                    Some(Region::new(
                        screen_pos.x,
                        screen_pos.y,
                        screen_dim.w,
                        screen_dim.h,
                    )),
                    None,
                )
            })
            .collect()
    }
}

struct CenterLayout {}

impl LayoutBuilder for CenterLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Tile,
            mirrorable: false,
            gap: true,
            persistent: false,
            single: false,
            wraps: true,
        };

        let data = TileData {
            main_count: 5,
            gap_size: 0,
            main_factor: 0.40f32,
            mirrored: false,
            frame_extents: None,
            ..Default::default()
        };

        Layout {
            kind: LayoutKind::Center,
            symbol: '|',
            name: "center".into(),
            config,
            data: LayoutData::Tile(data),
            default_data: LayoutData::Tile(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for CenterLayout {
    fn func(
        clients: &[&Client],
        _focus: Option<Window>,
        screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement> {
        if let LayoutData::Tile(data) = data {
            let (screen_pos, screen_dim) = screen.values();

            clients
                .iter()
                .map(|c| {
                    let window = c.window();

                    let w_ratio: f32 = data.main_factor / 0.95;
                    let h_ratio: f32 =
                        ((Layout::MAX_MAIN_COUNT + 1) - data.main_count) as f32
                            / (Layout::MAX_MAIN_COUNT + 1) as f32;

                    Placement::new(
                        window,
                        Some(
                            Region::new(
                                screen_pos.x,
                                screen_pos.y,
                                screen_dim.w,
                                screen_dim.h,
                            )
                            .from_absolute_inner_center(&Dim {
                                w: (screen_dim.w as f32 * w_ratio) as u32,
                                h: (screen_dim.h as f32 * h_ratio) as u32,
                            }),
                        ),
                        None,
                    )
                })
                .collect()
        } else {
            Vec::with_capacity(0)
        }
    }
}

struct PaperLayout {}

impl LayoutBuilder for PaperLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Tile,
            mirrorable: true,
            gap: false,
            persistent: true,
            single: false,
            wraps: false,
        };

        let data = TileData::default();

        Layout {
            kind: LayoutKind::Paper,
            symbol: ';',
            name: "paper".into(),
            config,
            data: LayoutData::Tile(data),
            default_data: LayoutData::Tile(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for PaperLayout {
    fn func(
        clients: &[&Client],
        focus: Option<Window>,
        screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement> {
        if let LayoutData::Tile(data) = data {
            let n = clients.len();

            if n == 1 {
                return vec![Placement::new(
                    clients[0].window(),
                    Some(*screen),
                    None,
                )];
            }

            let (screen_pos, screen_dim) = screen.values();
            let min_w = 0.5;

            let cw = (screen_dim.w as f32
                * if data.main_factor > min_w {
                    data.main_factor
                } else {
                    min_w
                }) as u32;

            let step = ((screen_dim.w - cw) as usize / (n - 1)) as i32;
            let focus = focus.unwrap();
            let mut after_focus = false;

            clients
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let window = c.window();
                    if window == focus {
                        after_focus = true;

                        Placement::new(
                            window,
                            Some(Region::new(
                                screen_pos.x + i as i32 * step,
                                screen_pos.y,
                                cw,
                                screen_dim.h,
                            )),
                            data.frame_extents,
                        )
                    } else {
                        let mut x = screen_pos.x + i as i32 * step;

                        if after_focus {
                            x += cw as i32 - step
                        };

                        Placement::new(
                            window,
                            Some(Region::new(
                                x,
                                screen_pos.y,
                                step as u32,
                                screen_dim.h,
                            )),
                            data.frame_extents,
                        )
                    }
                })
                .collect()
        } else {
            Vec::with_capacity(0)
        }
    }
}

struct PaperCenterLayout {}

impl LayoutBuilder for PaperCenterLayout {
    fn build() -> Layout {
        let config = LayoutConfig {
            method: LayoutMethod::Tile,
            mirrorable: false,
            gap: false,
            persistent: true,
            single: false,
            wraps: false,
        };

        let data = TileData {
            main_count: 0,
            main_factor: 0.95f32,
            gap_size: (Layout::MAX_GAP_SIZE as f32 / 2f32) as u32,
            ..Default::default()
        };

        Layout {
            kind: LayoutKind::PaperCenter,
            symbol: ';',
            name: "papercenter".into(),
            config,
            data: LayoutData::Tile(data),
            default_data: LayoutData::Tile(data),
            logic: Self::func,
        }
    }
}

impl LayoutFunc for PaperCenterLayout {
    fn func(
        clients: &[&Client],
        focus: Option<Window>,
        screen: &Region,
        data: &LayoutData,
    ) -> Vec<Placement> {
        if let LayoutData::Tile(data) = data {
            let n = clients.len();

            if n == 1 {
                return vec![Placement::new(
                    clients[0].window(),
                    Some(*screen),
                    None,
                )];
            }

            let min_w = 0.5;
            let max_w = 0.95;
            let w_ratio: f32 = data.main_factor / 0.95;
            let h_ratio: f32 = ((Layout::MAX_MAIN_COUNT + 1) - data.main_count)
                as f32
                / (Layout::MAX_MAIN_COUNT + 1) as f32;

            let screen = Region::new(
                screen.pos.x,
                screen.pos.y,
                screen.dim.w,
                screen.dim.h,
            )
            .from_absolute_inner_center(&Dim {
                w: (screen.dim.w as f32 * w_ratio) as u32,
                h: (screen.dim.h as f32 * h_ratio) as u32,
            });

            let (screen_pos, screen_dim) = screen.values();

            let cw = data.gap_size as f32 / Layout::MAX_GAP_SIZE as f32;
            let cw = (screen_dim.w as f32
                * if cw > min_w {
                    if cw <= max_w {
                        data.gap_size as f32 / Layout::MAX_GAP_SIZE as f32
                    } else {
                        max_w
                    }
                } else {
                    min_w
                }) as u32;

            let step = (screen_dim.w - cw) / (n - 1) as u32;
            let focus = focus.unwrap();
            let mut after_focus = false;

            clients
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let window = c.window();
                    if window == focus {
                        after_focus = true;

                        Placement::new(
                            window,
                            Some(Region::new(
                                screen_pos.x + i as i32 * step as i32,
                                screen_pos.y,
                                cw,
                                screen_dim.h,
                            )),
                            data.frame_extents,
                        )
                    } else {
                        let mut x = screen_pos.x + (i as i32) * (step as i32);

                        if after_focus {
                            x += cw as i32 - step as i32
                        };

                        Placement::new(
                            window,
                            Some(Region::new(
                                x,
                                screen_pos.y,
                                step,
                                screen_dim.h,
                            )),
                            data.frame_extents,
                        )
                    }
                })
                .collect()
        } else {
            Vec::with_capacity(0)
        }
    }
}

impl std::fmt::Debug for Layout {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Layout")
            .field("kind", &self.kind)
            .field("symbol", &self.symbol)
            .field("name", &self.name)
            .field("config", &self.config)
            .field("data", &self.data)
            .field("logic", &stringify!(&self.logic))
            .finish()
    }
}
