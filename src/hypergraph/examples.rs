use super::*;

// I first wanted to implement an iterator that yields bitwise combinations, but the following make
// that hard currently:
//
// - I would want it to be a lending iterator, which advances the state of some buffer owned by the
//   iterator in the next() call and then lends the buffer out to the caller. (This can be done now
//   that we have generic associated types.)
//
// - However, the iterator would need to lend out the buffer in a situation where it, itself, still
//   holds onto that buffer for the next call. I couldn't figure out how to do that without
//   significant effort. I figured it'd be easier to *not* implement anything iterator-like, and
//   just make a struct that can be modified, so that the caller is in charge of ownership.

struct BitwiseCombinations<I>
where
    I: HNumber,
{
    state: Vec<I>,
    bits_in_top_word: u32,
}

impl<I> BitwiseCombinations<I>
where
    I: HNumber,
{
    pub fn new(n: usize, k: usize) -> Self {
        let bit_size_u = std::mem::size_of::<I>() * 8;
        let bit_size = bit_size_u as u32;
        let (num_words, bits_in_top_word) = if n % bit_size_u == 0 {
            (n / bit_size_u, bit_size)
        } else {
            (n / bit_size_u + 1, (n % bit_size_u) as u32)
        };

        let mut state = vec![I::zero(); num_words];
        let num_one_words: usize = k / bit_size_u;
        state[0..num_one_words].fill(I::max_value());
        let num_one_bits = k % bit_size_u;
        if num_one_bits > 0 {
            state[num_one_words] = (I::one() << num_one_bits) - I::one();
        }

        Self {
            state,
            bits_in_top_word,
        }
    }

    fn trailing_ones_and_zeroes(&self) -> Option<(u32, u32)> {
        let bit_size = (std::mem::size_of::<I>() * 8) as u32;

        let mut idx = 0usize;
        let mut zeroes_here = self.state[0].trailing_zeros();
        let mut zeroes = zeroes_here;

        while zeroes_here == bit_size && idx < self.state.len() - 1 {
            idx += 1;
            zeroes_here = self.state[idx].trailing_zeros();
            zeroes += zeroes_here;
        }

        if idx == self.state.len() - 1 && zeroes_here == bit_size {
            // The value contained is all zeroes.
            return None;
        }

        let next_word = self.state[idx] >> (zeroes_here as usize);
        let mut ones_here = next_word.trailing_ones();
        let mut ones = ones_here;

        if idx == self.state.len() - 1 {
            return Some((ones, zeroes));
        } else if ones_here + zeroes_here == bit_size {
            // Now we know that idx < self.state.len() - 1; we need to examine the next word.
            idx += 1;
            ones_here = self.state[idx].trailing_ones();
            ones += ones_here;
            while ones_here == bit_size && idx < self.state.len() - 1 {
                idx += 1;
                ones_here = self.state[idx].trailing_ones();
                ones += ones_here;
            }
        }

        Some((ones, zeroes))
    }

    #[inline]
    fn mask_complement(mask: I) -> I {
        I::max_value() ^ mask
    }

    // An I where the m rightmost bits are zero, then n bits are one, and then the rest is zero.
    #[inline]
    fn mask_ones_zeroes_right(n: u32, m: u32) -> I {
        let bit_size = (std::mem::size_of::<I>() * 8) as u32;

        if n + m == bit_size {
            if m == bit_size {
                I::zero()
            } else {
                Self::mask_zeroes_right(m)
            }
        } else {
            let n_usize = n as usize;
            let m_usize = m as usize;
            (I::one() << (n_usize + m_usize)) - (I::one() << m_usize)
        }
    }

    // An I where the m rightmost bits are one, then n bits are zero, then the rest is one.
    #[inline]
    fn mask_zeroes_ones_right(n: u32, m: u32) -> I {
        Self::mask_complement(Self::mask_ones_zeroes_right(n, m))
    }

    // An I where the n rightmost bits are one and all others are zero.
    #[inline]
    fn mask_ones_right(n: u32) -> I {
        Self::mask_ones_zeroes_right(n, 0)
    }

    // An I where the n rightmost bits are zero and all others are one.
    #[inline]
    fn mask_zeroes_right(n: u32) -> I {
        Self::mask_complement(Self::mask_ones_right(n))
    }

    // Modify self to represent the next combination. Returns true if it succeeded, false if this
    // was the last combination.
    fn next_combination(&mut self) -> bool {
        if self.state.is_empty() {
            return false;
        }

        let bit_size = (std::mem::size_of::<I>() * 8) as u32;

        // Let the (semantic) value contained end in n>=0 zeroes (from the right) and then m>0 ones
        // and then a zero. We swap that last zero with its neighbouring one and move the remaining
        // m-1 ones all the way to the right:
        //
        // n == 2, m == 3         n == 0, m == 5
        // 0bXXXXXX011100         0bXXXXXX011111
        //      V                       V
        // 0bXXXXXX100011         0bXXXXXX101111
        //
        // Afterwards, the pattern from left to right is, a single one, then n+1 zeroes, then m-1
        // ones. Note that the bit counts may cross word boundaries.
        if let Some((m, n)) = self.trailing_ones_and_zeroes() {
            if m + n == (self.state.len() as u32 - 1) * bit_size + self.bits_in_top_word {
                // There is no greater value that we can yield.
                return false;
            }

            // Set m-1 one bits on the right
            let mut idx = 0usize;
            let mut ones_to_set = m - 1;
            while ones_to_set >= bit_size {
                self.state[idx] = I::max_value();
                ones_to_set -= bit_size;
                idx += 1;
            }
            self.state[idx] = self.state[idx] | Self::mask_ones_right(ones_to_set);
            // Now 'ones_to_set' is the number of ones set in this last word.

            // Set n+1 zero bits in the middle
            let mut zeroes_to_set = n + 1;
            if ones_to_set + zeroes_to_set <= bit_size {
                self.state[idx] =
                    self.state[idx] & Self::mask_zeroes_ones_right(zeroes_to_set, ones_to_set);
            } else {
                self.state[idx] = self.state[idx] & Self::mask_ones_right(ones_to_set);
                zeroes_to_set -= bit_size - ones_to_set;
                idx += 1;
                while zeroes_to_set >= bit_size {
                    self.state[idx] = I::zero();
                    zeroes_to_set -= bit_size;
                    idx += 1;
                }
                self.state[idx] = self.state[idx] & Self::mask_zeroes_right(zeroes_to_set);
            }

            // Set the one bit on the left
            let word = (m + n) / bit_size;
            let bit = (m + n) % bit_size;
            self.state[word as usize] = self.state[word as usize] | (I::one() << (bit as usize));

            true
        } else {
            false
        }
    }
}

pub fn binomial<I: HNumber>(n: usize, k: usize) -> Hypergraph<I> {
    if k == 0 || n < k {
        panic!("cannot create binomial hypergraph with parameters {n} and {k}");
    }
    let mut h = Hypergraph::<I>::new(n as u32);
    let mut bitwise_combinations = BitwiseCombinations::<I>::new(n, k);
    h.add_edge(&bitwise_combinations.state);

    while bitwise_combinations.next_combination() {
        h.add_edge(&bitwise_combinations.state);
    }

    h
}

// pub fn kuratowski<I: HNumber>(n: usize, k: usize) -> Hypergraph<I> {}

mod tests {
    use super::*;

    #[test]
    fn test_binomial_1() {
        let h = binomial::<u32>(1, 1);
        assert_eq!(h.n_vertices, 1);
        assert_eq!(h.hyperedges, vec![1]);
        assert_eq!(h.weights, vec![NonZeroU32::new(1).unwrap()]);
    }

    #[test]
    fn test_binomial_2() {
        let h = binomial::<u32>(4, 2);
        assert_eq!(h.n_vertices, 4);
        assert_eq!(
            h.hyperedges,
            vec![0b0011, 0b0101, 0b0110, 0b1001, 0b1010, 0b1100]
        );
        assert!(h.weights.iter().all(|i| i.get() == 2))
    }

    #[test]
    fn test_binomial_small() {
        for n in 3..=11usize {
            for k in 1..=n {
                let h = binomial::<u8>(n, k);
                assert_eq!(h.n_vertices, n as u32);
                assert!(h.weights.iter().all(|i| i.get() == k as u32));

                let n_choose_k: usize =
                    (n - k + 1..=n).product::<usize>() / (1..=k).product::<usize>();
                assert_eq!(h.hyperedges.len() / h.chunk_size, n_choose_k);
            }
        }
    }

    #[test]
    fn test_binomial_50() {
        let h = binomial::<u32>(50, 5);
        assert_eq!(h.n_vertices, 50);
        assert_eq!(h.hyperedges.len() / h.chunk_size, 2118760);
        assert!(h.weights.iter().all(|i| i.get() == 5));
    }
}
