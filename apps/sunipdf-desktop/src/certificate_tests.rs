use super::*;

#[test]
fn verification_consent_applies_once_and_is_reset_on_reopen() {
    let mut state = CertificateState::default();
    state.open(false);
    assert!(!state.allow_network);
    assert!(matches!(
        state.verify_action(),
        CertificateAction::Verify(VerificationOptions {
            allow_network: false
        })
    ));
    state.results(Vec::new());
    state.allow_network = true;
    assert!(matches!(
        state.verify_action(),
        CertificateAction::Verify(VerificationOptions {
            allow_network: true
        })
    ));
    assert!(state.busy && !state.allow_network);
    state.results(Vec::new());
    assert!(matches!(
        state.verify_action(),
        CertificateAction::Verify(VerificationOptions {
            allow_network: false
        })
    ));
    state.allow_network = true;
    state.open(false);
    assert!(!state.allow_network);
    state.allow_network = true;
    state.open(true);
    assert!(!state.allow_network);
}
