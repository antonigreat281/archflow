use crate::scene::{SceneElement, SceneGraph};

/// Extract viewBox value from an SVG string, e.g. `viewBox="0 0 64 64"` → `"0 0 64 64"`
fn extract_viewbox(svg: &str) -> Option<String> {
    let lower = svg.to_lowercase();
    let idx = lower.find("viewbox=\"")?;
    let start = idx + 9; // length of `viewbox="`
    let rest = &svg[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Strip outer `<svg ...>` and `</svg>` wrapper, returning only inner content.
fn strip_svg_wrapper(svg: &str) -> String {
    let s = svg.trim();
    // Remove XML declaration if present
    let s = if s.starts_with("<?xml") {
        match s.find("?>") {
            Some(i) => s[i + 2..].trim(),
            None => s,
        }
    } else {
        s
    };
    // Remove opening <svg ...> tag
    let s = if let Some(idx) = s.find('>') {
        if s[..idx].contains("<svg") || s[..idx].contains("<SVG") {
            &s[idx + 1..]
        } else {
            s
        }
    } else {
        return s.to_string();
    };
    // Remove closing </svg> tag
    let s = s.trim();
    let s = if s.ends_with("</svg>") || s.ends_with("</SVG>") {
        &s[..s.len() - 6]
    } else {
        s
    };
    s.trim().to_string()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn render_element(buf: &mut String, element: &SceneElement, indent: usize) {
    let pad = "  ".repeat(indent);
    match element {
        SceneElement::Rect {
            x,
            y,
            width,
            height,
            rx,
            fill,
            stroke,
            stroke_width,
            stroke_dasharray,
            shadow,
        } => {
            let filter = if *shadow {
                " filter=\"url(#shadow)\""
            } else {
                ""
            };
            let dash = stroke_dasharray
                .as_ref()
                .map(|da| format!(" stroke-dasharray=\"{da}\""))
                .unwrap_or_default();
            let stroke_attr = if stroke == "none" {
                String::new()
            } else {
                format!(" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}")
            };
            buf.push_str(&format!(
                "{pad}<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" \
                 rx=\"{rx}\" fill=\"{fill}\"{stroke_attr}{filter}/>\n"
            ));
        }
        SceneElement::Text {
            x,
            y,
            content,
            font_size,
            font_family,
            fill,
            anchor,
            font_weight,
        } => {
            buf.push_str(&format!(
                "{pad}<text x=\"{x}\" y=\"{y}\" font-size=\"{font_size}\" \
                 font-family=\"{font_family}\" fill=\"{fill}\" font-weight=\"{font_weight}\" \
                 text-anchor=\"{anchor}\" dominant-baseline=\"middle\">{}</text>\n",
                escape_xml(content)
            ));
        }
        SceneElement::Path {
            d,
            stroke,
            stroke_width,
            stroke_dasharray,
            marker_end,
        } => {
            let dash = stroke_dasharray
                .as_ref()
                .map(|da| format!(" stroke-dasharray=\"{da}\""))
                .unwrap_or_default();
            let marker = if *marker_end {
                " marker-end=\"url(#arrowhead)\""
            } else {
                ""
            };
            buf.push_str(&format!(
                "{pad}<path d=\"{d}\" fill=\"none\" \
                 stroke=\"{stroke}\" stroke-width=\"{stroke_width}\" \
                 stroke-linecap=\"round\"{dash}{marker}/>\n"
            ));
        }
        SceneElement::RawSvg {
            x,
            y,
            width,
            height,
            content,
        } => {
            // Extract viewBox from inner SVG if present, otherwise use width/height
            let inner_vb = extract_viewbox(content);
            let vb = inner_vb.unwrap_or_else(|| format!("0 0 {width} {height}"));
            // Strip outer <svg> and </svg> tags from content to avoid nested svg issues
            let inner = strip_svg_wrapper(content);
            buf.push_str(&format!(
                "{pad}<svg x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" \
                 viewBox=\"{vb}\" preserveAspectRatio=\"xMidYMid meet\">\n"
            ));
            buf.push_str(&inner);
            buf.push('\n');
            buf.push_str(&format!("{pad}</svg>\n"));
        }
        SceneElement::Group { id, children } => {
            buf.push_str(&format!("{pad}<g id=\"{id}\">\n"));
            for child in children {
                render_element(buf, child, indent + 1);
            }
            buf.push_str(&format!("{pad}</g>\n"));
        }
    }
}

pub fn render_svg(scene: &SceneGraph) -> String {
    let mut buf = String::with_capacity(8192);

    buf.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"##,
        scene.width, scene.height, scene.width, scene.height
    ));
    buf.push('\n');

    // Defs
    buf.push_str(&format!(
        r##"  <defs>
    <filter id="shadow" x="-4%" y="-4%" width="108%" height="116%">
      <feDropShadow dx="0" dy="2" stdDeviation="4" flood-color="#000000" flood-opacity="0.08"/>
    </filter>
    <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto" markerUnits="strokeWidth">
      <path d="M0,0 L8,3 L0,6 L2,3 Z" fill="{}"/>
    </marker>
  </defs>
"##,
        scene.edge_color
    ));

    // Background
    buf.push_str(&format!(
        "  <rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n",
        scene.background
    ));

    // Elements
    for element in &scene.elements {
        render_element(&mut buf, element, 1);
    }

    buf.push_str("</svg>\n");
    buf
}
