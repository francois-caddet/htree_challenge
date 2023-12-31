use htree_challenge::tree::*;

#[test]
pub fn insert() {
    let mut store = HMap::new();

    // check the ubsertion is done correctly.
    for i in 0u8..6u8 {
        store.push(blake3::hash(&[i]), i);
        let j = store.get(i as usize).unwrap();
        assert_eq!(i, j);
    }

    // check it can find any element
    for i in 0..6 {
        let j = store.get(i as usize).unwrap();
        assert_eq!(i, j);
    }
}

#[test]
pub fn insert_proof() {
    let mut store = HMap::new();
    for i in 0..6 {
        let pi = store.push(blake3::hash(&[i]), i);
        let pj = store.proof(i as usize).unwrap();
        assert_eq!(pi, pj);
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
        let proof = store.proof(i as usize).unwrap();
        let new_root = proof.prove_on(hi);
        assert_eq!(proof.hash(), root);
        root = Some(*new_root);
    }
    for i in 0..6 {
        let di = store.get(i).unwrap();
        let proof = store.proof(i).unwrap();
        eprintln!("{}", di);
        assert!(proof.prove_on(blake3::hash(&[di])).against(root.unwrap()))
    }
}
