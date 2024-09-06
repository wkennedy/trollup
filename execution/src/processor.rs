//! A helper to initialize Solana SVM API's `TransactionBatchProcessor`.

use solana_program_runtime::loaded_programs::{ProgramCache, ProgramRuntimeEnvironment};
use solana_rbpf::program::BuiltinProgram;

use {
    solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_program_runtime::loaded_programs::{
        BlockRelation, ForkGraph, LoadProgramMetrics, ProgramCacheEntry,
    },
    solana_sdk::{account::ReadableAccount, clock::Slot, feature_set::FeatureSet, transaction},
    solana_svm::{
        account_loader::CheckedTransactionDetails,
        transaction_processing_callback::TransactionProcessingCallback,
        transaction_processor::TransactionBatchProcessor,
    },
    solana_system_program::system_processor,
    std::sync::{Arc, RwLock},
};
/// In order to use the `TransactionBatchProcessor`, another trait - Solana
/// Program Runtime's `ForkGraph` - must be implemented, to tell the batch
/// processor how to work across forks.
///
/// Since Trollup doesn't use slots or forks, this implementation is mocked.
pub(crate) struct TrollupForkGraph {}

impl ForkGraph for TrollupForkGraph {
    fn relationship(&self, _a: Slot, _b: Slot) -> BlockRelation {
        BlockRelation::Unrelated
    }

    // /// Returns the epoch of the given slot
    // fn slot_epoch(&self, _slot: Slot) -> Option<Epoch> {
    //     Some(0)
    // }
}

/// This function encapsulates some initial setup required to tweak the
/// `TransactionBatchProcessor` for use within PayTube.
///
/// We're simply configuring the mocked fork graph on the SVM API's program
/// cache, then adding the System program to the processor's builtins.
pub(crate) fn create_transaction_batch_processor<CB: TransactionProcessingCallback>(
    callbacks: &CB,
    feature_set: &FeatureSet,
    compute_budget: &ComputeBudget,
) -> (TransactionBatchProcessor<TrollupForkGraph>, Arc<RwLock<TrollupForkGraph>>) {
    let processor = TransactionBatchProcessor::<TrollupForkGraph>::default();
    let fork_graph = Arc::new(RwLock::new(TrollupForkGraph {}));

    {
        let mut cache = processor.program_cache.write().unwrap();

        // Initialize the mocked fork graph.
        cache.set_fork_graph(Arc::downgrade(&fork_graph));

        // Initialize a proper cache environment.
        // (Use Loader v4 program to initialize runtime v2 if desired)
        cache.environments.program_runtime_v1 = Arc::new(
            create_program_runtime_environment_v1(feature_set, compute_budget, false, true)
                .unwrap(),
        );

        // Add the SPL Token program to the cache.
        if let Some(program_account) = callbacks.get_account_shared_data(&spl_token::id()) {
            let elf_bytes = program_account.data();
            let program_runtime_environment = cache.environments.program_runtime_v1.clone();
            cache.assign_program(
                spl_token::id(),
                Arc::new(
                    ProgramCacheEntry::new(
                        &solana_sdk::bpf_loader::id(),
                        program_runtime_environment,
                        0,
                        0,
                        elf_bytes,
                        elf_bytes.len(),
                        &mut LoadProgramMetrics::default(),
                    )
                    .unwrap(),
                ),
            );
        }
    }

    // Add the system program builtin.
    processor.add_builtin(
        callbacks,
        solana_sdk::system_program::id(),
        "system_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"system_program".len(),
            system_processor::Entrypoint::vm,
        ),
    );

    // Add the BPF Loader v2 builtin, for the SPL Token program.
    processor.add_builtin(
        callbacks,
        solana_sdk::bpf_loader::id(),
        "solana_bpf_loader_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"solana_bpf_loader_program".len(),
            solana_bpf_loader_program::Entrypoint::vm,
        ),
    );

    (processor, fork_graph)
}

/// This functions is also a mock. In the Agave validator, the bank pre-checks
/// transactions before providing them to the SVM API. We mock this step in
/// PayTube, since we don't need to perform such pre-checks.
pub(crate) fn get_transaction_check_results(
    len: usize,
    lamports_per_signature: u64,
) -> Vec<transaction::Result<CheckedTransactionDetails>> {
    vec![
        transaction::Result::Ok(CheckedTransactionDetails {
            nonce: None,
            lamports_per_signature,
        });
        len
    ]
}


static MOCK_ENVIRONMENT: std::sync::OnceLock<ProgramRuntimeEnvironment> =
    std::sync::OnceLock::<ProgramRuntimeEnvironment>::new();

fn get_mock_env() -> ProgramRuntimeEnvironment {
    MOCK_ENVIRONMENT
        .get_or_init(|| Arc::new(BuiltinProgram::new_mock()))
        .clone()
}

fn new_mock_cache<FG: ForkGraph>() -> ProgramCache<FG> {
    let mut cache = ProgramCache::new(0, 0);
    cache.environments.program_runtime_v1 = get_mock_env();
    cache
}