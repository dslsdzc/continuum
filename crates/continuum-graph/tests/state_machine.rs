use continuum_graph::{is_terminal, transition, NodeState, StateError};

#[test]
fn thirteen_states_are_declared() {
    assert_eq!(NodeState::ALL.len(), 13);
}

#[test]
fn legal_transitions_are_accepted() {
    // 设计第 9 节的表
    let legal = [
        (NodeState::Pending, NodeState::Ready),
        (NodeState::Ready, NodeState::Queued),
        (NodeState::Queued, NodeState::Running),
        (NodeState::Running, NodeState::Verifying),
        (NodeState::Verifying, NodeState::Completed),
        (NodeState::Running, NodeState::Waiting),
        (NodeState::Waiting, NodeState::Ready),
        (NodeState::Running, NodeState::Blocked),
        (NodeState::Blocked, NodeState::Ready),
        (NodeState::Suspended, NodeState::Ready),
        (NodeState::Running, NodeState::Failed),
        (NodeState::Verifying, NodeState::Failed),
        (NodeState::Running, NodeState::Lost),
        (NodeState::Verifying, NodeState::Lost),
    ];
    for (from, to) in legal {
        assert_eq!(
            transition(from, to).expect("合法迁移应通过"),
            to,
            "迁移 {from:?} → {to:?} 应合法"
        );
    }
}

#[test]
fn invalidation_is_reachable_from_every_state() {
    // §306 要求已完成节点同样可被失效
    for from in NodeState::ALL {
        transition(from, NodeState::Invalidated)
            .unwrap_or_else(|e| panic!("{from:?} → INVALIDATED 应合法，实际 {e}"));
    }
}

#[test]
fn cancellation_is_reachable_from_every_state() {
    for from in NodeState::ALL {
        transition(from, NodeState::Cancelled)
            .unwrap_or_else(|e| panic!("{from:?} → CANCELLED 应合法，实际 {e}"));
    }
}

#[test]
fn running_and_blocked_can_be_suspended() {
    transition(NodeState::Running, NodeState::Suspended).expect("RUNNING → SUSPENDED 应合法");
    transition(NodeState::Waiting, NodeState::Suspended).expect("WAITING → SUSPENDED 应合法");
    transition(NodeState::Blocked, NodeState::Suspended).expect("BLOCKED → SUSPENDED 应合法");
}

#[test]
fn illegal_transitions_are_rejected() {
    let illegal = [
        (NodeState::Pending, NodeState::Running),
        (NodeState::Ready, NodeState::Completed),
        (NodeState::Queued, NodeState::Completed),
        (NodeState::Completed, NodeState::Running),
        (NodeState::Failed, NodeState::Ready),
        (NodeState::Lost, NodeState::Running),
        (NodeState::Verifying, NodeState::Ready),
        (NodeState::Suspended, NodeState::Running),
        (NodeState::Blocked, NodeState::Queued),
    ];
    for (from, to) in illegal {
        match transition(from, to) {
            Err(StateError::Illegal { from: f, to: t }) => {
                assert_eq!((f, t), (from, to), "错误信息应回报实际的两个状态");
            }
            Ok(_) => panic!("迁移 {from:?} → {to:?} 应被拒绝"),
        }
    }
}

#[test]
fn terminal_states_are_identified() {
    for s in [
        NodeState::Completed,
        NodeState::Failed,
        NodeState::Cancelled,
        NodeState::Invalidated,
        NodeState::Lost,
    ] {
        assert!(is_terminal(s), "{s:?} 应为终态");
    }
    for s in [
        NodeState::Pending,
        NodeState::Ready,
        NodeState::Queued,
        NodeState::Running,
        NodeState::Waiting,
        NodeState::Blocked,
        NodeState::Suspended,
        NodeState::Verifying,
    ] {
        assert!(!is_terminal(s), "{s:?} 不应为终态");
    }
}
