use num_traits::Num;

pub trait HNumber: Num + Clone {
    fn count_ones(self) -> u32;
}

macro_rules! hnumber_impl {
    ($($t:ty)*) => ($(
        impl HNumber for $t {
            fn count_ones(self) -> u32 {
                self.count_ones()
            }
        }
    )*)
}

hnumber_impl! {
    u8 u16 u32 u64 u128 usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_ones_for_test(n: impl HNumber) -> u32 {
        n.count_ones()
    }

    #[test]
    fn test_count_ones() {
        let u0 = 17u8;
        assert_eq!(count_ones_for_test(u0), 2);

        let u1 = (1u64 << 60) + (1u64 << 48) + (1u64 << 5);
        assert_eq!(count_ones_for_test(u1), 3);
    }
}
