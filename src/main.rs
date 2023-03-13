use bevy::{
    prelude::*,
    sprite::collide_aabb::collide,
    window::PrimaryWindow,
};

const GRID_LENGTH: usize = 6;
const GRID_LEFT: f32 = 0.0;
const GRID_TOP: f32 = 0.0;

const BLOCK_SIZE: f32 = 50.0;
const SPACE_BETWEEN_BLOCKS: f32 = 5.0;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct GridValue(Option<u32>);

#[derive(Bundle)]
struct GridBlockBundle {
    value: GridValue,
    sprite: SpriteBundle,
}

impl GridBlockBundle {
    pub fn from_pos(pos: Vec2) -> GridBlockBundle {
        GridBlockBundle {
            value: GridValue(None),
            sprite: SpriteBundle {
                transform: Transform {
                    translation: pos.extend(0.0),
                    scale: Vec3::new(BLOCK_SIZE, BLOCK_SIZE, 1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: Color::WHITE,
                    ..default()
                },
                ..default()
            },
        }
    }
}

#[derive(Component)]
struct Grid {
    blocks: Vec<Vec<Entity>>
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainCamera));

    let mut blocks = Vec::with_capacity(GRID_LENGTH);
    for i in 0..GRID_LENGTH {
        let mut block_row = Vec::with_capacity(GRID_LENGTH);
        let block_y = GRID_TOP - i as f32 * (BLOCK_SIZE + SPACE_BETWEEN_BLOCKS);
        for j in 0..GRID_LENGTH {
            let block_x = GRID_LEFT + j as f32 * (BLOCK_SIZE + SPACE_BETWEEN_BLOCKS);
            let block_entity = commands.spawn(GridBlockBundle::from_pos(Vec2::new(block_x, block_y))).id();
            block_row.push(block_entity);
        }
        blocks.push(block_row);
    }

    commands.spawn(Grid {blocks});
}

fn check_mouse_grid_collision(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut block_query: Query<(&Transform, &mut Sprite), With<GridValue>>,
    grid_query: Query<&Grid>,
) {
    let window = window_query.single();
    let (camera, camera_transform) = camera_query.single();
    if let Some(cursor_pos) = window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        let grid = grid_query.single();
        for (transform, mut sprite) in &mut block_query {
            let collision = collide(
                cursor_pos.extend(0.0), Vec2::new(1.0, 1.0),
                transform.translation, Vec2::new(BLOCK_SIZE, BLOCK_SIZE)
            );
            sprite.color = match collision {
                Some(_) => {
                    Color::ORANGE
                },
                None => {
                    Color::WHITE
                }
            }
        }
    }
}
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(check_mouse_grid_collision)
        .run();
}
