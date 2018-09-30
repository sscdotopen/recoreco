//! ## Helper methods for dealing with interaction data
//!
//! This module provides a few convenience functions to consume interaction data stored in CSV
//! files, which by our experience is the most common data format for item interactions. Note that
//! these implementations do not assume that they can hold the whole CSV in memory.
//!
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

extern crate csv;
extern crate serde;
extern crate fnv;
extern crate serde_json;

use std;
use std::io;
use std::io::prelude::*;
use std::io::stdout;
use std::fs::File;
use std::path::Path;

use fnv::FnvHashSet;

use stats::Renaming;

/// Reads a CSV input file. We expect **NO headers**, and a **user-item pair per line**
/// with **tab separation**, which denotes an interaction between a user and this item, e.g.,
///
/// <pre>
/// alice&#9;apple
/// alice&#9;dog
/// alice&#9;pony
/// bob&#9;apple
/// bob&#9;pony
/// charles&#9;pony
/// charles&#9;bike
/// </pre>
pub fn csv_reader(file: &str) -> Result<csv::Reader<std::fs::File>, csv::Error> {
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_path(file)?;

    Ok(reader)
}

/// Converts a `csv::Reader` for an interaction file into an `Iterator<Item=(String, String)>` over
/// the contained interactions.
///
/// This iterator can be used to construct a `recoreco::stats::DataDictionary` via
/// `recoreco::stats::DataDictionary::from` or to compute  highly associated paris of items
/// via `recoreco::indicators`.
pub fn interactions_from_csv<'a, R>(
    reader: &'a mut csv::Reader<R>
) -> impl Iterator<Item=(String, String)> + 'a
    where R: std::io::Read {

    reader.deserialize()
        .filter_map(|result| {
            if result.is_ok() {
                // TODO handle potential errors here?
                let (user, item): (String, String) = result.unwrap();
                Some((user, item))
            } else {
                None
            }
        })
        .into_iter()
}

/// Struct used for JSON serialization of computed indicators. Field names will be used in JSON.
#[derive(Serialize)]
struct Indicators<'a> {
    for_item: &'a str,
    indicated_items: FnvHashSet<&'a str>,
}

/// Output the computed indicators in JSON format, using the original identifiers from the
/// inputfile. If an `indicators_path` is supplied, we write to a file at the specified path,
/// otherwise, we output to stdout. Each line holds a JSON representation 
///
/// `{ "for_item": "michael jackson", "indicated_items": ["justin timberlake", "queen"] }`
///
pub fn write_indicators(
    indicators: &[FnvHashSet<u32>],
    renaming: &Renaming,
    indicators_path: Option<String>,
) -> io::Result<()> {

    let mut out: Box<Write> = match indicators_path {
        Some(path) => Box::new(File::create(&Path::new(&path))?),
        _ => Box::new(stdout()),
    };

    for (item_index, indicated_item_indices) in indicators.into_iter().enumerate() {

        let for_item = renaming.item_name(item_index as u32);

        let indicated_items: FnvHashSet<&str> = indicated_item_indices
            .into_iter()
            .map(|item_index| renaming.item_name(*item_index as u32))
            .collect();

        let indicators_as_json = json!(
            Indicators {
                for_item,
                indicated_items
            });

        writeln!(out, "{}", indicators_as_json.to_string())?;
    }

    Ok(())
}