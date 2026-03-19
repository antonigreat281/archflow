use napi_derive::napi;

/// Parse an Archflow DSL string and return the JSON IR.
#[napi]
pub fn parse_dsl(dsl: String) -> napi::Result<String> {
    archflow_core::parse_dsl_to_json(&dsl)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Render an Archflow DSL string directly to SVG.
#[napi]
pub fn render_dsl(dsl: String) -> napi::Result<String> {
    archflow_core::render_dsl(&dsl)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Render a JSON IR string to SVG.
#[napi]
pub fn render_svg(json_ir: String) -> napi::Result<String> {
    archflow_core::render_svg(&json_ir)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}
