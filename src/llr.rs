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

use std::cmp::Ordering;

/// Result type used to find the top-k anomalous items per item via a binary heap
#[derive(PartialEq,Debug)]
pub struct ScoredItem {
    pub item: u32,
    pub score: f64,
}

/// Ordering for our max-heap, not that we must use a special implementation here as there is no
/// total order on floating point numbers.
fn cmp_reverse(scored_item_a: &ScoredItem, scored_item_b: &ScoredItem) -> Ordering {
    match scored_item_a.score.partial_cmp(&scored_item_b.score) {
        Some(Ordering::Less) => Ordering::Greater,
        Some(Ordering::Greater) => Ordering::Less,
        Some(Ordering::Equal) => Ordering::Equal,
        None => Ordering::Equal
    }
}

impl Eq for ScoredItem {}

impl Ord for ScoredItem {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_reverse(self, other)
    }
}

impl PartialOrd for ScoredItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(cmp_reverse(self, other))
    }
}

/// Precompute a table of logarithms which will be used for lookups later
pub fn logarithms_table(max_arg: usize) -> Vec<f64> {

    (0..max_arg)
        .map(|index| {
            if index == 0 {
                0.0
            } else {
                (index as f64).ln()
            }
        })
        .collect()
}

/// Highly optimized implementation of the loglikelihood-based G²-test. We enforce inlining of the
/// logarithms computation, apply manual common subexpression elimination and leverage a precomputed
/// logarithms table for a given range.
///
/// The following url gives some details on the original implementation:
///
/// https://github.com/apache/mahout/blob/08e02602e947ff945b9bd73ab5f0b45863df3e53/math/src/main/java/org/apache/mahout/math/stats/LogLikelihood.java
///
/// We would also like to thank Frank McSherry for his help in optimizing this piece of code:
/// https://www.reddit.com/r/rust/comments/6qmnbo/why_is_my_scala_program_twice_as_fast_as_my_rust/dl0x1bj/
///
#[inline(always)]
pub fn log_likelihood_ratio(k11: u64, k12: u64, k21: u64, k22: u64, logarithms: &[f64]) -> f64 {

    let xlx_all = x_logx(k11 + k12 + k21 + k22);

    let log_k11 = logarithms[k11 as usize];
    let log_k12 = logarithms[k12 as usize];
    let log_k21 = logarithms[k21 as usize];
    let log_k11_12 = logarithms[(k11 + k12) as usize];
    let log_k11_21 = logarithms[(k11 + k21) as usize];

    let row_entropy = xlx_all - x_times_logx(k11 + k12, log_k11_12) - x_logx(k21 + k22);
    let column_entropy = xlx_all - x_times_logx(k11 + k21, log_k11_21) - x_logx(k12 + k22);
    let matrix_entropy = xlx_all - x_times_logx(k11, log_k11) - x_times_logx(k12, log_k12) -
        x_times_logx(k21, log_k21) - x_logx(k22);

    if row_entropy + column_entropy < matrix_entropy {
        0.0 // Round off error
    } else {
        2.0 * (row_entropy + column_entropy - matrix_entropy)
    }
}


#[inline(always)]
pub fn x_logx(x: u64) -> f64 {
    //Note we only call this for values >= k22 > 0, therefore we can omit the 0 check
    x as f64 * (x as f64).ln()
}

#[inline(always)]
fn x_times_logx(x: u64, log_x: f64) -> f64 {
    x as f64 * log_x
}


#[cfg(test)]
mod tests {

    use std::collections::BinaryHeap;
    use std::f64::EPSILON;
    use llr;
    use llr::ScoredItem;

    #[test]
    fn scored_item_ordering_reversed() {
        let item_a = ScoredItem { item: 1, score: 0.5 };
        let item_b = ScoredItem { item: 2, score: 1.5 };
        let item_c = ScoredItem { item: 3, score: 0.3 };

        assert!(item_a > item_b);
        assert!(item_a < item_c);
        assert!(item_b < item_c);
    }

    #[test]
    fn llr() {
        // Some cases from http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.14.5962
        let logs = llr::logarithms_table(500 * 500);

        assert!(close_enough_to(llr::log_likelihood_ratio(110, 2442, 111, 29114, &logs), 270.72));
        assert!(close_enough_to(llr::log_likelihood_ratio(29, 13, 123, 31612, &logs), 263.90));
        assert!(close_enough_to(llr::log_likelihood_ratio(9, 12, 429, 31327, &logs), 48.94));
    }

    fn close_enough_to(value: f64, expected: f64) -> bool {
        (value - expected).abs() < 0.01
    }

    fn within_epsilon(value: f64, expected: f64) -> bool {
        (value - expected).abs() < EPSILON
    }

    #[test]
    fn topk() {

        const K: usize = 3;

        let items = [
            ScoredItem { item: 1, score: 0.5 },
            ScoredItem { item: 2, score: 1.5 },
            ScoredItem { item: 3, score: 0.3 },
            ScoredItem { item: 4, score: 3.5 },
            ScoredItem { item: 5, score: 2.5 },
        ];

        let mut heap = BinaryHeap::with_capacity(K);

        for scored_item in &items {
            if heap.len() < K {
                heap.push(scored_item);
            } else {
                let mut top = heap.peek_mut().unwrap();
                if scored_item < *top {
                    *top = scored_item;
                }
            }
        }

        let top_k = heap.into_sorted_vec();

        assert_eq!(top_k.len(), 3);

        assert_eq!(top_k[0].item, 4);
        assert!(within_epsilon(top_k[0].score, 3.5));

        assert_eq!(top_k[1].item, 5);
        assert!(within_epsilon(top_k[1].score, 2.5));

        assert_eq!(top_k[2].item, 2);
        assert!(within_epsilon(top_k[2].score, 1.5));
    }
}