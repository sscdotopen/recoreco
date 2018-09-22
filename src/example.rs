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

extern crate recoreco;

use recoreco::stats::{Renaming,DataDictionary};

fn main() {

    // Read the data to create a dictionary of consecutive ids
    let data_dict = DataDictionary::from(read_interactions().into_iter());

    println!(
        "Found {} interactions between {} users and {} items.",
        data_dict.num_interactions(),
        data_dict.num_users(),
        data_dict.num_items(),
    );

    // Compute the indicators (highly associated items per item)
    let indicators = recoreco::indicators(
        read_interactions().into_iter(),
        &data_dict,
        2,
        10,
        500,
        500
    );

    // Restores original item names
    let renaming = Renaming::from(data_dict);
    
    for (item_index, item_indicators) in indicators.iter().enumerate() {
        let item_name = renaming.item_name(item_index as u32);
        println!("Indicators for {}:", item_name);

        for indicated_item_index in item_indicators.iter() {
            let indicated_item_name = renaming.item_name(*indicated_item_index);
            println!("\t{}", indicated_item_name);
        }
    }

}

fn read_interactions() -> Vec<(String, String)> {
    vec![
        ("user_a".to_string(), "item_a".to_string()),
        ("user_a".to_string(), "item_b".to_string()),
        ("user_b".to_string(), "item_b".to_string()),
        ("user_c".to_string(), "item_a".to_string()),
    ]
}