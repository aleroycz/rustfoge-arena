use std::sync::Arc;

use hecs::Entity;

use rf_engine::math::{Transform, Vec3};
use rf_engine::renderer::{GpuTexture, Material, Mesh};
use rf_engine::scene::spawn_spatial;
use rf_engine::EngineContext;



fn load_gltf_entities(
    ctx: &mut EngineContext,
    path: &str,
    fallback_color: [u8; 4],
) -> Option<Vec<Entity>> {
    let device = ctx.renderer.device();
    let queue = ctx.renderer.queue();
    let material_layout = ctx.renderer.material_bind_group_layout();

    let scene = match rf_assets::load_gltf(path) {
        Ok(scene) => scene,
        Err(err) => {
            log::warn!("failed to load {path}: {err:#}");
            return None;
        }
    };

    let materials: Vec<Arc<Material>> = scene
        .materials
        .iter()
        .map(|mat| {
            let texture = match &mat.base_color_image {
                Some(image) => GpuTexture::from_rgba8(
                    device,
                    queue,
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    path,
                ),
                None => GpuTexture::solid_color(device, queue, fallback_color),
            };
            Arc::new(Material::new(
                device,
                queue,
                material_layout,
                texture,
                mat.base_color_factor,
                mat.metallic,
                mat.roughness,
            ))
        })
        .collect();

    let fallback_material = Arc::new(Material::new(
        device,
        queue,
        material_layout,
        GpuTexture::solid_color(device, queue, fallback_color),
        [1.0, 1.0, 1.0, 1.0],
        0.3,
        0.5,
    ));

    let mut entities = Vec::new();
    for mesh in &scene.meshes {
        let gpu_mesh = Arc::new(Mesh::new(device, &mesh.vertices, &mesh.indices));
        let material = mesh
            .material_index
            .and_then(|i| materials.get(i).cloned())
            .unwrap_or_else(|| fallback_material.clone());
        let render = ctx.renderer.create_render_object(gpu_mesh, material);
        let entity = spawn_spatial(ctx.world, Transform::IDENTITY);
        let _ = ctx.world.insert_one(entity, render);
        entities.push(entity);
    }
    Some(entities)
}

pub(super) fn load_rifle_view_model(ctx: &mut EngineContext) -> Vec<Entity> {
    load_gltf_entities(ctx, "assets/models/rifle.glb", [40, 40, 45, 255]).unwrap_or_else(|| {
        let device = ctx.renderer.device();
        let queue = ctx.renderer.queue();
        let material_layout = ctx.renderer.material_bind_group_layout();
        let texture = GpuTexture::solid_color(device, queue, [40, 40, 45, 255]);
        let material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            texture,
            [1.0, 1.0, 1.0, 1.0],
            0.3,
            0.5,
        ));
        let mesh = Arc::new(Mesh::cuboid(device, Vec3::new(0.15, 0.15, 0.8)));
        let render = ctx.renderer.create_render_object(mesh, material);
        let entity = spawn_spatial(ctx.world, Transform::IDENTITY);
        let _ = ctx.world.insert_one(entity, render);
        vec![entity]
    })
}

pub(super) fn load_dancing_cat_model(ctx: &mut EngineContext) -> Vec<Entity> {
    load_gltf_entities(ctx, "assets/models/dancing_cat.glb", [220, 190, 160, 255]).unwrap_or_else(
        || {
            let device = ctx.renderer.device();
            let queue = ctx.renderer.queue();
            let material_layout = ctx.renderer.material_bind_group_layout();
            let texture = GpuTexture::solid_color(device, queue, [220, 190, 160, 255]);
            let material = Arc::new(Material::new(
                device,
                queue,
                material_layout,
                texture,
                [1.0, 1.0, 1.0, 1.0],
                0.0,
                0.6,
            ));
            let mesh = Arc::new(Mesh::uv_sphere(device, 0.35, 12, 12));
            let render = ctx.renderer.create_render_object(mesh, material);
            let entity = spawn_spatial(ctx.world, Transform::IDENTITY);
            let _ = ctx.world.insert_one(entity, render);
            vec![entity]
        },
    )
}
