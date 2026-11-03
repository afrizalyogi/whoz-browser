use crate::protocol::{SyncDataType, SyncEntity};
use std::collections::HashMap;

pub struct SyncStore {
    entities: HashMap<String, SyncEntity>,
    pub last_sync_timestamp: u64,
}

impl SyncStore {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            last_sync_timestamp: 0,
        }
    }

    pub fn upsert(&mut self, entity: SyncEntity) {
        self.entities.insert(entity.id.clone(), entity);
    }

    pub fn get_by_type(&self, data_type: SyncDataType) -> Vec<&SyncEntity> {
        self.entities
            .values()
            .filter(|e| e.data_type == data_type && !e.is_deleted)
            .collect()
    }

    pub fn mark_deleted(&mut self, id: &str) {
        if let Some(entity) = self.entities.get_mut(id) {
            entity.is_deleted = true;
        }
    }

    pub fn count(&self) -> usize {
        self.entities.values().filter(|e| !e.is_deleted).count()
    }
}
