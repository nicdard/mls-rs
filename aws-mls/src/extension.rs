use crate::group::proposal::ProposalType;
use crate::provider::crypto::HpkePublicKey;
use crate::time::MlsTime;
use crate::tree_kem::node::NodeVec;
use crate::{identity::CredentialType, identity::SigningIdentity};
use aws_mls_core::identity::IdentityProvider;
use futures::TryStreamExt;
use std::fmt::Debug;
use tls_codec_derive::{TlsDeserialize, TlsSerialize, TlsSize};

pub use aws_mls_core::extension::*;

const APPLICATION_ID_EXT_ID: u16 = 1;
const RATCHET_TREE_EXT_ID: u16 = 2;
const REQUIRED_CAPABILITIES_EXT_ID: u16 = 3;
const EXTERNAL_PUB_EXT_ID: u16 = 4;
const EXTERNAL_SENDERS_EXT_ID: u16 = 5;

#[derive(Clone, Debug, PartialEq, Eq, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ApplicationIdExt {
    #[tls_codec(with = "crate::tls::ByteVec")]
    pub identifier: Vec<u8>,
}

impl TlsCodecExtension for ApplicationIdExt {
    fn extension_type() -> ExtensionType {
        ExtensionType::from(APPLICATION_ID_EXT_ID)
    }
}

#[derive(Clone, Debug, PartialEq, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct RatchetTreeExt {
    pub(crate) tree_data: NodeVec,
}

impl TlsCodecExtension for RatchetTreeExt {
    fn extension_type() -> ExtensionType {
        ExtensionType::from(RATCHET_TREE_EXT_ID)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, TlsDeserialize, TlsSerialize, TlsSize, Default)]
pub struct RequiredCapabilitiesExt {
    #[tls_codec(with = "crate::tls::DefVec")]
    pub extensions: Vec<ExtensionType>,
    #[tls_codec(with = "crate::tls::DefVec")]
    pub proposals: Vec<ProposalType>,
    #[tls_codec(with = "crate::tls::DefVec")]
    pub credentials: Vec<CredentialType>,
}

impl TlsCodecExtension for RequiredCapabilitiesExt {
    fn extension_type() -> ExtensionType {
        ExtensionType::from(REQUIRED_CAPABILITIES_EXT_ID)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ExternalPubExt {
    #[tls_codec(with = "crate::tls::ByteVec")]
    pub external_pub: HpkePublicKey,
}

impl TlsCodecExtension for ExternalPubExt {
    fn extension_type() -> ExtensionType {
        ExtensionType::from(EXTERNAL_PUB_EXT_ID)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, TlsDeserialize, TlsSerialize, TlsSize)]
#[non_exhaustive]
pub struct ExternalSendersExt {
    #[tls_codec(with = "crate::tls::DefVec")]
    pub allowed_senders: Vec<SigningIdentity>,
}

impl ExternalSendersExt {
    pub fn new(allowed_senders: Vec<SigningIdentity>) -> Self {
        Self { allowed_senders }
    }

    pub async fn verify_all<I: IdentityProvider>(
        &self,
        provider: &I,
        timestamp: Option<MlsTime>,
    ) -> Result<(), I::Error> {
        futures::stream::iter(self.allowed_senders.iter().map(Ok))
            .try_for_each(|id| provider.validate(id, timestamp))
            .await
    }
}

impl TlsCodecExtension for ExternalSendersExt {
    fn extension_type() -> ExtensionType {
        ExtensionType::from(EXTERNAL_SENDERS_EXT_ID)
    }
}

#[cfg(any(test, feature = "benchmark"))]
pub(crate) mod test_utils {
    use std::convert::Infallible;

    use super::*;

    pub const TEST_EXTENSION_TYPE: u16 = 42;

    #[derive(TlsSize, TlsSerialize, TlsDeserialize, Clone, Debug, PartialEq)]
    pub(crate) struct TestExtension {
        pub(crate) foo: u8,
    }

    impl From<u8> for TestExtension {
        fn from(value: u8) -> Self {
            Self { foo: value }
        }
    }

    impl MlsExtension for TestExtension {
        type SerializationError = Infallible;

        type DeserializationError = Infallible;

        fn extension_type() -> ExtensionType {
            ExtensionType::from(TEST_EXTENSION_TYPE)
        }

        fn to_bytes(&self) -> Result<Vec<u8>, Self::SerializationError> {
            Ok([self.foo].to_vec())
        }

        fn from_bytes(data: &[u8]) -> Result<Self, Self::DeserializationError> {
            Ok(TestExtension { foo: data[0] })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client::test_utils::TEST_CIPHER_SUITE,
        extension::test_utils::TestExtension,
        group::test_utils::random_bytes,
        identity::{test_utils::get_test_signing_identity, BasicCredential},
    };

    use super::*;
    use assert_matches::assert_matches;

    use tls_codec::{Deserialize, Serialize};
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn test_key_id_extension() {
        let test_id = vec![0u8; 32];
        let test_extension = ApplicationIdExt {
            identifier: test_id.clone(),
        };

        let as_extension = test_extension.into_extension().unwrap();

        assert_eq!(as_extension.extension_type, APPLICATION_ID_EXT_ID.into());

        let restored = ApplicationIdExt::from_extension(&as_extension).unwrap();
        assert_eq!(restored.identifier, test_id);
    }

    #[test]
    fn test_ratchet_tree() {
        let ext = RatchetTreeExt {
            tree_data: NodeVec::from(vec![None, None]),
        };

        let as_extension = ext.clone().into_extension().unwrap();
        assert_eq!(as_extension.extension_type, RATCHET_TREE_EXT_ID.into());

        let restored = RatchetTreeExt::from_extension(&as_extension).unwrap();
        assert_eq!(ext, restored)
    }

    #[test]
    fn test_required_capabilities() {
        let ext = RequiredCapabilitiesExt {
            extensions: vec![0.into(), 1.into()],
            proposals: vec![42.into(), 43.into()],
            credentials: vec![BasicCredential::credential_type()],
        };

        let as_extension = ext.clone().into_extension().unwrap();

        assert_eq!(
            as_extension.extension_type,
            REQUIRED_CAPABILITIES_EXT_ID.into()
        );

        let restored = RequiredCapabilitiesExt::from_extension(&as_extension).unwrap();
        assert_eq!(ext, restored)
    }

    #[test]
    fn test_bad_deserialize_data() {
        let bad_data = vec![255u8; 32];

        let test_extension = Extension::new(APPLICATION_ID_EXT_ID.into(), bad_data);

        let capabilities: Result<ApplicationIdExt, ExtensionError> =
            ApplicationIdExt::from_extension(&test_extension);

        assert!(capabilities.is_err());
    }

    #[test]
    fn test_bad_deserialize_type() {
        let test_extension = Extension::new(RATCHET_TREE_EXT_ID.into(), vec![0u8; 32]);

        assert!(ApplicationIdExt::from_extension(&test_extension).is_err());
    }

    #[test]
    fn test_extension_list_get_set() {
        let mut list = ExtensionList::new();

        let required_capabilities = RequiredCapabilitiesExt::default();

        let ext_senders = ExternalSendersExt {
            allowed_senders: vec![],
        };

        // Add the extensions to the list
        list.set_from(required_capabilities.clone()).unwrap();
        list.set_from(ext_senders.clone()).unwrap();

        assert_eq!(list.len(), 2);

        assert_eq!(
            list.get_as::<RequiredCapabilitiesExt>().unwrap(),
            Some(required_capabilities)
        );

        assert_eq!(
            list.get_as::<ExternalSendersExt>().unwrap(),
            Some(ext_senders)
        );

        assert_eq!(list.get_as::<TestExtension>().unwrap(), None);

        // Overwrite the extension in the list
        let ext_senders = ExternalSendersExt {
            allowed_senders: vec![get_test_signing_identity(TEST_CIPHER_SUITE, vec![]).0],
        };

        list.set_from(ext_senders.clone()).unwrap();
        assert_eq!(list.len(), 2);

        assert_eq!(
            list.get_as::<ExternalSendersExt>().unwrap(),
            Some(ext_senders.clone())
        );

        assert_eq!(
            list.get_as::<ExternalSendersExt>().unwrap(),
            Some(ext_senders)
        );

        assert_eq!(list.get_as::<TestExtension>().unwrap(), None);
    }

    #[test]
    fn test_extension_list_has_ext() {
        let mut list = ExtensionList::new();

        let lifetime = ApplicationIdExt {
            identifier: random_bytes(32),
        };

        list.set_from(lifetime).unwrap();

        assert!(list.has_extension(APPLICATION_ID_EXT_ID.into()));
        assert!(!list.has_extension(42.into()));
    }

    #[derive(TlsSerialize, TlsSize)]
    struct ExtensionsVec(#[tls_codec(with = "crate::tls::DefVec")] Vec<Extension>);

    #[test]
    fn extension_list_is_serialized_like_a_sequence_of_extensions() {
        let extension_vec = vec![
            RequiredCapabilitiesExt::default().into_extension().unwrap(),
            ExternalSendersExt {
                allowed_senders: vec![],
            }
            .into_extension()
            .unwrap(),
        ];

        let extension_list: ExtensionList = ExtensionList::from(extension_vec.clone());

        assert_eq!(
            ExtensionsVec(extension_vec)
                .tls_serialize_detached()
                .unwrap(),
            extension_list.tls_serialize_detached().unwrap(),
        );
    }

    #[test]
    fn deserializing_extension_list_fails_on_duplicate_extension() {
        let extensions = ExtensionsVec(vec![
            RequiredCapabilitiesExt::default().into_extension().unwrap(),
            RequiredCapabilitiesExt::default().into_extension().unwrap(),
        ]);

        let serialized_extensions = extensions.tls_serialize_detached().unwrap();

        assert_matches!(
            ExtensionList::tls_deserialize(&mut &*serialized_extensions),
            Err(tls_codec::Error::DecodingError(_))
        );
    }
}
