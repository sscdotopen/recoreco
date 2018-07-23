extern crate rand;
extern crate fnv;
extern crate scoped_pool;

use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::time::Instant;

use rand::Rng;
use fnv::{FnvHashMap, FnvHashSet};
use scoped_pool::Pool;

mod llr;
pub mod utils;
mod types;
pub mod stats;
pub mod recommend;

use llr::ScoredItem;
use types::{DenseVector, SparseVector, SparseMatrix, SparseBinaryMatrix};



pub fn indicators(
    interactions: &Vec<(u32, u32)>,
    num_users: usize, num_items: usize,
    pool_size: usize,
    k: usize,
) -> SparseBinaryMatrix {

    let pool = Pool::new(pool_size);

    const F_MAX: u32 = 500;
    const K_MAX: u32 = 500;

    // larger of both values needs to be added
    const MAX_COOCCURRENCES: usize = (F_MAX * K_MAX + K_MAX) as usize;
    let pre_computed_logarithms: Vec<f64> = llr::logarithms_table(MAX_COOCCURRENCES);

    // Downsampled history matrix A
    let mut user_non_sampled_interaction_counts = types::new_dense_vector(num_users);
    let mut user_interaction_counts = types::new_dense_vector(num_users);
    let mut item_interaction_counts = types::new_dense_vector(num_items);
    let mut samples_of_a: Vec<Vec<u32>> = vec![Vec::with_capacity(10); num_users];

    // Cooccurrence matrix C
    let mut c: SparseMatrix =
        vec![FnvHashMap::with_capacity_and_hasher(10, Default::default()); num_items];
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

    for &(user, item) in interactions.iter() {

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
            let reference_to_pre_computed_logarithms = &pre_computed_logarithms;

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

    let duration_for_batch = utils::to_millis(batch_start.elapsed());
    println!("{} cooccurrences observed, {}ms training time, {} items rescored",
        num_cooccurrences_observed, duration_for_batch, items_to_rescore.len());

    indicators.into_iter()
        .map(|entry| {
            let mut heap = entry.lock().unwrap();
            let mut items = FnvHashSet::with_capacity_and_hasher(heap.len(), Default::default());
            for scored_item in heap.drain() {
                items.insert(scored_item.item);
            }

            items
        })
        .collect()
}


fn rescore(
    item: u32,
    cooccurrence_counts: &SparseVector,
    row_sums_of_c: &DenseVector,
    num_cooccurrences_observed: &u64,
    indicators: &Mutex<BinaryHeap<ScoredItem>>,
    k: usize,
    logarithms_table: &Vec<f64>,
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