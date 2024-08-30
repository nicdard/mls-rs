
use mls_rs_core::{group::GroupState, group::EpochRecord, group::GroupStateStorage};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasm_bindgen::JsValue;
use web_sys::Storage;

pub(crate) const DEFAULT_EPOCH_RETENTION_LIMIT: u64 = 3;

pub(crate) const DEFAULT_STORAGE_KEY: &'static str = "SSF-MLS-STATE";

#[derive(Debug, Error)]
pub enum WebGroupStateStorageError {
    #[error("Local storage was not found")]
    LocalStorageUnavailable,
    #[error("JS error {0}")]
    JsValue(String)
}

impl From<JsValue> for WebGroupStateStorageError {
    fn from(e: JsValue) -> Self {
        Self::JsValue(format!("{e:?}"))
    }
}

impl mls_rs_core::error::IntoAnyError for WebGroupStateStorageError {
    fn into_dyn_error(self) -> Result<Box<dyn std::error::Error + Send + Sync>, Self> {
        Ok(self.into())
    }
}

fn get_local_storage() -> Result<Storage, WebGroupStateStorageError> {
    Ok(web_sys::window()
        .ok_or(WebGroupStateStorageError::LocalStorageUnavailable)?
        .local_storage()?
        .ok_or(WebGroupStateStorageError::LocalStorageUnavailable)?
    )
}

// https://github.com/rustwasm/wasm-bindgen/blob/main/examples/todomvc/src/store.rs

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupDB {
    db: Map<
}

pub struct WebLocalStateStorage {
    max_epoch_retention: u64,
}

impl WebLocalStateStorage {
    pub(crate) fn new() -> WebLocalStateStorage {
        WebLocalStateStorage {
            max_epoch_retention: DEFAULT_EPOCH_RETENTION_LIMIT
        }
    }

    pub(crate) fn with_max_epoch_retention(self, max_epoch_retention: u64) -> Self {
        Self {
            max_epoch_retention,
        }
    }


    pub fn group_ids(&self) -> Result<Vec<Vec<String>>, WebGroupStateStorageError> {
        let storage = get_local_storage()?;
        
        let value = storage.get_item(&DEFAULT_STORAGE_KEY)?;

        value.map(|db| {
            
        })

        let mut statement = connection
            .prepare("SELECT group_id FROM mls_group")
            .map_err(|e| SqLiteDataStorageError::SqlEngineError(e.into()))?;

        let res = statement
            .query_map([], |row| row.get(0))
            .map_err(|e| SqLiteDataStorageError::SqlEngineError(e.into()))?
            .try_fold(Vec::new(), |mut ids, id| {
                ids.push(id.map_err(|e| SqLiteDataStorageError::DataConversionError(e.into()))?);
                Ok::<_, SqLiteDataStorageError>(ids)
            })
            .map_err(|e| SqLiteDataStorageError::SqlEngineError(e.into()))?;

        Ok(res)
    }

}

impl GroupStateStorage for WebLocalStateStorage {
    type Error = WebGroupStateStorageError;
    
    #[doc = " Fetch a group state from storage."]
    fn state(&self, group_id: &[u8]) -> Result<Option<Vec<u8> > ,Self::Error>  {
        todo!()
    }
    
    #[doc = " Lazy load cached epoch data from a particular group."]
    fn epoch(&self, group_id: &[u8], epoch_id:u64) -> Result<Option<Vec<u8> > ,Self::Error>  {
        todo!()
    }
    
    #[doc = " Write pending state updates."]
    #[doc = ""]
    #[doc = " The group id that this update belongs to can be retrieved with"]
    #[doc = " [`GroupState::id`]. Prior epoch id values can be retrieved with"]
    #[doc = " [`EpochRecord::id`]."]
    #[doc = ""]
    #[doc = " The protocol implementation handles managing the max size of a prior epoch"]
    #[doc = " cache and the deleting of prior states based on group activity."]
    #[doc = " The maximum number of prior epochs that will be stored is controlled by the"]
    #[doc = " `Preferences::max_epoch_retention` function in `mls_rs`."]
    #[doc = " value. Requested deletes are communicated by the `delete_epoch_under`"]
    #[doc = " parameter being set to `Some`."]
    #[doc = ""]
    #[doc = " # Warning"]
    #[doc = ""]
    #[doc = " It is important to consider error recovery when creating an implementation"]
    #[doc = " of this trait. Calls to [`write`](GroupStateStorage::write) should"]
    #[doc = " optimally be a single atomic transaction in order to avoid partial writes"]
    #[doc = " that may corrupt the group state."]
    fn write(&mut self, state:GroupState, epoch_inserts:Vec<EpochRecord>, epoch_updates:Vec<EpochRecord>) -> Result<(),Self::Error>  {
        todo!()
    }
    
    #[doc = " The [`EpochRecord::id`] value that is associated with a stored"]
    #[doc = " prior epoch for a particular group."]
    fn max_epoch_id(&self, group_id: &[u8]) -> Result<Option<u64> ,Self::Error>  {
        todo!()
    }
}