use jameskills_core::domain::agent::{AgentCapabilityId, AgentId, CapabilitySupport};
use jameskills_infra::agents::AgentRegistry;

#[test]
fn built_in_registry_has_five_closed_app_owned_profiles() {
    let registry = AgentRegistry::built_in().unwrap();

    assert_eq!(registry.profiles().count(), AgentId::ALL.len());
    for id in AgentId::ALL {
        let profile = registry
            .get(id)
            .expect("all app-owned profile IDs are registered");
        assert_eq!(profile.id(), id);
        assert!(!profile.cli_name().is_empty());
        for capability_id in AgentCapabilityId::ALL {
            let assessment = profile.capabilities().assessment(capability_id);
            let evidence = assessment
                .evidence()
                .expect("every profile status cites source/date evidence");
            assert!(!evidence.source_id().is_empty());
            assert!(!evidence.observed_at().is_empty());
            assert_ne!(assessment.support(), CapabilitySupport::Supported);
        }
    }
}

#[test]
fn unimplemented_adapters_do_not_claim_verified_features() {
    let registry = AgentRegistry::built_in().unwrap();

    for profile in registry.profiles() {
        assert_eq!(
            profile.capabilities().get(AgentCapabilityId::UserInstall),
            CapabilitySupport::NeedsVerification
        );
        assert_eq!(
            profile
                .capabilities()
                .get(AgentCapabilityId::DiscoveryVerification),
            CapabilitySupport::NeedsVerification
        );
    }
}

#[test]
fn antigravity_registry_is_vendor_plugin_user_scope_only() {
    let registry = AgentRegistry::built_in().unwrap();
    let profile = registry.get(AgentId::Antigravity).unwrap();

    assert_eq!(profile.cli_name(), "agy");
    assert_eq!(
        profile.supports_scope(jameskills_core::domain::Scope::User),
        CapabilitySupport::NeedsVerification
    );
    assert_eq!(
        profile.supports_scope(jameskills_core::domain::Scope::Project),
        CapabilitySupport::Unsupported
    );
    assert_eq!(
        profile.install_mode(),
        jameskills_core::domain::agent::AgentInstallMode::VendorPlugin
    );
}

#[test]
fn registry_rejects_missing_or_duplicate_profiles() {
    let built_in = AgentRegistry::built_in().unwrap();
    let mut profiles = built_in.profiles().cloned().collect::<Vec<_>>();

    profiles.pop();
    assert!(AgentRegistry::new(profiles).is_err());

    let mut profiles = built_in.profiles().cloned().collect::<Vec<_>>();
    profiles.push(profiles[0].clone());
    assert!(AgentRegistry::new(profiles).is_err());
}
