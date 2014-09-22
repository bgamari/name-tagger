#![feature(phase)]

extern crate collections;
extern crate serialize;
#[phase(plugin)] extern crate docopt_macros;
extern crate docopt;

use std::io::{BufferedReader, File};
use docopt::FlagParser;
use suffix_tree::{SuffixTree, Cursor};

docopt!(Args, "
Usage: name-tagger [-w] [-i] DICT

Options:
    -w, --words-only        Only allow matches to start on word boundaries
    -i, --case-insensitive  Only allow matches to start on word boundaries
")

#[deriving(Clone)]
struct Candidate<'a, V: 'a> {
    cursor: Cursor<'a, char, V>,
    strict: bool,
}


fn is_punctuation(ch: char) -> bool {
    let punct = &"/|-.\\:,;";
    punct.contains_char(ch)
}

pub fn main() {
    let args: Args = FlagParser::parse().unwrap_or_else(|e| e.exit());
    let words_only = args.flag_words_only;
    let expand_case = args.flag_case_insensitive;

    // read in dictionary
    let dict_path = Path::new(args.arg_DICT);
    let mut dict_reader = BufferedReader::new(File::open(&dict_path));
    let mut dict: SuffixTree<char, String> = SuffixTree::new();
    for i in dict_reader.lines() {
        let i = i.unwrap();
        let parts: Vec<&str> = i.as_slice().trim_right_chars('\n').splitn(1, '\t').collect();
        match parts.len() {
            2 => {
                let t: Vec<char> = parts[1].chars().collect();
                let fuzzy = t.iter().map(|c| if is_punctuation(*c) { '.' } else { c.to_lowercase() })
                    .collect();
                dict.insert(fuzzy, parts[0].to_string());
                dict.insert(t, parts[0].to_string());
            },
            _ => {}
        }
    }

    let expand = if expand_case {
        |ch: char| -> Vec<char> {
            if ch.is_lowercase() {
                vec!(ch.to_uppercase())
            } else if ch.is_uppercase() {
                vec!(ch.to_lowercase())
            } else if is_punctuation(ch) {
                vec!('.')
            } else {
                vec!()
            }
        }
    } else {
        |_| Vec::new()
    };

    let start_pred = if words_only {
        |c: char| c.is_whitespace()
    } else {
        |_| true
    };

    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let matches = find_matches(&dict, |c| start_pred(c),
                                   |c| expand(c), line.as_slice().chars());

        for m in matches.iter() {
            println!("{}\t{}\t{}\t{}\t{}", m.start, m.end,
                     String::from_chars(m.seq.as_slice()), m.strict,
                     m.node.value.as_ref().unwrap());
        }
        println!("");
    }
}

struct Match<'a, V: 'a> {
    start: uint,
    end: uint,
    seq: Vec<char>,
    node: &'a SuffixTree<char, V>,
    strict: bool,
}

fn find_matches<'a, Iter: Iterator<char>, V>
    (dict: &'a SuffixTree<char, V>,
     start_pred: |char| -> bool,
     expand: |char| -> Vec<char>,
     iter: Iter) -> Vec<Match<'a, V>> {

    let mut cands: Vec<Candidate<V>> = Vec::new();
    let mut matches: Vec<Match<V>> = Vec::new();
    let mut start = true;
    for (offset, ch) in iter.enumerate() {
        if start {
            cands.push(Candidate {cursor: Cursor::new(dict),
                                  strict: true});
            start = false;
        }

        cands = cands.into_iter().flat_map(|cand: Candidate<'a, V>| {
            let new_cands: Vec<Candidate<V>> = match cand.cursor.clone().go(ch) {
                Some(next) => vec!(Candidate {cursor: next, strict: cand.strict}),
                None => {
                    let new: Vec<Candidate<V>> =
                        expand(ch).into_iter().filter_map(|ex_ch| {
                            match cand.cursor.clone().go(ex_ch) {
                                Some(ex_cur) => Some(Candidate {cursor: ex_cur,
                                                                strict: false}),
                                None => None,
                            }
                        }).collect();
                    new
                }
            };
            new_cands.into_iter()
        }).collect();

        for cand in cands.iter() {
            if cand.cursor.get().is_terminal() {
                // we have a hit
                matches.push(Match{
                    start: 1 + offset - cand.cursor.path.len(),
                    end: 1 + offset,
                    seq: cand.cursor.path.clone(),
                    node: cand.cursor.get(),
                    strict: cand.strict,
                });
            }
        }
        if start_pred(ch) {
            start = true;
        }
    }
    matches
}

pub mod suffix_tree {
    use collections::treemap::TreeMap;

    pub struct SuffixTree<E, V> {
        suffixes: TreeMap<E, SuffixTree<E, V>>,
        pub value: Option<V>,
    }

    impl<E: Ord + Clone, V> SuffixTree<E, V> {
        pub fn new() -> SuffixTree<E, V> {
            SuffixTree {
                suffixes: TreeMap::new(),
                value: None,
            }
        }

        pub fn is_terminal(&self) -> bool {
            self.value.is_some()
        }

        pub fn insert(&mut self, el: Vec<E>, value: V) {
            unsafe {
                let mut tree: *mut SuffixTree<E, V> = self;
                for i in el.into_iter() {
                    let new = match (*tree).suffixes.find_mut(&i) {
                        Some(next) => next as *mut SuffixTree<E, V>,
                        None => {
                            (*tree).suffixes.insert(i.clone(), SuffixTree::new());
                            (*tree).suffixes.find_mut(&i).unwrap() as *mut SuffixTree<E, V>
                        }
                    };
                    tree = new;
                }
                (*tree).value = Some(value);
            }
        }
    }

    pub struct Cursor<'a, E: 'a, V: 'a> {
        cursor: &'a SuffixTree<E, V>,
        pub path: Vec<E>,
    }

    impl<'a, E: Clone, V> Clone for Cursor<'a, E, V> {
        fn clone(&self) -> Cursor<'a, E, V> {
            Cursor {cursor: self.cursor, path: self.path.clone()}
        }
    }

    impl<'a, E: Ord, V> Cursor<'a, E, V> {
        pub fn new(array: &'a SuffixTree<E, V>) -> Cursor<'a, E, V> {
            Cursor {
                cursor: array,
                path: Vec::new(),
            }
        }

        pub fn go(mut self, el: E) -> Option<Cursor<'a, E, V>> {
            match self.cursor.suffixes.find(&el) {
                Some(next) => {
                    self.cursor = next;
                    self.path.push(el);
                    Some(self)
                }
                None => None
            }
        }

        pub fn get(&self) -> &'a SuffixTree<E, V> {
            self.cursor
        }
    }
}
