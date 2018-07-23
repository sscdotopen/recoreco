extern crate fnv;

use std::collections::BinaryHeap;
use std::cmp::Ordering;

use types;
use types::SparseBinaryMatrix;

#[derive(PartialEq,Eq,Debug)]
struct CountedItem {
    item: u32,
    count: u16,
}

impl Ord for CountedItem {
    fn cmp(&self, other: &Self) -> Ordering { self.item.cmp(&other.item).reverse() }
}

impl PartialOrd for CountedItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}


pub fn recommend(
    histories: &SparseBinaryMatrix,
    indicators: &SparseBinaryMatrix,
    num_items_to_recommend: usize
) -> Vec<Vec<u32>> {

    let num_users = histories.len();

    let mut recommendations: Vec<Vec<u32>> = Vec::with_capacity(num_users);
    recommendations.extend((0..num_users).map(|_| Vec::new()));

    for user_idx in 1_u32..num_users as u32 {
        let history = &histories[user_idx as usize];

        let mut other_item_counts = types::new_sparse_vector(10);

        for item_idx in history.iter() {
            for other_item in indicators[*item_idx as usize].iter() {

                if !history.contains(&other_item) {
                    let count = other_item_counts.entry(*other_item).or_insert(0);
                    *count += 1;
                }
            }
        }

        let mut heap = BinaryHeap::with_capacity(num_items_to_recommend);

        for (item_idx, count) in other_item_counts.iter() {
            let counted_item = CountedItem { item: *item_idx, count: *count };

            if heap.len() < num_items_to_recommend {
                heap.push(counted_item);
            } else {
                let mut top = heap.peek_mut().unwrap();
                if counted_item < *top {
                    *top = counted_item;
                }
            }
        }

        recommendations[user_idx as usize] = heap
            .into_iter()
            .map(|counted_item| counted_item.item)
            .collect();
    }

    recommendations
}