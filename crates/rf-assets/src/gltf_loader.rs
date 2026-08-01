use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use rf_animation::{AnimationClip, Joint, Keyframe, Skeleton, Track};
use rf_math::{Mat4, Quat, Transform, Vec3};
use rf_renderer::Vertex;

pub struct GltfMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
}

pub struct GltfMaterial {
    pub base_color_factor: [f32; 4],
    pub base_color_image: Option<image::RgbaImage>,
    pub metallic: f32,
    pub roughness: f32,
}

pub struct GltfScene {
    pub meshes: Vec<GltfMesh>,
    pub materials: Vec<GltfMaterial>,
    pub skeleton: Option<Skeleton>,
    pub animations: Vec<AnimationClip>,
}

pub fn load_gltf(path: impl AsRef<Path>) -> Result<GltfScene> {
    let path = path.as_ref();
    let (document, buffers, images) =
        gltf::import(path).with_context(|| format!("loading glTF {path:?}"))?;

    let meshes = load_meshes(&document, &buffers);
    let materials = load_materials(&document, &images);

    let skin = document.skins().next();
    let joint_node_to_index: HashMap<usize, usize> = skin
        .as_ref()
        .map(|skin| {
            skin.joints()
                .enumerate()
                .map(|(i, node)| (node.index(), i))
                .collect()
        })
        .unwrap_or_default();

    let skeleton = skin
        .as_ref()
        .map(|skin| build_skeleton(&document, skin, &buffers, &joint_node_to_index));
    let animations = load_animations(&document, &buffers, &joint_node_to_index);

    Ok(GltfScene {
        meshes,
        materials,
        skeleton,
        animations,
    })
}

fn load_meshes(document: &gltf::Document, buffers: &[gltf::buffer::Data]) -> Vec<GltfMesh> {
    let mut meshes = Vec::new();
    for scene in document.scenes() {
        for node in scene.nodes() {
            collect_node_meshes(&node, Mat4::IDENTITY, buffers, &mut meshes);
        }
    }
    meshes
}

fn collect_node_meshes(
    node: &gltf::Node,
    parent_world: Mat4,
    buffers: &[gltf::buffer::Data],
    out: &mut Vec<GltfMesh>,
) {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent_world * local;
    let normal_matrix = world.inverse().transpose();

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<[f32; 3]> = positions.collect();
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|n| n.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|uv| uv.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let vertices: Vec<Vertex> = positions
                .into_iter()
                .zip(normals)
                .zip(uvs)
                .map(|((position, normal), uv)| {
                    let world_position = world.transform_point3(Vec3::from(position));
                    let world_normal = normal_matrix
                        .transform_vector3(Vec3::from(normal))
                        .normalize_or_zero();
                    Vertex {
                        position: world_position.into(),
                        normal: world_normal.into(),
                        uv,
                    }
                })
                .collect();

            let indices: Vec<u32> = match reader.read_indices() {
                Some(indices) => indices.into_u32().collect(),
                None => (0..vertices.len() as u32).collect(),
            };

            out.push(GltfMesh {
                vertices,
                indices,
                material_index: primitive.material().index(),
            });
        }
    }

    for child in node.children() {
        collect_node_meshes(&child, world, buffers, out);
    }
}

fn load_materials(document: &gltf::Document, images: &[gltf::image::Data]) -> Vec<GltfMaterial> {
    document
        .materials()
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let base_color_image = pbr.base_color_texture().and_then(|info| {
                let image = &images[info.texture().source().index()];
                gltf_image_to_rgba(image)
            });
            GltfMaterial {
                base_color_factor: pbr.base_color_factor(),
                base_color_image,
                metallic: pbr.metallic_factor(),
                roughness: pbr.roughness_factor(),
            }
        })
        .collect()
}

fn gltf_image_to_rgba(image: &gltf::image::Data) -> Option<image::RgbaImage> {
    use gltf::image::Format;
    let make = |channels: u32, convert: fn(&[u8]) -> [u8; 4]| {
        let pixel_count = (image.width * image.height) as usize;
        let mut out = Vec::with_capacity(pixel_count * 4);
        for i in 0..pixel_count {
            let start = i * channels as usize;
            out.extend_from_slice(&convert(&image.pixels[start..start + channels as usize]));
        }
        image::RgbaImage::from_raw(image.width, image.height, out)
    };

    match image.format {
        Format::R8G8B8A8 => {
            image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
        }
        Format::R8G8B8 => make(3, |p| [p[0], p[1], p[2], 255]),
        _ => None,
    }
}

fn build_skeleton(
    document: &gltf::Document,
    skin: &gltf::Skin,
    buffers: &[gltf::buffer::Data],
    joint_node_to_index: &HashMap<usize, usize>,
) -> Skeleton {
    let parent_of_node: HashMap<usize, usize> = document
        .nodes()
        .flat_map(|node| {
            let parent = node.index();
            node.children()
                .map(move |child| (child.index(), parent))
                .collect::<Vec<_>>()
        })
        .collect();

    let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
    let inverse_bind_matrices: Vec<Mat4> = reader
        .read_inverse_bind_matrices()
        .map(|m| m.map(|m| Mat4::from_cols_array_2d(&m)).collect())
        .unwrap_or_default();

    let joints = skin
        .joints()
        .enumerate()
        .map(|(i, node)| {
            let (translation, rotation, scale) = node.transform().decomposed();
            let parent = parent_of_node
                .get(&node.index())
                .and_then(|parent_node| joint_node_to_index.get(parent_node))
                .copied();
            Joint {
                name: node
                    .name()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("joint_{i}")),
                parent,
                local_rest_pose: Transform {
                    translation: Vec3::from(translation),
                    rotation: Quat::from_array(rotation),
                    scale: Vec3::from(scale),
                },
                inverse_bind_matrix: inverse_bind_matrices
                    .get(i)
                    .copied()
                    .unwrap_or(Mat4::IDENTITY),
            }
        })
        .collect();

    Skeleton::new(joints)
}

fn load_animations(
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    joint_node_to_index: &HashMap<usize, usize>,
) -> Vec<AnimationClip> {
    if joint_node_to_index.is_empty() {
        return Vec::new();
    }

    document
        .animations()
        .map(|animation| {
            let name = animation
                .name()
                .map(str::to_owned)
                .unwrap_or_else(|| "animation".to_string());
            let mut clip = AnimationClip::new(name);

            let mut translations = HashMap::new();
            let mut rotations = HashMap::new();
            let mut scales = HashMap::new();

            for channel in animation.channels() {
                let Some(&joint_index) = joint_node_to_index.get(&channel.target().node().index())
                else {
                    continue;
                };
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                let Some(inputs) = reader.read_inputs() else {
                    continue;
                };
                let times: Vec<f32> = inputs.collect();
                let Some(outputs) = reader.read_outputs() else {
                    continue;
                };

                use gltf::animation::util::ReadOutputs;
                match outputs {
                    ReadOutputs::Translations(values) => {
                        let keyframes = times
                            .iter()
                            .zip(values)
                            .map(|(&time, value)| Keyframe {
                                time,
                                value: Vec3::from(value),
                            })
                            .collect();
                        translations.insert(joint_index, Track::new(keyframes));
                    }
                    ReadOutputs::Rotations(values) => {
                        let keyframes = times
                            .iter()
                            .zip(values.into_f32())
                            .map(|(&time, value)| Keyframe {
                                time,
                                value: Quat::from_array(value),
                            })
                            .collect();
                        rotations.insert(joint_index, Track::new(keyframes));
                    }
                    ReadOutputs::Scales(values) => {
                        let keyframes = times
                            .iter()
                            .zip(values)
                            .map(|(&time, value)| Keyframe {
                                time,
                                value: Vec3::from(value),
                            })
                            .collect();
                        scales.insert(joint_index, Track::new(keyframes));
                    }
                    ReadOutputs::MorphTargetWeights(_) => {}
                }
            }

            let joint_indices: std::collections::HashSet<usize> = translations
                .keys()
                .chain(rotations.keys())
                .chain(scales.keys())
                .copied()
                .collect();

            for joint_index in joint_indices {
                let translation = translations
                    .remove(&joint_index)
                    .unwrap_or_else(|| Track::constant(Vec3::ZERO));
                let rotation = rotations
                    .remove(&joint_index)
                    .unwrap_or_else(|| Track::constant(Quat::IDENTITY));
                let scale = scales
                    .remove(&joint_index)
                    .unwrap_or_else(|| Track::constant(Vec3::ONE));
                clip.add_joint_tracks(joint_index, translation, rotation, scale);
            }

            clip
        })
        .collect()
}
