use std::sync::atomic::AtomicI32;

use dust_dds::infrastructure::domain::DomainId;

pub static TEST_DOMAIN_ID_GENERATOR: DomainIdGenerator = DomainIdGenerator::new();

pub struct DomainIdGenerator {
    id: AtomicI32,
}

impl DomainIdGenerator {
    pub const fn new() -> Self {
        Self {
            id: AtomicI32::new(0),
        }
    }

    pub fn generate_unique_domain_id(&self) -> DomainId {
        self.id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
