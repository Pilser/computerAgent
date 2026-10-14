//! Mermaid diagrams -> SVG bytes via `merman` (pure Rust).
//!
//! [`render_mermaid`] is total: any parse/layout failure yields `None` and
//! the chat view falls back to a code block. [`render_cached`] memoizes SVG
//! bytes by source hash in a process-wide map so re-opened threads and
//! repeated assistant messages never re-render the same diagram.
//!
//! NOTE: `merman` bundles the ELK layout engine (EPL-2.0); that license
//! notice and source provenance are inherited by distributed artifacts.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};

/// SVG bytes for `source`, or `None` when it is blank or fails to render.
pub fn render_mermaid(source: &str) -> Option<Vec<u8>> {
    if source.trim().is_empty() {
        return None;
    }
    let output = Renderer::new()
        .render(RenderRequest::svg(
            source,
            OperationControl::new(),
            SvgRequest::default(),
        ))
        .ok()?;
    match output {
        RenderOutput::Svg(Some(svg)) => Some(svg.svg().as_bytes().to_vec()),
        _ => None,
    }
}

fn cache() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    static SVG_CACHE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
    SVG_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(source: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// `bytes://` URI for the cached SVG (stable per source; the svg loader
/// keys on the `.svg` extension).
pub fn image_uri(source: &str) -> String {
    format!("bytes://diagram-{}.svg", cache_key(source))
}

/// [`render_mermaid`] memoized by source hash (repeat calls skip re-render).
pub fn render_cached(source: &str) -> Option<Vec<u8>> {
    let key = cache_key(source);
    if let Some(hit) = cache().lock().expect("svg cache lock").get(&key) {
        return Some(hit.clone());
    }
    let svg = render_mermaid(source)?;
    cache()
        .lock()
        .expect("svg cache lock")
        .insert(key, svg.clone());
    Some(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOW: &str = "flowchart TD\nA[Start] --> B[Done]\n";

    #[test]
    fn blank_source_renders_nothing() {
        assert_eq!(render_mermaid(""), None);
        assert_eq!(render_mermaid("   \n"), None);
    }

    #[test]
    fn flowchart_renders_svg() {
        let svg = render_mermaid(FLOW).expect("flowchart renders");
        assert!(!svg.is_empty());
        assert!(svg.starts_with(b"<svg"));
    }

    #[test]
    fn prose_is_not_a_diagram() {
        assert_eq!(render_mermaid("hello world, just chatting"), None);
    }

    #[test]
    fn cache_repeats_bytes_without_rerender() {
        let first = render_cached(FLOW).expect("flowchart renders");
        let second = render_cached(FLOW).expect("cached repeat");
        assert_eq!(first, second);
        assert!(cache().lock().expect("lock").contains_key(&cache_key(FLOW)));
    }

    #[test]
    fn uri_is_bytes_scheme_with_svg_extension() {
        let uri = image_uri(FLOW);
        assert!(uri.starts_with("bytes://diagram-"));
        assert!(uri.ends_with(".svg"));
    }
}
