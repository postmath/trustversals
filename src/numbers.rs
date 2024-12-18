use num_traits::int::PrimInt;
use std::fmt::Debug;

pub trait HNumber: PrimInt + Clone + Debug {}

macro_rules! hnumber_impl {
    ($($t:ty)*) => ($(
        impl HNumber for $t {
        }
    )*)
}

hnumber_impl! {
    u8 u16 u32 u64 u128 usize
}
