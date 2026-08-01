use rapier3d::prelude::*;
use rf_ecs::World as EcsWorld;
use rf_math::{Quat, Transform, Vec3};

use crate::RigidBody as RigidBodyComponent;

pub struct PhysicsWorld {
    pub gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    pub rigid_bodies: RigidBodySet,
    pub colliders: ColliderSet,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, -9.81, 0.0))
    }
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity: vec3_to_na(gravity),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            rigid_bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;
        let physics_hooks = ();
        let event_handler = ();
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &physics_hooks,
            &event_handler,
        );
    }

    pub fn add_box_body(
        &mut self,
        body_type: RigidBodyType,
        transform: Transform,
        half_extents: Vec3,
        density: f32,
        restitution: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::new(body_type)
            .translation(vec3_to_na(transform.translation))
            .rotation(quat_to_scaled_axis(transform.rotation))
            .build();
        let body_handle = self.rigid_bodies.insert(body);

        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .density(density)
            .restitution(restitution)
            .build();
        let collider_handle =
            self.colliders
                .insert_with_parent(collider, body_handle, &mut self.rigid_bodies);

        (body_handle, collider_handle)
    }

    pub fn add_static_ground(&mut self, half_extents: Vec3) -> (RigidBodyHandle, ColliderHandle) {
        self.add_box_body(
            RigidBodyType::Fixed,
            Transform::IDENTITY,
            half_extents,
            1.0,
            0.1,
        )
    }

    pub fn add_heightfield_body(
        &mut self,
        center: Vec3,
        heights: &nalgebra::DMatrix<f32>,
        scale: Vec3,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::new(RigidBodyType::Fixed)
            .translation(vec3_to_na(center))
            .build();
        let body_handle = self.rigid_bodies.insert(body);
        let collider = ColliderBuilder::heightfield(heights.clone(), vec3_to_na(scale)).build();
        let collider_handle =
            self.colliders
                .insert_with_parent(collider, body_handle, &mut self.rigid_bodies);
        (body_handle, collider_handle)
    }

    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.rigid_bodies.remove(
            handle,
            &mut self.island_manager,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    pub fn body(&self, handle: RigidBodyHandle) -> &RigidBody {
        &self.rigid_bodies[handle]
    }

    pub fn body_mut(&mut self, handle: RigidBodyHandle) -> &mut RigidBody {
        &mut self.rigid_bodies[handle]
    }

    pub fn set_planar_velocity(&mut self, handle: RigidBodyHandle, velocity_xz: Vec3) {
        let body = &mut self.rigid_bodies[handle];
        let current_y = body.linvel().y;
        body.set_linvel(Vector::new(velocity_xz.x, current_y, velocity_xz.z), true);
    }

    pub fn apply_impulse(&mut self, handle: RigidBodyHandle, impulse: Vec3) {
        self.rigid_bodies[handle].apply_impulse(vec3_to_na(impulse), true);
    }

    pub fn is_grounded(&self, handle: RigidBodyHandle, probe_distance: f32) -> bool {
        let body = &self.rigid_bodies[handle];
        let origin = Point::from(*body.translation());
        let ray = Ray::new(origin, Vector::new(0.0, -1.0, 0.0));
        let filter = QueryFilter::default().exclude_rigid_body(handle);
        self.query_pipeline
            .cast_ray(
                &self.rigid_bodies,
                &self.colliders,
                &ray,
                probe_distance,
                true,
                filter,
            )
            .is_some()
    }
}

fn vec3_to_na(v: Vec3) -> Vector<f32> {
    Vector::new(v.x, v.y, v.z)
}

fn na_to_vec3(v: &Vector<f32>) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn quat_to_scaled_axis(q: Quat) -> Vector<f32> {
    let (axis, angle) = q.to_axis_angle();
    Vector::new(axis.x, axis.y, axis.z) * angle
}

fn na_rotation_to_quat(r: &Rotation<f32>) -> Quat {
    let q = r.quaternion();
    Quat::from_xyzw(q.i, q.j, q.k, q.w)
}

pub fn sync_transforms(world: &mut EcsWorld, physics: &PhysicsWorld) {
    for (_entity, (transform, body)) in world.query_mut::<(&mut Transform, &RigidBodyComponent)>() {
        let rb = &physics.rigid_bodies[body.0];
        transform.translation = na_to_vec3(rb.translation());
        transform.rotation = na_rotation_to_quat(rb.rotation());
    }
}
