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
extern crate recoreco;
extern crate num_cpus;
extern crate getopts;

use std::error::Error;
use std::env;
use getopts::Options;

use recoreco::io;
use recoreco::stats::{DataDictionary, Renaming};

fn main() {

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("i", "inputfile", "Input file name (required). The input consists of interactions \
        between user and items. The input file must contain a user and item pair per line, \
        separated by a tab.", "PATH");
    opts.optopt("o", "outputfile", "Output file name (optional, output will be written to stdout \
        by default).", "PATH");
    opts.optopt("n", "num-indicators", "Number of indicators to compute per item (optional, \
        defaults to 10).", "NUMBER");
    opts.optflag("h", "help", "Print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(matches) => matches,
        Err(failure) => {
            let hint = failure.to_string();
            return print_usage_and_exit(&program, opts, Some(&hint))
        },
    };

    if matches.opt_present("h") {
        return print_usage_and_exit(&program, opts, None);
    }

    if !matches.opt_present("i") {
        return print_usage_and_exit(
            &program,
            opts,
            Some("Please specify an inputfile via --inputfile."),
        );
    }

    let interactions_path = matches.opt_str("i").unwrap();
    let indicators_path = matches.opt_str("o");

    let k: usize = match matches.opt_get_default("n", 10) {
        Ok(k) => k,
        Err(failure) => {
            let hint = format!("Problem with option 'n': {}", failure.to_string());
            return print_usage_and_exit(&program, opts, Some(&hint))
        },
    };

    compute_indicators(&interactions_path, k, indicators_path).unwrap();
}

fn print_usage_and_exit(
    program: &str,
    opts: Options,
    hint: Option<&str>
) {

    if let Some(hint) = hint {
        eprintln!("\n{}\n", hint);
    }

    let brief = format!("Usage: {} [options]", program);
    eprint!("{}", opts.usage(&brief));
}

fn compute_indicators(
    interactions_path: &str,
    n: usize,
    indicators_path: Option<String>
) -> Result<(), Box<Error>> {

    // We use constants here for the moment, should result in a good runtime/quality ratio.
    const F_MAX: u32 = 500;
    const K_MAX: u32 = 500;

    println!("Reading {} to compute data statistics (pass 1/2)", interactions_path);

    let reader_pass_one = io::csv_reader(&interactions_path)?;
    let data_dict = DataDictionary::from(reader_pass_one);

    println!(
        "Found {} interactions between {} users and {} items.",
        data_dict.num_interactions(),
        data_dict.num_users(),
        data_dict.num_items(),
    );

    println!("Reading {} to compute {} item indicators per item (pass 2/2)", interactions_path, n);

    let mut reader_pass_two = io::csv_reader(&interactions_path)?;
    let interactions = io::interactions_from_csv(&mut reader_pass_two);

    let indicators = recoreco::indicators(
        interactions,
        &data_dict,
        num_cpus::get(),
        n,
        F_MAX,
        K_MAX
    );

    // Build reverse index, make sure we consume the data dictionary
    let renaming: Renaming = data_dict.into();

    println!("Writing indicators...");
    recoreco::io::write_indicators(&indicators, &renaming, indicators_path)?;

    Ok(())
}
