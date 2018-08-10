# recoreco
Fast item-to-item recommendations on the command line.

## Quickstart

TBD.

## Example: Finding related music artists with recoreco

`$ git clone https://github.com/sscdotopen/recoreco.git && cd recoreco`

[http://www.dtic.upf.edu/~ocelma/MusicRecommendationDataset/lastfm-360K.html](http://www.dtic.upf.edu/~ocelma/MusicRecommendationDataset/lastfm-360K.html)

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

`$ cat lastfm-dataset-360K/usersha1-artmbid-artname-plays.tsv|cut -f1,3 > plays.csv`

```
$ head plays.csv 
00000c289a1829a808ac09c00daf10bc3c4e223b	betty blowtorch
00000c289a1829a808ac09c00daf10bc3c4e223b	die Ärzte
00000c289a1829a808ac09c00daf10bc3c4e223b	melissa etheridge
00000c289a1829a808ac09c00daf10bc3c4e223b	elvenking
00000c289a1829a808ac09c00daf10bc3c4e223b	juliette & the licks
```


```
$ cargo run --release -- --inputfile=plays.csv --outputfile=artists.json

Reading plays.csv to compute data statistics (pass 1/2)
Found 17535655 interactions between 358868 users and 292365 items.
Reading plays.csv to compute 10 item indicators per item (pass 2/2)
194996130 cooccurrences observed, 34015ms training time, 292365 items rescored
Writing indicators...
```

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


`$ jq 'select(.for_item=="black flag")' artists.json`

```json
{
  "for_item": "black flag",
  "indicated_items": [
    "minutemen",
    "misfits",
    "circle jerks",
    "hüsker dü",
    "dead kennedys",
    "minor threat",
    "fugazi",
    "descendents",
    "bad brains",
    "adolescents"
  ]
}
```

## Background

TBD.

