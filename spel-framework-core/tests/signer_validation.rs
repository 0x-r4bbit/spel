//! Test that #[account(signer)] generates runtime validation checks, and that
//! `#[account(init)]`'s guarantee survives its move to the apply phase.
//!
//! This is an expansion test — we cannot run the macro in a unit test directly,
//! so we test the validation functions that would be generated.

use nssa_core::account::AccountId;
use nssa_core::program::{write_once, AccountMeta};
use spel_framework_core::error::SpelError;

/// Simulate the validation function that the macro would generate for:
/// ```
/// #[plan]
/// pub fn transfer(
///     #[account(mut)] from: AccountMeta,
///     #[account(signer)] authority: AccountMeta,
///     #[account(mut)] to: AccountMeta,
/// ) -> SpelResult { ... }
/// ```
fn __validate_transfer(accounts: &[AccountMeta]) -> Result<(), SpelError> {
    // Account index 1 has #[account(signer)]
    if !accounts[1].is_authorized {
        return Err(SpelError::Unauthorized {
            message: format!("Account {} (index {}) must be a signer", "authority", 1),
        });
    }
    Ok(())
}

/// Simulate validation for an init + signer instruction:
/// ```
/// #[plan]
/// pub fn create_state(
///     #[account(init)] state: AccountMeta,
///     #[account(signer)] creator: AccountMeta,
/// ) -> SpelResult { ... }
/// ```
///
/// Only the signer check lives here now. `#[account(init)]` asked
/// `account != Account::default()`, which needs account data a plan does not
/// receive; its guarantee moved to the apply half, simulated by
/// [`apply_create_state`] below.
fn __validate_create_state(accounts: &[AccountMeta]) -> Result<(), SpelError> {
    // Account index 1 has #[account(signer)]
    if !accounts[1].is_authorized {
        return Err(SpelError::Unauthorized {
            message: format!("Account {} (index {}) must be a signer", "creator", 1),
        });
    }
    Ok(())
}

/// Simulate the apply half that now carries `#[account(init)]`'s guarantee.
///
/// `write_once` is the framework's canonical form: it accepts a write into an
/// empty shard, accepts an idempotent rewrite of identical bytes, and panics on
/// anything else. A panic fails the whole transaction, which is what the
/// `AccountAlreadyInitialized` error used to do from the plan phase.
fn apply_create_state(pre_data: &[u8], new_data: Vec<u8>) -> Vec<u8> {
    write_once(pre_data, new_data)
}

/// A plan-phase handle. No account data — a plan never sees any.
fn make_account(id: [u8; 32], authorized: bool) -> AccountMeta {
    AccountMeta::new(AccountId::new(id), authorized, AccountId::new([0u8; 32]))
}

#[test]
fn test_signer_authorized_passes() {
    let accounts = vec![
        make_account([1u8; 32], false), // from (mut, not signer)
        make_account([2u8; 32], true),  // authority (signer) ← authorized
        make_account([3u8; 32], false), // to (mut, not signer)
    ];
    assert!(__validate_transfer(&accounts).is_ok());
}

#[test]
fn test_signer_unauthorized_fails() {
    let accounts = vec![
        make_account([1u8; 32], false),
        make_account([2u8; 32], false), // authority NOT authorized
        make_account([3u8; 32], false),
    ];
    let err = __validate_transfer(&accounts).unwrap_err();
    match err {
        SpelError::Unauthorized { message } => {
            assert!(message.contains("authority"));
            assert!(message.contains("index 1"));
        },
        _ => panic!("Expected Unauthorized error, got {err:?}"),
    }
}

/// The plan half passes regardless of shard contents: it cannot see them.
#[test]
fn test_plan_half_accepts_any_shard_state() {
    let accounts = vec![
        make_account([1u8; 32], false), // state (init — unchecked in the plan phase)
        make_account([2u8; 32], true),  // creator (signer, authorized)
    ];
    assert!(__validate_create_state(&accounts).is_ok());
}

/// An empty shard is the "fresh" case `#[account(init)]` used to assert.
#[test]
fn test_init_empty_shard_accepts_the_write() {
    assert_eq!(apply_create_state(&[], vec![42]), vec![42]);
}

/// Rewriting the same bytes is idempotent, so a replayed effect is not a failure.
#[test]
fn test_init_identical_rewrite_is_allowed() {
    assert_eq!(apply_create_state(&[42], vec![42]), vec![42]);
}

/// The guarantee that actually moved: a non-empty shard cannot be overwritten with
/// different data. This was `AccountAlreadyInitialized` from the plan phase; it is
/// now a panic from the apply phase, which fails the transaction just the same.
#[test]
#[should_panic(expected = "shard already holds different data")]
fn test_init_already_initialized_panics_in_apply() {
    let _ = apply_create_state(&[42], vec![7]);
}

/// init and signer are now enforced in *different phases*, so one cannot mask the
/// other: the signer check fails in the plan half before any apply runs.
#[test]
fn test_init_and_signer_are_checked_in_different_phases() {
    let accounts = vec![
        make_account([1u8; 32], false),
        make_account([2u8; 32], false), // not authorized
    ];
    let err = __validate_create_state(&accounts).unwrap_err();
    assert!(matches!(err, SpelError::Unauthorized { .. }));
}
