pub mod examples;
pub mod transversals;

use crate::hyperedge::BitPosition;
use crate::numbers::HNumber;
use std::cmp::Ordering;
use std::num::NonZeroU32;

/// A hypergraph with `n_vertices` vertices and `len(weights)` hyperedges. Each hyperedge is
/// described as a bitmap of `chunk_size` entries of `hyperedges`; the ith hyperedge consists of
/// `hyperedges[i * chunk_size .. (i+1) * chunk_size]`. The presence of vertex 0 in this hyperedge
/// is indicated by the lowest bit of `hyperedges[(i+1) * chunk_size - 1]`. The empty hyperedge is not
/// allowed. The ith hyperedge has weight (size) `weights[i]`.
pub struct Hypergraph<I>
where
    I: HNumber,
{
    n_vertices: u32,
    chunk_size: usize,
    hyperedges: Vec<I>,
    weights: Vec<NonZeroU32>,
}

impl<I> Hypergraph<I>
where
    I: HNumber,
{
    /// Create an empty hypergraph on `n_vertices` vertices.
    pub fn new(n_vertices: u32) -> Self {
        let bit_size = std::mem::size_of::<I>() * 8;
        let n_v = n_vertices as usize;
        let cs = n_v / bit_size + ((n_v % bit_size != 0) as usize);
        Self {
            n_vertices,
            chunk_size: cs,
            hyperedges: vec![],
            weights: vec![],
        }
    }

    /// Add a hyperedge to the hypergraph. The hyperedge is given by the slice `edge`, which must
    /// consist of `chunk_size` entries (this code will panic otherwise).
    pub fn add_edge(&mut self, edge: &[I]) {
        if edge.len() != self.chunk_size {
            panic!(
                "received slice of length {0} while chunk size is {1}",
                edge.len(),
                self.chunk_size
            );
        }

        let weight: u32 = edge.iter().map(|x| x.count_ones()).sum();
        self.weights
            .push(NonZeroU32::new(weight).expect("cannot add the empty edge"));
        self.hyperedges.extend_from_slice(edge);
    }

    /// Sort the hyperedges in order of increasing weight, and within a weight class, sort the
    /// hyperedges lexicographically (meaning, by the first bit, then by the second bit, etc). This
    /// returns a vector `positions` such that the hyperedges of weight `i+1` are found in the
    /// range `hyperedges[positions[i]..positions[i+1]]`.
    fn bucket_sort_weights_external(
        hyperedges: &mut [I],
        weights: &mut [NonZeroU32],
        chunk_size: usize,
        n_vertices: u32,
    ) -> Vec<usize> {
        // We want weight_count[i] to be the number of hyperedges of weight i+1. weight_count has
        // n_vertices entries, so that positions has the expected number of entries.
        let mut weight_count = vec![0; n_vertices as usize];
        for w in weights.iter() {
            weight_count[w.get() as usize - 1] += 1;
        }

        // We want positions[i-1] .. positions[i] to be the index range where hyperedges of weight
        // i+1 will be found in the sorted order, with positions[-1] interpreted as 0. (This is a
        // consequence of rust's scan function being inclusive; with an exclusive scan, we would
        // have had positions[i] .. positions[i+1].)
        let mut positions: Vec<usize> = weight_count
            .into_iter()
            .scan(0, |state, x| {
                *state += x;
                Some(*state)
            })
            .collect();

        // Now we scatter self.hyperedges into new_hyperedges according to positions. We fill each
        // segment positions[i-1] .. positions[i] from the end, decreasing positions[i] before
        // writing to that index. That means that we end up with the reverse: after the loop,
        // positions[i] .. positions[i+1] will contain the hyperedges of weight i, where we
        // interpret positions[self.n_vertices] as self.weights.len().
        let mut new_hyperedges = vec![I::zero(); hyperedges.len()];
        for i in 0..weights.len() {
            let w = (weights[i].get() - 1) as usize;
            positions[w] -= 1;
            let p = positions[w];
            new_hyperedges[p * chunk_size..(p + 1) * chunk_size]
                .copy_from_slice(&hyperedges[i * chunk_size..(i + 1) * chunk_size]);
        }
        hyperedges.copy_from_slice(&new_hyperedges);

        // We add a final entry so that this works for each of the subranges.
        positions.push(weights.len());

        // Now let's update the weights.
        for i in 0..n_vertices {
            let j = i as usize;
            weights[positions[j]..positions[j + 1]].fill(NonZeroU32::new(i + 1).unwrap());
        }

        positions
    }

    /// Sort the hypergraph that would be given by the given vectors. This returns a vector
    /// `positions` such that the hyperedges of weight `i+1` are found in the range
    /// `hyperedges[positions[i]..positions[i+1]]`.
    pub fn sort_external(
        hyperedges: &mut [I],
        weights: &mut [NonZeroU32],
        chunk_size: usize,
        n_vertices: u32,
    ) -> Vec<usize> {
        // We can't easily use a "canned" sorting algorithm, because we need to sort hyperedges and
        // weights in tandem. But we can do a bucket sort by weight: the possible weights range from
        // 1 to n_vertices, which we should be willing to store.

        let positions =
            Hypergraph::bucket_sort_weights_external(hyperedges, weights, chunk_size, n_vertices);

        // Finally, we sort each of the segments of equal weight.
        for i in 0..positions.len() - 1 {
            sort_slice_of_chunks(
                &mut hyperedges[positions[i] * chunk_size..positions[i + 1] * chunk_size],
                chunk_size,
            );
        }

        assert!(Hypergraph::is_sorted_external(
            hyperedges, weights, chunk_size
        ));
        positions
    }

    /// Sort the hyperedges in order of increasing weight, and within a weight class, sort the
    /// hyperedges by increasing value.
    pub fn sort(&mut self) {
        // We can't easily use a "canned" sorting algorithm, because we need to sort hyperedges and
        // weights in tandem. But we can do a bucket sort by weight: the possible weights range from
        // 1 to n_vertices, which we should be willing to store.

        let Hypergraph {
            ref mut hyperedges,
            ref mut weights,
            ..
        } = self;
        Hypergraph::sort_external(hyperedges, weights, self.chunk_size, self.n_vertices);
    }

    /// Check whether the hypergraph given by the provided vectors is sorted in order of increasing
    /// weight, and within a weight class, sorted by increasing edge value.
    pub fn is_sorted_external(hyperedges: &[I], weights: &[NonZeroU32], chunk_size: usize) -> bool {
        for i in 1..weights.len() {
            if weights[i] < weights[i - 1] {
                return false;
            } else if weights[i] == weights[i - 1] {
                if compare_chunks(hyperedges, chunk_size, i - 1, i) == Ordering::Greater {
                    return false;
                }
            }
        }
        true
    }

    /// Check whether the hypergraph is sorted in order of increasing weight, and within a weight
    /// class, sorted by increasing edge value.
    pub fn is_sorted(&self) -> bool {
        Hypergraph::is_sorted_external(&self.hyperedges, &self.weights, self.chunk_size)
    }
}

impl<I> Hypergraph<I>
where
    I: std::fmt::Binary + HNumber,
{
    pub fn print_edges(&self) {
        self.print_edges_external(&*self.hyperedges);
    }

    pub fn print_edges_external(&self, hyperedges: &[I]) {
        let mut i = 0;
        for e in hyperedges {
            if i == self.chunk_size - 1 {
                eprintln!("{0:01$b}", e, std::mem::size_of::<I>() * 8);
            } else {
                eprint!("{0:01$b} ", e, std::mem::size_of::<I>() * 8);
            }
            i = (i + 1) % self.chunk_size;
        }
    }
}

fn compare_chunks<I>(chunks: &[I], chunk_size: usize, a: usize, b: usize) -> Ordering
where
    I: HNumber,
{
    for i in 0..chunk_size {
        let ord = chunks[a * chunk_size + i].cmp(&chunks[b * chunk_size + i]);
        match ord {
            Ordering::Less | Ordering::Greater => return ord,
            Ordering::Equal => {}
        }
    }
    Ordering::Equal
}

fn sort_slice_of_chunks<I>(chunks: &mut [I], chunk_size: usize)
where
    I: HNumber,
{
    let n_slices = chunks.len() / chunk_size;
    let mut indices: Vec<_> = (0..n_slices).collect();
    indices.sort_unstable_by(|&a, &b| compare_chunks(chunks, chunk_size, a, b));

    let new_order: Vec<I> = indices
        .into_iter()
        .flat_map(|i| (i * chunk_size..(i + 1) * chunk_size).map(|i| chunks[i]))
        .collect();

    for (dest, src) in chunks.iter_mut().zip(new_order) {
        *dest = src;
    }
}

pub(crate) fn nz_one() -> NonZeroU32 {
    NonZeroU32::new(1).expect("One should always be non-zero")
}
pub(crate) fn nz_two() -> NonZeroU32 {
    NonZeroU32::new(2).expect("Two should always be non-zero")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_chunk_size() {
        let h0 = Hypergraph::<u64>::new(0);
        assert_eq!(h0.chunk_size, 0);

        let h1 = Hypergraph::<u16>::new(10);
        assert_eq!(h1.chunk_size, 1);

        let h2 = Hypergraph::<u64>::new(1025);
        assert_eq!(h2.chunk_size, 17);
    }

    #[test]
    fn add_some_edges() {
        let mut h0 = Hypergraph::<u32>::new(50);
        assert_eq!(h0.chunk_size, 2);

        let e00 = (1u32 << 30) + (1u32 << 15) + (1u32 << 3);
        let e01 = (1u32 << 14) + (1u32 << 12) + (1u32 << 4);
        let e0 = vec![e00, e01];
        h0.add_edge(&e0);
        assert_eq!(h0.hyperedges.len(), h0.chunk_size);
        assert_eq!(h0.hyperedges[0], e00);
        assert_eq!(h0.hyperedges[1], e01);
        assert_eq!(h0.weights.len(), 1);
        assert_eq!(h0.weights[0], NonZeroU32::new(6).unwrap());
    }

    #[test]
    #[should_panic(expected = "received slice of length 1 while chunk size is 2")]
    fn add_wrong_size_chunk() {
        let mut h0 = Hypergraph::<u32>::new(50);
        assert_eq!(h0.chunk_size, 2);

        let e00 = (1u32 << 30) + (1u32 << 15) + (1u32 << 3);
        let e0 = vec![e00];
        h0.add_edge(&e0);
    }

    #[test]
    #[should_panic(expected = "cannot add the empty edge")]
    fn add_empty_edge() {
        let mut h0 = Hypergraph::<u32>::new(50);
        assert_eq!(h0.chunk_size, 2);

        let e0 = vec![0, 0];
        h0.add_edge(&e0);
    }

    #[test]
    fn test_sort_slice_of_chunks() {
        let mut v: Vec<u32> = vec![1, 2, 4, 1, 2, 3, 2, 2, 1];
        sort_slice_of_chunks(&mut v, 3);
        assert_eq!(v, vec![1, 2, 3, 1, 2, 4, 2, 2, 1]);
    }

    #[test]
    fn test_sort() {
        let mut h = Hypergraph::<u32>::new(5);
        h.add_edge(&[14]); // 1-2-3
        h.add_edge(&[5]); // 0-2
        h.add_edge(&[3]); // 0-1
        h.add_edge(&[16]); // 4

        assert_eq!(h.hyperedges, vec![14, 5, 3, 16]);
        let expected_weights: Vec<_> = (vec![3, 2, 2, 1])
            .into_iter()
            .map(|i| NonZeroU32::new(i).unwrap())
            .collect();
        assert_eq!(h.weights, expected_weights);

        h.sort();

        assert_eq!(h.hyperedges, vec![16, 3, 5, 14]);
        let expected_weights: Vec<_> = (vec![1, 2, 2, 3])
            .into_iter()
            .map(|i| NonZeroU32::new(i).unwrap())
            .collect();
        assert_eq!(h.weights, expected_weights);
    }
}
