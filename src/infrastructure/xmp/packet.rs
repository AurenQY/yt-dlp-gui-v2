//! Format-independent XMP property editing over a namespace-aware XML tree.
use super::xml::{self, Element, Namespaces, Node, expanded};

pub(crate) const DC: &str = "http://purl.org/dc/elements/1.1/";
pub(crate) const XMP: &str = "http://ns.adobe.com/xap/1.0/";
pub(crate) const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub(crate) const IDQ: &str = "http://ns.adobe.com/xmp/Identifier/qual/1.0/";
pub(crate) const RIGHTS: &str = "http://ns.adobe.com/xap/1.0/rights/";
pub(crate) const EXIF: &str = "http://ns.adobe.com/exif/1.0/";
pub(crate) const IPTC: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";
pub(super) const NOTE: &str = "http://ns.adobe.com/xmp/note/";

#[derive(Clone, Debug)]
pub(crate) struct Packet {
    pub(super) root: Element,
}

impl Packet {
    pub fn new() -> Self {
        let mut root = Element::new("x:xmpmeta");
        root.set_attr("xmlns:x", "adobe:ns:meta/");
        let mut rdf = Element::new("rdf:RDF");
        rdf.set_attr("xmlns:rdf", RDF);
        root.children.push(Node::Element(rdf));
        Self { root }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > super::MAX_PACKET_BYTES {
            return Err("XMP packet exceeds processing limit".into());
        }
        let mut packet = Self {
            root: xml::parse(bytes)?,
        };
        let (rdf, context) =
            rdf_mut(&mut packet.root, &Namespaces::new()).ok_or("Missing XMP rdf:RDF")?;
        let mut subject = None;
        for child in &rdf.children {
            if let Node::Element(description) = child {
                let ns = description.namespaces(&context);
                if !is_description(description, &ns) {
                    return Err("Unsupported XMP RDF node; original left unchanged".into());
                }
                let about = description
                    .attributes
                    .iter()
                    .find(|(key, _)| expanded(key, &ns, true) == Some((RDF, "about")))
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("");
                if subject.is_some_and(|old| old != about) {
                    return Err("Multiple XMP RDF subjects; original left unchanged".into());
                }
                subject = Some(about);
            }
        }
        Ok(packet)
    }
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        let bytes = xml::serialize(&self.root)?;
        if bytes.len() > super::MAX_PACKET_BYTES {
            return Err("XMP packet exceeds processing limit".into());
        }
        Ok(bytes)
    }
    pub fn delete(&mut self, namespace: &str, property: &str) {
        if let Some((rdf, context)) = rdf_mut(&mut self.root, &Namespaces::new()) {
            for child in &mut rdf.children {
                let Node::Element(description) = child else {
                    continue;
                };
                let ns = description.namespaces(&context);
                if !is_description(description, &ns) {
                    continue;
                }
                description
                    .attributes
                    .retain(|(name, _)| expanded(name, &ns, true) != Some((namespace, property)));
                description.children.retain(|node| match node {
                    Node::Element(node) => {
                        expanded(&node.name, &node.namespaces(&ns), false)
                            != Some((namespace, property))
                    }
                    _ => true,
                });
            }
        }
    }
    pub fn simple(&self, namespace: &str, property: &str) -> Option<String> {
        if let Some((node, context)) = self.property(namespace, property) {
            return simple_value(&node, &context);
        }
        self.abbreviated(namespace, property)
    }
    pub fn set_simple(&mut self, namespace: &str, property: &str, value: &str) {
        let mut node = property_node(namespace, property);
        node.children.push(Node::Text(value.into()));
        self.put(namespace, property, node);
    }
    pub fn localized(&self, namespace: &str, property: &str) -> Option<String> {
        let (node, ns) = self.property(namespace, property)?;
        let (alt, context) = child_named(&node, &ns, RDF, "Alt")?;
        for child in &alt.children {
            let Node::Element(item) = child else {
                continue;
            };
            if expanded(&item.name, &item.namespaces(&context), false) == Some((RDF, "li"))
                && item.attr("xml:lang") == Some("x-default")
            {
                return Some(item.text());
            }
        }
        None
    }
    pub fn set_localized(&mut self, namespace: &str, property: &str, value: &str) {
        let mut node = self
            .property(namespace, property)
            .map(|(mut node, ns)| {
                bind_context(&mut node, &ns);
                node
            })
            .unwrap_or_else(|| property_node(namespace, property));
        let ns = node.namespaces(&Namespaces::new());
        let position = node.children.iter().position(|child| matches!(child,Node::Element(child) if expanded(&child.name,&child.namespaces(&ns),false) == Some((RDF,"Alt"))));
        if position.is_none() {
            node.children.clear();
            node.children.push(Node::Element(rdf_node("Alt")));
        }
        let Node::Element(alt) = &mut node.children[position.unwrap_or(0)] else {
            unreachable!()
        };
        let context = alt.namespaces(&ns);
        let previous = alt.children.iter().find_map(|child| match child {
            Node::Element(item) if item.attr("xml:lang") == Some("x-default") => Some(item.text()),
            _ => None,
        });
        let mut found = false;
        for child in &mut alt.children {
            let Node::Element(item) = child else {
                continue;
            };
            if expanded(&item.name, &item.namespaces(&context), false) != Some((RDF, "li")) {
                continue;
            }
            // Keep translations; mirrors of the previous default track its update.
            if item.attr("xml:lang") == Some("x-default")
                || previous.as_ref().is_some_and(|old| item.text() == *old)
            {
                if item.attr("xml:lang") == Some("x-default") {
                    found = true;
                }
                item.children = vec![Node::Text(value.into())];
            }
        }
        if !found {
            let mut item = rdf_node("li");
            item.set_attr("xml:lang", "x-default");
            item.children.push(Node::Text(value.into()));
            alt.children.insert(0, Node::Element(item));
        }
        self.put(namespace, property, node);
    }
    pub fn array(&self, namespace: &str, property: &str) -> Vec<String> {
        self.array_items(namespace, property)
            .into_iter()
            .filter_map(|(item, context)| simple_value(&item, &context))
            .collect()
    }
    pub fn set_array(&mut self, namespace: &str, property: &str, values: &[&str], ordered: bool) {
        let mut node = property_node(namespace, property);
        let mut array = rdf_node(if ordered { "Seq" } else { "Bag" });
        for value in values {
            let mut item = rdf_node("li");
            item.children.push(Node::Text((*value).into()));
            array.children.push(Node::Element(item));
        }
        node.children.push(Node::Element(array));
        self.put(namespace, property, node);
    }
    pub fn has_identifier(&self, value: &str, scheme: &str) -> bool {
        self.array_items(XMP, "Identifier")
            .iter()
            .any(|(item, ns)| {
                simple_value(item, ns).as_deref() == Some(value)
                    && child_named(item, ns, IDQ, "Scheme")
                        .is_some_and(|(node, _)| node.text() == scheme)
            })
    }
    pub fn ensure_identifier(&mut self, value: &str, scheme: &str) {
        if self.has_identifier(value, scheme) {
            return;
        }
        let mut node = property_node(XMP, "Identifier");
        let mut bag = rdf_node("Bag");
        for (mut item, context) in self.array_items(XMP, "Identifier") {
            bind_context(&mut item, &context);
            // Repair an unqualified/misqualified copy rather than duplicate identity.
            if simple_value(&item, &context).as_deref() != Some(value) {
                bag.children.push(Node::Element(item));
            }
        }
        let mut item = rdf_node("li");
        item.set_attr("rdf:parseType", "Resource");
        let mut raw = rdf_node("value");
        raw.children.push(Node::Text(value.into()));
        let mut qualifier = property_node(IDQ, "Scheme");
        qualifier.children.push(Node::Text(scheme.into()));
        item.children
            .extend([Node::Element(raw), Node::Element(qualifier)]);
        bag.children.push(Node::Element(item));
        node.children.push(Node::Element(bag));
        self.put(XMP, "Identifier", node);
    }
    pub(super) fn merge(&mut self, extended: Self) -> Result<(), String> {
        let (rdf, context) =
            rdf_mut(&mut self.root, &Namespaces::new()).ok_or("Missing main RDF")?;
        let mut root = extended.root;
        let (extra, ns) = rdf_mut(&mut root, &Namespaces::new()).ok_or("Missing extended RDF")?;
        for child in &extra.children {
            if let Node::Element(child) = child {
                let mut child = child.clone();
                bind_context(&mut child, &ns);
                rdf.children.push(Node::Element(child));
            }
        }
        // Ensure the context is not accidentally relied on by relocated nodes.
        let _ = context;
        Ok(())
    }
    pub(super) fn move_largest_property_to(
        &mut self,
        destination: &mut Self,
        excluded: (&str, &str),
    ) -> Result<bool, String> {
        super::packet_transfer::move_largest(&mut self.root, &mut destination.root, excluded)
    }
    fn put(&mut self, namespace: &str, property: &str, node: Element) {
        self.delete(namespace, property);
        let Some((rdf, context)) = rdf_mut(&mut self.root, &Namespaces::new()) else {
            return;
        };
        for child in &mut rdf.children {
            if let Node::Element(description) = child {
                if is_description(description, &description.namespaces(&context)) {
                    description.children.push(Node::Element(node));
                    return;
                }
            }
        }
        let mut description = rdf_node("Description");
        description.set_attr("rdf:about", "");
        description.children.push(Node::Element(node));
        rdf.children.push(Node::Element(description));
    }
    fn property(&self, namespace: &str, property: &str) -> Option<(Element, Namespaces)> {
        let (rdf, context) = rdf_ref(&self.root, &Namespaces::new())?;
        for child in &rdf.children {
            let Node::Element(description) = child else {
                continue;
            };
            let ns = description.namespaces(&context);
            if !is_description(description, &ns) {
                continue;
            }
            for child in &description.children {
                if let Node::Element(node) = child {
                    let context = node.namespaces(&ns);
                    if expanded(&node.name, &context, false) == Some((namespace, property)) {
                        return Some((node.clone(), context));
                    }
                }
            }
        }
        None
    }
    fn abbreviated(&self, namespace: &str, property: &str) -> Option<String> {
        let (rdf, context) = rdf_ref(&self.root, &Namespaces::new())?;
        for child in &rdf.children {
            if let Node::Element(node) = child {
                let ns = node.namespaces(&context);
                if is_description(node, &ns) {
                    for (key, value) in &node.attributes {
                        if expanded(key, &ns, true) == Some((namespace, property)) {
                            return Some(value.clone());
                        }
                    }
                }
            }
        }
        None
    }
    fn array_items(&self, namespace: &str, property: &str) -> Vec<(Element, Namespaces)> {
        let Some((node, ns)) = self.property(namespace, property) else {
            return Vec::new();
        };
        for kind in ["Bag", "Seq", "Alt"] {
            if let Some((array, context)) = child_named(&node, &ns, RDF, kind) {
                return array
                    .children
                    .iter()
                    .filter_map(|child| match child {
                        Node::Element(item)
                            if expanded(&item.name, &item.namespaces(&context), false)
                                == Some((RDF, "li")) =>
                        {
                            Some((item.clone(), item.namespaces(&context)))
                        }
                        _ => None,
                    })
                    .collect();
            }
        }
        Vec::new()
    }
}

fn property_node(namespace: &str, name: &str) -> Element {
    let prefix = match namespace {
        DC => "dc",
        XMP => "xmp",
        IDQ => "xmpidq",
        RIGHTS => "xmpRights",
        EXIF => "exif",
        IPTC => "Iptc4xmpCore",
        NOTE => "xmpNote",
        _ => "property",
    };
    let mut node = Element::new(&format!("{prefix}:{name}"));
    node.set_attr(&format!("xmlns:{prefix}"), namespace);
    node
}
fn rdf_node(name: &str) -> Element {
    let mut node = Element::new(&format!("rdf:{name}"));
    node.set_attr("xmlns:rdf", RDF);
    node
}
pub(super) fn bind_context(node: &mut Element, ns: &Namespaces) {
    for (prefix, uri) in ns {
        if prefix != "xml" {
            let name = if prefix.is_empty() {
                "xmlns".into()
            } else {
                format!("xmlns:{prefix}")
            };
            if node.attr(&name).is_none() {
                node.set_attr(&name, uri);
            }
        }
    }
}
pub(super) fn is_description(node: &Element, ns: &Namespaces) -> bool {
    expanded(&node.name, ns, false) == Some((RDF, "Description"))
}
pub(super) fn rdf_mut<'a>(
    node: &'a mut Element,
    inherited: &Namespaces,
) -> Option<(&'a mut Element, Namespaces)> {
    let context = node.namespaces(inherited);
    if expanded(&node.name, &context, false) == Some((RDF, "RDF")) {
        return Some((node, context));
    }
    for child in &mut node.children {
        if let Node::Element(child) = child {
            if let Some(result) = rdf_mut(child, &context) {
                return Some(result);
            }
        }
    }
    None
}
fn rdf_ref<'a>(node: &'a Element, inherited: &Namespaces) -> Option<(&'a Element, Namespaces)> {
    let context = node.namespaces(inherited);
    if expanded(&node.name, &context, false) == Some((RDF, "RDF")) {
        return Some((node, context));
    }
    for child in &node.children {
        if let Node::Element(child) = child {
            if let Some(result) = rdf_ref(child, &context) {
                return Some(result);
            }
        }
    }
    None
}
fn child_named<'a>(
    node: &'a Element,
    ns: &Namespaces,
    namespace: &str,
    name: &str,
) -> Option<(&'a Element, Namespaces)> {
    for child in &node.children {
        if let Node::Element(child) = child {
            let context = child.namespaces(ns);
            if expanded(&child.name, &context, false) == Some((namespace, name)) {
                return Some((child, context));
            }
        }
    }
    None
}
fn simple_value(node: &Element, ns: &Namespaces) -> Option<String> {
    if let Some((value, context)) = child_named(node, ns, RDF, "value") {
        return simple_value(value, &context);
    }
    if let Some((_, value)) = node
        .attributes
        .iter()
        .find(|(key, _)| expanded(key, ns, true) == Some((RDF, "resource")))
    {
        return Some(value.clone());
    }
    if node
        .children
        .iter()
        .any(|child| matches!(child, Node::Element(_)))
    {
        None
    } else {
        Some(node.text())
    }
}
