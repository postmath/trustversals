use crate::numbers::HNumber;
use std::num::NonZeroU32;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Hyperedge<'a, I>
where
    I: HNumber,
{
    pub edge: &'a [I],
    pub weight: NonZeroU32,
}

impl<'a, I> Hyperedge<'a, I>
where
    I: HNumber,
{
    pub fn new(edge: &'a [I], weight: NonZeroU32) -> Self {
        Self { edge, weight }
    }
}

impl<'a, I> PartialOrd for Hyperedge<'a, I>
where
    I: HNumber,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let cmp_result = self.weight.cmp(&other.weight);
        match cmp_result {
            core::cmp::Ordering::Equal => Some(self.edge.cmp(&other.edge)),
            _ => Some(cmp_result),
        }
    }
}
impl<'a, I> Ord for Hyperedge<'a, I>
where
    I: HNumber,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let cmp_result = self.weight.cmp(&other.weight);
        match cmp_result {
            core::cmp::Ordering::Equal => self.edge.cmp(&other.edge),
            _ => cmp_result,
        }
    }
}

/// A bit position within a slice of words. `word` indicates which word we're referring to and `bit`
/// indicates which bit in that word we're referring to.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct BitPosition {
    pub word: usize,
    pub bit: u32,
}

/// Ordering for BitPosition is such that positions with a lower word number are considered
/// "greater" (i.e., they come last in a sorted list). If the word numbers are equal, then the
/// bit numbers are compared in the usual way (lower bit number is "lesser"). This causes the
/// corresponding singleton edges to be ordered lexicographically.
impl PartialOrd for BitPosition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.word == other.word {
            Some(self.bit.cmp(&other.bit))
        } else {
            Some(other.word.cmp(&self.word))
        }
    }
}

impl Ord for BitPosition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
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
