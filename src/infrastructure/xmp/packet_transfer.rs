//! Generic property transfer for bounded container payloads; no file-format logic.
use super::{
    packet::{RDF, bind_context, is_description, rdf_mut},
    xml::{self, Element, Namespaces, Node, expanded},
};
enum Location {
    Attribute(usize, usize),
    Child(usize, usize),
}

pub(super) fn move_largest(
    source: &mut Element,
    destination: &mut Element,
    excluded: (&str, &str),
) -> Result<bool, String> {
    let (rdf, context) = rdf_mut(source, &Namespaces::new()).ok_or("Missing source RDF")?;
    let mut largest = None;
    let mut largest_size = 0;
    for (description_index, child) in rdf.children.iter().enumerate() {
        let Node::Element(description) = child else {
            continue;
        };
        let ns = description.namespaces(&context);
        if !is_description(description, &ns) {
            continue;
        }
        for (index, (name, value)) in description.attributes.iter().enumerate() {
            if name == "xmlns" || name.starts_with("xmlns:") {
                continue;
            }
            let Some((namespace, local)) = expanded(name, &ns, true) else {
                continue;
            };
            if namespace.is_empty()
                || namespace == RDF
                || namespace == "http://www.w3.org/XML/1998/namespace"
                || (namespace, local) == excluded
            {
                continue;
            }
            let size = value.len() + name.len();
            if size > largest_size {
                largest = Some(Location::Attribute(description_index, index));
                largest_size = size;
            }
        }
        for (index, child) in description.children.iter().enumerate() {
            let Node::Element(node) = child else {
                continue;
            };
            let node_ns = node.namespaces(&ns);
            if expanded(&node.name, &node_ns, false) == Some(excluded) {
                continue;
            }
            let size = xml::serialize(node)?.len();
            if size > largest_size {
                largest = Some(Location::Child(description_index, index));
                largest_size = size;
            }
        }
    }
    let Some(location) = largest else {
        return Ok(false);
    };
    let description_index = match location {
        Location::Attribute(d, _) | Location::Child(d, _) => d,
    };
    let Node::Element(description) = &mut rdf.children[description_index] else {
        unreachable!()
    };
    let ns = description.namespaces(&context);
    let mut moved = match location {
        Location::Attribute(_, index) => {
            let (name, value) = description.attributes.remove(index);
            let mut node = Element::new(&name);
            node.children.push(Node::Text(value));
            node
        }
        Location::Child(_, index) => {
            let Node::Element(node) = description.children.remove(index) else {
                unreachable!()
            };
            node
        }
    };
    bind_context(&mut moved, &ns);
    let mut target = Element::new("rdf:Description");
    target.set_attr("xmlns:rdf", RDF);
    for (key, value) in &description.attributes {
        if expanded(key, &ns, true) == Some((RDF, "about")) {
            target.set_attr("rdf:about", value);
        }
        if key.starts_with("xml:") {
            target.set_attr(key, value);
        }
    }
    if target.attr("rdf:about").is_none() {
        target.set_attr("rdf:about", "");
    }
    target.children.push(Node::Element(moved));
    let (dest, _) = rdf_mut(destination, &Namespaces::new()).ok_or("Missing destination RDF")?;
    dest.children.push(Node::Element(target));
    Ok(true)
}
