//! # SPEL Framework Core
//!
//! Core types and traits for the SPEL program framework.

pub mod context;
#[cfg(feature = "decode")]
pub mod decode;
pub mod error;
pub mod idl;
pub mod pda;
pub mod slot_layout;
pub mod spel_output;
pub mod types;
pub mod validation;

#[cfg(feature = "idl-gen")]
pub mod account_types;
#[cfg(feature = "idl-gen")]
pub mod extension;
#[cfg(feature = "idl-gen")]
pub mod idl_gen;

#[cfg(feature = "idl-gen")]
pub mod dep_walk;

#[cfg(test)]
mod test_utils;

pub use slot_layout::{FixedBorshSize, SlotLayoutProbe};

pub mod prelude {
    pub use crate::error::{SpelError, SpelResult};
    pub use crate::pda::{compute_pda, compute_pda_multi, seed_from_str, ToSeed};
    pub use crate::types::{AccountConstraint, SpelOutput, SpelOutputParts};

    // nssa_core::account
    //
    // An account is `{ nonce, data: AccountData { shards } }` since LEZ v0.3.0.
    // `AccountWithMetadata`, `BalanceDiff` and `Data` are gone with it: a plan sees
    // `AccountMeta` (id, authorization, selected shard) and never account data; an
    // apply sees one `ShardData`. Native balance is the shard at
    // `NATIVE_TOKEN_PROGRAM_ID`, so there is no balance field to diff.
    pub use nssa_core::account::{
        Account, AccountData, AccountId, ProgramShardSelector, ShardData,
    };

    // nssa_core::program
    //
    // `ProgramId` stays exported, but since LEZ v0.2.5 it is only a program's image
    // id — a program's *identity* (owner, caller, PDA authority) is an `AccountId`.
    //
    // `AccountMeta` is re-exported under its own name even though
    // `crate::types::AccountMeta` exists: the latter is IDL metadata about an
    // account *parameter*, this is the runtime handle a plan receives. Programs
    // want this one, so the prelude gives it the plain name; the IDL type is
    // reached as `types::AccountMeta`.
    pub use nssa_core::program::{
        AccountMeta, ApplyInput, ApplyOutput, BlockValidityWindow, ChainedCall, InvalidWindow,
        PdaSeed, PlanInput, PlanOutput, ProgramEvent, ProgramId, ShardEffect,
        TimestampValidityWindow, ValidityWindow,
    };

    // nssa_core extras
    pub use nssa_core::{BlockId, Identifier, Timestamp};

    // Guest entry points. `run_program(plan, apply)` is the v0.3.0 shape; the
    // generated `main` uses the lower-level `read_program_call` so it can own the
    // dispatch, and `apply_keep` / `apply_write` are what a hand-written apply
    // arm returns through.
    pub use nssa_core::program::{
        apply_keep, apply_write, read_program_call, run_program, write_once, CallKind,
        InstructionData, Plan, ProgramCall,
    };

    // Execution context for instruction handlers (issue #172)
    pub use crate::context::ProgramContext;

    pub use crate::slot_layout::{FixedBorshSize, SlotLayoutProbe};

    // nssa::public_transaction (host-only)
    #[cfg(feature = "host")]
    pub use nssa::public_transaction::{Message, WitnessSet};
}
