//! Build script — registers the GitHub GraphQL schema with cynic so that all
//! `#[derive(cynic::*)]` macros in this crate can validate against it without
//! each having to repeat the `schema_path` attribute.

fn main() {
    cynic_codegen::register_schema("github")
        .from_sdl_file("schemas/github.graphql")
        .expect("schemas/github.graphql must be valid SDL")
        .as_default()
        .expect("failed to register github schema as default");
}
