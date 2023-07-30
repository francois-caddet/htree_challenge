pub use std::ops::Deref;
pub use std::hash::Hash;
use std::cell::Cell;

pub struct HMap<D: Hash> {
    data: Vec<D>,
    tree: Tree,
}

enum Tree {
    Empty,
    Leaf {
        hash: blake3::Hash,
    },
    Node {
        left: Box<Tree>,
        right: Box<Tree>,
    },
}

pub struct Proof{nth: usize, hashes: Vec<blake3::Hash>}

pub struct PartialProof(blake3::Hash);

impl Proof {
    fn prove_on(&self, hash: blake3::Hash) -> PartialProof {
        let Proof{nth, hashes} = self;
        let mut mask = 0x1 << hashes.len();
        PartialProof(
            hashes.iter().rfold(hash, |ag, h| {
                let mut hasher = blake3::Hasher::new();
                if nth & mask > 0 {
                    hasher.update(ag.as_bytes()).update(h.as_bytes());
                } else {
                    hasher.update(h.as_bytes()).update(ag.as_bytes());
                }
                mask >>= 1;
                hasher.finalize()
            })
        )
    }
}

impl<D: Hash> HMap<D> {
    pub fn new() -> Self {
        Self {
            data: vec![],
            tree: Tree::Empty,
        }
    }

    pub fn push(&mut self, hash: blake3::Hash, data: D) -> Proof {
        let nth = self.data.len();
        if nth == 0 {
            self.tree = Tree::Leaf{hash};
        }
        let mut pos = nth;
        let mut hashes = Vec::new();
        let mut current_node = &mut self.tree;
        let node = loop {
            if let Tree::Node{left, right} = current_node {
                pos >>= 1;
                if pos & 0x1 > 0 {
                    hashes.push(right.hash());
                    current_node = left;
                } else {
                    hashes.push(left.hash());
                    current_node = right;
                }
            } else if let Tree::Leaf{hash} = current_node {
                break Tree::Leaf{hash: hash.clone()};
            }
        };
        *current_node = node.merge(Tree::Leaf{hash});
        self.data.push(data);
        Proof{nth, hashes}
    }
}

impl Tree {
    pub fn merge(self, with: Self) -> Self {
        if matches!(self, Tree::Empty) {
            return with;
        } else if matches!(with, Tree::Empty) {
            return self;
        }
        Self::Node {
            left: Box::new(self),
            right: Box::new(with),
        }
    }

    fn hash(&self) -> blake3::Hash {
        match  self {
            Self::Node{left, right} => blake3::Hasher::new()
                .update(left.hash().as_bytes())
                .update(right.hash().as_bytes())
                .finalize(),
            Self::Leaf{hash} => *hash,
            _ => panic!(),
        }
    }
}

impl Default for Tree {
    fn default() -> Self {
        Tree::Empty
    }
}
