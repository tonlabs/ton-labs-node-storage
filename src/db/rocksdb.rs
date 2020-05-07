use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

use rocksdb::{DB, IteratorMode, Options, Snapshot, WriteBatch};

use ton_types::{fail, Result};

use crate::db::traits::{DbKey, Kvc, KvcReadable, KvcSnapshotable, KvcTransaction, KvcTransactional, KvcWriteable};
use crate::error::StorageError;
use crate::types::DbSlice;

#[derive(Debug)]
pub struct RocksDb {
    db: Arc<Option<DB>>,
    path: PathBuf,
}

impl RocksDb {
    /// Creates new instance with given path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        let pathbuf = path.as_ref().to_path_buf();
        RocksDb {
            db: Arc::new(Some(DB::open_default(path)
                .expect("Cannot open DB"))),
            path: pathbuf
        }
    }

    pub(crate) fn db(&self) -> Result<&DB> {
        if let Some(ref db) = *self.db {
            Ok(db)
        } else {
            Err(StorageError::DbIsDropped)?
        }
    }
}

/// Implementation of key-value collection for RocksDB
impl Kvc for RocksDb {
    fn len(&self) -> Result<usize> {
        fail!("len() is not supported for RocksDb")
    }

    fn destroy(&mut self) -> Result<()> {
        if Arc::get_mut(&mut self.db)
            .ok_or(StorageError::HasActiveTransactions)?
            .is_some()
        {
            std::mem::replace(&mut self.db, Arc::new(None));
        }

        Ok(DB::destroy(&Options::default(), &self.path)?)
    }
}

/// Implementation of readable key-value collection for RocksDB. Actual implementation is blocking.
impl<K: DbKey> KvcReadable<K> for RocksDb {
    fn get(&self, key: &K) -> Result<DbSlice> {
        self.db()?.get_pinned(key.key())?
            .map(|value| value.into())
            .ok_or(StorageError::KeyNotFound(hex::encode(key.key())).into())
    }

    fn contains(&self, key: &K) -> Result<bool> {
        self.db()?.get_pinned(key.key())
            .map(|value| value.is_some())
            .map_err(|err| err.into())
    }

    fn for_each(&self, predicate: &mut dyn FnMut(&[u8], &[u8]) -> Result<bool>) -> Result<bool> {
        for (key, value) in self.db()?.iterator(IteratorMode::Start) {
            if !predicate(key.as_ref(), value.as_ref())? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Implementation of writable key-value collection for RocksDB. Actual implementation is blocking.
impl<K: DbKey> KvcWriteable<K> for RocksDb {
    fn put(&self, key: &K, value: &[u8]) -> Result<()> {
        self.db()?.put(key.key(), value)
            .map_err(|err| err.into())
    }

    fn delete(&self, key: &K) -> Result<()> {
        self.db()?.delete(key.key())
            .map_err(|err| err.into())
    }
}

/// Implementation of support for take snapshots for RocksDB.
impl<K: DbKey> KvcSnapshotable<K> for RocksDb {
    fn snapshot<'db>(&'db self) -> Result<Arc<dyn KvcReadable<K> + 'db>> {
        Ok(Arc::new(RocksDbSnapshot(self.db()?.snapshot())))
    }
}

struct RocksDbSnapshot<'db>(Snapshot<'db>);

impl Debug for RocksDbSnapshot<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("[snapshot]")
    }
}

impl Kvc for RocksDbSnapshot<'_> {
    fn len(&self) -> Result<usize> {
        fail!("len() is not supported for RocksDb")
    }

    fn destroy(&mut self) -> Result<()> {
        fail!("destroy() is not supported for snapshots")
    }
}

impl<K: DbKey> KvcReadable<K> for RocksDbSnapshot<'_> {
    fn get(&self, key: &K) -> Result<DbSlice> {
        self.0.get(key.key())?
            .map(|value| value.into())
            .ok_or(StorageError::KeyNotFound(hex::encode(key.key())).into())
    }

    fn contains(&self, key: &K) -> Result<bool> {
        self.0.get(key.key())
            .map(|value| value.is_some())
            .map_err(|err| err.into())
    }

    fn for_each(&self, predicate: &mut dyn FnMut(&[u8], &[u8]) -> Result<bool>) -> Result<bool> {
        for (key, value) in self.0.iterator(IteratorMode::Start) {
            if !predicate(key.as_ref(), value.as_ref())? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Implementation of transaction support for key-value collection for RocksDB.
impl<K: DbKey> KvcTransactional<K> for RocksDb {
    fn begin_transaction(&self) -> Result<Box<dyn KvcTransaction<K>>> {
        Ok(Box::new(RocksDbTransaction::new(Arc::clone(&self.db))))
    }
}

pub struct RocksDbTransaction {
    db: Arc<Option<DB>>,
    batch: Mutex<WriteBatch>,
}

/// Implementation of transaction for key-value collection for RocksDB.
impl RocksDbTransaction {
    fn new(db: Arc<Option<DB>>) -> Self {
        Self {
            db,
            batch: Mutex::new(WriteBatch::default())
        }
    }
}

impl<K: DbKey> KvcTransaction<K> for RocksDbTransaction {
    fn put(&self, key: &K, value: &[u8]) -> Result<()> {
        self.batch.lock().unwrap().put(key.key(), value)
            .map_err(|err| err.into())
    }

    fn delete(&self, key: &K) -> Result<()> {
        self.batch.lock().unwrap().delete(key.key())
            .map_err(|err| err.into())
    }

    fn clear(&self) -> Result<()> {
        self.batch.lock().unwrap().clear()
            .map_err(|err| err.into())
    }

    fn commit(self: Box<Self>) -> Result<()> {
        let batch = self.batch.into_inner().unwrap();
        if let Some(ref db) = *self.db {
            db.write(batch)
            .map_err(|err| err.into())
        } else {
            Err(StorageError::DbIsDropped)?
        }
    }

    fn len(&self) -> usize {
        self.batch.lock().unwrap().len()
    }

    fn is_empty(&self) -> bool {
        self.batch.lock().unwrap().is_empty()
    }
}