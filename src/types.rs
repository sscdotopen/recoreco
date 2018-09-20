/**
 * RecoReco
 * Copyright (C) 2018 Sebastian Schelter
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

extern crate fnv;

use fnv::{FnvHashMap, FnvHashSet};

pub type DenseVector = Vec<u32>;

pub type SparseVector = FnvHashMap<u32, u16>;
pub type SparseMatrix = Vec<SparseVector>;

pub type SparseBinaryMatrix = Vec<FnvHashSet<u32>>;

pub fn new_dense_vector(dimensions: usize) -> DenseVector {
    vec![0; dimensions]
}

pub fn new_sparse_matrix(num_rows: usize) -> SparseMatrix {
    vec![FnvHashMap::with_capacity_and_hasher(0, Default::default()); num_rows]
}
