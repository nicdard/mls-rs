use async_trait::async_trait;
use aws_mls_core::key_package::{KeyPackageData, KeyPackageStorage};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

use crate::SqLiteDataStorageError;

#[derive(Debug, Clone)]
pub struct SqLiteKeyPackageStore {
    connection: Arc<Mutex<Connection>>,
}

impl SqLiteKeyPackageStore {
    pub(crate) fn new(connection: Connection) -> SqLiteKeyPackageStore {
        SqLiteKeyPackageStore {
            connection: Arc::new(Mutex::new(connection)),
        }
    }

    fn insert(
        &mut self,
        id: &[u8],
        key_package: KeyPackageData,
    ) -> Result<(), SqLiteDataStorageError> {
        let connection = self.connection.lock().unwrap();

        connection
            .execute(
                "INSERT INTO key_package (id, data) VALUES (?,?)",
                params![
                    id,
                    bincode::serialize(&key_package)
                        .map_err(|e| SqLiteDataStorageError::DataConversionError(e.into()))?
                ],
            )
            .map(|_| ())
            .map_err(|e| SqLiteDataStorageError::SqlEngineError(e.into()))
    }

    fn get(&self, id: &[u8]) -> Result<Option<KeyPackageData>, SqLiteDataStorageError> {
        let connection = self.connection.lock().unwrap();

        connection
            .query_row(
                "SELECT data FROM key_package WHERE id = ?",
                params![id],
                |row| Ok(bincode::deserialize(&row.get::<_, Vec<u8>>(0)?).unwrap()),
            )
            .optional()
            .map_err(|e| SqLiteDataStorageError::SqlEngineError(e.into()))
    }

    fn delete(&self, id: &[u8]) -> Result<(), SqLiteDataStorageError> {
        let connection = self.connection.lock().unwrap();

        connection
            .execute("DELETE FROM key_package where id = ?", params![id])
            .map(|_| ())
            .map_err(|e| SqLiteDataStorageError::SqlEngineError(e.into()))
    }
}

#[async_trait]
impl KeyPackageStorage for SqLiteKeyPackageStore {
    type Error = SqLiteDataStorageError;

    async fn insert(&mut self, id: Vec<u8>, pkg: KeyPackageData) -> Result<(), Self::Error> {
        self.insert(id.as_slice(), pkg)
    }

    async fn get(&self, id: &[u8]) -> Result<Option<KeyPackageData>, Self::Error> {
        self.get(id)
    }

    async fn delete(&mut self, id: &[u8]) -> Result<(), Self::Error> {
        (*self).delete(id)
    }
}

#[cfg(test)]
mod tests {
    use super::SqLiteKeyPackageStore;
    use crate::{
        sqlite_storage::{connection_strategy::MemoryStrategy, test_utils::gen_rand_bytes},
        SqLiteDataStorageEngine, SqLiteDataStorageError,
    };
    use assert_matches::assert_matches;
    use aws_mls_core::{crypto::HpkeSecretKey, key_package::KeyPackageData};

    fn test_storage() -> SqLiteKeyPackageStore {
        SqLiteDataStorageEngine::new(MemoryStrategy)
            .unwrap()
            .key_package_repository()
            .unwrap()
    }

    fn test_key_package() -> (Vec<u8>, KeyPackageData) {
        let key_id = gen_rand_bytes(32);
        let key_package = KeyPackageData::new(
            gen_rand_bytes(256),
            HpkeSecretKey::from(gen_rand_bytes(256)),
            HpkeSecretKey::from(gen_rand_bytes(256)),
        );

        (key_id, key_package)
    }

    #[test]
    fn key_package_insert() {
        let mut storage = test_storage();
        let (key_package_id, key_package) = test_key_package();

        storage
            .insert(&key_package_id, key_package.clone())
            .unwrap();

        let from_storage = storage.get(&key_package_id).unwrap().unwrap();
        assert_eq!(from_storage, key_package);
    }

    #[test]
    fn duplicate_insert_should_fail() {
        let mut storage = test_storage();
        let (key_package_id, key_package) = test_key_package();

        storage
            .insert(&key_package_id, key_package.clone())
            .unwrap();

        let dupe_res = storage.insert(&key_package_id, key_package);

        assert_matches!(dupe_res, Err(SqLiteDataStorageError::SqlEngineError(_)));
    }

    #[test]
    fn key_package_not_found() {
        let mut storage = test_storage();
        let (key_package_id, key_package) = test_key_package();

        storage.insert(&key_package_id, key_package).unwrap();

        let (another_package_id, _) = test_key_package();

        assert!(storage.get(&another_package_id).unwrap().is_none());
    }

    #[test]
    fn key_package_delete() {
        let mut storage = test_storage();
        let (key_package_id, key_package) = test_key_package();

        storage.insert(&key_package_id, key_package).unwrap();

        storage.delete(&key_package_id).unwrap();
        assert!(storage.get(&key_package_id).unwrap().is_none());
    }
}
