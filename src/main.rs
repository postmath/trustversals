use hypergraph::examples;

mod hypergraph;
mod numbers;

fn main() {
    let mut h = examples::binomial::<u32>(10, 5);
    h.sort();
    h.print_edges();

    let mut h = examples::lovasz::<u32>(4);
    h.sort();
    h.print_edges();
}
