use crate::{
    extension::ExtensionType,
    group::{proposal_filter::ProposalBundle, BorrowedProposal, ProposalType, Sender},
    key_package::KeyPackageValidationError,
    protocol_version::ProtocolVersion,
    tree_kem::{
        leaf_node::LeafNodeError, leaf_node_validator::LeafNodeValidationError, RatchetTreeError,
    },
};
use aws_mls_core::extension::ExtensionError;
use std::marker::PhantomData;
use thiserror::Error;

pub trait ProposalFilter: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// This is called to validate a received commit. It should report any error making the commit
    /// invalid.
    fn validate(&self, proposals: &ProposalBundle) -> Result<(), Self::Error>;

    /// This is called when preparing a commit. By-reference proposals causing the commit to be
    /// invalid should be filtered out. If a by-value proposal causes the commit to be invalid,
    /// an error should be returned.
    fn filter(&self, proposals: ProposalBundle) -> Result<ProposalBundle, Self::Error>;

    fn boxed(self) -> BoxedProposalFilter<Self::Error>
    where
        Self: Send + Sync + Sized + 'static,
    {
        Box::new(self)
    }
}

pub type BoxedProposalFilter<E> = Box<dyn ProposalFilter<Error = E> + Send + Sync>;

macro_rules! delegate_proposal_filter {
    ($implementer:ty) => {
        impl<T: ProposalFilter + ?Sized> ProposalFilter for $implementer {
            type Error = T::Error;

            fn validate(&self, proposals: &ProposalBundle) -> Result<(), Self::Error> {
                (**self).validate(proposals)
            }

            fn filter(&self, proposals: ProposalBundle) -> Result<ProposalBundle, Self::Error> {
                (**self).filter(proposals)
            }
        }
    };
}

delegate_proposal_filter!(Box<T>);
delegate_proposal_filter!(&T);

#[derive(Debug)]
#[non_exhaustive]
pub struct ProposalFilterContext {
    pub committer: Sender,
    pub proposer: Sender,
}

pub struct SimpleProposalFilter<F> {
    pub(crate) committer: Sender,
    pub(crate) filter: F,
}

impl<F, E> ProposalFilter for SimpleProposalFilter<F>
where
    F: Fn(&ProposalFilterContext, &BorrowedProposal<'_>) -> Result<(), E> + Send + Sync,
    E: std::error::Error + Send + Sync + 'static,
{
    type Error = E;

    fn validate(&self, proposals: &ProposalBundle) -> Result<(), Self::Error> {
        proposals.iter_proposals().try_for_each(|proposal| {
            let context = ProposalFilterContext {
                committer: self.committer.clone(),
                proposer: proposal.sender.clone(),
            };

            (self.filter)(&context, &proposal.proposal)
        })
    }

    fn filter(&self, mut proposals: ProposalBundle) -> Result<ProposalBundle, Self::Error> {
        proposals.retain(|proposal| {
            let context = ProposalFilterContext {
                committer: self.committer.clone(),
                proposer: proposal.sender.clone(),
            };

            Ok((self.filter)(&context, &proposal.proposal).map_or(false, |_| true))
        })?;

        Ok(proposals)
    }
}

#[derive(Clone, Debug)]
pub struct PassThroughProposalFilter<E> {
    phantom: PhantomData<fn() -> E>,
}

impl<E> PassThroughProposalFilter<E> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<E> Default for PassThroughProposalFilter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> ProposalFilter for PassThroughProposalFilter<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Error = E;

    fn validate(&self, _: &ProposalBundle) -> Result<(), Self::Error> {
        Ok(())
    }

    fn filter(&self, proposals: ProposalBundle) -> Result<ProposalBundle, Self::Error> {
        Ok(proposals)
    }
}

#[derive(Debug, Error)]
pub enum ProposalFilterError {
    #[error(transparent)]
    KeyPackageValidationError(#[from] KeyPackageValidationError),
    #[error(transparent)]
    LeafNodeValidationError(#[from] LeafNodeValidationError),
    #[error(transparent)]
    RatchetTreeError(#[from] RatchetTreeError),
    #[error(transparent)]
    ExtensionError(#[from] ExtensionError),
    #[error(transparent)]
    LeafNodeError(#[from] LeafNodeError),
    #[error("Commiter must not include any update proposals generated by the commiter")]
    InvalidCommitSelfUpdate,
    #[error("A PreSharedKey proposal must have a PSK of type External or type Resumption and usage Application")]
    InvalidTypeOrUsageInPreSharedKeyProposal,
    #[error("Expected PSK nonce with length {expected} but found length {found}")]
    InvalidPskNonceLength { expected: usize, found: usize },
    #[error("Protocol version {proposed:?} in ReInit proposal is less than version {original:?} in original group")]
    InvalidProtocolVersionInReInit {
        proposed: ProtocolVersion,
        original: ProtocolVersion,
    },
    #[error("More than one proposal applying to leaf {0:?}")]
    MoreThanOneProposalForLeaf(u32),
    #[error("More than one GroupContextExtensions proposal")]
    MoreThanOneGroupContextExtensionsProposal,
    #[error("Invalid {} proposal of type {proposal_type:?} for sender {sender:?}", by_ref_or_value_str(*.by_ref))]
    InvalidProposalTypeForSender {
        proposal_type: ProposalType,
        sender: Sender,
        by_ref: bool,
    },
    #[error("External commit must have exactly one ExternalInit proposal")]
    ExternalCommitMustHaveExactlyOneExternalInit,
    #[error("External commit must have a new leaf")]
    ExternalCommitMustHaveNewLeaf,
    #[error("External sender cannot commit")]
    ExternalSenderCannotCommit,
    #[error("Missing update path in external commit")]
    MissingUpdatePathInExternalCommit,
    #[error("External commit contains removal of other identity")]
    ExternalCommitRemovesOtherIdentity,
    #[error("External commit contains more than one Remove proposal")]
    ExternalCommitWithMoreThanOneRemove,
    #[error("Duplicate PSK IDs")]
    DuplicatePskIds,
    #[error("Invalid proposal type {0:?} in external commit")]
    InvalidProposalTypeInExternalCommit(ProposalType),
    #[error("Committer can not remove themselves")]
    CommitterSelfRemoval,
    #[error(transparent)]
    UserDefined(Box<dyn std::error::Error + Send + Sync>),
    #[error("Only members can commit proposals by reference")]
    OnlyMembersCanCommitProposalsByRef,
    #[error("Other proposal with ReInit")]
    OtherProposalWithReInit,
    #[error("Removing blank node at index {0:?}")]
    RemovingBlankNode(u32),
    #[error("Unsupported group extension {0:?}")]
    UnsupportedGroupExtension(ExtensionType),
    #[error("Unsupported custom proposal type {0:?}")]
    UnsupportedCustomProposal(ProposalType),
    #[error(transparent)]
    PskIdValidationError(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    IdentityProviderError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Invalid index {0:?} for member proposer")]
    InvalidMemberProposer(u32),
    #[error("Invalid external sender index {0}")]
    InvalidExternalSenderIndex(u32),
    #[error("External sender without External Senders extension")]
    ExternalSenderWithoutExternalSendersExtension,
}

impl ProposalFilterError {
    pub fn user_defined<E>(e: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::UserDefined(e.into())
    }
}

fn by_ref_or_value_str(by_ref: bool) -> &'static str {
    if by_ref {
        "by reference"
    } else {
        "by value"
    }
}
