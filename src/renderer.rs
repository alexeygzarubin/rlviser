use crate::{
    flat::rocketsim,
    udp::{ToBevyVec, ToBevyVecFlat},
};
use ahash::AHashMap;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct RenderGroups {
    pub groups: AHashMap<i32, Vec<rocketsim::Render>>,
}

impl From<rocketsim::Color> for Color {
    fn from(value: rocketsim::Color) -> Self {
        Self::srgba(value.r, value.g, value.b, value.a)
    }
}

fn render_gizmos(renders: Res<RenderGroups>, mut gizmos: Gizmos) {
    for renders in renders.groups.values() {
        for render in renders.iter() {
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
