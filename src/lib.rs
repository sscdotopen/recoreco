//! ## RecoReco - fast item-based recommendations on the command line.
//!
//! **Recoreco** computes highly associated pairs of items (in the sense of 'people who are
//! interested in X are also interested in Y') from interactions between users and items. It is a
//! command line tool that expects a CSV file as input, where each line denotes an interaction
//! between a user and an item and consists of a user identifier and an item identifier separated
//! by a tab character. Recoreco by default outputs 10 associated items per item (with no particular
//! ranking) in JSON format.
//!
//! If you would like to learn more about the math behind the approach that **recoreco** is built
//! on, checkout the book on [practical machine learning: innovations in recommendation](https://mapr.com/practical-machine-learning/)
//! and the talk on [real-time puppies and ponies](https://www.slideshare.net/tdunning/realtime-puppies-and-ponies-evolving-indicator-recommendations-in-realtime)
//! from my friend [Ted Dunning](https://twitter.com/ted_dunning).

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
use types::{SparseVector, SparseMatrix, SparseBinaryMatrix};
use stats::DataDictionary;

/// Compute item indicators from a stream of interactions.
///
/// * `interactions` - the observed interactions
/// * `data_dict` - a data dictionary which maps string to integer identifiers
/// * `pool_size`  - the number of CPUs to use for the computation
/// * `num_indicators_per_item` - the number of highly associated items to compute per item (use 10 as default)
/// * `f_max` - the maximum number of interactions to account for per user (use 500 as default)
/// * `k_max` - The maximum number of interactions to account for per item (use 500 as default)
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// extern crate recoreco;
/// use recoreco::stats::{DataDictionary, Renaming};
/// use recoreco::indicators;
///
/// /* Our input data comprises of observed interactions between users and items.
///    The identifiers used can be strings of arbitrary length and structure. */
///
/// let interactions = vec![
///     ("alice".to_owned(), "apple".to_owned()),
///     ("alice".to_owned(), "dog".to_owned()),
///     ("alice".to_owned(), "pony".to_owned()),
///     ("bob".to_owned(), "apple".to_owned()),
///     ("bob".to_owned(), "pony".to_owned()),
///     ("charles".to_owned(), "pony".to_owned()),
///     ("charles".to_owned(), "bike".to_owned())
/// ];
///
/// /* Internally, recoreco uses consecutive integer ids and requires some knowledge about
///    the statistics of the data for efficient allocation. Therefore, we read the
///    interaction data once to compute a data dictionary that helps us map from string to
///    integer identifiers and has basic statistics of the data */
///
/// let data_dict = DataDictionary::from(interactions.iter());
///
/// println!(
///     "Found {} interactions between {} users and {} items.",
///     data_dict.num_interactions(),
///     data_dict.num_users(),
///     data_dict.num_items(),
/// );
///
/// /* Now we read the interactions a second time and compute the indicator matrix from item
///    cooccurrences. The result is the so-called indicator matrix, where each entry
///    indicates highly associated pairs of items. */
///
/// let indicated_items = indicators(
///     interactions.into_iter(),
///     &data_dict,
///     2,
///     10,
///     500,
///     500
/// );
///
/// /* The renaming data structure helps us map the integer ids back to the original
///    string ids. */
///
/// let renaming = Renaming::from(data_dict);
///
/// /* We print the resulting highly associated pairs of items. */
/// for (item_index, indicated_items_for_item) in indicated_items.iter().enumerate() {
///     let item_name = renaming.item_name(item_index as u32);
///     println!("Items highly associated with {}:", item_name);
///
///     for indicated_item_index in indicated_items_for_item.iter() {
///         let indicated_item_name = renaming.item_name(*indicated_item_index as u32);
///         println!("\t{}", indicated_item_name);
///     }
/// }
/// ```
pub fn indicators<T>(
    interactions: T,
    data_dict: &DataDictionary,
    pool_size: usize,
    num_indicators_per_item: usize,
    f_max: u32,
    k_max: u32
) -> SparseBinaryMatrix
where T: Iterator<Item=(String,String)> {

    let num_items = data_dict.num_items();
    let num_users = data_dict.num_users();

    let pool = Pool::new(pool_size);

    let max_cooccurrences = (f_max * f_max) as usize;

    // Precompute most logarithms
    let precomputed_logarithms: Vec<f64> = llr::logarithms_table(max_cooccurrences);

    // Downsampled history matrix A
    let mut user_non_sampled_interaction_counts = types::new_dense_vector(num_users);
    let mut user_interaction_counts = types::new_dense_vector(num_users);
    let mut item_interaction_counts = types::new_dense_vector(num_items);
    let mut samples_of_a: Vec<Vec<u32>> = vec![Vec::new(); num_users];

    // Cooccurrence matrix C
    let mut c: SparseMatrix = types::new_sparse_matrix(num_items);
    let mut row_sums_of_c = types::new_dense_vector(num_items);

    // Indicator matrix I
    let indicators: Vec<Mutex<BinaryHeap<ScoredItem>>> = (0..num_items)
        .map(|_| Mutex::new(BinaryHeap::with_capacity(num_indicators_per_item)))
        .collect();

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

        if item_interaction_counts[item_idx] < f_max {

            let mut user_history = &mut samples_of_a[user_idx];
            let num_items_in_user_history = user_history.len();

            if user_interaction_counts[user_idx] < k_max {

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
        for item in &items_to_rescore {

            let row = &c[*item as usize];
            let indicators_for_item = &indicators[*item as usize];
            let reference_to_row_sums_of_c = &row_sums_of_c;
            let reference_to_pre_computed_logarithms = &precomputed_logarithms;

            scope.execute(move || {
                rescore(
                    *item,
                    row,
                    reference_to_row_sums_of_c,
                    num_cooccurrences_observed,
                    indicators_for_item,
                    num_indicators_per_item,
                    reference_to_pre_computed_logarithms,
                )
            });
        }
    });

    let duration_for_batch = to_millis(batch_start.elapsed());
    println!(
        "{} cooccurrences observed, {}ms training time, {} items rescored",
        num_cooccurrences_observed,
        duration_for_batch,
        items_to_rescore.len(),
    );

    indicators
        .into_iter()
        .map(|entry| {
            let mut heap = entry.lock().unwrap();

            let items: FnvHashSet<u32> = heap
                .drain()
                .map(|scored_item| scored_item.item)
                .collect(); // Checked that size_hint() gives correct bounds

            items
        })
        .collect()
}

fn to_millis(duration: Duration) -> u64 {
    (duration.as_secs() * 1_000) + u64::from(duration.subsec_millis())
}

fn rescore(
    item: u32,
    cooccurrence_counts: &SparseVector,
    row_sums_of_c: &[u32],
    num_cooccurrences_observed: u64,
    indicators: &Mutex<BinaryHeap<ScoredItem>>,
    n: usize,
    logarithms_table: &[f64],
) {

    let mut indicators_for_item = indicators.lock().unwrap();

    for (other_item, num_cooccurrences) in cooccurrence_counts.iter() {

        if *other_item != item {
            let k11 = u64::from(*num_cooccurrences);
            let k12 = u64::from(row_sums_of_c[item as usize]) - k11;
            let k21 = u64::from(row_sums_of_c[*other_item as usize]) - k11;
            let k22 = num_cooccurrences_observed + k11 - k12 - k21;

            let llr_score = llr::log_likelihood_ratio(k11, k12, k21, k22, logarithms_table);

            let scored_item = ScoredItem { item: *other_item, score: llr_score };

            if indicators_for_item.len() < n {
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