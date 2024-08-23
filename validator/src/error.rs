use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Commitment transaction failed.")]
    CommitmentTransactionFailed,
    #[error("Proof verification failed. Public inputs are not valid for the given proof.")]
    ProofVerificationFailed
}