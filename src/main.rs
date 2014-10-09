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
    -w, --whole-name        Forbid matches of substrings of names
    -i, --insensitive       Permit matches to differ from name in case and punctuation
")

#[deriving(Clone)]
struct Candidate<'a, V: 'a> {
    cursor: Cursor<'a, char, V>,
}


fn is_punctuation(ch: char) -> bool {
    let punct = &"/|-.\\:,;+()";
    punct.contains_char(ch)
}

#[deriving(Show)]
enum TermType {
    Exact, Fuzzy, WholeWord, FuzzyWholeWord
}

type STree = SuffixTree<char, (TermType, String)>;

pub fn normalize(ch: char) -> char {
    if is_punctuation(ch) {
        '.'
    } else {
        ch.to_lowercase()
    }
}

pub fn main() {
    let args: Args = FlagParser::parse().unwrap_or_else(|e| e.exit());
    let name_only = args.flag_whole_name;
    let fuzzy = args.flag_insensitive;

    // read in dictionary
    let dict_path = Path::new(args.arg_DICT);
    let mut dict_reader = BufferedReader::new(File::open(&dict_path));
    let mut dict: STree = SuffixTree::new();
    for i in dict_reader.lines() {
        let i = i.unwrap();
        let parts: Vec<&str> = i.as_slice().trim_right_chars('\n').splitn(1, '\t').collect();
        match parts.len() {
            2 => {
                let t: Vec<char> = parts[1].chars().collect();
                if name_only {
                    dict.insert(Some(' ').into_iter()
                                .chain(t.clone().into_iter())
                                .chain(Some(' ').into_iter()),
                                (WholeWord, parts[0].to_string()));

                    if fuzzy {
                        let normalized = t.into_iter().map(|ch| normalize(ch));
                        dict.insert(Some(' ').into_iter().chain(normalized).chain(Some(' ').into_iter()),
                                    (FuzzyWholeWord, parts[0].to_string()));
                    }
                } else {
                    dict.insert(t.clone().into_iter(), (Exact, parts[0].to_string()));
                    if fuzzy {
                        let normalized = t.into_iter().map(|ch| normalize(ch));
                        dict.insert(normalized, (Fuzzy, parts[0].to_string()));
                    }
                }
            },
            _ => {}
        }
    }

    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let line = line.as_slice().trim_right_chars('\n');
        let matches =
            find_matches(&dict,
                         Some(' ').into_iter()
                         .chain(line.chars())
                         .chain(Some(' ').into_iter()));

        for m in matches.into_iter() {
            let &(ref ty, ref value) = m.node.value.as_ref().unwrap();
            println!("{}\t{}\t{}\t{}\t{}\t{}",
                     m.start - 1, m.end - 1,
                     String::from_chars(m.seq.as_slice()), true,
                     ty, value);
        }

        let matches =
            find_matches(&dict,
                         Some(' ').into_iter()
                         .chain(line.chars().map(|ch| normalize(ch)))
                         .chain(Some(' ').into_iter()));

        for m in matches.into_iter() {
            let &(ref ty, ref value) = m.node.value.as_ref().unwrap();
            println!("{}\t{}\t{}\t{}\t{}\t{}",
                     m.start - 1, m.end - 1,
                     String::from_chars(m.seq.as_slice()), false,
                     ty, value);
        }
        println!("");
    }
}

struct Match<'a, V: 'a> {
    start: uint,
    end: uint,
    seq: Vec<char>,
    node: &'a SuffixTree<char, V>,
}

fn find_matches<'a, Iter: Iterator<char>, V>
    (dict: &'a SuffixTree<char, V>,
     iter: Iter) -> Vec<Match<'a, V>> {

    let mut cands: Vec<Candidate<V>> = Vec::new();
    let mut matches: Vec<Match<V>> = Vec::new();
    for (offset, ch) in iter.enumerate() {
        cands.push(Candidate {cursor: Cursor::new(dict)});

        cands = cands.into_iter().flat_map(|cand: Candidate<'a, V>| {
            match cand.cursor.clone().go(ch) {
                Some(next) => vec!(Candidate {cursor: next}),
                None => vec!(),
            }.into_iter()
        }).collect();

        for cand in cands.iter() {
            if cand.cursor.get().is_terminal() {
                // we have a hit
                matches.push(Match{
                    start: 1 + offset - cand.cursor.path.len(),
                    end: 1 + offset,
                    seq: cand.cursor.path.clone(),
                    node: cand.cursor.get(),
                });
            }
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

        pub fn insert<Iter: Iterator<E>>(&mut self, mut el: Iter, value: V) {
            unsafe {
                let mut tree: *mut SuffixTree<E, V> = self;
                for i in el {
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
