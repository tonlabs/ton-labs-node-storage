use std::io::{Cursor, Write};
use std::sync::Arc;

use ton_types::{ByteOrderRead, Cell, CellData, Result};
use ton_types::UInt256;

use crate::base_impl;
use crate::db::traits::{KvcTransaction, KvcTransactional};
use crate::dynamic_boc_db::DynamicBocDb;
use crate::types::{CellId, Reference, StorageCell};

base_impl!(CellDb, KvcTransactional, CellId);

impl CellDb {
    /// Gets cell from key-value storage by cell id
    pub fn get_cell(&self, cell_id: &CellId, boc_db: Arc<DynamicBocDb>) -> Result<StorageCell> {
        Self::deserialize_cell(self.db.get(&cell_id)?.as_ref(), boc_db)
    }

    /// Puts cell into transaction
    pub fn put_cell<T: KvcTransaction<CellId> + ?Sized>(transaction: &T, cell_id: &CellId, cell: Cell) -> Result<()> {
        transaction.put(cell_id, &Self::serialize_cell(cell)?);
        Ok(())
    }

    /// Binary serialization of cell data
    fn serialize_cell(cell: Cell) -> Result<Vec<u8>> {
        let references_count = cell.references_count() as u8;

        assert!(references_count < 5);

        let mut data: Vec<u8> = Vec::new();

        cell.cell_data().serialize(&mut data)?;
        data.write(&[references_count])?;

        for i in 0..references_count {
            data.write(cell.reference(i as usize)?.repr_hash().as_slice())?;
        }

        assert!(data.len() > 0);

        Ok(data)
    }

    /// Binary deserialization of cell data
    fn deserialize_cell(data: &[u8], boc_db: Arc<DynamicBocDb>) -> Result<StorageCell> {
        assert!(data.len() > 0);

        let mut reader = Cursor::new(data);
        let cell_data = CellData::deserialize(&mut reader)?;
        let references_count = reader.read_byte()?;
        let mut references = Vec::with_capacity(references_count as usize);
        for _ in 0..references_count {
            let hash = UInt256::from(reader.read_u256()?);
            references.push(Reference::NeedToLoad(hash));
        }

        Ok(StorageCell::with_params(cell_data, references, boc_db, 0))
    }
}
