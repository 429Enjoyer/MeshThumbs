//! Expand static DEF/USE references and normalize ImageTexture URL lists before
//! handing XML geometry to Assimp. No scripts or remote resources are executed.
use std::{collections::HashMap, path::Path};

use anyhow::{ensure, Context, Result};
use roxmltree::{Document, Node, NodeId, ParsingOptions};

use crate::MAX_MODEL_BYTES;

pub(super) fn normalize(path: &Path) -> Result<Vec<u8>> {
    let xml = std::fs::read_to_string(path).context("X3D requires XML text")?;
    normalize_xml(path, &xml)
}

pub(super) fn normalize_xml(path: &Path, xml: &str) -> Result<Vec<u8>> {
    // External DTDs are not fetched. Published X3D examples include a DOCTYPE;
    // roxmltree limits entity expansion, and we also bound nodes and output.
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: true,
            nodes_limit: 1_000_000,
            ..Default::default()
        },
    )?;
    let root = document.root_element();
    ensure!(root.has_tag_name("X3D"), "expected an XML X3D document");
    let mut definitions = HashMap::new();
    for node in root.descendants().filter(Node::is_element) {
        ensure!(
            !matches!(
                node.tag_name().name(),
                "Inline" | "ProtoDeclare" | "ExternProtoDeclare" | "ProtoInstance"
            ),
            "X3D Inline scenes and prototypes are unsupported"
        );
        if let Some(name) = node.attribute("DEF") {
            ensure!(
                definitions.insert(name, node).is_none(),
                "duplicate X3D DEF name"
            );
        }
    }
    let mut output = String::new();
    let mut count = 0;
    write_node(
        root,
        &definitions,
        path.parent().unwrap_or(Path::new(".")),
        &mut Vec::new(),
        &mut count,
        &mut output,
    )?;
    ensure!(
        output.contains("<Shape"),
        "scene contains no shapes to render"
    );
    Ok(output.into_bytes())
}

fn write_node<'a, 'input>(
    mut node: Node<'a, 'input>,
    definitions: &HashMap<&str, Node<'a, 'input>>,
    parent: &Path,
    ancestors: &mut Vec<NodeId>,
    count: &mut usize,
    output: &mut String,
) -> Result<()> {
    if let Some(reference) = node.attribute("USE") {
        let original = definitions
            .get(reference)
            .context("missing X3D DEF reference")?;
        ensure!(
            original.tag_name() == node.tag_name(),
            "X3D USE type does not match DEF"
        );
        node = *original;
    }
    *count += 1;
    ensure!(
        *count <= 100_000 && ancestors.len() < 128,
        "X3D scene expansion limit exceeded"
    );
    ensure!(!ancestors.contains(&node.id()), "cyclic X3D USE reference");
    ancestors.push(node.id());
    let name = node.tag_name().name();
    if name == "IndexedFaceSet" {
        validate_faces(node, definitions)?;
    }
    if name == "Box" {
        // Assimp's generated box mapping is a stub. Supply explicit per-face
        // UVs so a textured X3D Box does not collapse to a single texel.
        write_box(node, output)?;
        ancestors.pop();
        return Ok(());
    }
    let start = output.len();
    output.push('<');
    output.push_str(name);
    for attribute in node.attributes() {
        if attribute.namespace().is_some() || matches!(attribute.name(), "DEF" | "USE") {
            continue;
        }
        let value = if name == "ImageTexture" && attribute.name() == "url" {
            texture_url(attribute.value(), parent)?
        } else {
            attribute.value().to_owned()
        };
        output.push(' ');
        output.push_str(attribute.name());
        output.push_str("=\"");
        escape(output, &value)?;
        output.push('"');
    }
    output.push('>');
    let children_start = output.len();
    for child in node.children().filter(Node::is_element) {
        write_node(child, definitions, parent, ancestors, count, output)?;
    }
    // Assimp's X3D importer can dereference an empty group's child list.
    // Empty grouping nodes contribute no geometry and can safely disappear.
    if matches!(name, "Group" | "Transform") && output.len() == children_start {
        output.truncate(start);
        ancestors.pop();
        return Ok(());
    }
    output.push_str("</");
    output.push_str(name);
    output.push('>');
    ensure!(
        output.len() as u64 <= MAX_MODEL_BYTES,
        "expanded X3D exceeds 300 MiB"
    );
    ancestors.pop();
    Ok(())
}

fn validate_faces<'a, 'input>(
    node: Node<'a, 'input>,
    definitions: &HashMap<&str, Node<'a, 'input>>,
) -> Result<()> {
    // Validate before native triangulation: Assimp's post-import validation
    // runs too late to protect against malformed coordinate/attribute indices.
    let child = |kind: &str| -> Result<Option<Node<'a, 'input>>> {
        let Some(child) = node.children().find(|n| n.has_tag_name(kind)) else {
            return Ok(None);
        };
        let child = if let Some(id) = child.attribute("USE") {
            *definitions.get(id).context("missing X3D DEF reference")?
        } else {
            child
        };
        ensure!(child.has_tag_name(kind), "X3D USE type does not match DEF");
        Ok(Some(child))
    };
    let coordinates = child("Coordinate")?.context("IndexedFaceSet requires Coordinate")?;
    let points = tuple_count(coordinates.attribute("point").unwrap_or(""), 3)?;
    let indices = index_list(node.attribute("coordIndex").unwrap_or(""))?;
    let mut face_sizes = Vec::new();
    let mut size = 0;
    for &index in &indices {
        if index == -1 {
            if size > 0 {
                ensure!(
                    size >= 3,
                    "IndexedFaceSet face has fewer than three vertices"
                );
                face_sizes.push(size);
                size = 0;
            }
        } else {
            ensure!(
                index >= 0 && (index as usize) < points,
                "coordinate index out of range"
            );
            size += 1;
        }
    }
    if size > 0 {
        ensure!(
            size >= 3,
            "IndexedFaceSet face has fewer than three vertices"
        );
        face_sizes.push(size);
    }
    ensure!(!face_sizes.is_empty(), "IndexedFaceSet contains no faces");
    for (kind, value, arity, index_name, per_vertex) in [
        (
            "Normal",
            "vector",
            3,
            "normalIndex",
            node.attribute("normalPerVertex") != Some("false"),
        ),
        (
            "Color",
            "color",
            3,
            "colorIndex",
            node.attribute("colorPerVertex") != Some("false"),
        ),
        ("TextureCoordinate", "point", 2, "texCoordIndex", true),
    ] {
        let attribute_indices = index_list(node.attribute(index_name).unwrap_or(""))?;
        let Some(attribute) = child(kind)? else {
            ensure!(attribute_indices.is_empty(), "indices without {kind}");
            continue;
        };
        let count = tuple_count(attribute.attribute(value).unwrap_or(""), arity)?;
        if attribute_indices.is_empty() {
            if per_vertex {
                ensure!(
                    indices.iter().all(|&i| i == -1 || (i as usize) < count),
                    "{kind} index out of range"
                );
            } else {
                ensure!(count >= face_sizes.len(), "too few {kind} values for faces");
            }
        } else if per_vertex {
            let mut sizes = Vec::new();
            let mut size = 0;
            for index in attribute_indices {
                if index == -1 {
                    if size > 0 {
                        sizes.push(size);
                        size = 0;
                    }
                } else {
                    ensure!(
                        index >= 0 && (index as usize) < count,
                        "{kind} index out of range"
                    );
                    size += 1;
                }
            }
            if size > 0 {
                sizes.push(size);
            }
            ensure!(
                sizes == face_sizes,
                "{kind} indices do not match coordinate faces"
            );
        } else {
            ensure!(
                attribute_indices.len() == face_sizes.len()
                    && attribute_indices
                        .iter()
                        .all(|&i| i >= 0 && (i as usize) < count),
                "invalid per-face {kind} indices"
            );
        }
    }
    Ok(())
}

fn tuple_count(value: &str, arity: usize) -> Result<usize> {
    let mut count = 0;
    for token in value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
    {
        ensure!(
            token.parse::<f32>()?.is_finite(),
            "non-finite geometry value"
        );
        count += 1;
    }
    ensure!(count % arity == 0, "incomplete geometry tuple");
    Ok(count / arity)
}

fn index_list(value: &str) -> Result<Vec<i32>> {
    value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<i32>().map_err(Into::into))
        .collect()
}

fn write_box(node: Node<'_, '_>, output: &mut String) -> Result<()> {
    use std::fmt::Write;
    let size = node
        .attribute("size")
        .unwrap_or("2 2 2")
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()?;
    ensure!(
        size.len() == 3 && size.iter().all(|v| v.is_finite() && *v > 0.0),
        "invalid X3D Box size"
    );
    let [x, y, z] = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
    output.push_str("<IndexedFaceSet coordIndex=\"4 5 6 7 -1 1 0 3 2 -1 5 1 2 6 -1 0 4 7 3 -1 7 6 2 3 -1 0 1 5 4 -1\" texCoordIndex=\"0 1 2 3 -1 0 1 2 3 -1 0 1 2 3 -1 0 1 2 3 -1 0 1 2 3 -1 0 1 2 3 -1\" solid=\"");
    escape(output, node.attribute("solid").unwrap_or("true"))?;
    output.push_str("\"><Coordinate point=\"");
    for [a, b, c] in [
        [-x, -y, -z],
        [x, -y, -z],
        [x, y, -z],
        [-x, y, -z],
        [-x, -y, z],
        [x, -y, z],
        [x, y, z],
        [-x, y, z],
    ] {
        write!(output, "{a} {b} {c} ")?;
    }
    output.push_str("\"/><TextureCoordinate point=\"0 0 1 0 1 1 0 1\"/></IndexedFaceSet>");
    ensure!(
        output.len() as u64 <= MAX_MODEL_BYTES,
        "expanded X3D exceeds 300 MiB"
    );
    Ok(())
}

pub(super) fn escape(output: &mut String, value: &str) -> Result<()> {
    for c in value.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(c),
        }
        ensure!(
            output.len() as u64 <= MAX_MODEL_BYTES,
            "expanded X3D exceeds 300 MiB"
        );
    }
    Ok(())
}

fn texture_url(value: &str, parent: &Path) -> Result<String> {
    let mut chars = value.chars().peekable();
    let mut candidates = Vec::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        let mut token = String::new();
        if c == '"' {
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    token.push(chars.next().context("invalid X3D URL escape")?);
                } else {
                    token.push(c);
                }
            }
            ensure!(closed, "unterminated X3D texture URL");
        } else {
            token.push(c);
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                token.push(chars.next().unwrap());
            }
        }
        if !token.contains("://") && !token.starts_with("data:") {
            candidates.push(token);
        }
    }
    let chosen = candidates
        .iter()
        .find(|candidate| {
            let decoded =
                urlencoding::decode(candidate).unwrap_or_else(|_| candidate.as_str().into());
            parent.join(decoded.replace('\\', "/")).is_file()
        })
        .or_else(|| candidates.first())
        .cloned()
        .unwrap_or_default();
    // A single unquoted URI token avoids the native MFString parser's quoted-
    // string bug. Texture loading later decodes percent escapes in local paths.
    Ok(chosen
        .replace(' ', "%20")
        .replace('\t', "%09")
        .replace('\n', "%0A")
        .replace('\r', "%0D"))
}
