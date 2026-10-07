//! Core types for the SPEL framework.
//!
//! These are thin wrappers/adapters that bridge framework ergonomics
//! with real SPEL core types.

use nssa_core::program::{
    BlockValidityWindow, ChainedCall, InvalidWindow, ProgramEvent, ShardEffect,
    TimestampValidityWindow, ValidityWindow,
};

/// What a plan-phase handler returns: the shape of `lee_core`'s `PlanOutput`.
///
/// A handler accumulates one of these and the generated `main` forwards it to the
/// real `Plan`. Keeping it a plain value (rather than handing handlers a
/// `&mut Plan`) is what lets host-side tests call handlers directly and inspect
/// what they planned without a zkVM.
///
/// There is no "unchanged account" entry any more. Up to LEZ v0.2.5 every
/// declared account had to appear in the output or the transaction failed with
/// `DeclaredAccountMissingFromOutput`, so handlers echoed read-only accounts back
/// as `AccountStateDiff::unchanged`. Under v0.3.0 an account with no effect is
/// simply untouched, and a shard read without a write is an effect whose apply
/// returns `None`. Emitting nothing is the correct way to leave a shard alone.
///
/// This struct is `#[non_exhaustive]` to allow future field additions without
/// breaking external programs that construct it via struct literals.
/// Use the builder methods (`plan()`, `effect()`, `guard()`, `with_*()`) instead.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SpelOutput {
    /// Per-shard effects, applied in this order.
    pub effects: Vec<ShardEffect>,
    pub chained_calls: Vec<ChainedCall>,
    /// Events emitted alongside the plan.
    pub events: Vec<ProgramEvent>,
    pub block_validity_window: BlockValidityWindow,
    pub timestamp_validity_window: TimestampValidityWindow,
}

impl SpelOutput {
    /// Create an empty output.
    pub fn empty() -> Self {
        Self {
            effects: vec![],
            chained_calls: vec![],
            events: vec![],
            block_validity_window: ValidityWindow::new_unbounded(),
            timestamp_validity_window: ValidityWindow::new_unbounded(),
        }
    }

    /// Emit an event alongside the plan.
    pub fn with_event(mut self, event: ProgramEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Restrict the block range in which the transaction is valid (infallible).
    ///
    /// Accepts any infallible range conversion: `1..`, `..100`, or `..` (unbounded).
    /// For the fallible variant (returns `Err` on empty/inverted ranges), see
    /// [`try_with_block_validity_window`](Self::try_with_block_validity_window).
    pub fn with_block_validity_window<W: Into<BlockValidityWindow>>(mut self, window: W) -> Self {
        self.block_validity_window = window.into();
        self
    }

    /// Restrict the block range in which the transaction is valid (fallible).
    ///
    /// Returns `Err(InvalidWindow)` if `window` is an empty range (e.g. `5..5` or `10..5`).
    /// For the infallible variant, see [`with_block_validity_window`](Self::with_block_validity_window).
    pub fn try_with_block_validity_window<
        W: TryInto<BlockValidityWindow, Error = InvalidWindow>,
    >(
        mut self,
        window: W,
    ) -> Result<Self, InvalidWindow> {
        self.block_validity_window = window.try_into()?;
        Ok(self)
    }

    /// Restrict the timestamp range in which the transaction is valid (infallible).
    ///
    /// Accepts any infallible range conversion: `1..`, `..100`, or `..` (unbounded).
    /// For the fallible variant (returns `Err` on empty/inverted ranges), see
    /// [`try_with_timestamp_validity_window`](Self::try_with_timestamp_validity_window).
    pub fn with_timestamp_validity_window<W: Into<TimestampValidityWindow>>(
        mut self,
        window: W,
    ) -> Self {
        self.timestamp_validity_window = window.into();
        self
    }

    /// Restrict the timestamp range in which the transaction is valid (fallible).
    ///
    /// Returns `Err(InvalidWindow)` if `window` is an empty range (e.g. `5..5` or `10..5`).
    /// For the infallible variant, see [`with_timestamp_validity_window`](Self::with_timestamp_validity_window).
    pub fn try_with_timestamp_validity_window<
        W: TryInto<TimestampValidityWindow, Error = InvalidWindow>,
    >(
        mut self,
        window: W,
    ) -> Result<Self, InvalidWindow> {
        self.timestamp_validity_window = window.try_into()?;
        Ok(self)
    }

    /// Destructure into individual components.
    ///
    /// Returns a [`SpelOutputParts`] struct instead of a raw tuple so that
    /// future field additions don't require updating every call site.
    pub fn into_parts(self) -> SpelOutputParts {
        SpelOutputParts {
            effects: self.effects,
            chained_calls: self.chained_calls,
            events: self.events,
            block_validity_window: self.block_validity_window,
            timestamp_validity_window: self.timestamp_validity_window,
        }
    }
}

/// Components of a [`SpelOutput`] after destructuring via [`SpelOutput::into_parts`].
///
/// This struct is `#[non_exhaustive]` so future field additions don't break
/// callers that pattern-match on it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SpelOutputParts {
    /// Per-shard effects, in the order the handler emitted them.
    pub effects: Vec<ShardEffect>,
    /// Chained calls to other programs.
    pub chained_calls: Vec<ChainedCall>,
    /// Events emitted alongside the plan.
    pub events: Vec<ProgramEvent>,
    /// Block range in which the transaction is valid.
    pub block_validity_window: BlockValidityWindow,
    /// Timestamp range in which the transaction is valid.
    pub timestamp_validity_window: TimestampValidityWindow,
}

/// Account constraint flags used by the proc-macro.
#[derive(Debug, Clone, Default)]
pub struct AccountConstraint {
    pub mutable: bool,
    pub init: bool,
    pub owner: Option<[u8; 32]>,
    pub signer: bool,
    pub seeds: Option<Vec<Vec<u8>>>,
}

/// Metadata about an instruction, used for IDL generation.
#[derive(Debug, Clone)]
pub struct InstructionMeta {
    pub name: String,
    pub accounts: Vec<AccountMeta>,
    pub args: Vec<ArgMeta>,
}

/// Metadata about an account parameter.
#[derive(Debug, Clone)]
pub struct AccountMeta {
    pub name: String,
    pub writable: bool,
    pub init: bool,
    pub owner: Option<String>,
    pub signer: bool,
    pub pda_seeds: Option<Vec<String>>,
}

/// Metadata about an instruction argument.
#[derive(Debug, Clone)]
pub struct ArgMeta {
    pub name: String,
    pub type_name: String,
}
