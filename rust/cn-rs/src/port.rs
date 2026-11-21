//! Clean-room port of `packages/profile-ui/src/cn.ts`: `cx`-style argument
//! flattening plus a Tailwind conflict merge — same variant and same-or-wider
//! utility group, last wins; unknown classes pass through untouched.

use serde_json::Value;

const DISPLAY: &str =
    "flex|inline-flex|grid|block|inline-block|inline|inline-grid|hidden|contents|flow-root|table";
const VALUED: &str = "w|h|min-w|min-h|max-w|max-h|size|basis|grow|shrink|order|z|inset|inset-x|inset-y|top|right|bottom|left|start|end|gap|gap-x|gap-y|col|row|col-span|row-span|text|font|leading|tracking|rounded|border|outline|ring|opacity|scale|translate-x|translate-y|rotate|duration|delay|ease|aspect|object|overflow|overscroll|indent|space-x|space-y|divide-x|divide-y|blur|shadow|cursor|select|resize|snap|scroll-m|scroll-p|scroll-ml|scroll-mt|grayscale|invert|contrast|saturate|hue-rotate|drop-shadow|backdrop-blur|brightness|sepia";

fn truthy(v: &Value) -> bool {
    match crate::js::as_number(v) {
        Some(n) => n != 0.0 && !n.is_nan(),
        None => {
            !crate::js::is_undefined(v)
                && !matches!(v, Value::Bool(false) | Value::Null)
                && !matches!(v, Value::String(s) if s.is_empty())
        }
    }
}

/// Flatten one `CxOptions` element into class-list tokens.
fn cx(v: &Value) -> Vec<String> {
    match (v, crate::js::as_number(v)) {
        (_, Some(n)) if n != 0.0 && !n.is_nan() => vec![crate::js::number_to_string(n)],
        (_, Some(_)) => vec![],
        (Value::String(s), _) if !s.is_empty() => vec![s.clone()],
        (Value::Array(a), _) => a.iter().flat_map(cx).collect(),
        (Value::Object(o), _) if !crate::js::is_undefined(v) => o
            .iter()
            .filter(|(_, v)| truthy(v))
            .map(|(k, _)| k.clone())
            .collect(),
        _ => vec![],
    }
}

/// The conflict group of a utility class, or `None` for classes the merge leaves alone.
fn group_of(b: &str) -> Option<String> {
    let Some((pre, tail)) = b.rsplit_once('-') else {
        return DISPLAY.split('|').any(|d| d == b).then(|| "display".into());
    };
    let num = tail
        .bytes()
        .all(|c| c.is_ascii_digit() || b"./".contains(&c))
        && tail.bytes().any(|c| c.is_ascii_digit());
    if !num {
        return VALUED.split('|').any(|v| v == pre).then(|| pre.to_string());
    }
    let g = pre
        .rsplit_once('-')
        .filter(|(_, s)| {
            tail.len() >= 2 && s.len() > 1 && s.bytes().all(|c| c.is_ascii_alphabetic())
        })
        .map_or_else(|| pre.to_string(), |(p2, _)| format!("{p2}.color"));
    Some(match g.as_str() {
        t if t.starts_with("border-") => format!("border.{g}"),
        t if t.len() == 2 && "pm".contains(&t[..1]) => format!("{}.{}", &t[..1], t),
        _ => g,
    })
}

/// Port of `cn` (`packages/profile-ui/src/cn.ts:8`).
pub fn cn(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    for c in cx(&inputs).iter().flat_map(|t| t.split_whitespace()) {
        let (v, b) = c.rsplit_once(':').unwrap_or(("", c));
        let g = group_of(b).unwrap_or_default();
        let d = format!("{v}\u{1}{g}");
        if !g.is_empty() {
            out.retain(|(k, _)| !(k == &d || k.starts_with(&format!("{d}."))));
        }
        out.push((d, c.into()));
    }
    let s = out.into_iter().map(|e| e.1).collect::<Vec<String>>();
    Ok(s.join(" ").into())
}
