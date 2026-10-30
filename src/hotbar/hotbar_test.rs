use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

use crate::block_edit::SelectedBlock;
use crate::hotbar::{
    hotbar_input_system, sync_hotbar_selection_system,
    Hotbar,
};
use crate::registry::BlockType;

#[test]
fn hotbar_selection_wrapping_and_cycling() {
    let mut hotbar = Hotbar::default();
    assert_eq!(hotbar.selected_index, 0);
    assert_eq!(hotbar.selected_block(), Some(BlockType::Grass));

    hotbar.cycle_next();
    assert_eq!(hotbar.selected_index, 1);
    assert_eq!(hotbar.selected_block(), Some(BlockType::Dirt));

    hotbar.cycle_next();
    assert_eq!(hotbar.selected_index, 0);

    hotbar.cycle_prev();
    assert_eq!(hotbar.selected_index, 1);

    hotbar.select_slot(0);
    assert_eq!(hotbar.selected_index, 0);
}

#[test]
fn hotbar_sync_updates_selected_block_resource() {
    let mut app = App::new();
    app.init_resource::<Hotbar>();
    app.init_resource::<SelectedBlock>();
    app.add_systems(Update, sync_hotbar_selection_system);

    app.update();
    assert_eq!(app.world().resource::<SelectedBlock>().0, BlockType::Grass);

    app.world_mut().resource_mut::<Hotbar>().cycle_next();
    app.update();
    assert_eq!(app.world().resource::<SelectedBlock>().0, BlockType::Dirt);
}

#[test]
fn hotbar_input_key_and_wheel() {
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<AccumulatedMouseScroll>();
    app.init_resource::<Hotbar>();
    app.add_systems(Update, hotbar_input_system);

    // Press '2'
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Digit2);
    app.update();
    assert_eq!(app.world().resource::<Hotbar>().selected_index, 1);

    // Mouse wheel down (y < 0) -> cycle next (wraps back to 0)
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear_just_pressed(KeyCode::Digit2);
    app.world_mut().resource_mut::<AccumulatedMouseScroll>().delta.y = -1.0;
    app.update();
    assert_eq!(app.world().resource::<Hotbar>().selected_index, 0);
}
