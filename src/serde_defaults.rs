// SPDX-License-Identifier: Apache-2.0
//
// Houdini  A container escape artist
// Copyright (c) 2022  William Findlay
//
// February 25, 2022  William Findlay  Created this.

//! Default helpers for serde types.

use crate::exploits::ExploitStatus;

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

pub fn default_setup_failure() -> ExploitStatus {
    ExploitStatus::SetupFailure
}

pub fn default_skip() -> ExploitStatus {
    ExploitStatus::Skip
}
