pub type Ident = u32;
pub type Index = usize;

pub trait Identify: PartialEq {
    fn id(&self) -> Ident;
}
