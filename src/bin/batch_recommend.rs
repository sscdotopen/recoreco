extern crate csv;
extern crate recoreco;
extern crate num_cpus;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use recoreco::utils;
use recoreco::stats::{DataDictionary, Renaming};
use recoreco::recommend;

fn main() {

    let interactions_path = "/home/ssc/Entwicklung/projects/recoreco/examples/movietweets10K.csv";
    //let interactions_path = "/home/ssc/Entwicklung/datasets/tedandme/yahoo-music/songs.tsv";
    let indicators_path = "/home/ssc/Entwicklung/projects/recoreco/examples/indicators.csv";
    let recommendations_path =
        "/home/ssc/Entwicklung/projects/recoreco/examples/recommendations.csv";

    println!("Reading {} to compute data statistics (pass 1/3)", interactions_path);
    let data_dict = DataDictionary::from(interactions_path);

    println!(
        "Found {} interactions between {} users and {} items.",
        data_dict.num_interactions(),
        data_dict.num_users(),
        data_dict.num_items(),
    );


    println!("Reading {} to compute item indicators (pass 2/3)", interactions_path);
    let interactions = utils::read_interactions(&interactions_path, &data_dict);

    let indicators = recoreco::indicators(
        &interactions,
        data_dict.num_users(),
        data_dict.num_items(),
        num_cpus::get(),
        10,
    );



    println!("Reading {} to load user histories (pass 3/3)", interactions_path);
    let histories = utils::read_histories(&interactions_path, &data_dict);

    let renaming = Renaming::from(data_dict);

    println!("Writing indicators to {}", indicators_path);
    let mut indicators_file = File::create(&Path::new(indicators_path)).unwrap();

    for (item_index, indicated_items) in indicators.iter().enumerate() {
        let current_item = renaming.item_name(item_index as u32);

        write!(indicators_file, "{}", current_item).unwrap();

        for item in indicated_items.into_iter() {
            write!(indicators_file, "\t{}", renaming.item_name(*item as u32)).unwrap();
        }
        write!(indicators_file, "\n").unwrap();
    }

    println!("Writing recommendations to {}", indicators_path);
    let mut recommendations_file = File::create(&Path::new(recommendations_path)).unwrap();

    let recommendations = recommend::recommend(&histories, &indicators, 10);

    for (user, recommended_items) in recommendations.iter().enumerate() {

        write!(recommendations_file, "{}", renaming.user_name(user as u32)).unwrap();

        for item in recommended_items.into_iter() {
            write!(recommendations_file, "\t{}", renaming.item_name(*item as u32)).unwrap();
        }
        write!(recommendations_file, "\n").unwrap();
    }

}
