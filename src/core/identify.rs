use winsys::window::Window;

pub type Ident = u32;
pub type Index = usize;

pub trait Identify: PartialEq {
    fn id(&self) -> Ident;
}

impl Identify for Window {
    fn id(&self) -> Ident {
        *self as Ident
    }
}
