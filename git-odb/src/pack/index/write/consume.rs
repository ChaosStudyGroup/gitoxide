use crate::{
    hash, loose,
    pack::{
        self,
        index::write::{Bytes, Cache, CacheEntry, Entry, Error, Mode},
    },
    zlib,
};
use git_object::{owned, HashKind};
use smallvec::alloc::collections::BTreeMap;
use std::io;

pub(crate) fn apply_deltas<F>(
    base_entries: Vec<&Entry>,
    _state: &mut (),
    _entries: &[Entry],
    caches: &parking_lot::Mutex<BTreeMap<u64, CacheEntry>>,
    _mode: &Mode<F>,
    hash_kind: HashKind,
) -> Result<Vec<(u64, owned::Id)>, Error>
where
    F: for<'r> Fn(u64, &'r mut Vec<u8>) -> Option<(pack::data::Header, u64)> + Send + Sync,
{
    let decompressed_bytes_from_cache = |pack_offset: &u64| -> Result<(bool, Vec<u8>), Error> {
        let cache = caches
            .lock()
            .get_mut(pack_offset)
            .expect("an entry for every pack offset")
            .cache();
        let (is_borrowed, cache) = match cache {
            Bytes::Borrowed(b) => (true, b),
            Bytes::Owned(b) => (false, b),
        };
        let bytes = match cache {
            Cache::Decompressed(b) => b,
            Cache::Compressed(b, decompressed_len) => {
                let mut out = Vec::with_capacity(decompressed_len);
                zlib::Inflate::default()
                    .once(&b, &mut io::Cursor::new(&mut out), true)
                    .map_err(|err| Error::ConsumeZlibInflate(err, "Failed to decompress entry"))?;
                out
            }
            Cache::Unset => unimplemented!("use resolver"),
        };
        Ok((is_borrowed, bytes))
    };
    let possibly_return_to_cache = |pack_offset: &u64, is_borrowed: bool, bytes: Vec<u8>| {
        if is_borrowed {
            caches
                .lock()
                .get_mut(pack_offset)
                .expect("an entry for every pack offset")
                .set_decompressed(bytes);
        }
    };
    let compute_hash = |kind: git_object::Kind, bytes: &[u8]| -> owned::Id {
        let mut write = hash::Write::new(io::sink(), hash_kind);
        loose::object::header::encode(kind, bytes.len() as u64, &mut write)
            .expect("write to sink and hash cannot fail");
        write.hash.update(bytes);
        owned::Id::from(write.hash.digest())
    };
    let mut out = Vec::with_capacity(base_entries.len()); // perfectly conservative guess

    for Entry { pack_offset, kind, .. } in base_entries {
        let (is_borrowed, base_bytes) = decompressed_bytes_from_cache(pack_offset)?;
        out.push((
            *pack_offset,
            compute_hash(kind.to_kind().expect("base object"), &base_bytes),
        ));
        possibly_return_to_cache(pack_offset, is_borrowed, base_bytes);
    }

    out.shrink_to_fit();
    Ok(out)
}