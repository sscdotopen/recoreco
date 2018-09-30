# recoreco
Fast item-to-item recommendations on the command line.

[![GitHub license](https://img.shields.io/github/license/sscdotopen/recoreco.svg)](https://github.com/sscdotopen/recoreco/blob/master/LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/sscdotopen/recoreco.svg)](https://github.com/sscdotopen/recoreco/issues)
[![Build Status](https://travis-ci.org/sscdotopen/recoreco.svg?branch=master)](https://travis-ci.org/sscdotopen/recoreco)
[![](http://meritbadge.herokuapp.com/recoreco)](https://crates.io/crates/recoreco)

## Installation

Currently, the only convenient way to install **recoreco** is via Rust's package manager [cargo](https://github.com/rust-lang/cargo):

```
$ cargo install recoreco
```

## Quickstart

**Recoreco** computes highly associated pairs of items (in the sense of _'people who are interested in X are also interested in Y'_) from interactions between users and items. 

It is a command line tool that expects a **CSV file** as input, where each line denotes an interaction between a user and an item and consists of a **user identifier** and an **item identifier** separated by a **tab character**. **Recoreco** by default outputs 10 associated items per item (with no particular ranking) in JSON format.

If you would like to learn a bit more about the math behind the approach that **recoreco** is built on, checkout the book on [practical machine learning: innovations in recommendation](https://mapr.com/practical-machine-learning/) and the talk on [real-time puppies and ponies](https://www.slideshare.net/tdunning/realtime-puppies-and-ponies-evolving-indicator-recommendations-in-realtime) from my friend [Ted Dunning](https://twitter.com/ted_dunning). 


## Example: Finding related music artists with recoreco

As an example, we will compute related artists from a [music dataset](http://www.dtic.upf.edu/~ocelma/MusicRecommendationDataset/lastfm-360K.html) crawled from last.fm. The data contains 17,535,655 interactions between 358,868 users and 292,365 bands.

As a first step, we download the data, uncompress it and have a look at the format:  
`$ wget http://mtg.upf.edu/static/datasets/last.fm/lastfm-dataset-360K.tar.gz`  
`$ tar xvfz lastfm-dataset-360K.tar.gz`

```
$ head lastfm-dataset-360K/usersha1-artmbid-artname-plays.tsv
00000c289a1829a808ac09c00daf10bc3c4e223b	3bd73256-3905-4f3a-97e2-8b341527f805	betty blowtorch	2137
00000c289a1829a808ac09c00daf10bc3c4e223b	f2fb0ff0-5679-42ec-a55c-15109ce6e320	die Ärzte	1099
00000c289a1829a808ac09c00daf10bc3c4e223b	b3ae82c2-e60b-4551-a76d-6620f1b456aa	melissa etheridge	897
00000c289a1829a808ac09c00daf10bc3c4e223b	3d6bbeb7-f90e-4d10-b440-e153c0d10b53	elvenking	717
00000c289a1829a808ac09c00daf10bc3c4e223b	bbd2ffd7-17f4-4506-8572-c1ea58c3f9a8	juliette & the licks	706
```

We need our inputs to only consist of user and item interactions, so we create a new CSV file which only contains the first column (the hashed userid) and the third column (the artist name) from the original data:

`$ cat lastfm-dataset-360K/usersha1-artmbid-artname-plays.tsv|cut -f1,3 > plays.csv`

Now the CSV file is in the correct format:

```
$ head plays.csv 
00000c289a1829a808ac09c00daf10bc3c4e223b	betty blowtorch
00000c289a1829a808ac09c00daf10bc3c4e223b	die Ärzte
00000c289a1829a808ac09c00daf10bc3c4e223b	melissa etheridge
00000c289a1829a808ac09c00daf10bc3c4e223b	elvenking
00000c289a1829a808ac09c00daf10bc3c4e223b	juliette & the licks
```

Next, we invoke **recoreco**, point it to the CSV file as input and ask it to write the output to a file called `artists.json`. It will read the CSV file twice, once for computing some statistics of the data, and a second time for computing the actual item-to-item recommendations. Note that **recoreco** is pretty fast, the computation takes less than a minute on my machine.

```
$ recoreco --inputfile=plays.csv --outputfile=artists.json

Reading plays.csv to compute data statistics (pass 1/2)
Found 17535655 interactions between 358868 users and 292365 items.
Reading plays.csv to compute 10 item indicators per item (pass 2/2)
194996130 cooccurrences observed, 34015ms training time, 292365 items rescored
Writing indicators...
```
The file `artists.json` now contains the results of the computation. Let's have a look at some artist recommendations using the JSON processor [jq](https://stedolan.github.io/jq/).

Who is strongly associated with _Michael Jackson_?

`$ jq 'select(.for_item=="michael jackson")' artists.json`

```json
{
  "for_item": "michael jackson",
  "indicated_items": [
    "justin timberlake",
    "queen",
    "kanye west",
    "amy winehouse",
    "britney spears",
    "madonna",
    "rihanna",
    "beyoncé",
    "daft punk",
    "u2"
  ]
}
```

One of my favorite bands is [Hot Water Music](https://www.youtube.com/watch?v=UsJ7zlwJnDg), lets see bands that people associate with them:

`$ jq 'select(.for_item=="hot water music")' artists.json`

```json
{
  "for_item": "hot water music",
  "indicated_items": [
    "lifetime",
    "the get up kids",
    "the lawrence arms",
    "the gaslight anthem",
    "dillinger four",
    "propagandhi",
    "the bouncing souls",
    "strike anywhere",
    "jawbreaker",
    "chuck ragan"
  ]
}

```

And finally, we look for artists similar to [Paco de Lucia](https://en.wikipedia.org/wiki/Paco_de_Luc%C3%ADa) in homage to Ted's days of building search engines for Veoh :)

`$ jq 'select(.for_item=="paco de lucia")' artists.json`

```json
{
  "for_item": "paco de lucia",
  "indicated_items": [
    "miguel poveda",
    "cserhati zsuzsa",
    "ramón veloz",
    "szarka tamás",
    "camaron de la isla",
    "cseh tamás - másik jános",
    "duquende",
    "amr diab",
    "chuck brown & eva cassidy",
    "keympa"
  ]
}
```

## Programmatic Usage

**recoreco** can also be included as a library in your rust program. We provide a [basic example](src/usage_tests.rs) on how to do this. Be sure to checkout the [documentation](https://docs.rs/recoreco/0.1.6/recoreco/) for further details.
