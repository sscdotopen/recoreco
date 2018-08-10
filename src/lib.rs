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

extern crate rand;
extern crate fnv;
extern crate scoped_pool;

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand::Rng;
use fnv::FnvHashSet;
use scoped_pool::Pool;

mod llr;
pub mod io;
pub mod types;
pub mod stats;

use llr::ScoredItem;
use types::{DenseVector, SparseVector, SparseMatrix, SparseBinaryMatrix};
use stats::DataDictionary;

pub fn indicators<T>(
    interactions: T,
    data_dict: &DataDictionary,
    pool_size: usize,
    k: usize,
) -> SparseBinaryMatrix
where T: Iterator<Item=(String,String)> {

    let num_items = data_dict.num_items();
    let num_users = data_dict.num_users();

    let pool = Pool::new(pool_size);

    const F_MAX: u32 = 500;
    const K_MAX: u32 = 500;
    const MAX_COOCCURRENCES: usize = (F_MAX * F_MAX) as usize;

    //TODO this could be constant size
    let precomputed_logarithms: Vec<f64> = llr::logarithms_table(MAX_COOCCURRENCES);

    // Downsampled history matrix A
    let mut user_non_sampled_interaction_counts = types::new_dense_vector(num_users);
    let mut user_interaction_counts = types::new_dense_vector(num_users);
    let mut item_interaction_counts = types::new_dense_vector(num_items);
    let mut samples_of_a: Vec<Vec<u32>> = vec![Vec::with_capacity(10); num_users];

    // Cooccurrence matrix C
    let mut c: SparseMatrix = types::new_sparse_matrix(num_items);
    let mut row_sums_of_c = types::new_dense_vector(num_items);

    // Indicator matrix I
    let mut indicators: Vec<Mutex<BinaryHeap<ScoredItem>>> = Vec::with_capacity(num_items);

    for _ in 0..num_items {
        indicators.push(Mutex::new(BinaryHeap::with_capacity(k)));
    }

    let mut num_cooccurrences_observed: u64 = 0;

    let mut rng = rand::XorShiftRng::new_unseeded();

    let batch_start = Instant::now();

    let mut items_to_rescore = FnvHashSet::default();

    for (user_str, item_str) in interactions {

        let item = *data_dict.item_index(&item_str);
        let user = *data_dict.user_index(&user_str);

        let item_idx = item as usize;
        let user_idx = user as usize;

        user_non_sampled_interaction_counts[user_idx] += 1;

        if item_interaction_counts[item_idx] < F_MAX {

            let mut user_history = samples_of_a.get_mut(user_idx).unwrap();
            let num_items_in_user_history = user_history.len();

            if user_interaction_counts[user_idx] < K_MAX {

                for other_item in user_history.iter() {

                    *c[item_idx].entry(*other_item).or_insert(0) += 1;
                    *c[*other_item as usize].entry(item).or_insert(0) += 1;

                    row_sums_of_c[*other_item as usize] += 1;
                    items_to_rescore.insert(*other_item);
                }

                row_sums_of_c[item_idx] += num_items_in_user_history as u32;
                num_cooccurrences_observed += 2 * num_items_in_user_history as u64;

                user_history.push(item);

                user_interaction_counts[user_idx] += 1;
                item_interaction_counts[item_idx] += 1;

                items_to_rescore.insert(item);

            } else {

                let num_interactions_seen_by_user =
                    user_non_sampled_interaction_counts[user_idx];

                let k: usize = rng.gen_range(0, num_interactions_seen_by_user as usize);

                if k < num_items_in_user_history {
                    let previous_item = user_history[k];

                    for (n, other_item) in user_history.iter().enumerate() {

                        if n != k {

                            *c[item_idx].entry(*other_item).or_insert(0) += 1;
                            *c[*other_item as usize].entry(item).or_insert(0) += 1;

                            *c[previous_item as usize].entry(*other_item).or_insert(0) -= 1;
                            *c[*other_item as usize].entry(previous_item).or_insert(0) -= 1;

                            items_to_rescore.insert(*other_item);
                        }
                    }

                    row_sums_of_c[item_idx] += num_items_in_user_history as u32 - 1;
                    row_sums_of_c[previous_item as usize] -=
                        num_items_in_user_history as u32 - 1;

                    user_history[k] = item;

                    item_interaction_counts[item_idx] += 1;
                    item_interaction_counts[previous_item as usize] -= 1;

                    items_to_rescore.insert(previous_item);
                    items_to_rescore.insert(item);
                }
            }
        }
    }

    pool.scoped(|scope| {
        for item in items_to_rescore.iter() {

            let row = &c[*item as usize];
            let indicators_for_item = &indicators[*item as usize];
            let reference_to_row_sums_of_c = &row_sums_of_c;
            let reference_to_pre_computed_logarithms = &precomputed_logarithms;

            scope.execute(move|| {
                rescore(
                    *item,
                    row,
                    reference_to_row_sums_of_c,
                    &num_cooccurrences_observed,
                    indicators_for_item,
                    k,
                    reference_to_pre_computed_logarithms,
                )
            });
        }
    });

    let duration_for_batch = to_millis(batch_start.elapsed());
    println!("{} cooccurrences observed, {}ms training time, {} items rescored",
        num_cooccurrences_observed, duration_for_batch, items_to_rescore.len());

    indicators.into_iter()
        .map(|entry| {
            let mut heap = entry.lock().unwrap();


            let items: FnvHashSet<u32> = heap.drain()
                .map(|scored_item| scored_item.item)
                // Checked that size_hint() gives correct bounds
                .collect();

            items
        })
        .collect()
}

fn to_millis(duration: Duration) -> u64 {
    (duration.as_secs() * 1_000) + (duration.subsec_nanos() / 1_000_000) as u64
}

fn rescore(
    item: u32,
    cooccurrence_counts: &SparseVector,
    row_sums_of_c: &DenseVector,
    num_cooccurrences_observed: &u64,
    indicators: &Mutex<BinaryHeap<ScoredItem>>,
    k: usize,
    logarithms_table: &[f64],
) {

    let mut indicators_for_item = indicators.lock().unwrap();
    indicators_for_item.clear();

    for (other_item, num_cooccurrences) in cooccurrence_counts.iter() {

        if *other_item != item {
            let k11 = *num_cooccurrences as u64;
            let k12 = row_sums_of_c[item as usize] as u64 - k11;
            let k21 = row_sums_of_c[*other_item as usize] as u64 - k11;
            let k22 = num_cooccurrences_observed + k11 - k12 - k21;

            let llr_score = llr::log_likelihood_ratio(k11, k12, k21, k22, logarithms_table);

            let scored_item = ScoredItem { item: *other_item, score: llr_score };

            if indicators_for_item.len() < k {
                indicators_for_item.push(scored_item);
            } else {
                let mut top = indicators_for_item.peek_mut().unwrap();
                if scored_item < *top {
                    *top = scored_item;
                }
            }
        }
    }
}