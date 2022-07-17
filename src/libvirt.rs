// SPDX-License-Identifier: Apache-2.0
//
// Houdini  A container escape artist
// Copyright (c) 2022  William Findlay
//
// February 25, 2022  William Findlay  Created this.

//! Helpers for spawning libvirt VMs for running Houdini.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

/// Source for the Linux kernel to use.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinuxKernelSource {
    Url(Url),
    BzImage(PathBuf),
}

/// Config for spawning a libvirt vm.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VmConfig {
    pub kernel_source: LinuxKernelSource,
}

#[cfg(test)]
mod tests {
    use crate::testutils::assert_serde_yaml;

    use super::*;

    #[test]
    fn test_serde_vm_config() {
        let yaml = r#"
            kernelSource:
                url: https://cdn.kernel.org/pub/linux/kernel/v5.x/linux-5.18.11.tar.xz
            "#;
        assert_serde_yaml::<VmConfig>(yaml);

        let yaml = r#"
            kernelSource:
                bzImage: /tmp/bzImage
            "#;
        assert_serde_yaml::<VmConfig>(yaml);
    }
}
