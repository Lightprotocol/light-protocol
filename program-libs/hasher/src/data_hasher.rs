use crate::HasherError;

pub trait DataHasher {
    fn hash<H: crate::Hasher>(&self) -> Result<[u8; 32], HasherError>;
}

macro_rules! impl_data_hasher_for_array {
    ($(
         // For each array, specify the length and then a bracketed list of indices.
         $len:literal => [$($index:tt),* $(,)?]
    )*) => {
        $(
            impl<T: DataHasher + Default> DataHasher for [T; $len] {
                fn hash<H: crate::Hasher>(&self) -> Result<[u8; 32], HasherError> {
                    // We call T’s hash on each element and then pass the resulting list to H::hash.
                    H::hashv(&[$( &self[$index].hash::<H>()?.as_slice() ),*])
                }
            }
        )*
    }
}

impl_data_hasher_for_array! {
    1 => [0]
}
impl_data_hasher_for_array! {
    2 => [0, 1]
}
impl_data_hasher_for_array! {
    3 => [0, 1, 2]
}
impl_data_hasher_for_array! {
    4 => [0, 1, 2, 3]
}
impl_data_hasher_for_array! {
    5 => [0, 1, 2, 3, 4]
}
impl_data_hasher_for_array! {
    6 => [0, 1, 2, 3, 4, 5]
}
impl_data_hasher_for_array! {
    7 => [0, 1, 2, 3, 4, 5, 6]
}
impl_data_hasher_for_array! {
    8 => [0, 1, 2, 3, 4, 5, 6, 7]
}
impl_data_hasher_for_array! {
    9 => [0, 1, 2, 3, 4, 5, 6, 7, 8]
}
impl_data_hasher_for_array! {
    10 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
}
impl_data_hasher_for_array! {
    11 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
}
impl_data_hasher_for_array! {
    12 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
}
