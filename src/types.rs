//! ## Type definitions to map from tensors to Rust collections
//!
//! Item-based recommenders need to work with different types of matrices (especially sparse ones).
//! This module defines the internal representation (e.g., the Rust collections) used for these
//! matrices.
//!
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

/// 32 bit integer vector, backed by a `Vec<u32>`
pub type DenseVector = Vec<u32>;

/// Sparse 16 bit integer vector, backed by a `FnvHashMap<u32, u16>`
pub type SparseVector = FnvHashMap<u32, u16>;

/// Sparse 16 bit integer matrix, row-wise representation, backed by a `Vec<FnvHashMap<u32, u16>>`
pub type SparseMatrix = Vec<SparseVector>;

/// Sparse binary matrix, row-wise representation, backed by a `Vec<FnvHashSet<u32>>`
pub type SparseBinaryMatrix = Vec<FnvHashSet<u32>>;

/// Allocates a dense zero vector with of size `dimensions`
pub fn new_dense_vector(dimensions: usize) -> DenseVector {
    vec![0; dimensions]
}

/// Allocates a sparse binary matrix with empty rows
pub fn new_sparse_matrix(num_rows: usize) -> SparseMatrix {
    vec![FnvHashMap::with_capacity_and_hasher(0, Default::default()); num_rows]
}
