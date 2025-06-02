use std::fmt::Debug;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Context {
    pub slot: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Response<T: Clone + PartialEq + Default + Debug> {
    pub context: Context,
    pub value: T,
}

impl<T: Clone + PartialEq + Default + Debug> Response<T> {
    pub fn indexer_slot(&self) -> u64 {
        self.context.slot
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Items<T: Clone + PartialEq + Default + Debug> {
    pub items: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemsWithCursor<T: Clone + PartialEq + Default + Debug> {
    pub items: Vec<T>,
    pub cursor: Option<String>,
}
