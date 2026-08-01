use std::collections::HashMap;

use hecs::Entity;
use rf_ecs::World;
use rf_math::Transform;

pub use rf_math::Transform as LocalTransform;

#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalTransform(pub Transform);

#[derive(Debug, Clone, Copy)]
pub struct Parent(pub Entity);

pub fn spawn_spatial(world: &mut World, transform: Transform) -> Entity {
    world.spawn((transform, GlobalTransform(transform)))
}

pub fn set_parent(world: &mut World, child: Entity, parent: Entity) {
    let _ = world.insert_one(child, Parent(parent));
}

pub fn update_global_transforms(world: &mut World) {
    let locals: HashMap<Entity, Transform> = world
        .query::<&Transform>()
        .iter()
        .map(|(e, t)| (e, *t))
        .collect();
    let parents: HashMap<Entity, Entity> = world
        .query::<&Parent>()
        .iter()
        .map(|(e, p)| (e, p.0))
        .collect();

    let mut globals: HashMap<Entity, Transform> = HashMap::with_capacity(locals.len());
    for &entity in locals.keys() {
        resolve_global(entity, &locals, &parents, &mut globals);
    }

    for (entity, global) in world.query_mut::<&mut GlobalTransform>() {
        if let Some(resolved) = globals.get(&entity) {
            global.0 = *resolved;
        }
    }
}

fn resolve_global(
    entity: Entity,
    locals: &HashMap<Entity, Transform>,
    parents: &HashMap<Entity, Entity>,
    globals: &mut HashMap<Entity, Transform>,
) -> Transform {
    if let Some(&global) = globals.get(&entity) {
        return global;
    }
    let local = locals.get(&entity).copied().unwrap_or(Transform::IDENTITY);
    let global = match parents.get(&entity) {
        Some(&parent) if parent != entity => {
            let parent_global = resolve_global(parent, locals, parents, globals);
            parent_global.mul_transform(&local)
        }
        _ => local,
    };
    globals.insert(entity, global);
    global
}
