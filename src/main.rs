#![feature(plugin, collections,unicode)]
#![plugin(docopt_macros)]

extern crate collections;

extern crate docopt;
extern crate rustc_serialize;
extern crate unicode;

use std::io::{BufReader, BufRead};
use std::fs::File;
use std::path::Path;
use suffix_tree::{SuffixTree, Cursor};

docopt!(Args, "
Usage: name-tagger [-w] [-i] DICT

Options:
    -w, --whole-name        Forbid matches of substrings of names
    -i, --insensitive       Permit matches to differ from name in case and punctuation
");

#[derive(Clone)]
struct Candidate<'a, V: 'a> {
    cursor: Cursor<'a, char, V>,
    start: usize,
}


fn is_punctuation(ch: char) -> bool {
    let punct = &"/|-.\\:,;+()";
    punct.contains(ch)
}

#[derive(Debug, Copy)]
enum TermType {
    Exact, Fuzzy, WholeWord, FuzzyWholeWord, WholeWordWithSymbols
}

type STree = SuffixTree<char, (TermType, String)>;

//let norma = str.map(|ch| if is_punctuation(ch) { '.' } else { ch }).flat_map(|ch| ch.to_lowercase())


fn normalize<'a, Iter: Iterator<Item=char> + 'a>(str: Iter) -> Box<Iterator<Item=char> + 'a> {
    Box::new(
	    str.map(|ch| if is_punctuation(ch) { '.' } else { ch })
	    .flat_map(|ch| ch.to_lowercase())
    )
}

pub fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let name_only = args.flag_whole_name;
    let fuzzy = args.flag_insensitive;

    // read in dictionary
    let dict_path = Path::new(&args.arg_DICT);
    let dict_reader = BufReader::new(File::open(&dict_path).unwrap());
    let mut dict: STree = SuffixTree::new();
    for i in dict_reader.lines() {
        let i = i.unwrap();
        let parts: Vec<&str> = i.trim_right_matches('\n').splitn(1, '\t').collect();
        match parts.len() {
            2 => {
                let t: Vec<char> = parts[1].chars().collect();
                if name_only {
                    dict.insert(Some(' ').into_iter()
                                .chain(t.clone().into_iter())
                                .chain(Some(' ').into_iter()),
                                (TermType::WholeWord, parts[0].to_string()));

                    if fuzzy {
                        let normalized = normalize(t.into_iter());
                        dict.insert(Some(' ').into_iter().chain(normalized).chain(Some(' ').into_iter()),
                                    (TermType::FuzzyWholeWord, parts[0].to_string()));
                    }
                } else {
                    dict.insert(t.clone().into_iter(), (TermType::Exact, parts[0].to_string()));
                    if fuzzy {
                        let normalized = normalize(t.into_iter());
                        dict.insert(normalized, (TermType::Fuzzy, parts[0].to_string()));
                    }
                }
            },
            _ => {}
        }
    }

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        use std::iter::FromIterator;
        let line = line.unwrap();
        let line = line.trim_right_matches('\n');
        let matches =
            find_matches(&dict,
                         Some(' ').into_iter()
                         .chain(line.chars())
                         .chain(Some(' ').into_iter())
            );

        for m in matches.into_iter() {
            let &(ty, ref value) = m.node.value.as_ref().unwrap();
            let seq: String = FromIterator::from_iter(m.seq.into_iter());
            println!("{}\t{}\t{}\t{}\t{:?}\t{}",
                     m.start - 1, m.end - 1, seq, true, ty, value);
        }

        let matches =
            find_matches(&dict,
                         Some(' ').into_iter()
                         .chain(normalize(line.chars()))
                         .chain(Some(' ').into_iter())
            );

        for m in matches.into_iter() {
            let &(ty, ref value) = m.node.value.as_ref().unwrap();
            let seq: String = FromIterator::from_iter(m.seq.into_iter());
            println!("{}\t{}\t{}\t{}\t{:?}\t{}",
                     m.start - 1, m.end - 1, seq, false, ty, value);
        }
        println!("");
    }
}

struct Match<'a, V: 'a> {
    start: usize,
    end: usize,
    seq: Vec<char>,
    node: &'a SuffixTree<char, V>,
}

fn find_matches<'a, Iter: Iterator<Item=char>, V>
    (dict: &'a SuffixTree<char, V>,
     input: Iter) -> Vec<Match<'a, V>> {

    let mut cands: Vec<Candidate<V>> = Vec::new();
    let mut matches: Vec<Match<V>> = Vec::new();
    for (offset, ch) in input.enumerate() {
		cands.push(Candidate {cursor: Cursor::new(dict), start: offset, term_type_downgraded = false});

		cands = cands.into_iter().flat_map(|cand: Candidate<'a, V>| {
			match cand.cursor.clone().go(ch) {
				Some(next) => vec!(Candidate {cursor: next, start: cand.start, term_type_downgraded = cand.term_type_downgraded}),
				None => vec!(),
//                None => if(skipSymbol) { vec!(Candidate {cursor: next, start: cand.start, term_type_downgraded = true}) }
			}.into_iter()
		}).collect();

		for cand in cands.iter() {
			if cand.cursor.get().is_terminal() {
				// we have a hit
				matches.push(Match{
					start: cand.start,
					end: 1 + offset,
					seq: cand.cursor.path.clone(),
					node: cand.cursor.get(),
				});
			}
		}
    }
    matches
}


trait Matcher {
    fn feed(offset:usize, ch:char) -> Vec<Match>;
}

struct ExactMatcher {
    tree: &SuffixTree,
	cands: Vec<Candidate>,
}
impl Matcher for ExactMatcher {
    fn feed(offset:usize, ch:char) -> Vec<Match> {
        let mut matches = Vec::new();
		cands.push(Candidate {cursor: Cursor::new(dict), start: offset});

		cands = cands.into_iter().flat_map(|cand: Candidate<'a, V>| {
			match cand.cursor.clone().go(ch) {
				Some(next) => vec!(Candidate {cursor: next, start: cand.start}),
				None => vec!(),
				//                None => if(skipSymbol) { vec!(Candidate {cursor: next, start: cand.start, term_type_downgraded = true}) }
			}.into_iter()
		}).collect();

		for cand in cands.iter() {
			if cand.cursor.get().is_terminal() {
				// we have a hit
				matches.push(Match{
					start: cand.start,
					end: 1 + offset,
					seq: cand.cursor.path.clone(),
					node: cand.cursor.get(),
				});
			}
		}
    }
}

struct Candidate<'a, V: 'a> {
cursor: Cursor<'a, char, V>,
start: usize,
}

fn consume_iterator<Iter: Iterator<Item=char>>(iter: Iter, mut localcursor: Cursor) -> Option<Cursor> {
	for lch in iter {
		match localcursor.go(lch) {
			Some(cur) => localcursor = cur,
			None => return None
		}
	}
	Some(cur)
}

fn consumeLowerCase(ch: char, mut localcursor: Cursor) -> Option<Cursor> {
    consume_iterator(ch.to_lowercase, localcursor)
}

fn consumeSkipSpace(ch:char, mut localcursor:Cursor) -> Option<Cursor> {
    if(ch.is_whitespace() || ch.is_newline()) {
		if(localcursor.head == ' ') {
			return localcursor
		}
	}
}

fn consumeSkip(ch:char) -> Option<Cursor> {

}

trait MyCandidate {
    fn consume(ch:char) -> Vec<MyCandidate>
}
impl ExactCandidate {
    fn consume(ch:char) -> Vec<MyCandidate> {
        match cand.cursor.clone().go(ch) {
            Some(next) => vec!(ExactCandidate{cursor:next, start:cand.start}})
            None => {
			    vec!(
                    SpaceSkipCandidate{cursor:cand.cursor.clone(), start:cand.start}.consume(ch),
                    LowerCaseCandidate{cursor:cand.cursor.clone(), start:cand.start}.consume(ch),
                );
			}
        }
    }
}
impl LowerCaseCandidate {
    fn consume(ch:char) -> Vec<MyCandidate> {
		match consumeLowerCase(ch, cand.cursor.clone()) {
			Some(next) => vec!(LowerCaseCandidate{cursor:next, start:cand.start}})
			None => {
				vec!(
				SpaceSkipLowerCaseCandidate{cursor:cand.cursor.clone(), start:cand.start}.consume(ch),
				);
			}
		}

    }
}

impl SpaceSkipCandidate {
    fn consume(ch:char) -> Vec<MyCandidate> {
        let localcursor = cand.cursor.clone();
        if(cand.cursor.head == ' ') {
			while(localcursor.go(' ').is_some()){}
		}
		match localcursor.go(ch) {
			Some(next) => vec!(SpaceSkipCandidate{cursor:localcursor, start:cand.start})
			None => vec!(SpaceSymbolSkipCandidate{cursor:localcursor, start:cand.start})
		}

    }
}


impl SpaceSkipLowerCaseCandidate {
	fn consume(ch:char) -> Vec<MyCandidate> {
		let mut localcursor = cand.cursor.clone();
		if(cand.cursor.head == ' ') {
			while(localcursor.go(' ').is_some()){}
		}
		for(lch in ch.to_lowercase()) {
			match localcursor.go(lch) {
				Some(cur) => localcursor = cur,
				None => return vec!()
			}
		}
		vec!(SpaceSkipLowerCaseCandidate{cursor:localcursor, start:cand.start})
	}
}

impl SpaceSymbolSkipLowerCaseCandidate {
	fn consume(ch:char) -> Vec<MyCandidate> {
		let localcursor = cand.cursor.clone();
		if(cand.cursor.head.is_space() || cand.cursor.head.is_symbol()) {
			while(localcursor.go(' ').is_some() || localcursor.go('.').is_some()){}
		}
		match ch.to_lowercase().map(|lch| localcursor.go(lch)).last()) { // brrrr
			Some(next) => vec!(SpaceSymbolSkipLowerCaseCandidate{cursor:localcursor, start:cand.start})
			None => vec!()
		}

	}
}


pub mod suffix_tree {
    use collections::BTreeMap;

    pub struct SuffixTree<E, V> {
        suffixes: BTreeMap<E, SuffixTree<E, V>>,
        pub value: Option<V>,
    }

    impl<E: Ord + Clone, V> SuffixTree<E, V> {
        pub fn new() -> SuffixTree<E, V> {
            SuffixTree {
                suffixes: BTreeMap::new(),
                value: None,
            }
        }

        pub fn is_terminal(&self) -> bool {
            self.value.is_some()
        }

        pub fn insert<Iter: Iterator<Item=E>>(&mut self, el: Iter, value: V) {
            unsafe {
                let mut tree: *mut SuffixTree<E, V> = self;
                for i in el {
                    let new = match (*tree).suffixes.get_mut(&i) {
                        Some(next) => next as *mut SuffixTree<E, V>,
                        None => {
                            (*tree).suffixes.insert(i.clone(), SuffixTree::new());
                            (*tree).suffixes.get_mut(&i).unwrap() as *mut SuffixTree<E, V>
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
            match self.cursor.suffixes.get(&el) {
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
