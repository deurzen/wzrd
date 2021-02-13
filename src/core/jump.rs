use crate::client::Client;
use crate::common::Index;
use crate::workspace::ClientSelector;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MatchMethod {
    Equals,
    Contains,
}

#[derive(Clone, Copy)]
pub enum JumpCriterium {
    OnWorkspaceBySelector(Index, &'static ClientSelector),
    ByName(&'static str, MatchMethod),
    ByClass(&'static str, MatchMethod),
    ByInstance(&'static str, MatchMethod),
    ForCond(&'static dyn Fn(&Client) -> bool),
}
