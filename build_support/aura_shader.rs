//! One effect source feeds both Glow and Wgpu; no runtime GLSL compiler is shipped.
pub fn compile() {
    const PATH: &str = "src/app/ui/main_tab_music_aura_fragment.glsl";
    println!("cargo:rerun-if-changed={PATH}");
    println!("cargo:rerun-if-changed=build_support/aura_shader.rs");
    let original = std::fs::read_to_string(PATH).expect("read Aura GLSL");
    let (_, rest) = original
        .split_once("uniform vec3 u_size_radius;")
        .expect("Aura uniforms");
    let (uniform_tail, body) = rest.split_once("const float PI").expect("Aura effect body");
    let uniforms = format!(
        "vec3 u_size_radius;{}",
        uniform_tail.replace("uniform ", "")
    );
    for srgb in [false, true] {
        let conversion = if srgb {
            "out_color.rgb = mix(out_color.rgb / 12.92, pow((out_color.rgb + 0.055) / 1.055, vec3(2.4)), greaterThan(out_color.rgb, vec3(0.04045)));"
        } else {
            ""
        };
        let source = format!(
            "#version 450\nlayout(location=0) in vec2 v_uv;\nlayout(location=0) out vec4 out_color;\nlayout(set=0,binding=0,std140) uniform Aura {{ {uniforms} }};\n#define main aura_main\nconst float PI{body}\n#undef main\nvoid main() {{ aura_main(); {conversion} }}\n"
        );
        let module = naga::front::glsl::Frontend::default()
            .parse(
                &naga::front::glsl::Options::from(naga::ShaderStage::Fragment),
                &source,
            )
            .unwrap_or_else(|error| panic!("Aura GLSL parse failed: {error:?}"));
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("validate Aura shader");
        let uniform = module
            .global_variables
            .iter()
            .find(|(_, variable)| variable.space == naga::AddressSpace::Uniform)
            .expect("Aura uniform block")
            .1;
        let naga::TypeInner::Struct { members, span } = &module.types[uniform.ty].inner else {
            panic!("Aura uniforms must be a struct");
        };
        assert_eq!(*span, 336, "Aura uniform byte contract changed");
        assert_eq!(members.len(), 21, "Aura uniform group count changed");
        for (index, member) in members.iter().enumerate() {
            assert_eq!(
                member.offset,
                index as u32 * 16,
                "Aura std140 offset changed"
            );
        }
        let wgsl = naga::back::wgsl::write_string(
            &module,
            &info,
            naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
        )
        .expect("write Aura WGSL");
        let name = if srgb { "aura_srgb.wgsl" } else { "aura.wgsl" };
        let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join(name);
        std::fs::write(path, wgsl).expect("write generated Aura shader");
    }
}
