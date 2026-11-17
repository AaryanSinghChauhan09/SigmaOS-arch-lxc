// Integration test for SigmaOS-arch-lxc
// Verifies the crate builds and links as a dependency.

use sigmaos_arch_lxc::{LxcConfig, LxcManager};

#[test]
fn crate_links_and_runs() {
    // Arrange
    let mut mgr = LxcManager::new();
    mgr.create(LxcConfig::new("test1")).unwrap();

    // Act
    let res = mgr.start("test1");

    // Assert
    assert!(res.is_ok());
    assert_eq!(mgr.list().len(), 1);
}
