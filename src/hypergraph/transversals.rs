use std::f32::consts::E;

use crate::hyperedge::{BitPosition, Hyperedge};
use crate::hypergraph::{nz_one, nz_two, Hypergraph, NonZeroU32};
use crate::numbers::HNumber;

impl<I> super::Hypergraph<I>
where
    I: HNumber,
{
    /// Create the transversal hypergraph of self.
    ///
    /// A transversal of a hypergraph H is a subset of the vertices that has a nonempty
    /// intersection with each hyperedge. A minimal transversal is a transversal of which no proper
    /// subset is a transversal. The transversal hypergraph of H is the hypergraph with the same
    /// vertex set as H, and the set of all minimal transversals of H as its set of hyperedges.
    ///
    /// We can define the set of hyperedges of a transversal hypergraph recursively, as follows.
    /// Consider hypergraphs on a fixed vertex set V.
    /// * The transversal hypergraph of the empty hypergraph (with no hyperedges) is the empty
    ///   hypergraph.
    /// * The transversal hypergraph of the hypergraph with one hyperedge {v1, .., vn} has
    ///   hyperedges {v1}, ..., {vn}.
    /// * Let H be a hypergraph with set of hyperedges E1 U E2 (a disjoint union). Let T1 be the
    ///   set of hyperedges of the transversal hypergraph of (V, E1) and T2 the set of hyperedges
    ///   of the transversal hypergraph of (V, E2). Then the transversal hypergraph of H consists
    ///   of all inclusionwise minimal elements of the pairwise unions of elements of T1 with
    ///   elements of T2.
    ///
    /// We will specialize out the case of two hyperedges. Let H have two hyperedges,
    /// e1 := {u1, ..., uk, v1, ..., vm} and e2 := {v1, ..., vm, w1, ..., wn}, where distinct
    /// symbols are distinct vertices. The transversals of H1 := (V, {e1}) and H2 := (V, {e2})
    /// consist of the singletons of e1 and of e2, respectively. Their pairwise unions are the sets
    /// {ui, vj}, {ui, wj}, {vi, vj}, {vi, vi} = {vi}, and {vi, wj}. The inclusionwise minimal sets
    /// among these are {ui, wj} and {vi}.
    pub fn transversal_hypergraph(&self) -> Self {
        let mut transversal_edges = Vec::<I>::new();
        let mut transversal_weights = Vec::<NonZeroU32>::new();

        self.make_transversal(
            &self.hyperedges,
            &mut transversal_edges,
            &mut transversal_weights,
        );
        Self {
            n_vertices: self.n_vertices,
            chunk_size: self.chunk_size,
            hyperedges: transversal_edges,
            weights: transversal_weights,
        }
    }

    fn make_transversal(
        &self,
        hyperedges: &[I],
        transversal_edges: &mut Vec<I>,
        transversal_weights: &mut Vec<NonZeroU32>,
    ) {
        match hyperedges.len() / self.chunk_size {
            0 => {
                // No hyperedges, no transversals.
                return;
            }
            1 => {
                // One hyperedge, all singletons are transversals.
                self.make_singleton_transversals(
                    hyperedges,
                    transversal_edges,
                    transversal_weights,
                );
            }
            2 => {
                // Two hyperedges, we can compute the transversals directly.
                self.make_two_hyperedge_transversals(
                    hyperedges,
                    transversal_edges,
                    transversal_weights,
                );
            }
            _ => {
                // More than two hyperedges, we need to recurse.
                self.make_n_hyperedge_transversals(
                    hyperedges,
                    transversal_edges,
                    transversal_weights,
                );
            }
        }
    }

    fn make_n_hyperedge_transversals(
        &self,
        hyperedges: &[I],
        transversal_edges: &mut Vec<I>,
        transversal_weights: &mut Vec<std::num::NonZero<u32>>,
    ) {
        let n = hyperedges.len() / self.chunk_size;
        let k = n / 2;
        let (left, right) = hyperedges.split_at(k * self.chunk_size);
        let (mut left_edges, mut left_weights) = (Vec::<I>::new(), Vec::<NonZeroU32>::new());
        let (mut right_edges, mut right_weights) = (Vec::<I>::new(), Vec::<NonZeroU32>::new());
        self.make_transversal(left, &mut left_edges, &mut left_weights);
        self.make_transversal(right, &mut right_edges, &mut right_weights);
        self.merge_edge_sets_into(
            transversal_edges,
            transversal_weights,
            &left_edges,
            &left_weights,
            &right_edges,
            &right_weights,
        );
    }

    /// Given two (sorted) sets of hyperedges, left and right, this generates the (sorted) set of
    /// union-wise minimal unions between one left and one right hyperedge. That is, first generate
    /// the set of {l ∪ r | l in left, r in right}, then remove all hyperedges h such that there is
    /// another hyperedge h' with h' ⊂ h.
    ///
    /// We assume that left and right, themselves, are inclusion-free, that is, there are no l, l'
    /// in left with l ⊂ l' and no r, r' in right with r ⊂ r'.
    ///
    /// In order to generate the edges in near-to-sorted order, we maintain a cursor c_left into
    /// left and a cursor c_right into right, in such a way that we pass all hyperedges in order:
    /// we always increase the cursor pointing to the smallest of the two hyperedges pointed to.
    /// When we process such a hyperedge, say l in left, we add l ∪ r for all r in right *strictly
    /// before* c_right to the output. Thus, every pair (l, r) is considered when the greatest of l
    /// and r is reached by its cursor. Note that this doesn't exactly generate the unions in sorted
    /// order: the cardinality of the unions may increase and decrease.
    ///
    /// We then do a pass, first to sort the results, then to filter out any hyperedges that
    /// contain another hyperedge.
    ///
    /// **Future improvement:**
    /// If any l in left is contained in any r in right, then l ∪ r = r. Moreover, no other union
    /// can be a proper subset of r, since that would require the right part of the union to be a
    /// proper subset of r, which is impossible since right is inclusion-free. Moreover, the union
    /// of l with any r' in right will have r as its subset. We could therefore maintain a bitset
    /// keeping track of which hyperedges occur in such an inclusion, and skip them in the inner
    /// loop.
    fn merge_edge_sets_into(
        &self,
        target_edges: &mut Vec<I>,
        target_weights: &mut Vec<NonZeroU32>,
        left_edges: &[I],
        left_weights: &[NonZeroU32],
        right_edges: &[I],
        right_weights: &[NonZeroU32],
    ) {
        let n_left = left_weights.len();
        let n_right = right_weights.len();
        let mut li = 0usize;
        let mut ri = 0usize;
        let mut left_edge = Hyperedge::new(&left_edges[0..self.chunk_size], left_weights[0]);
        let mut right_edge = Hyperedge::new(&right_edges[0..self.chunk_size], right_weights[0]);
        while li < n_left || ri < n_right {
            if ri == n_right || (li < n_left && left_edge < right_edge) {
                // process left_edge and advance li
                for rj in 0..ri {
                    // Add the union of left_edge with right_edges[rj]
                    for c in 0..self.chunk_size {
                        target_edges
                            .push(left_edge.edge[c] | right_edges[rj * self.chunk_size + c]);
                    }
                    let new_edge = &target_edges[(target_edges.len() - self.chunk_size)..];
                    let weight: u32 = new_edge.iter().map(|x| x.count_ones()).sum();
                    target_weights.push(
                        NonZeroU32::new(weight)
                            .expect("the union of nonempty edges shouldn't be empty"),
                    );
                }

                li += 1;
                if (li < n_left) {
                    left_edge.edge = &left_edges[li * self.chunk_size..(li + 1) * self.chunk_size];
                    left_edge.weight = left_weights[li];
                }
            } else if li == n_left || right_edge < left_edge {
                // process right_edge and advance ri
                for lj in 0..li {
                    // Add the union of right_edge with left_edges[lj]
                    for c in 0..self.chunk_size {
                        target_edges
                            .push(right_edge.edge[c] | left_edges[lj * self.chunk_size + c]);
                    }
                    let new_edge = &target_edges[(target_edges.len() - self.chunk_size)..];
                    let weight: u32 = new_edge.iter().map(|x| x.count_ones()).sum();
                    target_weights.push(
                        NonZeroU32::new(weight)
                            .expect("the union of nonempty edges shouldn't be empty"),
                    );
                }

                ri += 1;
                if (ri < n_right) {
                    right_edge.edge =
                        &right_edges[ri * self.chunk_size..(ri + 1) * self.chunk_size];
                    right_edge.weight = right_weights[ri];
                }
            } else {
                // left_edge == right_edge; their union is just one of them, so add it once and advance both cursors.
                for c in 0..self.chunk_size {
                    target_edges.push(left_edges[li * self.chunk_size + c]);
                }
                target_weights.push(left_weights[li]);

                li += 1;
                if (li < n_left) {
                    left_edge.edge = &left_edges[li * self.chunk_size..(li + 1) * self.chunk_size];
                    left_edge.weight = left_weights[li];
                }
                ri += 1;
                if (ri < n_right) {
                    right_edge.edge =
                        &right_edges[ri * self.chunk_size..(ri + 1) * self.chunk_size];
                    right_edge.weight = right_weights[ri];
                }
            }
        }

        let positions = Hypergraph::sort_external(
            target_edges,
            target_weights,
            self.chunk_size,
            self.n_vertices,
        );

        // Finally, we do two passes to remove any hyperedge that contains another hyperedge. In
        // both, we copy entries from read_index to write_index if they need to be kept. First we
        // remove any duplicate edges.
        //
        // Note that we always have write_index <= read_index.
        let mut write_index = 1usize;
        for read_index in 1..target_weights.len() {
            if target_edges[read_index * self.chunk_size..(read_index + 1) * self.chunk_size]
                != target_edges[(read_index - 1) * self.chunk_size..read_index * self.chunk_size]
            {
                // Not a duplicate, keep it.
                if write_index != read_index {
                    for c in 0..self.chunk_size {
                        target_edges[write_index * self.chunk_size + c] =
                            target_edges[read_index * self.chunk_size + c];
                    }
                    target_weights[write_index] = target_weights[read_index];
                }
                write_index += 1;
            }
        }

        target_edges.truncate(write_index * self.chunk_size);
        target_weights.truncate(write_index);

        // Now we remove entries where there is an edge that is a proper subset. Such an edge is
        // necessarily of lower weight. For each edge, we run check_index over the indices of
        // lower weight edges.
        write_index = 0usize;
        'outer_lower_weight: for read_index in write_index..target_weights.len() {
            {
                let read_edge =
                    &target_edges[read_index * self.chunk_size..(read_index + 1) * self.chunk_size];
                let read_weight = target_weights[read_index].get();
                for check_index in 0..write_index {
                    if target_weights[check_index].get() == read_weight {
                        // We have reached the weight class of read_edge; no need to check further.
                        break;
                    }
                    let check_edge = &target_edges
                        [check_index * self.chunk_size..(check_index + 1) * self.chunk_size];
                    if read_edge
                        .iter()
                        .zip(check_edge.iter())
                        .all(|(r, c)| *r & *c == *c)
                    {
                        // read_edge contains check_edge, so skip read_edge
                        continue 'outer_lower_weight;
                    }
                }
            }

            // We didn't find any hyperedge contained in read_edge, so we keep it.
            if write_index != read_index {
                for c in 0..self.chunk_size {
                    target_edges[write_index * self.chunk_size + c] =
                        target_edges[read_index * self.chunk_size + c];
                }
                target_weights[write_index] = target_weights[read_index];
            }
            write_index += 1;
        }

        target_edges.truncate(write_index * self.chunk_size);
        target_weights.truncate(write_index);
    }

    /// Create the transversal hypergraph of a single hyperedge.
    fn make_singleton_transversals(
        &self,
        hyperedges: &[I],
        transversal_edges: &mut Vec<I>,
        transversal_weights: &mut Vec<NonZeroU32>,
    ) {
        let bit_size = std::mem::size_of::<I>() * 8;
        let n = self.weights[0].get() as usize;
        transversal_edges.reserve(n * self.chunk_size);
        transversal_weights.reserve(n);

        for c in (0..self.chunk_size).rev() {
            let upperbound = if c > 0 {
                bit_size as u32
            } else {
                self.n_vertices % (bit_size as u32)
            };
            let mut mask: I = I::one();
            for _ in 0u32..upperbound {
                if hyperedges[c] & mask == mask {
                    transversal_weights.push(nz_one());

                    for _ in 0..c {
                        transversal_edges.push(I::zero());
                    }
                    transversal_edges.push(mask);
                    for _ in c + 1..self.chunk_size {
                        transversal_edges.push(I::zero());
                    }
                }
                mask = mask << 1;
            }
        }
    }

    /// Create the transversal hypergraph of two hyperedges.
    fn make_two_hyperedge_transversals(
        &self,
        hyperedges: &[I],
        transversal_edges: &mut Vec<I>,
        transversal_weights: &mut Vec<NonZeroU32>,
    ) {
        let bit_size = std::mem::size_of::<I>() * 8;

        // These contain (word, bit) pairs for bits that are only in one of the hyperedges.
        let mut only_e1 = Vec::<BitPosition>::new();
        let mut only_e2 = Vec::<BitPosition>::new();

        for c in (0..self.chunk_size).rev() {
            let upperbound = if c > 0 {
                bit_size as u32
            } else {
                self.n_vertices % (bit_size as u32)
            };
            let mut mask: I = I::one();
            for i in 0u32..upperbound {
                match (
                    hyperedges[c] & mask == mask,
                    hyperedges[c + self.chunk_size] & mask == mask,
                ) {
                    (true, false) => {
                        // Only in e1.
                        only_e1.push(BitPosition { word: c, bit: i });
                    }
                    (false, true) => {
                        // Only in e2.
                        only_e2.push(BitPosition { word: c, bit: i });
                    }
                    (true, true) => {
                        // In both e1 and e2. This leads to a singleton hyperedge that we insert directly.
                        transversal_weights.push(nz_one());
                        for _ in 0..c {
                            transversal_edges.push(I::zero());
                        }
                        transversal_edges.push(mask);
                        for _ in c + 1..self.chunk_size {
                            transversal_edges.push(I::zero());
                        }
                    }
                    _ => {}
                }
                mask = mask << 1;
            }
        }

        let n_hyperedges = only_e1.len() * only_e2.len();
        transversal_weights.reserve(n_hyperedges);
        transversal_edges.reserve(n_hyperedges * self.chunk_size);

        // We want to add each pair of bits from only_e1 and only_e2 as a hyperedge, in ascending order. In order to do
        // this, we maintain two indices, one for each of the two lists, indicating the leftmost bit for which we have
        // not yet added all hyperedges containing that bit and a bit to the right of it from the other list.
        let mut i1 = 0;
        let mut i2 = 0;
        while i1 < only_e1.len() || i2 < only_e2.len() {
            if i2 == only_e2.len() || (i1 < only_e1.len() && only_e1[i1] < only_e2[i2]) {
                // We can add all hyperedges with the bit from only_e1[i1] and a bit to the right of it from only_e2.
                let e1 = &only_e1[i1];
                for j in 0..i2 {
                    let e2 = &only_e2[j];
                    transversal_weights.push(nz_two());

                    for _ in 0..e1.word {
                        transversal_edges.push(I::zero());
                    }

                    if e1.word == e2.word {
                        // Push a word having both bits set
                        transversal_edges.push(
                            (I::one() << (e1.bit as usize)) | (I::one() << (e2.bit as usize)),
                        );
                    } else {
                        // Push a word with the e1 bit
                        transversal_edges.push(I::one() << (e1.bit as usize));
                        for _ in e1.word + 1..e2.word {
                            transversal_edges.push(I::zero());
                        }
                        // Push a word with the e2 bit
                        transversal_edges.push(I::one() << (e2.bit as usize));
                    }
                    for _ in e2.word + 1..self.chunk_size {
                        transversal_edges.push(I::zero());
                    }
                }
                i1 += 1;
            } else {
                // We can add all hyperedges with the bit from only_e2[i2] and a bit to the right of it from only_e1.
                let e2 = &only_e2[i2];
                for j in 0..i1 {
                    let e1 = &only_e1[j];
                    transversal_weights.push(nz_two());

                    for _ in 0..e2.word {
                        transversal_edges.push(I::zero());
                    }

                    if e2.word == e1.word {
                        // Push a word having both bits set
                        transversal_edges.push(
                            (I::one() << (e2.bit as usize)) | (I::one() << (e1.bit as usize)),
                        );
                    } else {
                        // Push a word with the e2 bit
                        transversal_edges.push(I::one() << (e2.bit as usize));
                        for _ in e2.word + 1..e1.word {
                            transversal_edges.push(I::zero());
                        }
                        // Push a word with the e1 bit
                        transversal_edges.push(I::one() << (e1.bit as usize));
                    }
                    for _ in e1.word + 1..self.chunk_size {
                        transversal_edges.push(I::zero());
                    }
                }
                i2 += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Hypergraph;

    #[test]
    fn test_empty_hypergraph_transversal() {
        // Create an empty hypergraph with 5 vertices
        let empty_hypergraph = Hypergraph::<u32>::new(5);

        // Verify it's actually empty
        assert_eq!(empty_hypergraph.hyperedges.len(), 0);
        assert_eq!(empty_hypergraph.weights.len(), 0);

        // Compute its transversal hypergraph
        let transversal = empty_hypergraph.transversal_hypergraph();

        // The transversal hypergraph of an empty hypergraph should also be empty
        assert_eq!(transversal.hyperedges.len(), 0);
        assert_eq!(transversal.weights.len(), 0);
        assert_eq!(transversal.n_vertices, empty_hypergraph.n_vertices);
        assert_eq!(transversal.chunk_size, empty_hypergraph.chunk_size);
    }

    #[test]
    fn test_single_hyperedge_three_vertices_transversal() {
        // Create a hypergraph with 3 vertices and one hyperedge containing all three vertices {0, 1, 2}
        let mut hypergraph = Hypergraph::<u32>::new(3);

        // Add hyperedge {0, 1, 2} - represented as binary 111 = 7
        hypergraph.add_edge(&[7u32]);

        // Verify the original hypergraph has one hyperedge with weight 3
        assert_eq!(hypergraph.hyperedges.len(), 1);
        assert_eq!(hypergraph.weights.len(), 1);
        assert_eq!(hypergraph.weights[0].get(), 3);

        // Compute its transversal hypergraph
        let transversal = hypergraph.transversal_hypergraph();

        // The transversal hypergraph should have 3 hyperedges: {0}, {1}, {2}
        // Each represented as singleton sets with weights of 1
        assert_eq!(transversal.weights.len(), 3);
        assert_eq!(transversal.hyperedges.len(), 3);
        assert_eq!(transversal.n_vertices, hypergraph.n_vertices);
        assert_eq!(transversal.chunk_size, hypergraph.chunk_size);

        // All transversal hyperedges should have weight 1 (singleton sets)
        for weight in &transversal.weights {
            assert_eq!(weight.get(), 1);
        }

        // The hyperedges should be {0}, {1}, {2} represented as 1, 2, 4 respectively
        let expected_edges = vec![1u32, 2u32, 4u32];
        assert_eq!(transversal.hyperedges, expected_edges);
    }

    #[test]
    fn test_two_hyperedges_transversal() {
        // Create a hypergraph with 6 vertices (0,1,2,3,4,5)
        let mut hypergraph = Hypergraph::<u32>::new(6);

        // Add first hyperedge {0,1,2,3} - represented as binary 001111 = 15
        // Add second hyperedge {2,3,4,5} - represented as binary 111100 = 60
        hypergraph.add_edge(&[15u32]);
        hypergraph.add_edge(&[60u32]);

        // Verify the original hypergraph has two hyperedges
        assert_eq!(hypergraph.hyperedges.len(), 2);
        assert_eq!(hypergraph.weights.len(), 2);
        assert_eq!(hypergraph.weights[0].get(), 4); // First edge has 4 vertices
        assert_eq!(hypergraph.weights[1].get(), 4); // Second edge has 4 vertices

        // Compute its transversal hypergraph
        let transversal = hypergraph.transversal_hypergraph();

        // The transversal hypergraph should contain:
        // - Vertices in intersection {2,3}: singleton sets {2}, {3}
        // - Pairs from disjoint parts {0,1} × {4,5}: {0,4}, {0,5}, {1,4}, {1,5}
        // Total: 6 minimal transversals
        assert_eq!(transversal.weights.len(), 6);
        assert_eq!(transversal.hyperedges.len(), 6);
        assert_eq!(transversal.n_vertices, hypergraph.n_vertices);
        assert_eq!(transversal.chunk_size, hypergraph.chunk_size);

        // Check that we have the expected transversals:
        // {2} = 4, {3} = 8, {0,4} = 17, {0,5} = 33, {1,4} = 18, {1,5} = 34
        let mut expected_edges = vec![4u32, 8u32, 17u32, 18u32, 33u32, 34u32];
        expected_edges.sort();
        let mut actual_edges = transversal.hyperedges.clone();
        actual_edges.sort();

        assert_eq!(actual_edges, expected_edges);

        // Check weights: singletons have weight 1, pairs have weight 2
        let mut singleton_count = 0;
        let mut pair_count = 0;
        for weight in &transversal.weights {
            match weight.get() {
                1 => singleton_count += 1,
                2 => pair_count += 1,
                _ => panic!("Unexpected weight: {}", weight.get()),
            }
        }
        assert_eq!(singleton_count, 2); // {2}, {3}
        assert_eq!(pair_count, 4); // {0,4}, {0,5}, {1,4}, {1,5}
    }

    #[test]
    fn test_three_hyperedges_transversal() {
        let mut hypergraph = Hypergraph::<u8>::new(14);

        #[rustfmt::skip]
        let edges = vec![
            0b00_00_11, 0b11_11_00_11,
            0b00_11_00, 0b11_00_11_11,
            0b11_00_00, 0b00_11_11_11,
        ];
        hypergraph.add_edge(&edges[0..2]);
        hypergraph.add_edge(&edges[2..4]);
        hypergraph.add_edge(&edges[4..6]);

        // Verify the original hypergraph has three hyperedges of 8 vertices each
        assert_eq!(hypergraph.hyperedges.len() / hypergraph.chunk_size, 3);
        assert_eq!(hypergraph.weights.len(), 3);
        assert_eq!(hypergraph.weights[0].get(), 8);
        assert_eq!(hypergraph.weights[1].get(), 8);
        assert_eq!(hypergraph.weights[2].get(), 8);

        // Compute its transversal hypergraph
        let transversal = hypergraph.transversal_hypergraph();

        // We expect:
        //
        // * the two vertices that are in common between all three hyperedges become two hyperedges
        //   of weight 1;
        // * we get a hyperedge of weight 2 by selecting one vertex in common between two edges,
        //   and one vertex that is only in the other; there are three such subdivisions into 2/1
        //   of the original hyperedges, and for each we can choose either of two vertices that are
        //   just in common between the two and either of the two vertices that belong solely to
        //   the remaining edge, which means there are 12 such;
        // * we get a hyperedge of weight 3 by selecting one vertex from each hyperedge that does
        //   not occur in the others, which means there are 8 such.
        assert_eq!(
            transversal.hyperedges.len() / transversal.chunk_size,
            2 + 24 + 8
        );
        assert!(transversal.weights[0..2].iter().all(|&w| w.get() == 1));
        assert!(transversal.weights[2..26].iter().all(|&w| w.get() == 2));
        assert!(transversal.weights[26..34].iter().all(|&w| w.get() == 3));
    }

    #[test]
    fn test_three_hyperedges_transversal_simple() {
        let mut hypergraph = Hypergraph::<u8>::new(7);

        hypergraph.add_edge(&[0b0011101]);
        hypergraph.add_edge(&[0b0101011]);
        hypergraph.add_edge(&[0b1000111]);

        // Verify the original hypergraph has three hyperedges of 4 vertices each
        assert_eq!(hypergraph.hyperedges.len() / hypergraph.chunk_size, 3);
        assert_eq!(hypergraph.weights.len(), 3);
        assert_eq!(hypergraph.weights[0].get(), 4);
        assert_eq!(hypergraph.weights[1].get(), 4);
        assert_eq!(hypergraph.weights[2].get(), 4);

        // Compute its transversal hypergraph
        let transversal = hypergraph.transversal_hypergraph();

        // We expect:
        //
        // * the vertex that is in common between all three hyperedges becomes one hyperedge
        //   of weight 1;
        // * we get a hyperedge of weight 2 in 6 ways;
        // * we get a hyperedge of weight 3 by selecting one vertex from each hyperedge that does
        //   not occur in the others, which means there is 1 such.
        assert_eq!(
            transversal.hyperedges.len() / transversal.chunk_size,
            1 + 6 + 1,
        );
        assert!(transversal.weights[0..1].iter().all(|&w| w.get() == 1));
        assert!(transversal.weights[1..7].iter().all(|&w| w.get() == 2));
        assert!(transversal.weights[7..8].iter().all(|&w| w.get() == 3));
    }
}
