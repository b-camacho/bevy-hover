use bevy::core_pipeline::bloom::{BloomCompositeMode, BloomSettings};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_debug_grid::*;

use bevy_inspector_egui::quick::WorldInspectorPlugin;
use std::collections::HashMap;
use std::f32::consts::PI;

mod hover;

#[derive(Resource)]
struct SphereRotVel {
    pub vel: Quat, // Sphere rotates by the `vel` quat each second
}

#[derive(Component)]
struct SphereRot {}

#[derive(Component)]
struct SphereSeg {
    idle_material: Handle<StandardMaterial>,
    hover_material: Handle<StandardMaterial>,
}

fn sphere_rot(
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ambient light
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
        vel: Quat::from_euler(EulerRot::ZYX, 0.1, 0.1, 0.0),
    });

    let mut sphere = commands.spawn(SpatialBundle::default());
    sphere.insert(SphereRot {});
    let sid = sphere.id();

    let mut ids = Vec::new();
    for idx in 0..80 {
        let mesh = asset_server.load(format!("ico.glb#Mesh{idx}/Primitive0"));
        let map_from_idx = |to_range| {
            let (to_start, to_end) = to_range;
            (idx as f32).map((0.0, 80.0), (to_start, to_end))
        };
        // hsla luminance goes from 0 to 1
        let l = map_from_idx((0.6, 0.8));
        // hsla hue goes from 0 to 360
        let h = map_from_idx((0.0, 360.0));

        let material = materials.add(StandardMaterial {
            base_color: Color::GRAY.with_l(l),
            ..default()
        });

        let hover_material = materials.add(StandardMaterial {
            base_color: Color::hsla(h, 0.5, l, 1.0),
            emissive: Color::hsla(h, 0.5, l, 1.0),
            ..default()
        });
        let mut seg = commands.spawn(PbrBundle {
            mesh,
            material: material.clone(),
            ..default()
        });
        seg.insert(SphereSeg {
            idle_material: material,
            hover_material: hover_material.clone(),
        });
        ids.push(seg.id());
        seg.insert(hover::Hoverable {
            material: Some(hover_material),
        });
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

#[derive(Resource)]
struct HoverMaterial(Handle<StandardMaterial>);

#[derive(Resource)]
struct HoverMaterialStore(HashMap<Entity, Handle<StandardMaterial>>);

fn add_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let hover_material = materials.add(StandardMaterial {
        base_color: Color::RED,
        ..Default::default()
    });

    commands.insert_resource(HoverMaterial(hover_material));
    commands.insert_resource(HoverMaterialStore(HashMap::new()));
}

fn update_material(
    mut commands: Commands,
    mut ev_hover_start: EventReader<hover::HoverStart>,
    mut ev_hover_end: EventReader<hover::HoverEnd>,
    mut query: Query<&mut SphereSeg>,
) {
    for ev in ev_hover_start.read() {
        println!("got hover start event for {ev:?}");
        if let Ok(seg) = query.get_mut(ev.hovered) {
            commands
                .entity(ev.hovered)
                .insert(seg.hover_material.clone());
        }
    }

    for ev in ev_hover_end.read() {
        if let Ok(seg) = query.get_mut(ev.hovered) {
            commands
                .entity(ev.hovered)
                .insert(seg.idle_material.clone());
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
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(DebugGridPlugin::with_floor_grid())
        .add_systems(Startup, setup)
        .add_systems(Startup, add_materials)
        .add_systems(Update, update_material)
        .add_systems(Update, sphere_rot)
        .add_plugins(hover::MouseRayPlugin)
        .run();
}

pub trait MapRange {
    type Num;
    fn map(&self, src: (Self::Num, Self::Num), dst: (Self::Num, Self::Num)) -> Self::Num;
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
}
