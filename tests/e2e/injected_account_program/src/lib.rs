//! `poke` declares no gate accounts, so the build fails if an injected param is not an account type.

#![allow(dead_code, unused_imports, unused_variables)]

use spel_framework::prelude::*;

use mini_gate::{mini_gate, require_gate};

#[lez_program]
#[mini_gate]
mod gated {
    #[allow(unused_imports)]
    use super::*;

    #[instruction]
    #[require_gate]
    pub fn poke(value: u64) -> SpelResult {
        Ok(SpelOutput::empty())
    }
}
