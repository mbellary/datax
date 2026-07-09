use super::turn_context::InteractionContext;
use crate::context::world_state::EnvironmentsState;
use crate::context::world_state::WorldState;
use crate::environment_selection::TurnEnvironmentSnapshot;
use datax_protocol::protocol::InteractionContextMessage;

pub(super) fn build_world_state_from_turn_context(
    turn_context: &InteractionContext,
    environment_subagents: &str,
) -> WorldState {
    let mut world_state = WorldState::default();
    if turn_context.config.include_environment_context {
        world_state.add_section(
            EnvironmentsState::from_turn_context(turn_context)
                .with_subagents(environment_subagents.to_string()),
        );
    }
    world_state
}

pub(super) fn build_world_state_from_environment_snapshot(
    turn_context: &InteractionContext,
    environments: &TurnEnvironmentSnapshot,
) -> WorldState {
    let mut world_state = WorldState::default();
    if turn_context.config.include_environment_context {
        world_state.add_section(EnvironmentsState::from_turn_context_with_environments(
            turn_context,
            environments,
        ));
    }
    world_state
}

pub(super) fn build_world_state_from_turn_context_item(
    turn_context_item: &InteractionContextMessage,
) -> WorldState {
    let mut world_state = WorldState::default();
    world_state.add_section(EnvironmentsState::from_turn_context_item(turn_context_item));
    world_state
}
