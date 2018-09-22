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
extern crate csv;

use fnv::FnvHashMap;

/// Maps from string identifiers to internal indexes.
///
/// Many interaction datasets contain string identifiers, however we want to internally work with
/// consecutive integer ids. We therefore keep track of the string identifiers of users and items
/// as well as the overall number of interactions
pub struct DataDictionary {
    user_dict: FnvHashMap<String, u32>,
    item_dict: FnvHashMap<String, u32>,
    num_interactions: u64,
}

/// Accessor methods for the data dictionary
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

    /// Builds up the data dictionary by consuming an iterator over string tuples representing
    /// user-item interactions. We assume that the first string in the tuple identifies a user and
    /// the second string identifies an item
    pub fn from_owned<T>(interactions: T) -> Self
        where T: Iterator<Item = (String, String)>
    {
        let mut user_index: u32 = 0;
        let mut user_dict: FnvHashMap<String, u32> = FnvHashMap::default();

        let mut item_index: u32 = 0;
        let mut item_dict: FnvHashMap<String, u32> = FnvHashMap::default();

        let mut num_interactions: u64 = 0;

        for (user, item) in interactions {

            user_dict.entry(user).or_insert_with(|| {
                let current_user_index = user_index;
                user_index += 1;
                current_user_index
            });

            item_dict.entry(item).or_insert_with(|| {
                let current_item_index = item_index;
                item_index += 1;
                current_item_index
            });

            num_interactions += 1;
        }

        DataDictionary { user_dict, item_dict, num_interactions }
    }

    /// Builds up the data dictionary by reading an iterator over string tuples representing
    /// user-item interactions. We assume that the first string in the tuple identifies a user and
    /// the second string identifies an item
    pub fn from<'a,T>(interactions: T) -> DataDictionary
        where T: Iterator<Item = &'a(String, String)> {

        let owned = interactions
            .map(|(user, item)| (user.to_owned(), item.to_owned()));

        DataDictionary::from_owned(owned)
    }

}


impl <T> From<T> for DataDictionary
    where T: Iterator<Item = (String, String)>
{
    fn from(iter: T) -> Self {
        let mut user_index: u32 = 0;
        let mut user_dict: FnvHashMap<String, u32> = FnvHashMap::default();

        let mut item_index: u32 = 0;
        let mut item_dict: FnvHashMap<String, u32> = FnvHashMap::default();

        let mut num_interactions: u64 = 0;

        for (user, item) in iter {

            user_dict.entry(user).or_insert_with(|| {
                let current_user_index = user_index;
                user_index += 1;
                current_user_index
            });

            item_dict.entry(item).or_insert_with(|| {
                let current_item_index = item_index;
                item_index += 1;
                current_item_index
            });

            num_interactions += 1;
        }

        DataDictionary { user_dict, item_dict, num_interactions }
    }
}


pub struct Renaming {
    item_names: FnvHashMap<u32, String>,
}

impl Renaming {

    /// Reverse mapping from internal indexes to original item names
    pub fn item_name(&self, item_index: u32) -> &str {
        &self.item_names[&item_index]
    }
}

/// Consume a DataDictionary to produce a Renaming for the reverse mapping
impl From<DataDictionary> for Renaming {

    fn from(data_dict: DataDictionary) -> Self {
        let item_names: FnvHashMap<u32, String> = data_dict.item_dict
            .into_iter()
            .map(|(name, item_id)| (item_id, name))
            .collect(); // Checked that size_hint() gives correct bounds

        Renaming { item_names }
    }
}


#[cfg(test)]
mod tests {

    extern crate fnv;

    use stats::{DataDictionary, Renaming};
    use fnv::FnvHashMap;

    #[test]
    fn dict_from_tuple_iterator() {

        let interactions = vec![
            ("user_a".to_string(), "item_a".to_string()),
            ("user_a".to_string(), "item_b".to_string()),
            ("user_b".to_string(), "item_b".to_string()),
            ("user_c".to_string(), "item_a".to_string()),
        ];

        let data_dict = DataDictionary::from(interactions.iter());

        assert_eq!(data_dict.num_users(), 3);
        assert_eq!(data_dict.num_items(), 2);
        assert_eq!(data_dict.num_interactions(), 4);

        assert_eq!(*data_dict.user_index("user_a"), 0);
        assert_eq!(*data_dict.user_index("user_c"), 2);

        assert_eq!(*data_dict.item_index("item_a"), 0);
        assert_eq!(*data_dict.item_index("item_b"), 1);

        // Make sure we don't lose ownership of interactions
        assert_eq!(interactions.len(), 4);
    }

    #[test]
    fn dict_from_owned_tuple_iterator() {

        let interactions = vec![
            ("user_a".to_string(), "item_a".to_string()),
            ("user_a".to_string(), "item_b".to_string()),
            ("user_b".to_string(), "item_b".to_string()),
            ("user_c".to_string(), "item_a".to_string()),
        ];

        let data_dict = DataDictionary::from_owned(interactions.into_iter());

        assert_eq!(data_dict.num_users(), 3);
        assert_eq!(data_dict.num_items(), 2);
        assert_eq!(data_dict.num_interactions(), 4);

        assert_eq!(*data_dict.user_index("user_a"), 0);
        assert_eq!(*data_dict.user_index("user_c"), 2);

        assert_eq!(*data_dict.item_index("item_a"), 0);
        assert_eq!(*data_dict.item_index("item_b"), 1);
    }

    #[test]
    fn renaming_from_dict() {

        let user_mapping = vec![
            ("user_a".to_string(), 0),
            ("user_b".to_string(), 1),
        ];

        let item_mapping = vec![
            ("item_a".to_string(), 0),
            ("item_b".to_string(), 1),
            ("item_c".to_string(), 2),
        ];

        let user_dict: FnvHashMap<String,u32> = user_mapping.into_iter().collect();
        let item_dict: FnvHashMap<String,u32> = item_mapping.into_iter().collect();

        let data_dict = DataDictionary { user_dict, item_dict, num_interactions: 10 };

        let renaming: Renaming = data_dict.into();

        assert_eq!(renaming.item_name(0), "item_a");
        assert_eq!(renaming.item_name(1), "item_b");
        assert_eq!(renaming.item_name(2), "item_c");
    }
}