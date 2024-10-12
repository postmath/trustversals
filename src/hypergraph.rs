use std::num::NonZeroU32;

/// A hypergraph with `n_vertices` vertices and `len(weights)` hyperedges. Each hyperedge is
/// described as a bitmap of `chunk_size` entries of `hyperedges`; the ith hyperedge consists of
/// `hyperedges[i * chunk_size .. (i+1) * chunk_size]`. The presence of vertex 0 in this hyperedge
/// is indicated by the lowest bit of `hyperedges[i * chunk_size]`. The empty hyperedge is not
/// allowed. The ith hyperedge has weight (size) `weights[i]`.
pub struct Hypergraph<I> {
    n_vertices: u32,
    chunk_size: usize,
    hyperedges: Vec<I>,
    weights: Vec<NonZeroU32>,
}

impl<I> Hypergraph<I> {
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
}
