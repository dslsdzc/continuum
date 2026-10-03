use continuum_effect::{transition, EffectState, StateError};

#[test]
fn the_documented_transitions_are_accepted() {
    use EffectState::*;
    let legal = [
        (Planned, Authorized),
        (Planned, Failed),
        (Planned, RolledBack),
        (Authorized, Executing),
        (Authorized, Failed),
        (Authorized, RolledBack),
        (Executing, Committed),
        (Executing, Failed),
        (Executing, Unknown),
        (Unknown, Committed),
        (Unknown, Failed),
        (Unknown, RolledBack),
        (Unknown, Unknown),
    ];
    for (from, to) in legal {
        assert_eq!(transition(from, to).unwrap(), to, "{from:?} → {to:?} 应被接受");
    }
}

#[test]
fn a_transition_outside_the_table_is_rejected() {
    let err = transition(EffectState::Committed, EffectState::Executing).unwrap_err();
    assert!(matches!(err, StateError::Illegal { .. }), "实际 {err:?}");
}

#[test]
fn the_three_terminal_states_have_no_outgoing_edges() {
    for from in [EffectState::Committed, EffectState::Failed, EffectState::RolledBack] {
        for to in EffectState::ALL {
            assert!(
                transition(from, to).is_err(),
                "{from:?} 是终态，不应有出边"
            );
        }
    }
}
