#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MatchMethod<T> {
    Equals(T),
    Contains(T),
}
