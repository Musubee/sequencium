use bevy::{prelude::*, sprite::collide_aabb::collide, utils::HashMap, window::PrimaryWindow};

use itertools::iproduct;
use std::cmp::max;
use std::str::FromStr;

const GRID_LENGTH: usize = 6;

const BLOCK_SIZE: f32 = 50.0;
const SPACE_BETWEEN_BLOCKS: f32 = 5.0;

const NUM_PLAYERS: u32 = 2;

const PLAYER_1_COMMITTED_COLOR: Color = Color::RED;
const PLAYER_2_COMMITTED_COLOR: Color = Color::BLUE;
const PLAYER_1_SELECTED_COLOR: Color = Color::SALMON;
const PLAYER_2_SELECTED_COLOR: Color = Color::TURQUOISE;

#[derive(Component)]
struct MainCamera;

#[derive(Component, Copy, Clone)]
struct GridValue(Option<u32>);

#[derive(Component)]
struct Selected;

#[derive(Bundle)]
struct GridBlockBundle {
    value: GridValue,
    spatial: SpatialBundle,
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum TurnState {
    #[default]
    Unselected,
    Committed,
    TurnEnd,
}

#[derive(Component)]
struct Grid {
    adj_list: HashMap<Entity, Vec<Entity>>,
}

#[derive(Resource)]
struct FontSpec {
    family: Handle<Font>,
}

impl FromWorld for FontSpec {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        FontSpec {
            family: asset_server.load("fonts/FiraMono-Medium.ttf"),
        }
    }
}

#[derive(Component, Copy, Clone)]
struct OwnedBy(Entity);

#[derive(Component, Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd)]
struct Score(u32);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct CurrentPlayer;

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    score: Score,
}

#[derive(Resource, Default)]
struct PlayerList {
    list: Vec<Entity>,
}

fn setup_board(mut commands: Commands, font_spec: Res<FontSpec>, player_list: Res<PlayerList>) {
    commands.spawn((Camera2dBundle::default(), MainCamera));

    let (grid_left, grid_top) = get_grid_top_left();

    let mut blocks: Vec<Vec<Entity>> = Vec::with_capacity(GRID_LENGTH);
    for i in 0..GRID_LENGTH {
        let mut block_row: Vec<Entity> = Vec::with_capacity(GRID_LENGTH);
        let block_y = grid_top - i as f32 * (BLOCK_SIZE + SPACE_BETWEEN_BLOCKS);
        for j in 0..GRID_LENGTH {
            let block_x = grid_left + j as f32 * (BLOCK_SIZE + SPACE_BETWEEN_BLOCKS);
            let (grid_value, owner) = get_init_block_data(i, j, &player_list.list);
            let grid_value_text = match grid_value {
                GridValue(Some(value)) => value.to_string(),
                GridValue(None) => "".to_string(),
            };
            let sprite_color = match owner {
                _ if owner == player_list.list[0] => PLAYER_1_COMMITTED_COLOR,
                _ if owner == player_list.list[1] => PLAYER_2_COMMITTED_COLOR,
                _ => Color::WHITE,
            };

            let block_entity = commands
                .spawn(GridBlockBundle {
                    value: grid_value,
                    spatial: SpatialBundle {
                        transform: Transform {
                            translation: Vec3::new(block_x, block_y, 0.0),
                            scale: Vec3::new(1.0, 1.0, 1.0),
                            ..default()
                        },
                        ..default()
                    },
                })
                .with_children(|parent| {
                    parent.spawn(SpriteBundle {
                        transform: Transform {
                            scale: Vec3::new(BLOCK_SIZE, BLOCK_SIZE, 1.0),
                            ..default()
                        },
                        sprite: Sprite {
                            color: sprite_color,
                            ..default()
                        },
                        ..default()
                    });
                    parent.spawn(Text2dBundle {
                        text: Text::from_section(
                            grid_value_text,
                            TextStyle {
                                font: font_spec.family.clone(),
                                font_size: 40.0,
                                color: Color::BLACK,
                            },
                        )
                        .with_alignment(TextAlignment::Center),
                        transform: Transform {
                            translation: Vec3::new(0.0, 0.0, 1.0),
                            ..default()
                        },
                        ..default()
                    });
                })
                .id();
            if owner != Entity::PLACEHOLDER {
                commands.entity(block_entity).insert(OwnedBy(owner));
            }
            block_row.push(block_entity);
        }
        blocks.push(block_row);
    }

    let mut adj_list: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for x in 0..GRID_LENGTH {
        for y in 0..GRID_LENGTH {
            let entity: Entity = blocks[x][y];
            // Generate neighbors of entity via cartesian product
            let neighbor_coords = iproduct!(-1..=1, -1..=1)
                .map(|(dx, dy)| {
                    (
                        usize::try_from(x as isize + dx),
                        usize::try_from(y as isize + dy),
                    )
                })
                .filter_map(|(neighbor_x, neighbor_y)| {
                    if let Ok(n_x) = neighbor_x {
                        if let Ok(n_y) = neighbor_y {
                            match n_x < GRID_LENGTH && n_y < GRID_LENGTH && (n_x != x || n_y != y) {
                                true => return Some((n_x, n_y)),
                                false => return None,
                            };
                        }
                    }
                    None
                });
            for coord in neighbor_coords {
                let (neighbor_x, neighbor_y) = coord;
                let neighbor_entity: Entity = blocks[neighbor_x][neighbor_y];
                match adj_list.get_mut(&entity) {
                    Some(neighbor_list) => neighbor_list.push(neighbor_entity),
                    None => {
                        adj_list.insert(entity, vec![neighbor_entity]);
                    }
                }
            }
        }
    }

    dbg!(&blocks);
    dbg!(&adj_list);
    commands.spawn(Grid { adj_list });
}

fn get_grid_top_left() -> (f32, f32) {
    let grid_width =
        GRID_LENGTH as f32 * (BLOCK_SIZE + SPACE_BETWEEN_BLOCKS) - SPACE_BETWEEN_BLOCKS;
    let grid_height =
        GRID_LENGTH as f32 * (BLOCK_SIZE + SPACE_BETWEEN_BLOCKS) - SPACE_BETWEEN_BLOCKS;
    let grid_left = -grid_width / 2.0;
    let grid_top = grid_height / 2.0;
    (grid_left, grid_top)
}

// The game starts with players owning blocks along the diagonal from
// the top left to bottom right of the board. Given indices, this function
// returns the value of the associated block and player who owns it.
//
// If the block is not along the diagonal, None is returned.
fn get_init_block_data(i: usize, j: usize, player_list: &[Entity]) -> (GridValue, Entity) {
    // Currently implemented assuming 2 players
    // If this changes, assertion will fail and function needs to be updated
    assert!(player_list.len() == 2);
    if i == j {
        let half_grid_length = GRID_LENGTH / 2;
        match i {
            i if i < half_grid_length => (GridValue(Some(i as u32 + 1)), player_list[0]),
            _ => (GridValue(Some((GRID_LENGTH - i) as u32)), player_list[1]),
        }
    } else {
        (GridValue(None), Entity::PLACEHOLDER)
    }
}
// Spawn players and populate player list
fn setup_players(mut commands: Commands, mut player_list: ResMut<PlayerList>) {
    for _ in 0..NUM_PLAYERS {
        let player_entity = commands
            .spawn(PlayerBundle {
                player: Player,
                score: Score(GRID_LENGTH as u32 / 2),
            })
            .id();
        player_list.list.push(player_entity);
    }
    dbg!("Player list: {:?}", &player_list.list);
    commands.entity(player_list.list[1]).insert(CurrentPlayer);
}

fn get_cursor_pos(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<Vec2> {
    let window = window_query.single();
    let (camera, camera_transform) = camera_query.single();
    window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
}

// System for when a player has not yet selected a potential tile to fill
// Checks for mouse collision with eligible tiles and updates sprite/text appropriately
// If a mouse click occurs over an eligible tile, transitions to TurnState::Selected
fn unselected(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    owned_block_query: Query<(Entity, &GridValue, &OwnedBy)>,
    unowned_block_query: Query<(Entity, &Transform, &GridValue), Without<OwnedBy>>,
    mut sprite_query: Query<(&Parent, &mut Sprite)>,
    mut text_query: Query<(&Parent, &mut Text)>,
    grid_query: Query<&Grid>,
    current_player_query: Query<Entity, With<CurrentPlayer>>,
    player_list: Res<PlayerList>,
    font_spec: Res<FontSpec>,
    mouse_input: Res<Input<MouseButton>>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    assert!(player_list.list.len() == 2);

    // Look for mouse collision with tile
    let mut collided_entity: Option<Entity> = None;
    if let Some(cursor_pos) = get_cursor_pos(window_query, camera_query) {
        for (entity, transform, _) in unowned_block_query.iter() {
            if let Some(_) = collide(
                cursor_pos.extend(0.0),
                Vec2::new(0.0, 0.0),
                transform.translation,
                Vec2::new(BLOCK_SIZE, BLOCK_SIZE),
            ) {
                collided_entity = Some(entity);
            } else {
                // Reset sprite color and text
                for (parent, mut sprite) in sprite_query.iter_mut() {
                    if let Ok((parent_entity, _, _)) = unowned_block_query.get(parent.get()) {
                        if parent_entity == entity {
                            sprite.color = Color::WHITE;
                        }
                    }
                }

                for (parent, mut text) in text_query.iter_mut() {
                    if let Ok((parent_entity, _, _)) = unowned_block_query.get(parent.get()) {
                        if parent_entity == entity {
                            *text = Text::from_section(
                                "".to_string(),
                                TextStyle {
                                    font: font_spec.family.clone(),
                                    font_size: 40.0,
                                    color: Color::GRAY,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    // Update sprite and color if collided tile is eligible
    if let Some(entity) = collided_entity {
        // Check that collided tile is eligible for player to select
        let adj_list = &grid_query.single().adj_list;
        let current_player = current_player_query.single();
        if let Some(largest_neighbor_value) =
            largest_neighbor(entity, owned_block_query, &adj_list, current_player)
        {
            // Color sprite
            for (parent, mut sprite) in sprite_query.iter_mut() {
                if let Ok((parent_entity, _, _)) = unowned_block_query.get(parent.get()) {
                    if parent_entity == entity {
                        sprite.color = match current_player {
                            _ if current_player == player_list.list[0] => PLAYER_1_SELECTED_COLOR,
                            _ => PLAYER_2_SELECTED_COLOR,
                        };
                    }
                }
            }

            // Update text
            for (parent, mut text) in text_query.iter_mut() {
                if let Ok((parent_entity, _, _)) = unowned_block_query.get(parent.get()) {
                    if parent_entity == entity {
                        *text = Text::from_section(
                            (largest_neighbor_value + 1).to_string(),
                            TextStyle {
                                font: font_spec.family.clone(),
                                font_size: 40.0,
                                color: Color::GRAY,
                            },
                        )
                        .with_alignment(TextAlignment::Center);
                    }
                }
            }

            // Transition if a value is committed via mouse click
            if mouse_input.just_pressed(MouseButton::Left) {
                commands.entity(entity).insert(Selected);
                next_state.set(TurnState::Committed);
            }
        }
    }
}

fn largest_neighbor(
    collided_entity: Entity,
    owned_block_query: Query<(Entity, &GridValue, &OwnedBy)>,
    adj_list: &HashMap<Entity, Vec<Entity>>,
    current_player: Entity,
) -> Option<u32> {
    let mut largest_neighbor_value: Option<u32> = None;
    // All blocks should be in the adjacency list, panic if this isn't the case
    let neighbor_list = adj_list.get(&collided_entity).unwrap();
    for (entity, &neighbor_grid_value, &OwnedBy(neighbor_owner)) in owned_block_query.iter() {
        if neighbor_list.contains(&entity) && neighbor_owner == current_player {
            if let GridValue(Some(v)) = neighbor_grid_value {
                if let Some(largest_v) = largest_neighbor_value {
                    largest_neighbor_value = Some(max(v, largest_v));
                } else {
                    largest_neighbor_value = Some(v);
                }
            }
        }
    }
    largest_neighbor_value
}

// Handles changing GridBlock values and player scores
fn selection_committed(
    mut commands: Commands,
    mut selected_block: Query<(Entity, &mut GridValue), With<Selected>>,
    mut sprite_query: Query<(&Parent, &mut Sprite)>,
    mut text_query: Query<(&Parent, &mut Text)>,
    mut player_score_query: Query<(Entity, &mut Score)>,
    current_player_query: Query<Entity, With<CurrentPlayer>>,
    player_list: Res<PlayerList>,
    font_spec: Res<FontSpec>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let (grid_block_entity, mut grid_value) = selected_block.single_mut();
    for (parent, text) in text_query.iter_mut() {
        if parent.get() == grid_block_entity {
            grid_value.0 = Some(u32::from_str(&text.sections[0].value).unwrap());
        }
    }

    let current_player = current_player_query.single();
    for (parent, mut sprite) in sprite_query.iter_mut() {
        if parent.get() == grid_block_entity {
            sprite.color = match current_player {
                _ if current_player == player_list.list[0] => PLAYER_1_COMMITTED_COLOR,
                _ => PLAYER_2_COMMITTED_COLOR,
            };
        }
    }

    for (parent, mut text) in text_query.iter_mut() {
        if parent.get() == grid_block_entity {
            *text = Text::from_section(
                grid_value.0.unwrap().to_string(),
                TextStyle {
                    font: font_spec.family.clone(),
                    font_size: 40.0,
                    color: Color::BLACK,
                },
            )
            .with_alignment(TextAlignment::Center);
        }
    }

    commands.entity(grid_block_entity).remove::<Selected>();
    commands
        .entity(grid_block_entity)
        .insert(OwnedBy(current_player));

    for (entity, mut score) in player_score_query.iter_mut() {
        if entity == current_player {
            score.0 = max(score.0, grid_value.0.unwrap());
        }
    }

    next_state.set(TurnState::TurnEnd);
}

// Handles switching whose turn it is
fn turn_end(
    mut commands: Commands,
    current_player_query: Query<Entity, With<CurrentPlayer>>,
    owned_block_query: Query<(Entity, &GridValue, &OwnedBy)>,
    unowned_block_query: Query<(Entity, &GridValue), Without<OwnedBy>>,
    grid_query: Query<&Grid>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    player_score_query: Query<(Entity, &mut Score)>,
    player_list: Res<PlayerList>,
    font_spec: Res<FontSpec>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    assert!(player_list.list.len() == 2);

    let next_player = match current_player_query.single() {
        _ if current_player_query.single() == player_list.list[0] => player_list.list[1],
        _ => player_list.list[0],
    };
    let adj_list = &grid_query.single().adj_list;

    if unowned_block_query.iter().count() == 0 {
        let (winning_player, &winning_score) = player_score_query
            .iter()
            .max_by_key(|(_, &score)| score)
            .unwrap();

        let winning_player_number: u32;
        if player_list.list[0] == winning_player {
            winning_player_number = 1;
        } else {
            winning_player_number = 2;
        }

        let window = window_query.single();
        let text_y = window.height() / 2.0 - 30.0;

        commands.spawn(Text2dBundle {
            text: Text::from_section(
                format!(
                    "Player {} wins with a score of {}!",
                    winning_player_number, winning_score.0
                ),
                TextStyle {
                    font: font_spec.family.clone(),
                    font_size: 40.0,
                    color: Color::BLACK,
                },
            ),
            transform: Transform::from_translation(Vec3::new(0.0, text_y, 3.0)),
            ..Default::default()
        });
    } else if next_player_has_moves(
        next_player,
        &adj_list,
        &owned_block_query,
        &unowned_block_query,
    ) {
        let current_player = current_player_query.single();
        for (i, &player) in player_list.list.iter().enumerate() {
            if player == current_player {
                commands.entity(player).remove::<CurrentPlayer>();
                commands
                    .entity(player_list.list[(i + 1) % player_list.list.len()])
                    .insert(CurrentPlayer);
            }
        }
        next_state.set(TurnState::Unselected);
    } else {
        next_state.set(TurnState::Unselected);
    }
}

fn next_player_has_moves(
    next_player: Entity,
    adj_list: &HashMap<Entity, Vec<Entity>>,
    owned_block_query: &Query<(Entity, &GridValue, &OwnedBy)>,
    unowned_block_query: &Query<(Entity, &GridValue), Without<OwnedBy>>,
) -> bool {
    for (entity, _, owned_by) in owned_block_query.iter() {
        if owned_by.0 == next_player {
            for &neighbor in adj_list.get(&entity).unwrap() {
                if unowned_block_query.get(neighbor).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<FontSpec>()
        .init_resource::<PlayerList>()
        .add_state::<TurnState>()
        .add_startup_system(setup_board)
        .add_startup_system(setup_players.before(setup_board))
        .add_system(unselected.in_set(OnUpdate(TurnState::Unselected)))
        .add_system(selection_committed.in_set(OnUpdate(TurnState::Committed)))
        .add_system(turn_end.in_set(OnUpdate(TurnState::TurnEnd)))
        .run();
}
