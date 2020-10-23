use std::sync::atomic::Ordering;

use ton_block::BlockIdExt;

use crate::types::BlockMeta;

mod package_index_db;

pub mod archive_manager;
pub mod package;
pub mod package_entry_id;
pub mod package_entry;

mod package_status_db;
mod package_status_key;
mod file_maps;
mod package_offsets_db;
mod package_info;
mod archive_slice;
mod package_entry_meta_db;
mod package_entry_meta;
mod package_id;

fn get_mc_seq_no_opt(block_handle: Option<(&BlockIdExt, &BlockMeta)>) -> u32 {
    if let Some((id, meta)) = block_handle {
        get_mc_seq_no(id, meta)
    } else {
        0
    }
}

fn get_mc_seq_no(block_id: &BlockIdExt, block_meta: &BlockMeta) -> u32 {
    if block_id.shard().is_masterchain() {
        block_id.seq_no()
    } else {
        block_meta.masterchain_ref_seq_no().load(Ordering::SeqCst)
    }
}

#[inline]
fn flags(block_meta: &BlockMeta) -> u32 {
    block_meta.flags().load(Ordering::SeqCst)
}

fn is_flag(block_meta: &BlockMeta, flag: u32) -> bool {
    flags(block_meta) & flag == flag
}

fn is_key_block(block_meta: &BlockMeta) -> bool {
    const FLAG_KEY_BLOCK: u32 = 1 << 11;
    is_flag(block_meta, FLAG_KEY_BLOCK)
}

fn is_data_inited(block_meta: &BlockMeta) -> bool {
    const FLAG_DATA: u32 = 1;
    is_flag(block_meta, FLAG_DATA)
}

fn is_proof_inited(block_meta: &BlockMeta) -> bool {
    const FLAG_PROOF: u32 = 1 << 1;
    is_flag(block_meta, FLAG_PROOF)
}

fn is_prooflink_inited(block_meta: &BlockMeta) -> bool {
    const FLAG_PROOF_LINK: u32 = 1 << 2;
    is_flag(block_meta, FLAG_PROOF_LINK)
}

fn is_moved_to_archive(block_meta: &BlockMeta) -> bool {
    const FLAG_MOVED_TO_ARCHIVE: u32 = 1 << 13;
    is_flag(block_meta, FLAG_MOVED_TO_ARCHIVE)
}
