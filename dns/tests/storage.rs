use cosmwasm_vm::{ReadonlyStorage, FfiResult, Storage};
use kv::{Raw, Store, Config, Item, Error, Bucket};

#[cfg(feature = "iterator")]
use cosmwasm_std::{Order};

/// KV is a Key-Value pair, returned from our iterators
pub type KV = (Vec<u8>, Vec<u8>);

pub struct MyMockStorage<'a> {
    bucket: Bucket<'a,Raw,Raw>
}

impl<'a> MyMockStorage<'a> {
    pub fn new(bucket: Bucket<'a, Raw,Raw>) -> Self {
        MyMockStorage{bucket }
    }
}

#[cfg(feature = "iterator")]
fn clone_item(item: Result<Item<Raw, Raw>, Error>) -> KV {
    let itr = item.unwrap();

    let key = itr.key::<Raw>().unwrap().clone().to_vec();
    let value = itr.value::<Raw>().unwrap().clone().to_vec();

    (key, value)
}

impl<'a> Default for MyMockStorage<'a>{
    fn default() -> Self {
        let cfg = Config::new("./tmp").temporary(true);
        let data = Store::new(cfg).unwrap();
        let bucket = data.bucket::<Raw, Raw>(None).unwrap();

        MyMockStorage{bucket}
    }
}

impl<'a> ReadonlyStorage for MyMockStorage<'static> {
    fn get(&self, key: &[u8]) -> FfiResult<Option<Vec<u8>>> {
        let result = self.bucket.get(key).unwrap();

        match result {
            Some(value) => Ok(Some(value.to_vec())),
            None => Ok(None)
        }
    }

    #[cfg(feature = "iterator")]
    /// range allows iteration over a set of keys, either forwards or backwards
    /// uses standard rust range notation, and eg db.range(b"foo"..b"bar") also works reverse
    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> FfiResult<Box<dyn Iterator<Item = FfiResult<KV>> + 'a>> {

        let mut max_key: Vec<u8>;
        let last_item = self.bucket.iter().last();
        if !last_item.is_none() {
            max_key = last_item.unwrap().unwrap().key::<Raw>().unwrap().to_vec();
            max_key.push(1);
        } else {
            max_key = Vec::new();
        }

        let starter = match start {
            Some(val) => Raw::from(val),
            None => Raw::default()
        };

        let ender = match end {
            Some(val) => Raw::from(val),
            None => Raw::from(max_key)
        };

        let itr = self.bucket.iter_range(starter,ender);

        Ok(match order {
            Order::Ascending => Box::new(itr.map(clone_item).map(FfiResult::Ok)),
            Order::Descending => Box::new(itr.rev().map(clone_item).map(FfiResult::Ok)),
        })
    }
}

impl<'a> Storage for MyMockStorage<'static> {
    fn set(&mut self, key: &[u8], value: &[u8]) -> FfiResult<()> {
        self.bucket.set(key.to_vec(), value.to_vec());
        // self.bucket.flush();

        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> FfiResult<()> {
        self.bucket.remove(key);
        // self.bucket.flush();

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "iterator")]
    // iterator_test_suite takes a storage, adds data and runs iterator tests
    // the storage must previously have exactly one key: "foo" = "bar"
    // (this allows us to test StorageTransaction and other wrapped storage better)
    fn iterator_test_suite(store: &mut MyMockStorage) {
        // ensure we had previously set "foo" = "bar"
        assert_eq!(store.get(b"foo").unwrap(), Some(b"bar".to_vec()));
        assert_eq!(
            store.range(None, None, Order::Ascending).unwrap().count(),
            1
        );

        // setup - add some data, and delete part of it as well
        store.set(b"ant", b"hill").expect("error setting value");
        store.set(b"ze", b"bra").expect("error setting value");

        // noise that should be ignored
        store.set(b"bye", b"bye").expect("error setting value");
        store.remove(b"bye").expect("error removing key");

        // unbounded
        {
            let iter = store.range(None, None, Order::Ascending).unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(
                elements,
                vec![
                    (b"ant".to_vec(), b"hill".to_vec()),
                    (b"foo".to_vec(), b"bar".to_vec()),
                    (b"ze".to_vec(), b"bra".to_vec()),
                ]
            );
        }

        // unbounded (descending)
        {
            let iter = store.range(None, None, Order::Descending).unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(
                elements,
                vec![
                    (b"ze".to_vec(), b"bra".to_vec()),
                    (b"foo".to_vec(), b"bar".to_vec()),
                    (b"ant".to_vec(), b"hill".to_vec()),
                ]
            );
        }

        // bounded
        {
            let iter = store
                .range(Some(b"f"), Some(b"n"), Order::Ascending)
                .unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(elements, vec![(b"foo".to_vec(), b"bar".to_vec())]);
        }

        // bounded (descending)
        {
            let iter = store
                .range(Some(b"air"), Some(b"loop"), Order::Descending)
                .unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(
                elements,
                vec![
                    (b"foo".to_vec(), b"bar".to_vec()),
                    (b"ant".to_vec(), b"hill".to_vec()),
                ]
            );
        }

        // bounded empty [a, a)
        {
            let iter = store
                .range(Some(b"foo"), Some(b"foo"), Order::Ascending)
                .unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(elements, vec![]);
        }

        // bounded empty [a, a) (descending)
        {
            let iter = store
                .range(Some(b"foo"), Some(b"foo"), Order::Descending)
                .unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(elements, vec![]);
        }

        // bounded empty [a, b) with b < a
        {
            let iter = store
                .range(Some(b"z"), Some(b"a"), Order::Ascending)
                .unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(elements, vec![]);
        }

        // bounded empty [a, b) with b < a (descending)
        {
            let iter = store
                .range(Some(b"z"), Some(b"a"), Order::Descending)
                .unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(elements, vec![]);
        }

        // right unbounded
        {
            let iter = store.range(Some(b"f"), None, Order::Ascending).unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(
                elements,
                vec![
                    (b"foo".to_vec(), b"bar".to_vec()),
                    (b"ze".to_vec(), b"bra".to_vec()),
                ]
            );
        }

        // right unbounded (descending)
        {
            let iter = store.range(Some(b"f"), None, Order::Descending).unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(
                elements,
                vec![
                    (b"ze".to_vec(), b"bra".to_vec()),
                    (b"foo".to_vec(), b"bar".to_vec()),
                ]
            );
        }

        // left unbounded
        {
            let iter = store.range(None, Some(b"f"), Order::Ascending).unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(elements, vec![(b"ant".to_vec(), b"hill".to_vec()),]);
        }

        // left unbounded (descending)
        {
            let iter = store.range(None, Some(b"no"), Order::Descending).unwrap();
            let elements: Vec<KV> = iter.filter_map(FfiResult::ok).collect();
            assert_eq!(
                elements,
                vec![
                    (b"foo".to_vec(), b"bar".to_vec()),
                    (b"ant".to_vec(), b"hill".to_vec()),
                ]
            );
        }
    }

    #[test]
    fn get_and_set() {
        let cfg = Config::new("./test").temporary(true);
        let data  = Store::new(cfg).unwrap();
        let bucket = data.bucket::<Raw, Raw>(Some("test1")).unwrap();
        let mut store = MyMockStorage::new(bucket);

        assert_eq!(None, store.get(b"foo").unwrap());
        store.set(b"foo", b"bar").unwrap();
        assert_eq!(Some(b"bar".to_vec()), store.get(b"foo").unwrap());
        assert_eq!(None, store.get(b"food").unwrap());


        // test different bucket with same bucket name are the same!
        let bucket2 = data.bucket::<Raw, Raw>(Some("test1")).unwrap();
        let mut store2 = MyMockStorage::new(bucket2);
        assert_eq!(Some(b"bar".to_vec()), store2.get(b"foo").unwrap());
    }

    #[test]
    fn delete() {
        let cfg = Config::new("./test").temporary(true);
        let data  = Store::new(cfg).unwrap();
        let bucket = data.bucket::<Raw, Raw>(None).unwrap();

        let mut store = MyMockStorage::new(bucket);

        store.set(b"foo", b"bar").unwrap();
        store.set(b"food", b"bank").unwrap();
        store.remove(b"foo").unwrap();

        assert_eq!(None, store.get(b"foo").unwrap());
        assert_eq!(Some(b"bank".to_vec()), store.get(b"food").unwrap());
    }

    #[test]
    #[cfg(feature = "iterator")]
    fn iterator() {
        let cfg = Config::new("./test").read_only(false).temporary(true);
        let data  = Store::new(cfg).unwrap();
        let bucket = data.bucket::<Raw, Raw>(Some("test".as_ref())).unwrap();

        let mut store = MyMockStorage::new(bucket);

        store.set(b"foo", b"bar").expect("error setting value");
        iterator_test_suite(&mut store);
    }
}