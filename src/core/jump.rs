use crate::client::Client;
use crate::compare::MatchMethod;
use crate::identify::Index;
use crate::workspace::ClientSelector;

#[derive(Clone, Copy)]
pub enum JumpCriterium {
    OnWorkspaceBySelector(Index, &'static ClientSelector),
    ByName(MatchMethod<&'static str>),
    ByClass(MatchMethod<&'static str>),
    ByInstance(MatchMethod<&'static str>),
    ForCond(&'static dyn Fn(&Client) -> bool),
}
