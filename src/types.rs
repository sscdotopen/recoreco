extern crate fnv;

use fnv::{FnvHashMap, FnvHashSet};

pub type DenseVector = Vec<u32>;

pub type SparseVector = FnvHashMap<u32,u16>;
pub type SparseMatrix = Vec<SparseVector>;

pub type SparseBinaryVector = FnvHashSet<u32>;
pub type SparseBinaryMatrix = Vec<SparseBinaryVector>;

pub fn new_dense_vector(dimensions: usize) -> DenseVector {
    vec![0; dimensions]
}

pub fn new_sparse_vector(capacity: usize) -> SparseVector {
    FnvHashMap::with_capacity_and_hasher(capacity, Default::default())
}

pub fn new_sparse_binary_matrix(num_rows: usize) -> SparseBinaryMatrix {
    vec![FnvHashSet::default(); num_rows]
}