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

#[cfg(test)]
mod tests {

    use super::super::indicators;
    use stats::{DataDictionary, Renaming};

    #[test]
    fn programmatic_usage() {

        /* Our input data comprises of observed interactions between users and items.
           The identifiers used can be strings of arbitrary length and structure. */
        let interactions = vec![
            (String::from("alice"), String::from("apple")),
            (String::from("alice"), String::from("dog")),
            (String::from("alice"), String::from("pony")),
            (String::from("bob"), String::from("apple")),
            (String::from("bob"), String::from("pony")),
            (String::from("charles"), String::from("pony")),
            (String::from("charles"), String::from("bike"))
        ];

        /* Internally, recoreco uses consecutive integer ids and requires some knowledge about the
           statistics of the data for efficient allocation. Therefore, we read the interaction data
           once to compute a data dictionary that helps us map from string to integer identifiers
           and has basic statistics of the data */
        let data_dict = DataDictionary::from(interactions.iter());

        println!(
            "Found {} interactions between {} users and {} items.",
            data_dict.num_interactions(),
            data_dict.num_users(),
            data_dict.num_items(),
        );

        /* Now we read the interactions a second time and compute the indicator matrix from item
           cooccurrences. The result is the so-called indicator matrix, where each entry indicates
           highly associated pairs of items. */
        let indicated_items = indicators(
            interactions.into_iter(),   // The observed interactions
            &data_dict, // The data dictionary which maps string to integer identifiers
            2,  // The number of CPUs to use for the computation
            10, // The number of highly associated items to compute per item
            500, // The maximum number of interactions to account for per user (use 500 as default)
            500 // The maximum number of interactions to account for per item (use 500 as default)
        );

        /* The renaming data structure helps us map the integer ids back to the original
           string ids. */
        let renaming = Renaming::from(data_dict);

        /* We print the resulting highly associated pairs of items. */
        for (item_index, indicated_items_for_item) in indicated_items.iter().enumerate() {
            let item_name = renaming.item_name(item_index as u32);
            println!("Items highly associated with {}:", item_name);

            for indicated_item_index in indicated_items_for_item.iter() {
                let indicated_item_name = renaming.item_name(*indicated_item_index as u32);
                println!("\t{}", indicated_item_name);
            }
        }

    }

}