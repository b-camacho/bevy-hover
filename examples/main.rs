use bevy::core_pipeline::bloom::{BloomCompositeMode, BloomSettings};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::f32::consts::PI;
use std::time::Duration;

use bevy_hover as hover;

#[derive(Component)]
struct SphereSeg {
    hover_start: Duration,
    press_start: Duration,
    hover_material: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.01,
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 400.0,
            ..default()
        },
        ..default()
    });

    commands.insert_resource(SphereRotVel {
        // rotate around 2 perpendicular axes at about 5.7 deg/s
        vel: Quat::from_euler(EulerRot::ZYX, 0.1, 0.1, 0.0),
    });

    let mut sphere = commands.spawn(SpatialBundle::default());
    sphere.insert(SphereRot {});
    let sid = sphere.id();
    let mut ids = Vec::new();

    for idx in 0..80 {
        let handle: Handle<Mesh> = asset_server.load(format!("ico.glb#Mesh{idx}/Primitive0"));

        let material = materials.add(StandardMaterial {
            base_color: Color::GRAY.with_l(0.7),
            ..default()
        });

        let mut seg = commands.spawn(PbrBundle {
            mesh: handle,
            material: material.clone(),
            ..default()
        });
        seg.insert(SphereSeg {
            hover_start: Duration::from_secs(0),
            press_start: Duration::from_secs(0),
            hover_material: material.clone(),
        });
        ids.push(seg.id());
        seg.insert(hover::Hoverable {});
    }
    commands.entity(sid).push_children(&ids);

    // camera
    commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 3.0),
                tonemapping: Tonemapping::TonyMcMapface,
                projection: Projection::Orthographic(OrthographicProjection {
                    scale: 0.005,
                    ..default()
                }),
                ..Default::default()
            },
            BloomSettings {
                intensity: 0.1,
                composite_mode: BloomCompositeMode::Additive,
                ..default()
            },
        ))
        .insert(hover::MouseRaySource);
}

fn on_hover(
    mut commands: Commands,
    mut query: Query<&mut SphereSeg>,
    mut ev_hover_start: EventReader<hover::HoverStart>,
    time: Res<Time>,
) {
    for ev in ev_hover_start.read() {
        if let Ok(mut seg) = query.get_mut(ev.hovered) {
            commands
                .entity(ev.hovered)
                .insert(seg.hover_material.clone());
            seg.hover_start = time.elapsed();
        }
    }
}

fn fade(
    mut query: Query<&mut SphereSeg>,
    time: Res<Time>,
    mut assets: ResMut<Assets<StandardMaterial>>,
) {
    for seg in query.iter_mut() {
        let elapsed = (time.elapsed() - seg.hover_start).as_millis();
        let v = (elapsed as f32).map_clamped((0.0, 1000.0), (0.75, 0.0));
        let a = assets.get_mut(seg.hover_material.clone()).unwrap();
        a.emissive.set_s(v);
        a.emissive.set_l(v);
    }
}

fn shrink(
    mut query: Query<(&mut SphereSeg, &mut Transform)>,
    time: Res<Time>,
) {
    for (seg, mut tr) in query.iter_mut() {
        let elapsed = (time.elapsed() - seg.press_start).as_millis();
        let v = (elapsed as f32).map_clamped((0.0, 1000.0), (0.8, 1.0));
        tr.scale = Vec3::new(v,v,v);
    }
}

fn on_press(
    mut query: Query<&mut SphereSeg>,
    mut ev_press: EventReader<hover::HoverPress>,
    mut assets: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for ev in ev_press.read() {
        if let Ok(mut seg) = query.get_mut(ev.entity) {
            let mat = assets.get_mut(seg.hover_material.clone()).unwrap();

            // on click: cycle color and reset hover timer
            seg.hover_start = time.elapsed();
            seg.press_start = time.elapsed();
            mat.emissive.set_h((mat.emissive.h() + 30.0) % 360.0);
        }
    }
}



fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(640f32, 640f32),
                        canvas: Some("#bevy".to_owned()),
                        ..default()
                    }),
                    ..default()
                })
                .build(),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, on_hover)
        .add_systems(Update, fade)
        .add_systems(Update, shrink)
        .add_systems(Update, rotate)
        .add_systems(Update, on_press)
        .add_plugins(hover::MouseRayPlugin)
        .run();
}

pub trait MapRange {
    type Num;
    fn map(&self, src: (Self::Num, Self::Num), dst: (Self::Num, Self::Num)) -> Self::Num;
    fn map_clamped(&self, src: (Self::Num, Self::Num), dst: (Self::Num, Self::Num)) -> Self::Num;
}

impl MapRange for f32 {
    type Num = f32;
    fn map(&self, src: (f32, f32), dst: (f32, f32)) -> f32 {
        if src.0 == src.1 {
            return dst.0; // avoid div by 0
        }
        let m = (dst.1 - dst.0) / (src.1 - src.0);
        let b = ((dst.0 * src.1) - (dst.1 * src.0)) / (src.1 - src.0);
        // y = mx+b
        (self * m) + b
    }
    fn map_clamped(&self, src: (f32, f32), dst: (f32, f32)) -> f32 {
        let clamped = if src.0 <= src.1 {
            self.clamp(src.0, src.1)
        } else {
            self.clamp(src.1, src.0)
        };

        clamped.map(src, dst)
    }
}


#[derive(Resource)]
struct SphereRotVel {
    pub vel: Quat, // Sphere rotates by the `vel` quat each second
}

#[derive(Component)]
struct SphereRot {}

/// Every tick, advance sphere's rotation
fn rotate(
    res_vel: Res<SphereRotVel>,
    mut transform: Query<&mut Transform, With<SphereRot>>,
    time: Res<Time>,
) {
    let delta = time.delta().as_secs_f32();
    let rot = res_vel.vel;
    let rot_scaled = {
        let (plane, angle) = rot.to_axis_angle();
        let angle_scaled = angle.map((0.0, PI), (0.0, PI * delta));
        Quat::from_axis_angle(plane, angle_scaled)
    };

    for mut tr in transform.iter_mut() {
        tr.rotate(rot_scaled);
    }
}
