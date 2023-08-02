//! Merkel tree append only storage.
//!
//! This module contains the structs needed to create a merkel tree storage and perform proofs on
//! it.
//!
//! # exemples
//!
//! ```
//! use htree_challenge::tree::*;
//! // create a merkel tree.
//!let mut store = HMap::new();
//!
//!// insert elements.
//! let data: [&[u8];3] = [b"one", b"two", b"three"];
//! for d in data {
//! store.push(blake3::hash(d), d);
//! }
//!
//!// get the root hash of the tree. In a Client/Server mode, the client will not use this value
//!// and compute itself the root (see the tests dir in the repo).
//!let root = store.root();
//!
//! // we check the data is not corupted.
//! for i in 0..3 {
//! assert!(store.proof(i).prove_on(blake3::hash(data[i])).against(root))
//! }
//! ```
pub use std::hash::Hash;
pub use std::ops::Deref;

/// The merkel tree storage.
#[derive(Debug, Default)]
pub struct HMap<D: Hash> {
    data: Vec<D>,
    tree: Tree,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum Tree {
    #[default]
    Empty,
    Leaf {
        hash: blake3::Hash,
    },
    Node {
        left: Box<Tree>,
        right: Box<Tree>,
    },
}

/// A proof used to check data are not corupted.
///
/// This can be obtained by a call to [HMap::get].
/// It's also returned at every insertion in a [HMap] via [HMap::push].
#[derive(Debug, PartialEq)]
pub struct Proof {
    nth: usize,
    hashes: Vec<blake3::Hash>,
}

pub struct PartialProof(blake3::Hash);

impl Proof {
    pub fn prove_on(&self, hash: blake3::Hash) -> PartialProof {
        let Proof { nth, hashes } = self;
        let len = hashes.len();
        let mut mask = if len > 0 { 0x1 << (len - 1) } else { 0 };
        eprintln!("{}, {:?}", mask, hashes);
        PartialProof(hashes.iter().rfold(hash, |ag, h| {
            let mut hasher = blake3::Hasher::new();
            if nth & mask > 0 {
                hasher.update(h.as_bytes()).update(ag.as_bytes());
            } else {
                hasher.update(ag.as_bytes()).update(h.as_bytes());
            }
            mask >>= 1;
            hasher.finalize()
        }))
    }

    pub fn hash(&self) -> Option<blake3::Hash> {
        // could be implemented without cloning.
        let Proof { nth, hashes } = self;
        let mut hashes = hashes.clone();
        let hash = hashes.pop();
        hash.map(|hash| *Proof { nth: *nth, hashes }.prove_on(hash))
    }
}

impl PartialProof {
    pub fn against(&self, hash: blake3::Hash) -> bool {
        self.0 == hash
    }
}

impl Deref for PartialProof {
    type Target = blake3::Hash;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Hash + Clone> HMap<D> {
    pub fn new() -> Self {
        Self {
            data: vec![],
            tree: Tree::Empty,
        }
    }

    pub fn root(&self) -> blake3::Hash {
        self.tree.hash()
    }

    pub fn push(&mut self, hash: blake3::Hash, data: D) -> Proof {
        let nth = self.data.len();
        let mut pos = nth;
        let mut hashes = Vec::new();
        let mut current_node = &mut self.tree;
        while let Tree::Node { left, right } = current_node {
            if pos & 0x1 > 0 {
                hashes.push(left.hash());
                current_node = right;
            } else {
                hashes.push(right.hash());
                current_node = left;
            }
            pos >>= 1;
        }
        if !self.data.is_empty() {
            hashes.push(current_node.hash());
        }
        *current_node = current_node.clone().merge(Tree::Leaf { hash });
        self.data.push(data);
        Proof { nth, hashes }
    }

    pub fn proof(&self, nth: usize) -> Proof {
        let mut current_node = &self.tree;
        let mut pos = nth;
        let mut hashes = Vec::new();
        while let Tree::Node { left, right } = current_node {
            if pos & 0x1 > 0 {
                hashes.push(left.hash());
                current_node = right.as_ref();
            } else {
                hashes.push(right.hash());
                current_node = left.as_ref();
            }
            pos >>= 1;
        }
        Proof { nth, hashes }
    }

    pub fn get(&self, nth: usize) -> (Proof, D) {
        (self.proof(nth), self.data[nth].clone())
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
        match self {
            Self::Node { left, right } => blake3::Hasher::new()
                .update(left.hash().as_bytes())
                .update(right.hash().as_bytes())
                .finalize(),
            Self::Leaf { hash } => *hash,
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// A merge should keep the order of the ops.
    /// Merge of empty trees is empty.
    fn sim_merge() {
        // both empty
        let left = Tree::Empty;
        let right = Tree::Empty;
        assert_eq!(left.merge(right), Tree::Empty,);
        // both leafs
        let left = Tree::Leaf {
            hash: blake3::hash(&[0u8]),
        };
        let right = Tree::Leaf {
            hash: blake3::hash(&[1u8]),
        };
        assert_eq!(
            left.clone().merge(right.clone()),
            Tree::Node {
                left: Box::new(left),
                right: Box::new(right),
            }
        );
    }

    #[test]
    /// When merging anything with an empty tree, it result to this thing.
    /// The merge is associative in this case.
    fn identity_merge() {
        // left is empty
        let left = Tree::Empty;
        let right = Tree::Leaf {
            hash: blake3::hash(&[1u8]),
        };
        assert_eq!(left.merge(right.clone()), right,);
        // right is empty;
        let left = Tree::Leaf {
            hash: blake3::hash(&[0u8]),
        };
        let right = Tree::Empty;
        assert_eq!(left.clone().merge(right), left,);
    }

    #[test]
    /// Merge is a simple op, it does not try to reshape the tree.
    fn deep_merge() {
        let left = Tree::Leaf {
            hash: blake3::hash(&[0u8]),
        };
        let right = Tree::Node {
            left: Box::new(Tree::Leaf {
                hash: blake3::hash(&[1u8]),
            }),
            right: Box::new(Tree::Leaf {
                hash: blake3::hash(&[2u8]),
            }),
        };
        assert_eq!(
            left.clone().merge(right.clone()),
            Tree::Node {
                left: Box::new(left),
                right: Box::new(right),
            }
        );
    }

    #[test]
    fn hash() {
        let left = Tree::Leaf {
            hash: blake3::hash(&[0u8]),
        };
        let right = Tree::Leaf {
            hash: blake3::hash(&[1u8]),
        };

        // a leaf hash is it's contained hash.
        assert_eq!(right.clone().hash(), blake3::hash(&[1u8]),);

        // resist over extention with 0.
        assert_ne!(
            left.clone().merge(right.clone()).hash(),
            right.clone().hash(),
        );
        assert_ne!(
            left.clone().merge(right.clone()).hash(),
            left.clone().hash(),
        );

        // A tree hash differ if elems are not in the same order.
        assert_ne!(
            left.clone().merge(right.clone()).hash(),
            right.merge(left).hash(),
        );
    }

    #[test]
    fn proof() {
        let store = HMap {
            data: vec![0],
            tree: Tree::Leaf {
                hash: blake3::hash(&[0u8]),
            },
        };
        assert_eq!(
            store.proof(0),
            Proof {
                nth: 0,
                hashes: vec![]
            }
        );

        let store = HMap {
            data: vec![0u8, 1u8],
            tree: Tree::Node {
                left: Box::new(Tree::Leaf {
                    hash: blake3::hash(&[0u8]),
                }),
                right: Box::new(Tree::Leaf {
                    hash: blake3::hash(&[1u8]),
                }),
            },
        };
        assert_eq!(
            store.proof(0),
            Proof {
                nth: 0,
                hashes: vec![blake3::hash(&[1u8])],
            }
        );
        assert_eq!(
            store.proof(1),
            Proof {
                nth: 1,
                hashes: vec![blake3::hash(&[0u8])],
            }
        );

        let store = HMap {
            data: vec![0u8, 1u8, 2u8],
            tree: Tree::Node {
                left: Box::new(Tree::Node {
                    left: Box::new(Tree::Leaf {
                        hash: blake3::hash(&[0u8]),
                    }),
                    right: Box::new(Tree::Leaf {
                        hash: blake3::hash(&[2u8]),
                    }),
                }),
                right: Box::new(Tree::Leaf {
                    hash: blake3::hash(&[1u8]),
                }),
            },
        };
        assert_eq!(
            store.proof(0),
            Proof {
                nth: 0,
                hashes: vec![blake3::hash(&[1u8]), blake3::hash(&[2u8]),],
            }
        );
        assert_eq!(
            store.proof(1),
            Proof {
                nth: 1,
                hashes: vec![blake3::Hasher::new()
                    .update(blake3::hash(&[0u8]).as_bytes())
                    .update(blake3::hash(&[2u8]).as_bytes())
                    .finalize()],
            }
        );
    }
}
