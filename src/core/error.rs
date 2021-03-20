#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StateChangeError {
    EarlyStop,
    LimitReached,
    StateUnchanged,
    InvalidCaller,
}
