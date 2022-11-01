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
extern crate rayon;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

use rand::Rng;
use fnv::FnvHashSet;
use rayon::prelude::*;

mod llr;
pub mod io;
pub mod types;
pub mod stats;

mod usage_tests;

use llr::ScoredItem;
use types::{SparseVector, SparseMatrix, IndicatorSet};
use stats::DataDictionary;

/// Compute item indicators from a stream of interactions.
///
/// * `interactions` - the observed interactions
/// * `data_dict` - a data dictionary which maps string to integer identifiers
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
///     (String::from("alice"), String::from("apple")),
///     (String::from("alice"), String::from("dog")),
///     (String::from("alice"), String::from("pony")),
///     (String::from("bob"), String::from("apple")),
///     (String::from("bob"), String::from("pony")),
///     (String::from("charles"), String::from("pony")),
///     (String::from("charles"), String::from("bike"))
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
    num_indicators_per_item: usize,
    f_max: u32,
    k_max: u32
) -> IndicatorSet
where
    T: Iterator<Item = (String, String)>
{

    let num_items = data_dict.num_items();
    let num_users = data_dict.num_users();

    let max_sum_of_cooccurrences_per_item = (f_max * k_max) as usize;

    // Precompute most logarithms
    let precomputed_logarithms: Vec<f64> = llr::logarithms_table(max_sum_of_cooccurrences_per_item);

    // Downsampled history matrix A
    let mut user_non_sampled_interaction_counts = types::new_dense_vector(num_users);
    let mut user_interaction_counts = types::new_dense_vector(num_users);
    let mut item_interaction_counts = types::new_dense_vector(num_items);
    let mut samples_of_a: Vec<Vec<u32>> = vec![Vec::new(); num_users];

    // Cooccurrence matrix C
    let mut c: SparseMatrix = types::new_sparse_matrix(num_items);
    let mut row_sums_of_c = types::new_dense_vector(num_items);

    let mut num_cooccurrences_observed: u64 = 0;

    let mut rng = rand::XorShiftRng::new_unseeded();

    let start = Instant::now();

    let mut items_to_rescore = FnvHashSet::default();

    for (user_str, item_str) in interactions {

        let item = *data_dict.item_index(&item_str);
        let user = *data_dict.user_index(&user_str);

        let item_idx = item as usize;
        let user_idx = user as usize;

        // Update number of observed interactions for user
        user_non_sampled_interaction_counts[user_idx] += 1;

        // Check whether we have seen enough interactions for this item yet
        if item_interaction_counts[item_idx] < f_max {

            // Retrieve current history sample for interacting user
            let user_history = &mut samples_of_a[user_idx];
            let num_items_in_user_history = user_history.len();

            // Check whether we have seen enough interactions for this user yet
            if user_interaction_counts[user_idx] < k_max {

                // Record coocurrences with all other items from user history
                for other_item in user_history.iter() {
                    *c[item_idx].entry(*other_item).or_insert(0) += 1;
                    *c[*other_item as usize].entry(item).or_insert(0) += 1;
                    row_sums_of_c[*other_item as usize] += 1;
                }

                // Add item to user history
                user_history.push(item);
                // Register items for rescoring
                items_to_rescore.extend(user_history.iter());
                items_to_rescore.insert(item);

                // Update statistics for user and item interaction counts and
                // cooccurrence matrix sums
                user_interaction_counts[user_idx] += 1;
                item_interaction_counts[item_idx] += 1;
                row_sums_of_c[item_idx] += num_items_in_user_history as u32;
                num_cooccurrences_observed += 2 * num_items_in_user_history as u64;

            } else {

                let num_interactions_seen_by_user =
                    user_non_sampled_interaction_counts[user_idx];

                let k: usize = rng.gen_range(0, num_interactions_seen_by_user as usize);

                if k < num_items_in_user_history {
                    let previous_item = user_history[k];

                    for (n, other_item) in user_history.iter().enumerate() {

                        if n != k {
                            // Adjust cooccurrence counts
                            *c[item_idx].entry(*other_item).or_insert(0) += 1;
                            *c[*other_item as usize].entry(item).or_insert(0) += 1;
                            *c[previous_item as usize].entry(*other_item).or_insert(0) -= 1;
                            *c[*other_item as usize].entry(previous_item).or_insert(0) -= 1;
                        }
                    }

                    // Register items for rescoring
                    items_to_rescore.extend(user_history.iter());
                    items_to_rescore.insert(item);

                    // update cooccurrence matrix sums
                    row_sums_of_c[item_idx] += num_items_in_user_history as u32 - 1;
                    row_sums_of_c[previous_item as usize] -=
                        num_items_in_user_history as u32 - 1;

                    // Replace previous item in user history
                    user_history[k] = item;

                    // Adjust item statistics
                    item_interaction_counts[item_idx] += 1;
                    item_interaction_counts[previous_item as usize] -= 1;
                }
            }
        }
    }

    // Compute top-n indicators per item in parallel
    let indicators = items_to_rescore
        .par_iter()
        .map(|item| {
            rescore(
                *item,
                &c[*item as usize],
                &row_sums_of_c,
                num_cooccurrences_observed,
                num_indicators_per_item,
                &precomputed_logarithms,
                //&renaming
            )
        })
        .collect::<Vec<(u32, FnvHashSet<u32>)>>();

    let duration = to_millis(start.elapsed());
    println!(
        "{} cooccurrences observed, {}ms training time, {} items rescored",
        num_cooccurrences_observed,
        duration,
        items_to_rescore.len(),
    );

    indicators
}

fn to_millis(duration: Duration) -> u64 {
    (duration.as_secs() * 1_000) + u64::from(duration.subsec_millis())
}

fn rescore(
    item: u32,
    cooccurrence_counts: &SparseVector,
    row_sums_of_c: &[u32],
    num_cooccurrences_observed: u64,
    n: usize,
    logarithms_table: &[f64],
) -> (u32, FnvHashSet<u32>) {

    // We can skip the scoring if we have seen less than n items
    if cooccurrence_counts.len() <= n {
        (item, cooccurrence_counts
            .keys()
            .cloned()
            .collect::<FnvHashSet<_>>())
    } else {
        // We'll use a heap to keep track of the current top-n scored items
        let mut top_indicators: BinaryHeap<ScoredItem> = BinaryHeap::with_capacity(n);

        for (other_item, num_cooccurrences) in cooccurrence_counts.iter() {
            if *other_item != item {

                // Compute counts of contingency table
                let k11 = u64::from(*num_cooccurrences);
                let k12 = u64::from(row_sums_of_c[item as usize]) - k11;
                let k21 = u64::from(row_sums_of_c[*other_item as usize]) - k11;
                let k22 = num_cooccurrences_observed + k11 - k12 - k21;

                // Compute LLR score
                let llr_score = llr::log_likelihood_ratio(k11, k12, k21, k22, logarithms_table);

                // Update heap holding top-n scored items for this item
                let scored_item = ScoredItem { item: *other_item, score: llr_score };

                if top_indicators.len() < n {
                    top_indicators.push(scored_item);
                } else {
                    let mut top = top_indicators.peek_mut().unwrap();
                    if scored_item < *top {
                        *top = scored_item;
                    }
                }
            }
        }

        let indicators_for_item: FnvHashSet<u32> = top_indicators
            .drain()
            .map(|scored_item| scored_item.item)
            .collect();

        (item, indicators_for_item)
    }
}
