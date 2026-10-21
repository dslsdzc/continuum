use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, ResourcePressure,
    SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

struct Cell {
    declaration: WorkloadDeclaration,
    cpu: u8,
    pressure: ResourcePressure,
    action: Option<ResourceAction>,
    phase: SuspensionPhase,
}

const USER: WorkloadDeclaration = WorkloadDeclaration {
    user_process: true,
    foreground: false,
    maintenance: false,
};
const FOREGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: true,
    maintenance: false,
};
const BACKGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: false,
    maintenance: false,
};
const MAINTENANCE: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: false,
    maintenance: true,
};

const CELLS: [Cell; 16] = [
    Cell {
        declaration: USER,
        cpu: 10,
        pressure: ResourcePressure::Low,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: USER,
        cpu: 60,
        pressure: ResourcePressure::Moderate,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: USER,
        cpu: 80,
        pressure: ResourcePressure::High,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: USER,
        cpu: 95,
        pressure: ResourcePressure::Critical,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: FOREGROUND,
        cpu: 10,
        pressure: ResourcePressure::Low,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: FOREGROUND,
        cpu: 60,
        pressure: ResourcePressure::Moderate,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: FOREGROUND,
        cpu: 80,
        pressure: ResourcePressure::High,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: FOREGROUND,
        cpu: 95,
        pressure: ResourcePressure::Critical,
        action: Some(ResourceAction::Throttle),
        phase: SuspensionPhase::Throttled,
    },
    Cell {
        declaration: BACKGROUND,
        cpu: 10,
        pressure: ResourcePressure::Low,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: BACKGROUND,
        cpu: 60,
        pressure: ResourcePressure::Moderate,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: BACKGROUND,
        cpu: 80,
        pressure: ResourcePressure::High,
        action: Some(ResourceAction::Throttle),
        phase: SuspensionPhase::Throttled,
    },
    Cell {
        declaration: BACKGROUND,
        cpu: 95,
        pressure: ResourcePressure::Critical,
        action: Some(ResourceAction::Suspend),
        phase: SuspensionPhase::Checkpointing,
    },
    Cell {
        declaration: MAINTENANCE,
        cpu: 10,
        pressure: ResourcePressure::Low,
        action: None,
        phase: SuspensionPhase::Running,
    },
    Cell {
        declaration: MAINTENANCE,
        cpu: 60,
        pressure: ResourcePressure::Moderate,
        action: Some(ResourceAction::Throttle),
        phase: SuspensionPhase::Throttled,
    },
    Cell {
        declaration: MAINTENANCE,
        cpu: 80,
        pressure: ResourcePressure::High,
        action: Some(ResourceAction::Suspend),
        phase: SuspensionPhase::Checkpointing,
    },
    Cell {
        declaration: MAINTENANCE,
        cpu: 95,
        pressure: ResourcePressure::Critical,
        action: Some(ResourceAction::Suspend),
        phase: SuspensionPhase::Checkpointing,
    },
];

#[test]
fn every_cell_of_the_four_by_four_ladder_is_explicit() {
    for cell in CELLS {
        let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
        let mut state = SuspensionState::new();
        let observation = ResourceObservation {
            cpu: Some(cell.cpu),
            ..Default::default()
        };
        assert_eq!(
            arbiter.observe(&observation, 0).resource_pressure(),
            cell.pressure
        );
        let decision = arbiter.direct(cell.declaration, &mut state, 0).unwrap();
        assert_eq!(decision.action(), cell.action, "cpu={}", cell.cpu);
        assert_eq!(decision.phase(), cell.phase, "cpu={}", cell.cpu);
        assert_eq!(state.phase(), cell.phase, "cpu={}", cell.cpu);
    }
}
