use crate::{
    flat::rocketsim,
    udp::{ToBevyVec, ToBevyVecFlat},
};
use ahash::AHashMap;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct RenderGroups {
    pub last: AHashMap<i32, Vec<rocketsim::Render>>,
    pub next: AHashMap<i32, Vec<rocketsim::Render>>,
}

impl From<rocketsim::Color> for Color {
    fn from(value: rocketsim::Color) -> Self {
        Self::srgba(value.r, value.g, value.b, value.a)
    }
}

fn interpolate_color(l: &rocketsim::Color, n: &rocketsim::Color, lerp_amount: f32) -> rocketsim::Color {
    rocketsim::Color {
        r: l.r + (n.r - l.r) * lerp_amount,
        g: l.g + (n.g - l.g) * lerp_amount,
        b: l.b + (n.b - l.b) * lerp_amount,
        a: l.a + (n.a - l.a) * lerp_amount,
    }
}

fn interpolate_render_vec(
    last: &[rocketsim::Render],
    next: &[rocketsim::Render],
    lerp_amount: f32,
) -> Vec<rocketsim::Render> {
    if last.len() != next.len() {
        println!("Interpolation lengths mismatch: last={} next={}", last.len(), next.len());
    }
    
    let mut interpolated = Vec::with_capacity(next.len());
    for (i, n) in next.iter().enumerate() {
        if let Some(l) = last.get(i) {
            match (l, n) {
                (rocketsim::Render::Line2D(l_line), rocketsim::Render::Line2D(n_line)) => {
                    interpolated.push(rocketsim::Render::Line2D(Box::new(rocketsim::Line2D {
                        start: rocketsim::Vec2 {
                            x: l_line.start.x + (n_line.start.x - l_line.start.x) * lerp_amount,
                            y: l_line.start.y + (n_line.start.y - l_line.start.y) * lerp_amount,
                        },
                        end: rocketsim::Vec2 {
                            x: l_line.end.x + (n_line.end.x - l_line.end.x) * lerp_amount,
                            y: l_line.end.y + (n_line.end.y - l_line.end.y) * lerp_amount,
                        },
                        color: interpolate_color(&l_line.color, &n_line.color, lerp_amount),
                    })));
                }
                (rocketsim::Render::Line3D(l_line), rocketsim::Render::Line3D(n_line)) => {
                    interpolated.push(rocketsim::Render::Line3D(Box::new(rocketsim::Line3D {
                        start: rocketsim::Vec3 {
                            x: l_line.start.x + (n_line.start.x - l_line.start.x) * lerp_amount,
                            y: l_line.start.y + (n_line.start.y - l_line.start.y) * lerp_amount,
                            z: l_line.start.z + (n_line.start.z - l_line.start.z) * lerp_amount,
                        },
                        end: rocketsim::Vec3 {
                            x: l_line.end.x + (n_line.end.x - l_line.end.x) * lerp_amount,
                            y: l_line.end.y + (n_line.end.y - l_line.end.y) * lerp_amount,
                            z: l_line.end.z + (n_line.end.z - l_line.end.z) * lerp_amount,
                        },
                        color: interpolate_color(&l_line.color, &n_line.color, lerp_amount),
                    })));
                }
                (rocketsim::Render::LineStrip(l_strip), rocketsim::Render::LineStrip(n_strip)) => {
                    if l_strip.positions.len() == n_strip.positions.len() {
                        let mut positions = Vec::with_capacity(n_strip.positions.len());
                        for (l_pos, n_pos) in l_strip.positions.iter().zip(n_strip.positions.iter()) {
                            positions.push(rocketsim::Vec3 {
                                x: l_pos.x + (n_pos.x - l_pos.x) * lerp_amount,
                                y: l_pos.y + (n_pos.y - l_pos.y) * lerp_amount,
                                z: l_pos.z + (n_pos.z - l_pos.z) * lerp_amount,
                            });
                        }
                        interpolated.push(rocketsim::Render::LineStrip(Box::new(rocketsim::LineStrip {
                            positions,
                            color: interpolate_color(&l_strip.color, &n_strip.color, lerp_amount),
                        })));
                    } else {
                        println!("Interpolation positions mismatch: last={} next={}", l_strip.positions.len(), n_strip.positions.len());
                        interpolated.push(n.clone());
                    }
                }
                _ => {
                    println!("Interpolation type mismatch or unknown render type");
                    interpolated.push(n.clone());
                }
            }
        } else {
            interpolated.push(n.clone());
        }
    }
    interpolated
}

fn render_gizmos(
    renders: Res<RenderGroups>,
    mut gizmos: Gizmos,
    time: Res<Time>,
    game_speed: Res<crate::settings::options::GameSpeed>,
    last_packet_time_elapsed: Res<crate::udp::LastPacketTimesElapsed>,
    packet_time_elapsed: Res<crate::udp::PacketTimeElapsed>,
    packet_smoothing: Res<crate::settings::options::PacketSmoothing>,
) {
    let lerp_amount = if matches!(*packet_smoothing, crate::settings::options::PacketSmoothing::Interpolate) && !game_speed.paused {
        let delta_time = packet_time_elapsed.elapsed_secs();
        if delta_time <= 2. {
            delta_time / last_packet_time_elapsed.avg()
        } else {
            1.0
        }
    } else {
        1.0
    };

    for (&group_id, next_renders) in renders.next.iter() {
        let interpolated_renders = if lerp_amount < 1.0 {
            if let Some(last_renders) = renders.last.get(&group_id) {
                interpolate_render_vec(last_renders, next_renders, lerp_amount)
            } else {
                next_renders.clone()
            }
        } else {
            next_renders.clone()
        };

        for render in interpolated_renders.iter() {
            match render {
                rocketsim::Render::Line2D(r) => {
                    gizmos.line_2d(r.start.to_bevy_flat(), r.end.to_bevy_flat(), r.color);
                }
                rocketsim::Render::Line3D(r) => {
                    gizmos.line(r.start.to_bevy(), r.end.to_bevy(), r.color);
                }
                rocketsim::Render::LineStrip(r) => {
                    gizmos.linestrip(r.positions.iter().copied().map(ToBevyVec::to_bevy), r.color);
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct DoRendering(pub bool);

pub struct UdpRendererPlugin;

impl Plugin for UdpRendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderGroups::default())
            .insert_resource(DoRendering(true))
            .add_systems(Update, render_gizmos.run_if(|do_rendering: Res<DoRendering>| do_rendering.0))
            .add_systems(
                Update,
                update_environment_visibility.run_if(resource_changed::<crate::settings::options::Options>),
            );
    }
}

#[allow(clippy::type_complexity)]
fn update_environment_visibility(
    options: Res<crate::settings::options::Options>,
    mut commands: Commands,
    mut meshes: Query<&mut Visibility, With<Mesh3d>>,
    mut camera: Query<(Entity, &mut Camera, Option<&bevy::pbr::Atmosphere>), With<crate::camera::PrimaryCamera>>,
    mut global_clear_color: ResMut<ClearColor>,
) {
    let vis = if options.render_environment {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for mut visibility in &mut meshes {
        *visibility = vis;
    }

    if let Ok((entity, mut cam, atmosphere)) = camera.single_mut() {
        if options.render_environment {
            cam.clear_color = ClearColorConfig::Default;
            global_clear_color.0 = Color::BLACK; // Make full black
            if atmosphere.is_none() {
                commands.entity(entity).insert(bevy::pbr::Atmosphere::EARTH);
            }
        } else {
            cam.clear_color = ClearColorConfig::Custom(Color::BLACK);
            global_clear_color.0 = Color::BLACK;
            if atmosphere.is_some() {
                commands.entity(entity).remove::<bevy::pbr::Atmosphere>();
            }
        }
    }
}
