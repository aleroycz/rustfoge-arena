use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};

#[derive(Debug, Clone, Copy)]
pub struct RigidBody(pub RigidBodyHandle);

#[derive(Debug, Clone, Copy)]
pub struct Collider(pub ColliderHandle);
