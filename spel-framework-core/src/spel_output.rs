//! Plan construction for `SpelOutput`.
//!
//! A plan-phase handler builds a [`SpelOutput`] and the generated `main` forwards
//! it to `lee_core`'s `Plan`. The helpers here mirror `Plan`'s own surface
//! (`effect`, `inspect`, `call`) so a handler reads the same way whether or not
//! it goes through SPEL.
//!
//! # Where the state diffs went
//!
//! Up to LEZ v0.2.5 a handler returned `AccountStateDiff` values: a balance delta
//! plus, when changed, replacement data for an account it had in hand. v0.3.0
//! splits that in two. The plan half decides *which shard gets which effect* and
//! never sees account data; the apply half receives one shard's bytes and returns
//! its replacement. So there is nothing here to build a diff from, and
//! `IntoStateDiff` is gone along with `BalanceDiff` — native balance is now the
//! shard at `NATIVE_TOKEN_PROGRAM_ID`, moved by chain-calling the native token
//! program rather than by writing a field.
//!
//! # Where the claims went
//!
//! Up to LEZ v0.2.4 a program declared ownership explicitly, via
//! `Claim::Authorized` / `Claim::Pda(seed)` attached to each post-state. v0.2.5
//! removed `Claim`: writing data to a default-owned account *was* the claim.
//! v0.3.0 removes the concept entirely — a program may write `shards[self]` at any
//! address, whenever it likes, and doing so denies that address to nobody. So
//! `#[account(init)]` has nothing to emit here; "is this shard fresh?" is
//! `pre_data.is_empty()`, asked in the apply half.
//!
//! The PDA *address* guarantee is unaffected — it never came from the claim. The
//! generated per-instruction validation still checks each `pda`-constrained
//! account against [`crate::pda::compute_pda`], and under v0.3.0 that check
//! carries more weight, not less: a guard proves only that a declared value
//! matches *some* shard, so the address assertion is what makes it the right one.

use borsh::BorshSerialize;
use nssa_core::account::AccountId;
use nssa_core::program::{AccountMeta as ShardAccountMeta, ChainedCall, ShardEffect};

use crate::types::SpelOutput;

impl SpelOutput {
    /// Build a plan from already-constructed effects and chained calls.
    ///
    /// Prefer the [`effect`](Self::effect) / [`guard`](Self::guard) /
    /// [`call`](Self::call) builders, which carry the owner assertion. This entry
    /// point exists for handlers that assemble their effects elsewhere.
    pub fn plan(effects: Vec<ShardEffect>, calls: Vec<ChainedCall>) -> Self {
        let mut out = Self::empty();
        out.effects = effects;
        out.chained_calls = calls;
        out
    }

    /// Emit an effect on one of **this program's own** shards.
    ///
    /// Equivalent to `Plan::effect`. The apply half receives `effect` together
    /// with that shard's current bytes, and returns `Some(data)` to write or
    /// `None` to leave it alone.
    ///
    /// # Panics
    /// Panics if `account` names another program's shard — emit those with
    /// [`guard`](Self::guard) instead. This mirrors `Plan::effect`, which is
    /// `inspect` with the executing program as the owner.
    pub fn effect<E: BorshSerialize>(
        self,
        account: &ShardAccountMeta,
        self_account_id: AccountId,
        effect: &E,
    ) -> Self {
        self.guard(account, self_account_id, effect)
    }

    /// Emit an effect on the shard of `owner`, which may be another program.
    ///
    /// Equivalent to `Plan::inspect`. An effect on a foreign shard is read-only:
    /// its apply **must** return `None`, or the runtime rejects the transaction
    /// with `ForeignShardWrite`. Note that the runtime does *not* catch the
    /// converse — a guard on one of our own shards that returns `Some` writes it.
    ///
    /// # Panics
    /// Panics if `account` does not name `owner`'s shard, with the same message
    /// shape as `Plan::inspect`.
    pub fn guard<E: BorshSerialize>(
        mut self,
        account: &ShardAccountMeta,
        owner: AccountId,
        guard: &E,
    ) -> Self {
        assert_eq!(
            account.program_account_id, owner,
            "An effect on {} names the shard of {}, not of {owner}",
            account.account_id, account.program_account_id
        );
        self.effects.push(ShardEffect::new(account, guard));
        self
    }

    /// Append a chained call to another program.
    ///
    /// Chained calls run *after* every effect of this call, depth-first, and are
    /// bounded by `MAX_NUMBER_CHAINED_CALLS` across the whole transaction.
    pub fn call(mut self, call: ChainedCall) -> Self {
        self.chained_calls.push(call);
        self
    }
}

#[cfg(test)]
mod tests {
    use nssa_core::account::ProgramShardSelector;

    use super::*;

    const SELF_PROGRAM: AccountId = AccountId::new([9; 32]);
    const OTHER_PROGRAM: AccountId = AccountId::new([8; 32]);

    #[derive(BorshSerialize)]
    struct Payload(u8);

    /// An account handle naming `program`'s shard of account `id`.
    fn meta(id: u8, program: AccountId) -> ShardAccountMeta {
        ShardAccountMeta::new(AccountId::new([id; 32]), false, program)
    }

    fn call_to(program: u8) -> ChainedCall {
        ChainedCall::new(
            AccountId::new([program; 32]),
            vec![ProgramShardSelector {
                account_id: AccountId::new([program; 32]),
                program_account_id: AccountId::new([program; 32]),
            }],
            &Payload(0),
        )
    }

    #[test]
    fn empty_plan_has_nothing() {
        let out = SpelOutput::empty();
        assert!(out.effects.is_empty());
        assert!(out.chained_calls.is_empty());
        assert!(out.events.is_empty());
    }

    #[test]
    fn effect_targets_our_own_shard() {
        let out = SpelOutput::empty().effect(&meta(1, SELF_PROGRAM), SELF_PROGRAM, &Payload(7));
        assert_eq!(out.effects.len(), 1);
        assert_eq!(out.effects[0].selector.program_account_id, SELF_PROGRAM);
        assert_eq!(out.effects[0].data, borsh::to_vec(&Payload(7)).unwrap());
    }

    #[test]
    fn guard_targets_a_foreign_shard() {
        let out = SpelOutput::empty().guard(&meta(1, OTHER_PROGRAM), OTHER_PROGRAM, &Payload(0));
        assert_eq!(out.effects.len(), 1);
        assert_eq!(out.effects[0].selector.program_account_id, OTHER_PROGRAM);
    }

    /// `effect` is `inspect` with ourselves as the owner, so naming someone
    /// else's shard is a planning error rather than a silent foreign write.
    #[test]
    #[should_panic(expected = "names the shard of")]
    fn effect_on_a_foreign_shard_panics() {
        let _ = SpelOutput::empty().effect(&meta(1, OTHER_PROGRAM), SELF_PROGRAM, &Payload(0));
    }

    #[test]
    #[should_panic(expected = "names the shard of")]
    fn guard_naming_the_wrong_owner_panics() {
        let _ = SpelOutput::empty().guard(&meta(1, SELF_PROGRAM), OTHER_PROGRAM, &Payload(0));
    }

    #[test]
    fn effects_keep_emission_order() {
        let out = SpelOutput::empty()
            .effect(&meta(1, SELF_PROGRAM), SELF_PROGRAM, &Payload(1))
            .effect(&meta(2, SELF_PROGRAM), SELF_PROGRAM, &Payload(2));
        let ids: Vec<_> = out.effects.iter().map(|e| e.selector.account_id).collect();
        assert_eq!(ids, vec![AccountId::new([1; 32]), AccountId::new([2; 32])]);
    }

    #[test]
    fn calls_reach_the_output() {
        let out = SpelOutput::empty().call(call_to(7));
        assert_eq!(out.chained_calls.len(), 1);
        assert_eq!(
            out.chained_calls[0].program_account_id,
            AccountId::new([7; 32])
        );
    }

    #[test]
    fn plan_takes_prebuilt_effects_and_calls() {
        let out = SpelOutput::plan(
            vec![ShardEffect::new(&meta(1, SELF_PROGRAM), &Payload(3))],
            vec![call_to(7)],
        );
        assert_eq!(out.effects.len(), 1);
        assert_eq!(out.chained_calls.len(), 1);
    }
}
