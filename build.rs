// build.rs performs tasks before the actual compilation
fn main() {
    println!("---- Execute build.rs for .proto compiling ----");
    // tonic_build::configure()
    //     .type_attribute("routaeguide.Point", "#[derive(Hash)]") // Automatically derive Hash for Point
    //     .compile_protos(&["proto/route_guide.proto"], &["proto"])
    //     .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));

}



