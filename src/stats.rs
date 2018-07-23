extern crate fnv;
extern crate csv;

use utils;
use fnv::FnvHashMap;

pub struct DataDictionary {
    user_dict: FnvHashMap<String,u32>,
    item_dict: FnvHashMap<String,u32>,
    num_interactions: u64,
}

impl DataDictionary {

    pub fn num_users(&self) -> usize {
        self.user_dict.len()
    }

    pub fn num_items(&self) -> usize {
        self.item_dict.len()
    }

    pub fn num_interactions(&self) -> u64 {
        self.num_interactions
    }

    pub fn user_index(&self, name: &str) -> &u32 {
        self.user_dict.get(name).unwrap()
    }

    pub fn item_index(&self, name: &str) -> &u32 {
        self.item_dict.get(name).unwrap()
    }
 }

impl DataDictionary {

    pub fn from(file: &str) -> Self {

        let mut csv_reader = utils::csv_reader(file);

        let mut user_index: u32 = 0;
        let mut user_dict: FnvHashMap<String,u32> =
            FnvHashMap::with_capacity_and_hasher(100, Default::default());

        let mut item_index: u32 = 0;
        let mut item_dict: FnvHashMap<String,u32> =
            FnvHashMap::with_capacity_and_hasher(100, Default::default());

        let mut num_interactions: u64 = 0;

        for record in csv_reader.decode() {
            let (user, item): (String, String) = record.unwrap();

            if !user_dict.contains_key(&user) {
                user_dict.insert(user, user_index);
                user_index += 1;
            }

            if !item_dict.contains_key(&item) {
                item_dict.insert(item, item_index);
                item_index += 1;
            }

            num_interactions += 1;
        }

        DataDictionary { user_dict, item_dict, num_interactions }
    }
}

pub struct Renaming {
    user_names: FnvHashMap<u32,String>,
    item_names: FnvHashMap<u32,String>,
}

impl Renaming {

    pub fn user_name(&self, user_index: u32) -> &str {
        &self.user_names[&user_index]
    }

    pub fn item_name(&self, item_index: u32) -> &str {
        &self.item_names[&item_index]
    }
}

impl From<DataDictionary> for Renaming {

    fn from(data_dict: DataDictionary) -> Self {

        let mut user_names: FnvHashMap<u32,String> =
            FnvHashMap::with_capacity_and_hasher(data_dict.num_users(), Default::default());

        let mut item_names: FnvHashMap<u32,String> =
            FnvHashMap::with_capacity_and_hasher(data_dict.num_items(), Default::default());

        for (user, user_id) in data_dict.user_dict.into_iter() {
            user_names.insert(user_id, user);
        }

        for (item, item_id) in data_dict.item_dict.into_iter() {
            item_names.insert(item_id, item);
        }

        Renaming { user_names, item_names }
    }
}