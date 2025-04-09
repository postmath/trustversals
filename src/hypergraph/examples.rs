use super::*;

#[inline]
fn mask_complement<I>(mask: I) -> I
where
    I: HNumber,
{
    I::max_value() ^ mask
}

/// An I where the m rightmost bits are zero, then n bits are one, and then the rest is zero.
#[inline]
fn mask_ones_zeroes_right<I>(n: u32, m: u32) -> I
where
    I: HNumber,
{
    let bit_size = (std::mem::size_of::<I>() * 8) as u32;

    if n + m == bit_size {
        if m == bit_size {
            I::zero()
        } else {
            mask_zeroes_right(m)
        }
    } else {
        let n_usize = n as usize;
        let m_usize = m as usize;
        (I::one() << (n_usize + m_usize)) - (I::one() << m_usize)
    }
}

/// An I where the m rightmost bits are one, then n bits are zero, then the rest is one.
#[inline]
fn mask_zeroes_ones_right<I>(n: u32, m: u32) -> I
where
    I: HNumber,
{
    mask_complement(mask_ones_zeroes_right(n, m))
}

/// An I where the n rightmost bits are one and all others are zero.
#[inline]
fn mask_ones_right<I>(n: u32) -> I
where
    I: HNumber,
{
    mask_ones_zeroes_right(n, 0)
}

/// An I where the n rightmost bits are zero and all others are one.
#[inline]
fn mask_zeroes_right<I>(n: u32) -> I
where
    I: HNumber,
{
    mask_complement(mask_ones_right(n))
}

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
        state[num_words - num_one_words..num_words].fill(I::max_value());
        let num_one_bits = k % bit_size_u;
        if num_one_bits > 0 {
            state[num_words - num_one_words - 1] = (I::one() << num_one_bits) - I::one();
        }

        Self {
            state,
            bits_in_top_word,
        }
    }

    fn trailing_ones_and_zeroes(&self) -> Option<(u32, u32)> {
        let bit_size = (std::mem::size_of::<I>() * 8) as u32;

        let mut idx = self.state.len() - 1;
        let mut zeroes_here = self.state[idx].trailing_zeros();
        let mut zeroes = zeroes_here;

        while zeroes_here == bit_size && idx > 0 {
            idx -= 1;
            zeroes_here = self.state[idx].trailing_zeros();
            zeroes += zeroes_here;
        }

        if idx == 0 && zeroes_here == bit_size {
            // The value contained is all zeroes.
            return None;
        }

        let next_word = self.state[idx] >> (zeroes_here as usize);
        let mut ones_here = next_word.trailing_ones();
        let mut ones = ones_here;

        if idx == 0 {
            return Some((ones, zeroes));
        } else if ones_here + zeroes_here == bit_size {
            // Now we know that idx > 0; we need to examine the next word.
            idx -= 1;
            ones_here = self.state[idx].trailing_ones();
            ones += ones_here;
            while ones_here == bit_size && idx > 0 {
                idx -= 1;
                ones_here = self.state[idx].trailing_ones();
                ones += ones_here;
            }
        }

        Some((ones, zeroes))
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
            let mut idx = self.state.len() - 1;
            let mut ones_to_set = m - 1;
            while ones_to_set >= bit_size {
                self.state[idx] = I::max_value();
                ones_to_set -= bit_size;
                idx -= 1;
            }
            self.state[idx] = self.state[idx] | mask_ones_right(ones_to_set);
            // Now 'ones_to_set' is the number of ones set in this last word.

            // Set n+1 zero bits in the middle
            let mut zeroes_to_set = n + 1;
            if ones_to_set + zeroes_to_set <= bit_size {
                self.state[idx] =
                    self.state[idx] & mask_zeroes_ones_right(zeroes_to_set, ones_to_set);
            } else {
                self.state[idx] = self.state[idx] & mask_ones_right(ones_to_set);
                zeroes_to_set -= bit_size - ones_to_set;
                idx -= 1;
                while zeroes_to_set >= bit_size {
                    self.state[idx] = I::zero();
                    zeroes_to_set -= bit_size;
                    idx -= 1;
                }
                self.state[idx] = self.state[idx] & mask_zeroes_right(zeroes_to_set);
            }

            // Set the one bit on the left
            let word = self.state.len() - 1 - ((m + n) / bit_size) as usize;
            let bit = (m + n) % bit_size;
            self.state[word] = self.state[word] | (I::one() << (bit as usize));

            true
        } else {
            false
        }
    }
}

/// Creates and returns the hypergraph on `n` vertices, with all hyperedges of cardinality `k`. This
/// panics if we don't have `n >= k > 0`.
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

/// A bit position within a slice of words. `word` indicates which word we're referring to and `bit`
/// indicates which bit in that word we're referring to.
#[derive(Debug)]
struct BitPosition {
    word: usize,
    bit: u32,
}

impl BitPosition {
    /// Returns a vector of BitPosition structs such that the distance between the first two is 1, then 2, then 3, etc., up to and including an interval of length
    /// `r` bits. Each word has `bit_size` bits. The last position is at bit 0 of a word. The first position is in word 0.
    pub fn increasing(r: usize, bit_size: u32) -> Vec<Self> {
        let bit_size_usize = bit_size as usize;
        let bits_needed = r * (r + 1) / 2;
        let words_needed = if bits_needed % (bit_size_usize) == 0 {
            bits_needed / bit_size_usize
        } else {
            bits_needed / bit_size_usize + 1
        };
        (1u32..=(r as u32))
            .chain(std::iter::once(0))
            .rev()
            .scan(
                BitPosition {
                    word: words_needed - 1,
                    bit: 0u32,
                },
                |state, value| {
                    state.bit += value;
                    state.word -= (state.bit / bit_size) as usize;
                    state.bit %= bit_size;
                    Some(BitPosition {
                        bit: state.bit,
                        word: state.word,
                    })
                },
            )
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

/// Creates the Lovasz hypergraph of order `r`. Its vertices are divided into groups of size 1, 2,
/// ..., `r`. The hyperedges, all of cardinality `r`, are all subsets of the following form: for
/// some `i` with `0 < i <= r`, they consist of all vertices in group `i`, together with one vertex
/// each out of groups `i + 1 ..= r`.
pub fn lovasz<I: HNumber>(r: usize) -> Hypergraph<I> {
    // We need to mark out groups of 1, 2, 3, ..., r bits. We do this with a vector of r+1
    // BitPositions such that the ith group occurs between groups[i] and groups[i+1].

    let bit_size_u = std::mem::size_of::<I>() * 8;
    let bit_size = bit_size_u as u32;

    let groups = BitPosition::increasing(r, bit_size);

    let top_position = &groups[0];
    let bits_in_top_word = top_position.bit;
    let (num_words, total_bits) = if bits_in_top_word == 0 {
        (groups[r].word, groups[r].word as u32 * bit_size)
    } else {
        (
            groups[r].word + 1,
            groups[r].word as u32 * bit_size + bits_in_top_word,
        )
    };
    let mut state = vec![I::zero(); num_words];
    // We use metastate to keep track of which bit is set in which group: metastate[i] = m for i > k
    // means that of the bits in group i, only bit m is set.
    let mut metastate = vec![0u32; r];

    let mut h = Hypergraph::<I>::new(total_bits);

    'outer_loop: for k in (0..r).rev() {
        // All bits in groups 0 .. k are currently unset. We set all bits in group k and for all
        // higher groups, initially set the rightmost bit in each group and unset all lower bits.

        // Step 1: clear all bits in group k+1 and beyond (including potentially some or all in group k).
        state[groups[k + 1].word..].fill(I::zero());

        // Step 2: set all bits in group k.
        if groups[k].word == groups[k + 1].word {
            // Group k falls within one word.
            state[groups[k].word] =
                mask_ones_zeroes_right(groups[k].bit - groups[k + 1].bit, groups[k + 1].bit)
        } else {
            state[groups[k + 1].word] = mask_zeroes_right(groups[k + 1].bit);
            state[groups[k].word + 1..groups[k + 1].word].fill(mask_ones_right(bit_size));
            state[groups[k].word] = mask_ones_right(groups[k].bit);
        }

        // Step 3: for groups k + 1 .. r, set the rightmost bit in that group.
        for m in k + 1..r {
            metastate[m] = 0;
            let BitPosition { word, bit } = &groups[m + 1];
            state[*word] = state[*word] | I::one() << (*bit as usize);
        }

        // Now the first hyperedge for this value of `k` is correctly initialized. Iterate through
        // the others in lexicographic order: in each iteration, find the last possible entry of
        // metastate to increase, then set each entry after that to 1.
        loop {
            h.add_edge(&state);

            // Find which metastate entry to increase.
            let mut to_increase = r - 1;
            loop {
                if to_increase == k {
                    // All metastate entries are at their maximal value, so we need to continue to
                    // the next iteration of the outer loop.
                    continue 'outer_loop;
                } else if (metastate[to_increase] as usize) < to_increase {
                    break;
                } else {
                    to_increase -= 1;
                }
            }

            metastate[to_increase] += 1;

            // Clear all bits of group to_increase and further on, but be careful to leave groups
            // to_increase - 1 and before intact.
            state[groups[to_increase].word + 1..].fill(I::zero());
            state[groups[to_increase].word] =
                state[groups[to_increase].word] & mask_zeroes_right(groups[to_increase].bit);

            let bit_index = groups[to_increase + 1].bit + metastate[to_increase];
            let word = groups[to_increase + 1].word - (bit_index / bit_size) as usize;
            let bit_index = bit_index % bit_size;
            state[word] = state[word] | I::one() << (bit_index as usize);

            for m in to_increase + 1..r {
                metastate[m] = 0;
                let BitPosition { word, bit } = &groups[m + 1];
                state[*word] = state[*word] | I::one() << (*bit as usize);
            }
        }
    }

    h
}

mod tests {
    use super::*;

    #[test]
    fn test_binomial_1() {
        let h = binomial::<u32>(1, 1);
        assert_eq!(h.n_vertices, 1);
        assert_eq!(h.hyperedges, vec![1]);
        assert!(h.weights.iter().all(|i| i.get() == 1));
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

    #[test]
    fn test_lovasz_1() {
        let h = lovasz::<u32>(1);
        assert_eq!(h.n_vertices, 1);
        assert_eq!(h.hyperedges, vec![1]);
        assert!(h.weights.iter().all(|i| i.get() == 1));
    }

    #[test]
    fn test_lovasz_2() {
        let h = lovasz::<u32>(2);
        assert_eq!(h.n_vertices, 3);
        let mut hs = h.hyperedges.clone();
        hs.sort();
        assert_eq!(hs, vec![3, 5, 6]);
        assert!(h.weights.iter().all(|i| i.get() == 2));
    }

    #[test]
    fn test_lovasz_4() {
        let h = lovasz::<u8>(4);
        assert_eq!(h.n_vertices, 10);

        #[rustfmt::skip]
        #[allow(clippy::unusual_byte_groupings)]
        let expected = vec![
            0b0_0, 0b0_000_1111,

            0b0_0, 0b0_111_0001,
            0b0_0, 0b0_111_0010,
            0b0_0, 0b0_111_0100,
            0b0_0, 0b0_111_1000,

            0b0_1, 0b1_001_0001,
            0b0_1, 0b1_001_0010,
            0b0_1, 0b1_001_0100,
            0b0_1, 0b1_001_1000,
            0b0_1, 0b1_010_0001,
            0b0_1, 0b1_010_0010,
            0b0_1, 0b1_010_0100,
            0b0_1, 0b1_010_1000,
            0b0_1, 0b1_100_0001,
            0b0_1, 0b1_100_0010,
            0b0_1, 0b1_100_0100,
            0b0_1, 0b1_100_1000,

            0b1_0, 0b1_001_0001,
            0b1_0, 0b1_001_0010,
            0b1_0, 0b1_001_0100,
            0b1_0, 0b1_001_1000,
            0b1_0, 0b1_010_0001,
            0b1_0, 0b1_010_0010,
            0b1_0, 0b1_010_0100,
            0b1_0, 0b1_010_1000,
            0b1_0, 0b1_100_0001,
            0b1_0, 0b1_100_0010,
            0b1_0, 0b1_100_0100,
            0b1_0, 0b1_100_1000,
            0b1_1, 0b0_001_0001,
            0b1_1, 0b0_001_0010,
            0b1_1, 0b0_001_0100,
            0b1_1, 0b0_001_1000,
            0b1_1, 0b0_010_0001,
            0b1_1, 0b0_010_0010,
            0b1_1, 0b0_010_0100,
            0b1_1, 0b0_010_1000,
            0b1_1, 0b0_100_0001,
            0b1_1, 0b0_100_0010,
            0b1_1, 0b0_100_0100,
            0b1_1, 0b0_100_1000,
        ];
        assert_eq!(h.hyperedges, expected);

        assert!(h.weights.iter().all(|i| i.get() == 4));
    }

    #[test]
    fn test_lovasz_10() {
        let h = lovasz::<u32>(10);

        // Check the number of vertices
        // The number of vertices is the sum of the first 10 natural numbers: 1 + 2 + ... + 10
        let expected_vertices = (10 * (10 + 1)) / 2;
        assert_eq!(h.n_vertices, expected_vertices);

        // Check the number of hyperedges. This is given by the recursion a_n = n*a_{n-1} + 1, a_0 = 0.
        let expected_hyperedges = 6235301;
        assert_eq!(h.hyperedges.len() / h.chunk_size, expected_hyperedges);

        // Check that all hyperedges have a weight of 10
        assert!(h.weights.iter().all(|i| i.get() == 10));
    }
}
