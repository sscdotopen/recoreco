extern crate csv;
extern crate fnv;

use std;
use std::time::Duration;

use types;
use types::SparseBinaryMatrix;

use stats::DataDictionary;

pub fn to_millis(duration: Duration) -> u64 {
    (duration.as_secs() * 1_000) + (duration.subsec_nanos() / 1_000_000) as u64
}

pub fn csv_reader(file: &str) -> csv::Reader<std::fs::File> {
    csv::Reader::from_file(file)
        .unwrap()
        .has_headers(false)
        .delimiter('\t' as u8)
}

/*pub fn interactions_from_csv(reader: &'static mut csv::Reader<std::fs::File>) -> impl Iterator<Item=(String,String)> {
    reader.records()
        .filter_map(Result::ok)
        .filter_map(|record| Some((record[0].clone(), record[1].clone())))
        .into_iter()
}*/


pub fn read_interactions(file: &str, data_dict: &DataDictionary) -> Vec<(u32,u32)> {

    let mut reader: csv::Reader<std::fs::File> = csv_reader(file);

    let mut interactions: Vec<(u32, u32)> =
        Vec::with_capacity(data_dict.num_interactions() as usize);

    for record in reader.decode() {
        let (user, item): (String, String) = record.unwrap();

        let user_index = data_dict.user_index(&user);
        let item_index = data_dict.item_index(&item);

        interactions.push((*user_index, *item_index));
    }

    interactions
}

pub fn read_histories(file: &str, data_dict: &DataDictionary) -> SparseBinaryMatrix {

    let mut histories = types::new_sparse_binary_matrix(data_dict.num_users());

    let mut csv_reader = csv_reader(file);

    for record in csv_reader.decode() {

        let (user, item): (String, String) = record.unwrap();

        let user_index = data_dict.user_index(&user);
        let item_index = data_dict.item_index(&item);

        histories[*user_index as usize].insert(*item_index);
    }

    histories
}