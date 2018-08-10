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

use types::SparseBinaryMatrix;
use stats::Renaming;

/// Reads a CSV input file. We expect NO headers, and a user-item tuple per line
/// with tab separation.
pub fn csv_reader(file: &str) -> Result<csv::Reader<std::fs::File>, csv::Error> {
    let reader = csv::Reader::from_file(file)?
        .has_headers(false)
        .delimiter('\t' as u8);

    Ok(reader)
}

pub fn interactions_from_csv<'a, R>(
    reader: &'a mut csv::Reader<R>
) -> impl Iterator<Item=(String, String)> + 'a
    where R: std::io::Read {

    reader.decode()
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
/// otherwise, we output to stdout.
pub fn write_indicators(
    indicators: &SparseBinaryMatrix,
    renaming: &Renaming,
    indicators_path: Option<String>,
) -> io::Result<()> {

    let mut out: Box<Write> = match indicators_path {
        Some(path) => Box::new(File::create(&Path::new(&path))?),
        _ => Box::new(stdout())
    };

    for (item_index, indicated_item_indices) in indicators.into_iter().enumerate() {

        let for_item = renaming.item_name(item_index as u32);

        let indicated_items: FnvHashSet<&str> = indicated_item_indices.into_iter()
            .map(|item_index| renaming.item_name(*item_index as u32))
            .collect();

        let indicators_as_json = json!(
            Indicators {
                for_item,
                indicated_items
            });

        write!(out, "{}\n", indicators_as_json.to_string())?;
    }

    Ok(())
}