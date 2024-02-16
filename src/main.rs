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

static NEED_MESH_SETUP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn setup_meshes(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    meshes: Res<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !NEED_MESH_SETUP.load(std::sync::atomic::Ordering::Relaxed) {
        return; // we already initialized the meshes
    }
    let meshes_handles = (0..80)
        .map(|idx| 
             {
                 let h = asset_server.get_handle(format!("ico.glb#Mesh{idx}/Primitive0"));
                 (h.clone().and_then(|h| meshes.get(h)), h)
             }
             )
        .collect::<Vec<_>>();
    if meshes_handles.iter().all(|(m, h)| m.is_some() && h.is_some()) {
        println!("all meshes loaded");
    } else {
        return;
    }

    let mut sphere = commands.spawn(SpatialBundle::default());
    sphere.insert(SphereRot {});
    let sid = sphere.id();

    let mut ids = Vec::new();

    for (mesh, handle) in meshes_handles {
        let handle = handle.unwrap();
        let mesh = mesh.unwrap();

        let pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION);
        let avg_z = match pos {
            Some(bevy::render::mesh::VertexAttributeValues::Float32x3(arr)) => {
                let (cnt, s) = arr
                    .iter()
                    .fold((0, 0.0), |(cnt, s), [_x, _y, z]| (cnt + 1, s + z));
                Some(s / (cnt as f32))
            }
            _ => None,
        }
        .unwrap();

        let map_from_height = |to_range| {
            let (to_start, to_end) = to_range;
            (avg_z).map((-1.0, 1.0), (to_start, to_end))
        };

        // hsla luminance goes from 0 to 1
        let l = map_from_height((0.6, 0.8));
        // hsla hue goes from 0 to 360
        let h = map_from_height((190.0, 330.0));

        let material = materials.add(StandardMaterial {
            base_color: Color::GRAY.with_l(l),
            ..default()
        });

        let hover_material = materials.add(StandardMaterial {
            base_color: Color::hsla(h, 0.5, 0.75, 1.0),
            emissive: Color::hsla(h, 0.5, 0.75, 1.0),
            ..default()
        });
        let mut seg = commands.spawn(PbrBundle {
            mesh: handle,
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

    NEED_MESH_SETUP.store(false, std::sync::atomic::Ordering::Relaxed);
    commands.entity(sid).push_children(&ids);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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

    for idx in 0..80 {
        // meshes load in the background
        let _: Handle<Mesh> = asset_server.load(format!("ico.glb#Mesh{idx}/Primitive0"));
    }

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

fn update_material(
    mut commands: Commands,
    mut ev_hover_start: EventReader<hover::HoverStart>,
    mut ev_hover_end: EventReader<hover::HoverEnd>,
    mut query: Query<&mut SphereSeg>,
) {
    for ev in ev_hover_start.read() {
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
        .add_systems(Update, update_material)
        .add_systems(Update, sphere_rot)
        .add_systems(Update, setup_meshes)
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
