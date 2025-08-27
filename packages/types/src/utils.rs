use std::sync::Arc;

pub mod data_url;
pub mod img;

#[inline]
pub fn ptr_key<T>(arc: &Arc<T>) -> usize {
    Arc::as_ptr(arc) as usize
}