use crate::radix::RadixNode;

#[test]
fn radix1() {
    let mut radix = RadixNode::new();

    radix.insert([1,2,3,4], 1).unwrap();
    radix.insert([1,3,3,4], 2).unwrap();
    radix.insert([1,2,3,4,5], 3).unwrap();
    radix.insert([1,2,3,4,5,6,7,8], 4).unwrap();
    radix.insert([1], 5).unwrap();

    radix.insert([2,3,4], 6).unwrap();
    radix.insert([2,3,4,5,6,7], 7).unwrap();
    radix.insert([2,3,4,5,6,7,8], 8).unwrap();

    let k = [1,2,3,4];
    let v = radix.get(&k);
    assert_eq!(v, Some(&1));

    let k = [1,3,3,4];
    let v = radix.get(&k);
    assert_eq!(v, Some(&2));

    let k = [1,2,3,4,5];
    let v = radix.get(&k);
    assert_eq!(v, Some(&3));

    let k = [1];
    let v = radix.get(&k);
    assert_eq!(v, Some(&5));

    let k = [2,3,4];
    let v = radix.get(&k);
    assert_eq!(v, Some(&6));

    let k = [2,3,4,5,6,7];
    let v = radix.get(&k);
    assert_eq!(v, Some(&7));

    let k = [2,3,4,5,6,7,8];
    let v = radix.get(&k);
    assert_eq!(v, Some(&8));

    let k = [1,2,3];
    let removed = radix.remove(&k, true);
    assert_eq!(removed, None);

    let k = [2,3,4];
    radix.remove(&k, false);

    let v = radix.get(&k);
    assert_eq!(v, None);

    let k = [2,3,4,5,6,7];
    let v = radix.get(&k);
    assert_eq!(v, Some(&7));

    let k = [2,3,4,5,6,7,8];
    let v = radix.get(&k);
    assert_eq!(v, Some(&8));

    let k = [1];
    radix.remove(&k, true);

    let v = radix.get(&k);
    assert_eq!(v, None);
}

