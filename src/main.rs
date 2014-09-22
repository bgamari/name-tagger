extern crate collections;

use std::io::{BufferedReader, File};
use suffix_tree::{SuffixTree, Cursor};

pub fn main() {
    let mut dict: SuffixTree<char> = SuffixTree::new();

    // read in dictionary
    let args = std::os::args();
    let dict_path = Path::new(args[1].clone());
    let mut dict_reader = BufferedReader::new(File::open(&dict_path));
    for i in dict_reader.lines() {
        let t: Vec<char> = i.unwrap().as_slice().trim().chars().collect();
        dict.insert(t);
    }

    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let matches = find_matches(&dict, line.as_slice().chars());
        for m in matches.iter() {
            println!("{}\t{}\t{}", m.start, m.end,
                     String::from_chars(m.seq.as_slice()));
        }
        println!("");
    }
}

struct Match<'a, E: 'a> {
    start: uint,
    end: uint,
    seq: Vec<E>
}

fn find_matches<'a, E: Clone + Ord, Iter: Iterator<E>>
    (dict: &'a SuffixTree<E>, iter: Iter) -> Vec<Match<'a, E>> {

    let mut cursors: Vec<Cursor<E>> = Vec::new();
    let mut matches = Vec::new();
    for (offset, ch) in iter.enumerate() {
        cursors.push(Cursor::new(dict));
        cursors = cursors.into_iter().filter_map(|cur| cur.go(ch.clone())).collect();
        for cur in cursors.iter() {
            if cur.get().is_terminal() {
                // we have a hit
                matches.push(Match{
                    start: offset - cur.path.len(),
                    end: offset,
                    seq: cur.path.clone(),
                });
            }
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
