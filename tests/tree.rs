use htree_challenge::tree::*;

#[test]
pub fn insert() {
    let mut store = HMap::new();

    // check the ubsertion is done correctly.
    for i in 0u8..6u8 {
        store.push(blake3::hash(&[i]), i);
        let (_, j) = store.get(i as usize);
        assert_eq!(i, j);
    }

    // check it can find any element
    for i in 0..7 {
        let (_, j) = store.get(i as usize);
        assert_eq!(i, j);
    }
}

#[test]
pub fn proof() {
    let mut store = HMap::new();
    let mut root = None;

    // check the ubsertion is done correctly.
    for i in 0u8..6u8 {
        let hi = blake3::hash(&[i]);
        store.push(hi, i);
        let (proof, _) = store.get(i as usize);
        let new_root = proof.prove_on(hi);
        assert_eq!(proof.hash(), root);
        root.map(|r| assert!(new_root.against(r)));
        root = Some(*new_root);
    }
}
