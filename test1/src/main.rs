use bevy::prelude::*;

const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 600.0;
const ARENA: Vec2 = Vec2::new(410.0, 220.0);
const ROUND_TIME: f32 = 45.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.045, 0.065, 0.11)))
        .insert_resource(Game::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gem Dash".into(),
                resolution: (WIDTH as u32, HEIGHT as u32).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                start_round,
                move_player,
                move_hazards,
                collect_gems,
                check_collision,
                update_hud,
            )
                .chain(),
        )
        .run();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Ready,
    Playing,
    Over,
}

#[derive(Resource)]
struct Game {
    phase: Phase,
    score: u32,
    time_left: f32,
    rng: u32,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            phase: Phase::Ready,
            score: 0,
            time_left: ROUND_TIME,
            rng: 0x8a4f_21c3,
        }
    }
}

impl Game {
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    fn gem_position(&mut self) -> Vec3 {
        Vec3::new(
            (self.random() * 2.0 - 1.0) * (ARENA.x - 35.0),
            (self.random() * 2.0 - 1.0) * (ARENA.y - 35.0),
            2.0,
        )
    }
}

#[derive(Component)]
struct Player;
#[derive(Component)]
struct Gem;
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct Message;
#[derive(Component)]
struct Hazard {
    velocity: Vec2,
    start: Vec2,
}

fn setup(mut commands: Commands, mut game: ResMut<Game>) {
    commands.spawn(Camera2d);
    let grid = Color::srgb(0.09, 0.13, 0.20);
    for x in (-400..=400).step_by(50) {
        commands.spawn((
            Sprite::from_color(grid, Vec2::new(1.0, 440.0)),
            Transform::from_xyz(x as f32, 0.0, -1.0),
        ));
    }
    for y in (-200..=200).step_by(50) {
        commands.spawn((
            Sprite::from_color(grid, Vec2::new(820.0, 1.0)),
            Transform::from_xyz(0.0, y as f32, -1.0),
        ));
    }
    for (size, position) in [
        (Vec2::new(824.0, 4.0), Vec3::new(0.0, 222.0, 0.0)),
        (Vec2::new(824.0, 4.0), Vec3::new(0.0, -222.0, 0.0)),
        (Vec2::new(4.0, 444.0), Vec3::new(-412.0, 0.0, 0.0)),
        (Vec2::new(4.0, 444.0), Vec3::new(412.0, 0.0, 0.0)),
    ] {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.23, 0.34, 0.48), size),
            Transform::from_translation(position),
        ));
    }
    commands.spawn((
        Player,
        Sprite::from_color(Color::srgb(0.25, 0.85, 0.95), Vec2::splat(30.0)),
        Transform::from_xyz(0.0, 0.0, 3.0),
    ));
    for _ in 0..8 {
        commands.spawn((
            Gem,
            Sprite::from_color(Color::srgb(1.0, 0.78, 0.22), Vec2::splat(18.0)),
            Transform::from_translation(game.gem_position())
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        ));
    }
    for (position, velocity) in [
        (Vec2::new(-240.0, 100.0), Vec2::new(155.0, -125.0)),
        (Vec2::new(210.0, -130.0), Vec2::new(-130.0, 170.0)),
        (Vec2::new(120.0, 150.0), Vec2::new(180.0, 100.0)),
    ] {
        commands.spawn((
            Hazard {
                velocity,
                start: position,
            },
            Sprite::from_color(Color::srgb(0.96, 0.28, 0.38), Vec2::splat(28.0)),
            Transform::from_xyz(position.x, position.y, 2.0),
        ));
    }
    commands.spawn((
        Hud,
        Text2d::new("GEMS  0                                      TIME  45"),
        TextFont::from_font_size(25.0),
        TextColor(Color::srgb(0.9, 0.94, 1.0)),
        Transform::from_xyz(0.0, 265.0, 5.0),
    ));
    commands.spawn((
        Message,
        Text2d::new("GEM DASH\nMove: WASD / arrows   •   Avoid red blocks\nPress SPACE to start"),
        TextFont::from_font_size(25.0),
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(Justify::Center),
        Transform::from_xyz(0.0, 0.0, 10.0),
    ));
}

fn start_round(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    mut player: Query<&mut Transform, With<Player>>,
    mut hazards: Query<(&mut Transform, &Hazard), Without<Player>>,
    mut message: Query<&mut Text2d, With<Message>>,
) {
    if !keys.just_pressed(KeyCode::Space) || game.phase == Phase::Playing {
        return;
    }
    game.phase = Phase::Playing;
    game.score = 0;
    game.time_left = ROUND_TIME;
    player.single_mut().unwrap().translation = Vec3::new(0.0, 0.0, 3.0);
    for (mut transform, hazard) in &mut hazards {
        transform.translation = hazard.start.extend(2.0);
    }
    message.single_mut().unwrap().0.clear();
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    game: Res<Game>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    if game.phase != Phase::Playing {
        return;
    }
    let x = (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as i32
        - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as i32;
    let y = (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)) as i32
        - (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) as i32;
    let direction = Vec2::new(x as f32, y as f32).normalize_or_zero();
    let mut transform = player.single_mut().unwrap();
    transform.translation += (direction * 310.0 * time.delta_secs()).extend(0.0);
    transform.translation.x = transform
        .translation
        .x
        .clamp(-ARENA.x + 17.0, ARENA.x - 17.0);
    transform.translation.y = transform
        .translation
        .y
        .clamp(-ARENA.y + 17.0, ARENA.y - 17.0);
}

fn move_hazards(
    time: Res<Time>,
    game: Res<Game>,
    mut hazards: Query<(&mut Transform, &mut Hazard)>,
) {
    if game.phase != Phase::Playing {
        return;
    }
    for (mut transform, mut hazard) in &mut hazards {
        transform.translation += (hazard.velocity * time.delta_secs()).extend(0.0);
        if transform.translation.x.abs() > ARENA.x - 16.0 {
            transform.translation.x = transform
                .translation
                .x
                .clamp(-ARENA.x + 16.0, ARENA.x - 16.0);
            hazard.velocity.x *= -1.0;
        }
        if transform.translation.y.abs() > ARENA.y - 16.0 {
            transform.translation.y = transform
                .translation
                .y
                .clamp(-ARENA.y + 16.0, ARENA.y - 16.0);
            hazard.velocity.y *= -1.0;
        }
    }
}

fn collect_gems(
    mut game: ResMut<Game>,
    player: Query<&Transform, With<Player>>,
    mut gems: Query<&mut Transform, (With<Gem>, Without<Player>)>,
) {
    if game.phase != Phase::Playing {
        return;
    }
    let player = player.single().unwrap().translation.truncate();
    for mut gem in &mut gems {
        if player.distance(gem.translation.truncate()) < 26.0 {
            game.score += 1;
            gem.translation = game.gem_position();
        }
    }
}

fn check_collision(
    mut game: ResMut<Game>,
    player: Query<&Transform, With<Player>>,
    hazards: Query<&Transform, (With<Hazard>, Without<Player>)>,
    mut message: Query<&mut Text2d, With<Message>>,
) {
    if game.phase != Phase::Playing {
        return;
    }
    let player = player.single().unwrap().translation.truncate();
    if hazards.iter().any(|hazard| {
        let distance = (player - hazard.translation.truncate()).abs();
        distance.x < 28.0 && distance.y < 28.0
    }) {
        game.phase = Phase::Over;
        message.single_mut().unwrap().0 = format!(
            "GAME OVER\nGems collected: {}\nPress SPACE to try again",
            game.score
        );
    }
}

fn update_hud(
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut hud: Query<&mut Text2d, With<Hud>>,
    mut message: Query<&mut Text2d, (With<Message>, Without<Hud>)>,
) {
    if game.phase == Phase::Playing {
        game.time_left = (game.time_left - time.delta_secs()).max(0.0);
        if game.time_left == 0.0 {
            game.phase = Phase::Over;
            message.single_mut().unwrap().0 = format!(
                "TIME UP!\nGems collected: {}\nPress SPACE to play again",
                game.score
            );
        }
    }
    hud.single_mut().unwrap().0 = format!(
        "GEMS  {}                                      TIME  {:02}",
        game.score,
        game.time_left.ceil() as u32
    );
}
