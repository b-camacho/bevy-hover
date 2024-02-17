use bevy::prelude::*;

use bevy::render::mesh::VertexAttributeValues;
use std::collections::HashMap;

#[derive(Component, Default)]
pub struct Hoverable;

#[derive(Component)]
pub struct Hover {
    // time elapsed from app start to hover event start
    pub since: std::time::Duration
}

#[derive(Resource)]
pub struct Hovered {
    pub inner: Option<Entity>,
}

#[derive(Event, Debug)]
pub struct HoverStart {
    pub hovered: Entity,
}

#[derive(Event, Debug)]
pub struct HoverEnd {
    pub hovered: Entity,
}

#[derive(Component, Default)]
struct MouseRay {
    ray: Ray,
}
#[derive(Component)]
pub struct MouseRaySource;

/// Ray extending from the image plane, through the mouse pointer, into the scene
impl MouseRay {
    /// returns cursor position in window space
    /// (-1,-1) -> bottom left and (1,1) -> upper right
    pub(crate) fn cursor_to_pos(position: &Vec2, window: &Window) -> Vec2 {
        let (window_width, window_height) = (window.width(), window.height());
        Vec2::new(
            position.x / window_width * 2.0 - 1.0,
            // cursor_pos is from a `winit::CursorMoved` event
            // where positive x goes right and positive y goes **down**
            // see https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.CursorMoved
            // in bevy, positive y goes **up**
            // flip y to convert
            1.0 - (position.y / window_height * 2.0),
        )
    }

    pub(crate) fn pos_from_camera(
        camera: &Camera,
        projection: &Projection,
        transform: &GlobalTransform,
        cursor_pos: Vec2, // [-1, 1]
    ) -> Ray {
        // worldspace - position in 3d, with the global coordinate frame
        // eyespace - position in 3d, with a coordinate frame centered on the camera
        // imagespace - position in the 2d image

        // position of the cursor, in imagespace [-1, 1]
        let clip_space_pos = Vec3::new(cursor_pos.x, cursor_pos.y, 0.0);

        // assuming the camera is at origin,
        // `camera.projection_matrix()` transforms worldspace points into imagespace points
        // when inverted, the matrix converts imagespace points into worldspace points
        let inverse_projection = camera.projection_matrix().inverse();

        match projection {
            Projection::Perspective(_) => {
                // transform cursor position from imagespace position into "eyespace"
                let eye_space_pos = inverse_projection.transform_point3(clip_space_pos);
                // but the camera can be at any position!
                // transform the "eyespace" position to the true worldspace position
                let world_space_pos = transform.compute_matrix() * eye_space_pos.extend(1.0);

                Ray {
                    // ray originates at the camera's focal point
                    origin: transform.translation(),
                    // ray extends from the focal point through the worldspace position of the
                    // mouse cursor
                    direction: (world_space_pos.truncate() - transform.translation()).normalize(),
                }
            }
            Projection::Orthographic(_) => {
                // same as the Prespective case, but ortho camera has no depth => disregard z component
                let mut eye_space_pos = inverse_projection.transform_point3(clip_space_pos);
                eye_space_pos.z = 0.0;
                let m = transform.compute_matrix();
                let world_space_pos = m * eye_space_pos.extend(1.0);

                // an orthographic camera has no focal point or vanishing point
                // in this case, camera rotation determines the ray direction
                Ray {
                    origin: world_space_pos.truncate(),
                    // I honestly thought I understood this pretty well until this point
                    // this is where we extract the rotation component of the camera's 4x4 3d affine
                    // transform matrix, then multiply a unit vector by the resulting 3x3 matrix
                    //
                    // only the resulting rotated vector is always equivalent to the 3rd column of
                    // the affine transform matrix * (-1)
                    direction: -1.0 * m.z_axis.truncate(),
                }
            }
        }
    }
}

fn add_mouse_ray(mut commands: Commands) {
    commands.spawn(MouseRay::default());
}

fn add_resources(mut commands: Commands) {
    commands.insert_resource(Hovered { inner: None });
}

fn update_mouse_ray(
    mut query: Query<&mut MouseRay>,
    windows: Query<&Window>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    camera_query: Query<(&Camera, &Projection, &GlobalTransform)>,
) {
    if let (Ok(window), Ok(mut mouse_ray)) = (windows.get_single(), query.get_single_mut()) {
        for event in cursor_moved_events.read() {
            let (camera, projection, camera_transform) = camera_query.single();
            let cursor_pos = MouseRay::cursor_to_pos(&event.position, window);
            let ray = MouseRay::pos_from_camera(camera, projection, camera_transform, cursor_pos);
            mouse_ray.ray = ray;
        }
    }
}

fn update_hover_state(
    mut commands: Commands,
    mesh_assets: Res<Assets<Mesh>>,
    ray_query: Query<&MouseRay>,
    mut ev_hover_start: EventWriter<HoverStart>,
    mut ev_hover_end: EventWriter<HoverEnd>,
    query: Query<(&Handle<Mesh>, &GlobalTransform, Entity), With<Hoverable>>,
    mut hovered: ResMut<Hovered>,
    time: Res<Time>,
) {
    for ray in ray_query.iter() {
        // Option<(distance, intersectee)>
        let mut intersect_nearest: Option<(f32, Entity)> = None;

        for (mesh_handle, transform, entity) in query.iter() {
            if let Some(mesh) = mesh_assets.get(mesh_handle) {
                let intersect = check_intersect(ray, mesh, transform);
                match (intersect, intersect_nearest) {
                    (Some(i), Some((i_n, _))) => {
                        if i_n > i {
                            intersect_nearest = Some((i, entity))
                        }
                    }
                    (Some(i), None) => intersect_nearest = Some((i, entity)),
                    _ => (),
                }
            }
        }
        if let Some((_, entity)) = intersect_nearest {
            if let Some(prev_hover) = hovered.inner {
                if prev_hover != entity {
                    commands.entity(prev_hover).remove::<Hover>();
                    ev_hover_end.send(HoverEnd {
                        hovered: prev_hover,
                    });

                    commands.entity(entity).insert(Hover { since: time.elapsed() });
                    ev_hover_start.send(HoverStart { hovered: entity });
                    hovered.inner = Some(entity);
                }
            } else {
                commands.entity(entity).insert(Hover { since: time.elapsed() });
                ev_hover_start.send(HoverStart { hovered: entity });
                hovered.inner = Some(entity);
            }
        } else {
            // no intersect, no entity currently hovered
            if let Some(prev_hover) = hovered.inner {
                commands.entity(prev_hover).remove::<Hover>();
                ev_hover_end.send(HoverEnd {
                    hovered: prev_hover,
                });
                hovered.inner = None;
            }
        }
    }
}

/// Some(distance) if there is an intersection
/// None otherwise
fn check_intersect(ray: &MouseRay, mesh: &Mesh, transform: &GlobalTransform) -> Option<f32> {
    if let Some(VertexAttributeValues::Float32x3(vertex_positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let inner_fn = |indices: &Vec<u32>| {
            let mut min_dist: Option<f32> = None;
            for tri in indices.chunks_exact(3) {
                let v0 = Vec3::from(vertex_positions[tri[0] as usize]);
                let v1 = Vec3::from(vertex_positions[tri[1] as usize]);
                let v2 = Vec3::from(vertex_positions[tri[2] as usize]);

                // Transform the vertices from model space to world space
                let mat = transform.compute_matrix();
                let v0 = mat.transform_point3(v0);
                let v1 = mat.transform_point3(v1);
                let v2 = mat.transform_point3(v2);

                // Use Moller-Trumbore algorithm here to check for intersection
                let dist = moller_trumbore(ray.ray.origin, ray.ray.direction, v0, v1, v2);
                match (dist, min_dist) {
                    (Some(d), Some(md)) if md > d => min_dist = Some(d),
                    (Some(d), None) => min_dist = Some(d),
                    _ => (),
                };
            }
            min_dist
        };

        match mesh.indices() {
            Some(bevy::render::mesh::Indices::U32(indices)) => inner_fn(indices),
            // TODO: very bad, clones mesh so I can avoid copy-pasting inner_fn
            Some(bevy::render::mesh::Indices::U16(indices)) => {
                inner_fn(&indices.iter().map(|x| *x as u32).collect())
            }
            None => None,
        }
    } else {
        None
    }
}

pub fn moller_trumbore(
    ray_origin: Vec3,
    ray_direction: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    //
    let epsilon = 0.000_001;
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray_direction.cross(edge2);
    let a = edge1.dot(h);

    if a > -epsilon && a < epsilon {
        return None; // This ray is parallel to this triangle
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray_direction.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);

    if t > epsilon {
        Some(t)
    } else {
        None
    }
}

pub struct MouseRayPlugin;

impl Plugin for MouseRayPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HoverStart>()
            .add_event::<HoverEnd>()
            .add_systems(Startup, add_mouse_ray)
            .add_systems(Startup, add_resources)
            .add_systems(Update, update_mouse_ray)
            .add_systems(Update, update_hover_state);
    }
}
