pub trait IndexedMerkleTree {
    fn get_index(&self) -> u64;
    fn is_newest(&self) -> bool;
    fn set_newest(&mut self, newest: bool);
}

#[macro_export]
macro_rules! impl_indexed_merkle_tree {
    ($strct:ident) => {
        impl $crate::indexed_merkle_tree::IndexedMerkleTree for $strct {
            fn get_index(&self) -> u64 {
                self.merkle_tree_nr
            }

            fn is_newest(&self) -> bool {
                if self.newest == 0 {
                    return false;
                }
                true
            }

            fn set_newest(&mut self, newest: bool) {
                match newest {
                    true => self.newest = 1,
                    false => self.newest = 0,
                }
            }
        }
    };
}
