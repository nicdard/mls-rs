// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// Copyright by contributors to this project.
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::{
    cipher_suite::CipherSuite,
    client::MlsError,
    group::framing::MLSMessage,
    key_package::validate_key_package_properties,
    protocol_version::ProtocolVersion,
    time::MlsTime,
    tree_kem::{
        leaf_node::LeafNodeSource,
        leaf_node_validator::{LeafNodeValidator, ValidationContext},
    },
    CryptoProvider,
};

pub mod builder;
mod config;
mod group;

use aws_mls_core::{crypto::SignatureSecretKey, identity::SigningIdentity};
pub(crate) use config::ExternalClientConfig;

use builder::{ExternalBaseConfig, ExternalClientBuilder};

pub use group::{ExternalGroup, ExternalReceivedMessage, ExternalSnapshot};

/// A client capable of observing a group's state without having
/// private keys required to read content.
///
/// This structure is useful when an application is sending
/// plaintext control messages in order to allow a central server
/// to facilitate communication between users.
///
/// # Warning
///
/// This structure will only be able to observe groups that were
/// created by clients that have the encrypt_controls
/// [preference](crate::client_builder::Preferences)
/// set to `false`. Any control messages that are sent encrypted
/// over the wire will break the ability of this client to track
/// the resulting group state.
pub struct ExternalClient<C> {
    config: C,
    signing_data: Option<(SignatureSecretKey, SigningIdentity)>,
}

impl ExternalClient<()> {
    pub fn builder() -> ExternalClientBuilder<ExternalBaseConfig> {
        ExternalClientBuilder::new()
    }
}

impl<C> ExternalClient<C>
where
    C: ExternalClientConfig + Clone,
{
    pub(crate) fn new(
        config: C,
        signing_data: Option<(SignatureSecretKey, SigningIdentity)>,
    ) -> Self {
        Self {
            config,
            signing_data,
        }
    }

    /// Begin observing a group based on a GroupInfo message created by
    /// [Group::group_info_message](crate::group::Group::group_info_message)
    ///
    ///`tree_data` is required to be provided out of band if the client that
    /// created GroupInfo message did not have the
    /// [ratchet tree extension preference](crate::client_builder::Preferences::ratchet_tree_extension)
    /// enabled at the time the welcome message was created. `tree_data` can
    /// be exported from a group using the
    /// [export tree function](crate::group::Group::export_tree).
    #[maybe_async::maybe_async]
    pub async fn observe_group(
        &self,
        group_info: MLSMessage,
        tree_data: Option<&[u8]>,
    ) -> Result<ExternalGroup<C>, MlsError> {
        ExternalGroup::join(
            self.config.clone(),
            self.signing_data.clone(),
            group_info,
            tree_data,
        )
        .await
    }

    /// Load an existing observed group by loading a snapshot that was
    /// generated by
    /// [ExternalGroup::snapshot](self::ExternalGroup::snapshot).
    #[maybe_async::maybe_async]
    pub async fn load_group(
        &self,
        snapshot: ExternalSnapshot,
    ) -> Result<ExternalGroup<C>, MlsError> {
        ExternalGroup::from_snapshot(self.config.clone(), snapshot).await
    }

    /// Utility function to validate key packages
    #[maybe_async::maybe_async]
    pub async fn validate_key_package(
        &self,
        package: MLSMessage,
        protocol: ProtocolVersion,
        cipher_suite: CipherSuite,
    ) -> Result<KeyPackageValidationOutput, MlsError> {
        let key_package = package
            .into_key_package()
            .ok_or(MlsError::UnexpectedMessageType)?;

        let cs = self
            .config
            .crypto_provider()
            .cipher_suite_provider(cipher_suite)
            .ok_or_else(|| MlsError::UnsupportedCipherSuite(cipher_suite))?;

        let id_provider = self.config.identity_provider();

        let validator = LeafNodeValidator::new(
            &cs,
            #[cfg(feature = "all_extensions")]
            None,
            &id_provider,
            None,
        );
        let context = ValidationContext::Add(Some(MlsTime::now()));

        validator
            .check_if_valid(&key_package.leaf_node, context)
            .await?;

        validate_key_package_properties(&key_package, protocol, &cs).await?;

        let expiration_timestamp =
            if let LeafNodeSource::KeyPackage(lifetime) = &key_package.leaf_node.leaf_node_source {
                lifetime.not_after
            } else {
                return Err(MlsError::InvalidLeafNodeSource);
            };

        Ok(KeyPackageValidationOutput {
            expiration_timestamp,
        })
    }
}

#[derive(Debug)]
pub struct KeyPackageValidationOutput {
    pub expiration_timestamp: u64,
}

#[cfg(test)]
pub(crate) mod tests_utils {
    pub use super::builder::test_utils::*;
}
