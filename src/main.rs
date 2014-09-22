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
struct Candidate<'a> {
    cursor: Cursor<'a, char>,
    strict: bool,
}

pub fn main() {
    let args: Args = FlagParser::parse().unwrap_or_else(|e| e.exit());
    let words_only = args.flag_words_only;
    let expand_case = args.flag_case_insensitive;

    // read in dictionary
    let dict_path = Path::new(args.arg_DICT);
    let mut dict_reader = BufferedReader::new(File::open(&dict_path));
    let mut dict: SuffixTree<char> = SuffixTree::new();
    for i in dict_reader.lines() {
        let t: Vec<char> = i.unwrap().as_slice().trim().chars().collect();
        dict.insert(t);
    }

    let expand = if expand_case {
        |ch: char| -> Vec<char> {
            if ch.is_lowercase() {
                vec!(ch.to_uppercase())
            } else {
                vec!(ch.to_lowercase())
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
            println!("{}\t{}\t{}\t{}", m.start, m.end,
                     String::from_chars(m.seq.as_slice()), m.strict);
        }
        println!("");
    }
}

struct Match {
    start: uint,
    end: uint,
    seq: Vec<char>,
    strict: bool,
}

fn find_matches<'a, Iter: Iterator<char>>
    (dict: &'a SuffixTree<char>,
     start_pred: |char| -> bool,
     expand: |char| -> Vec<char>,
     iter: Iter) -> Vec<Match> {

    let mut cands: Vec<Candidate> = Vec::new();
    let mut matches = Vec::new();
    let mut start = true;
    for (offset, ch) in iter.enumerate() {
        if start {
            cands.push(Candidate {cursor: Cursor::new(dict),
                                  strict: true});
            start = false;
        }

        cands = cands.into_iter().flat_map(|cand: Candidate<'a>| {
            let new_cands: Vec<Candidate> = match cand.cursor.clone().go(ch) {
                Some(next) => vec!(Candidate {cursor: next, strict: cand.strict}),
                None => {
                    let new: Vec<Candidate> =
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

    pub struct SuffixTree<E> {
        suffixes: TreeMap<E, SuffixTree<E>>,
        terminal: bool,
    }

    impl<E: Ord + Clone> SuffixTree<E> {
        pub fn new() -> SuffixTree<E> {
            SuffixTree {
                suffixes: TreeMap::new(),
                terminal: false,
            }
        }

        pub fn is_terminal(&self) -> bool {
            self.terminal
        }

        pub fn insert(&mut self, el: Vec<E>) {
            unsafe {
                let mut tree: *mut SuffixTree<E> = self;
                for i in el.into_iter() {
                    let new = match (*tree).suffixes.find_mut(&i) {
                        Some(next) => next as *mut SuffixTree<E>,
                        None => {
                            (*tree).suffixes.insert(i.clone(), SuffixTree::new());
                            (*tree).suffixes.find_mut(&i).unwrap() as *mut SuffixTree<E>
                        }
                    };
                    tree = new;
                }
                (*tree).terminal = true;
            }
        }
    }

    #[deriving(Clone)]
    pub struct Cursor<'a, E: 'a> {
        cursor: &'a SuffixTree<E>,
        pub path: Vec<E>,
    }

    impl<'a, E: Ord> Cursor<'a, E> {
        pub fn new(array: &'a SuffixTree<E>) -> Cursor<'a, E> {
            Cursor {
                cursor: array,
                path: Vec::new(),
            }
        }

        pub fn go(mut self, el: E) -> Option<Cursor<'a, E>> {
            match self.cursor.suffixes.find(&el) {
                Some(next) => {
                    self.cursor = next;
                    self.path.push(el);
                    Some(self)
                }
                None => None
            }
        }

        pub fn get(&self) -> &'a SuffixTree<E> {
            self.cursor
        }
    }
}
