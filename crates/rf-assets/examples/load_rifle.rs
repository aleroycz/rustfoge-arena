fn check(path: &str) {
    println!("--- {path} ---");
    match rf_assets::load_gltf(path) {
        Ok(scene) => {
            println!("meshes: {}", scene.meshes.len());
            println!("materials: {}", scene.materials.len());
        }
        Err(err) => println!("FAILED: {err:#}"),
    }
}

fn main() {
    check("assets/models/rifle.glb");
    check("assets/models/dancing_cat.glb");
}
