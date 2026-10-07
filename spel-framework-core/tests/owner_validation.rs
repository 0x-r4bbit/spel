//! Test that #[account(owner = self_program_id)] generates runtime validation checks.
//!
//! This is an expansion test — we simulate the validation functions that the macro
//! would generate for owner constraints.

use nssa_core::account::AccountId;
use nssa_core::program::AccountMeta;

use spel_framework_core::error::SpelError;

/// Simulate the validation function that the macro would generate for:
/// ```
/// #[plan]
/// pub fn initialize_holding(
///     ctx: ProgramContext,
///     #[account(owner = self_program_id)]
///     definition_account: AccountMeta,
///     #[account(init, signer)]
///     holding_account: AccountMeta,
/// ) -> SpelResult { ... }
/// ```
///
/// Note what is *not* here: `#[account(init)]` no longer generates a plan-phase
/// check. "This account is fresh" was `account != Account::default()`, which needs
/// account data a plan does not receive — and it is the wrong question now, since
/// an address is not owned. What matters is whether *this program's shard* of it is
/// empty, which is `pre_data.is_empty()` in the apply half, where
/// `lee_core::program::write_once` is the canonical guard. The constraint still
/// reaches the IDL, because clients use it to decide what a transaction must create.
fn __validate_initialize_holding(
    accounts: &[AccountMeta],
    self_account_id: &AccountId,
) -> Result<(), SpelError> {
    // Account index 0 has #[account(owner = self_program_id)].
    //
    // Ownership is no longer a field: a shard is keyed by the program that owns
    // it, so this asks whether the handle selects *our* shard. That is strictly
    // stronger than the v0.2.5 check — bytes under our key can only have been
    // written by us, so matching the selector makes provenance structural rather
    // than historical.
    if accounts[0].program_account_id != *self_account_id {
        return Err(SpelError::AccountOwnerMismatch {
            account_name: "definition_account".to_string(),
        });
    }
    // Account index 1 has #[account(signer)]
    if !accounts[1].is_authorized {
        return Err(SpelError::Unauthorized {
            message: format!(
                "Account {} (index {}) must be a signer",
                "holding_account", 1
            ),
        });
    }
    Ok(())
}

/// Since LEZ v0.2.5 a program is identified by the `AccountId` of its header account.
fn make_program_id(bytes: [u8; 32]) -> AccountId {
    AccountId::new(bytes)
}

/// A handle selecting `owner`'s shard of `id`.
fn make_account_with_owner(id: [u8; 32], owner: AccountId, authorized: bool) -> AccountMeta {
    AccountMeta::new(AccountId::new(id), authorized, owner)
}

#[test]
fn test_owner_matches_self_program_id() {
    let program_id = make_program_id([1u8; 32]);
    let accounts = vec![
        // definition_account owned by this program ✓
        make_account_with_owner([2u8; 32], program_id, false),
        // holding_account: init + signer (empty, authorized)
        make_account_with_owner([3u8; 32], AccountId::default(), true),
    ];
    assert!(__validate_initialize_holding(&accounts, &program_id).is_ok());
}

#[test]
fn test_owner_mismatch_returns_error() {
    let program_id = make_program_id([1u8; 32]);
    let other_program = make_program_id([99u8; 32]);
    let accounts = vec![
        // definition_account owned by DIFFERENT program ✗
        make_account_with_owner([2u8; 32], other_program, false),
        make_account_with_owner([3u8; 32], AccountId::default(), true),
    ];
    let result = __validate_initialize_holding(&accounts, &program_id);
    assert!(result.is_err());
    match result.unwrap_err() {
        SpelError::AccountOwnerMismatch { account_name } => {
            assert_eq!(account_name, "definition_account");
        },
        other => panic!("expected AccountOwnerMismatch, got: {other:?}"),
    }
}

#[test]
fn test_owner_check_runs_before_signer() {
    // Owner check is first in the validation chain, so it surfaces even when the
    // signer check would also fail. The `init` half of this test is gone: a
    // non-empty shard is not expressible in a plan and no longer checked here.
    let program_id = make_program_id([1u8; 32]);
    let other_program = make_program_id([99u8; 32]);
    let accounts = vec![
        // definition_account owned by DIFFERENT program ✗
        make_account_with_owner([2u8; 32], other_program, false),
        // holding_account: NOT authorized (signer violated)
        make_account_with_owner([3u8; 32], AccountId::default(), false),
    ];
    let result = __validate_initialize_holding(&accounts, &program_id);
    // Owner check runs first, so we get AccountOwnerMismatch, not the signer error.
    match result.unwrap_err() {
        SpelError::AccountOwnerMismatch { account_name } => {
            assert_eq!(account_name, "definition_account");
        },
        other => panic!("expected AccountOwnerMismatch (owner check runs first), got: {other:?}"),
    }
}

#[test]
fn test_owner_error_code() {
    let err = SpelError::AccountOwnerMismatch {
        account_name: "test_account".to_string(),
    };
    assert_eq!(err.error_code(), 1010);
}
