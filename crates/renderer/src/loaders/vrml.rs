//! Static VRML97 -> XML X3D, then the existing bounded X3D/Assimp path.
//! Grammar: https://www.web3d.org/documents/specifications/14772/V2.0/part1/grammar.html
//! No scene scripts, prototypes, external scenes, or network resources are run.
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{bail, ensure, Context, Result};

use crate::MAX_MODEL_BYTES;

pub(super) fn normalize(path: &Path) -> Result<Vec<u8>> {
    let source = std::fs::read_to_string(path).context("VRML requires UTF-8 text")?;
    let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
    let (header, body) = source.split_once(['\r', '\n']).unwrap_or((source, ""));
    let mut header = header.split_whitespace();
    ensure!(
        header.next() == Some("#VRML")
            && header.next() == Some("V2.0")
            && header.next() == Some("utf8"),
        "only VRML 2.0 / VRML97 is supported"
    );
    let mut parser = Parser {
        lexer: Lexer {
            source: body,
            offset: 0,
        },
        lookahead: None,
        definitions: HashMap::new(),
        nodes: 0,
        value_bytes: 0,
    };
    let mut roots = Vec::new();
    while !matches!(parser.peek()?, Token::End) {
        if let Some(node) = parser.node(0)? {
            ensure!(
                child_allowed("children", node.kind),
                "invalid VRML root node"
            );
            roots.push(node);
        }
    }
    let mut xml = String::from("<X3D><Scene>");
    for node in &roots {
        node.write(&mut xml)?;
    }
    xml.push_str("</Scene></X3D>");
    super::x3d::normalize_xml(path, &xml)
}

#[derive(Debug)]
enum Token<'a> {
    Word(&'a str),
    String(String),
    Symbol(char),
    End,
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Lexer<'a> {
    fn next(&mut self) -> Result<Token<'a>> {
        let bytes = self.source.as_bytes();
        while self.offset < bytes.len() {
            match bytes[self.offset] {
                b' ' | b'\t' | b'\r' | b'\n' | b',' => self.offset += 1,
                b'#' => {
                    while self.offset < bytes.len() && !matches!(bytes[self.offset], b'\r' | b'\n')
                    {
                        self.offset += 1;
                    }
                }
                _ => break,
            }
        }
        if self.offset == bytes.len() {
            return Ok(Token::End);
        }
        let start = self.offset;
        let first = bytes[start];
        self.offset += 1;
        if matches!(first, b'{' | b'}' | b'[' | b']') {
            return Ok(Token::Symbol(first as char));
        }
        if first == b'"' {
            let mut value = String::new();
            while self.offset < bytes.len() {
                let c = self.source[self.offset..].chars().next().unwrap();
                self.offset += c.len_utf8();
                if c == '"' {
                    return Ok(Token::String(value));
                }
                if c == '\\' {
                    let escaped = self.source[self.offset..]
                        .chars()
                        .next()
                        .context("unterminated VRML escape")?;
                    ensure!(matches!(escaped, '"' | '\\'), "invalid VRML string escape");
                    value.push(escaped);
                    self.offset += escaped.len_utf8();
                } else {
                    value.push(c);
                }
            }
            bail!("unterminated VRML string");
        }
        while self.offset < bytes.len()
            && !matches!(
                bytes[self.offset],
                b' ' | b'\t' | b'\r' | b'\n' | b',' | b'#' | b'"' | b'{' | b'}' | b'[' | b']'
            )
        {
            self.offset += 1;
        }
        Ok(Token::Word(&self.source[start..self.offset]))
    }
}

struct Node<'a> {
    kind: &'a str,
    attributes: Vec<(&'a str, String)>,
    children: Vec<Node<'a>>,
}

impl Node<'_> {
    fn write(&self, out: &mut String) -> Result<()> {
        // Explorer supplies camera, lighting and background. These nodes carry
        // no geometry; parse them for correctness but do not pass them to Assimp.
        if matches!(
            self.kind,
            "WorldInfo"
                | "NavigationInfo"
                | "Viewpoint"
                | "Background"
                | "DirectionalLight"
                | "PointLight"
                | "SpotLight"
        ) {
            return Ok(());
        }
        out.push('<');
        out.push_str(self.kind);
        for (name, value) in &self.attributes {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            super::x3d::escape(out, value)?;
            out.push('"');
        }
        out.push('>');
        for child in &self.children {
            child.write(out)?;
        }
        out.push_str("</");
        out.push_str(self.kind);
        out.push('>');
        ensure!(
            out.len() as u64 <= MAX_MODEL_BYTES,
            "expanded VRML exceeds 300 MiB"
        );
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Field {
    Numbers(usize, bool, bool),
    Bool,
    Text(bool),
    Node(bool),
}

// Arity and node-field types are required by the VRML grammar; guessing where
// a field ends would silently misinterpret valid single-value MF fields.
fn field(kind: &str, name: &str) -> Option<Field> {
    use Field::*;
    Some(match (kind, name) {
        ("Group" | "Transform", "children") => Node(true),
        ("Group" | "Transform", "bboxCenter" | "bboxSize") => Numbers(3, false, false),
        ("Transform", "translation" | "scale" | "center") => Numbers(3, false, false),
        ("Transform", "rotation" | "scaleOrientation") => Numbers(4, false, false),
        ("Shape", "appearance" | "geometry") => Node(false),
        ("Appearance", "material" | "texture" | "textureTransform") => Node(false),
        ("Material", "diffuseColor" | "emissiveColor" | "specularColor") => {
            Numbers(3, false, false)
        }
        ("Material", "ambientIntensity" | "shininess" | "transparency") => Numbers(1, false, false),
        ("ImageTexture", "url") => Text(true),
        ("ImageTexture", "repeatS" | "repeatT") => Bool,
        ("TextureTransform", "translation" | "scale" | "center") => Numbers(2, false, false),
        ("TextureTransform", "rotation") => Numbers(1, false, false),
        ("IndexedFaceSet", "coord" | "normal" | "color" | "texCoord") => Node(false),
        ("IndexedFaceSet", "coordIndex" | "normalIndex" | "colorIndex" | "texCoordIndex") => {
            Numbers(1, true, true)
        }
        ("IndexedFaceSet", "ccw" | "solid" | "convex" | "colorPerVertex" | "normalPerVertex") => {
            Bool
        }
        ("IndexedFaceSet", "creaseAngle") => Numbers(1, false, false),
        ("Coordinate", "point") | ("Normal", "vector") | ("Color", "color") => {
            Numbers(3, true, false)
        }
        ("TextureCoordinate", "point") => Numbers(2, true, false),
        ("Box", "size") => Numbers(3, false, false),
        ("Sphere", "radius")
        | ("Cylinder", "radius" | "height")
        | ("Cone", "bottomRadius" | "height") => Numbers(1, false, false),
        ("Cylinder", "bottom" | "top" | "side") | ("Cone", "bottom" | "side") => Bool,
        ("WorldInfo", "title") | ("Viewpoint", "description") => Text(false),
        ("WorldInfo", "info") | ("NavigationInfo", "type") => Text(true),
        ("NavigationInfo", "avatarSize") | ("Background", "groundAngle" | "skyAngle") => {
            Numbers(1, true, false)
        }
        ("NavigationInfo", "headlight") | ("Viewpoint", "jump") => Bool,
        ("NavigationInfo", "speed" | "visibilityLimit") | ("Viewpoint", "fieldOfView") => {
            Numbers(1, false, false)
        }
        ("Viewpoint", "position") => Numbers(3, false, false),
        ("Viewpoint", "orientation") => Numbers(4, false, false),
        ("Background", "groundColor" | "skyColor") => Numbers(3, true, false),
        (
            "Background",
            "backUrl" | "bottomUrl" | "frontUrl" | "leftUrl" | "rightUrl" | "topUrl",
        ) => Text(true),
        ("DirectionalLight" | "PointLight" | "SpotLight", "on") => Bool,
        ("DirectionalLight" | "PointLight" | "SpotLight", "color")
        | ("DirectionalLight" | "SpotLight", "direction")
        | ("PointLight" | "SpotLight", "location" | "attenuation") => Numbers(3, false, false),
        ("DirectionalLight" | "PointLight" | "SpotLight", "ambientIntensity" | "intensity")
        | ("PointLight" | "SpotLight", "radius")
        | ("SpotLight", "beamWidth" | "cutOffAngle") => Numbers(1, false, false),
        _ => return None,
    })
}

fn child_allowed(field: &str, kind: &str) -> bool {
    match field {
        "children" => matches!(
            kind,
            "Group"
                | "Transform"
                | "Shape"
                | "WorldInfo"
                | "NavigationInfo"
                | "Viewpoint"
                | "Background"
                | "DirectionalLight"
                | "PointLight"
                | "SpotLight"
        ),
        "appearance" => kind == "Appearance",
        "geometry" => matches!(
            kind,
            "IndexedFaceSet" | "Box" | "Sphere" | "Cylinder" | "Cone"
        ),
        "material" => kind == "Material",
        "texture" => kind == "ImageTexture",
        "textureTransform" => kind == "TextureTransform",
        "coord" => kind == "Coordinate",
        "normal" => kind == "Normal",
        "color" => kind == "Color",
        "texCoord" => kind == "TextureCoordinate",
        _ => false,
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    lookahead: Option<Token<'a>>,
    definitions: HashMap<&'a str, (&'a str, String)>,
    nodes: usize,
    value_bytes: usize,
}

impl<'a> Parser<'a> {
    fn peek(&mut self) -> Result<&Token<'a>> {
        if self.lookahead.is_none() {
            self.lookahead = Some(self.lexer.next()?);
        }
        Ok(self.lookahead.as_ref().unwrap())
    }
    fn next(&mut self) -> Result<Token<'a>> {
        match self.lookahead.take() {
            Some(token) => Ok(token),
            None => self.lexer.next(),
        }
    }
    fn symbol(&mut self, expected: char) -> Result<()> {
        ensure!(
            matches!(self.next()?, Token::Symbol(c) if c == expected),
            "expected VRML '{expected}'"
        );
        Ok(())
    }
    fn word(&mut self) -> Result<&'a str> {
        match self.next()? {
            Token::Word(w) => Ok(w),
            other => bail!("expected VRML word, got {other:?}"),
        }
    }
    fn identifier(&mut self) -> Result<&'a str> {
        let name = self.word()?;
        ensure!(
            !name.is_empty()
                && !name.starts_with(|c: char| c.is_ascii_digit() || "+-.".contains(c))
                && !name.chars().any(|c| c.is_control() || "'\\.".contains(c)),
            "invalid VRML identifier"
        );
        Ok(name)
    }
    fn node(&mut self, depth: usize) -> Result<Option<Node<'a>>> {
        self.nodes += 1;
        ensure!(
            depth < 64 && self.nodes <= 100_000,
            "VRML scene complexity limit exceeded"
        );
        let mut kind = self.word()?;
        if kind == "NULL" {
            return Ok(None);
        }
        if kind == "USE" {
            let name = self.identifier()?;
            let (kind, id) = self
                .definitions
                .get(name)
                .context("missing VRML DEF reference")?;
            return Ok(Some(Node {
                kind,
                attributes: vec![("USE", id.clone())],
                children: vec![],
            }));
        }
        let mut attributes = Vec::new();
        if kind == "DEF" {
            let name = self.identifier()?;
            kind = self.identifier()?;
            // VRML allows a DEF name to be rebound. Give each declaration a
            // unique XML name so earlier USEs still refer to the earlier node.
            let id = format!("vrml{}", self.nodes);
            self.definitions.insert(name, (kind, id.clone()));
            attributes.push(("DEF", id));
        }
        ensure!(
            child_allowed("children", kind)
                || [
                    "appearance",
                    "geometry",
                    "material",
                    "texture",
                    "textureTransform",
                    "coord",
                    "normal",
                    "color",
                    "texCoord"
                ]
                .iter()
                .any(|f| child_allowed(f, kind)),
            "unsupported VRML node or statement: {kind}"
        );
        self.symbol('{')?;
        let mut children = Vec::new();
        let mut fields = HashSet::new();
        while !matches!(self.peek()?, Token::Symbol('}')) {
            let name = self.identifier()?;
            ensure!(fields.insert(name), "duplicate VRML field {kind}.{name}");
            let ty = field(kind, name)
                .with_context(|| format!("unsupported VRML field {kind}.{name}"))?;
            let multi = matches!(
                ty,
                Field::Numbers(_, true, _) | Field::Text(true) | Field::Node(true)
            );
            let bracketed = multi && matches!(self.peek()?, Token::Symbol('['));
            if bracketed {
                self.symbol('[')?;
            }
            let mut value = String::new();
            loop {
                if bracketed && matches!(self.peek()?, Token::Symbol(']')) {
                    self.symbol(']')?;
                    break;
                }
                if let Field::Node(_) = ty {
                    if let Some(child) = self.node(depth + 1)? {
                        ensure!(
                            child_allowed(name, child.kind),
                            "invalid node in VRML {kind}.{name}"
                        );
                        children.push(child);
                    }
                } else {
                    let item = self.value(ty)?;
                    self.value_bytes += item.len() + 1;
                    ensure!(
                        self.value_bytes as u64 <= MAX_MODEL_BYTES,
                        "expanded VRML exceeds 300 MiB"
                    );
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(&item);
                }
                if !bracketed {
                    break;
                }
            }
            if !matches!(ty, Field::Node(_)) {
                attributes.push((name, value));
            }
        }
        self.symbol('}')?;
        if matches!(kind, "Sphere" | "Cylinder" | "Cone") {
            for (name, value) in &attributes {
                if matches!(*name, "radius" | "height" | "bottomRadius") {
                    ensure!(
                        value.parse::<f32>()? > 0.0,
                        "VRML primitive dimensions must be positive"
                    );
                }
            }
        }
        Ok(Some(Node {
            kind,
            attributes,
            children,
        }))
    }
    fn value(&mut self, ty: Field) -> Result<String> {
        match ty {
            Field::Bool => match self.word()? {
                "TRUE" => Ok("true".into()),
                "FALSE" => Ok("false".into()),
                _ => bail!("invalid VRML boolean"),
            },
            Field::Text(multi) => {
                let Token::String(text) = self.next()? else {
                    bail!("expected quoted VRML string");
                };
                if multi {
                    Ok(format!(
                        "\"{}\"",
                        text.replace('\\', "\\\\").replace('"', "\\\"")
                    ))
                } else {
                    Ok(text)
                }
            }
            Field::Numbers(arity, _, integer) => {
                let mut values = Vec::with_capacity(arity);
                for _ in 0..arity {
                    let text = self.word()?;
                    if integer {
                        let digits = text.trim_start_matches(['+', '-']);
                        let value = if let Some(hex) = digits
                            .strip_prefix("0x")
                            .or_else(|| digits.strip_prefix("0X"))
                        {
                            let magnitude = i64::from_str_radix(hex, 16)?;
                            i32::try_from(if text.starts_with('-') {
                                -magnitude
                            } else {
                                magnitude
                            })?
                        } else {
                            text.parse::<i32>()?
                        };
                        values.push(value.to_string());
                    } else {
                        let value: f32 = text.parse()?;
                        ensure!(value.is_finite(), "non-finite VRML number");
                        values.push(value.to_string());
                    }
                }
                Ok(values.join(" "))
            }
            Field::Node(_) => unreachable!(),
        }
    }
}
