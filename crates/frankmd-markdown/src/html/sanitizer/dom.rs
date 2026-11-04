//! A small arena DOM built by html5ever, plus its HTML serializer.
//!
//! The sanitizer needs to walk and mutate the tree the way DOMPurify does
//! (hoist the children of a removed element into its place, detach nodes
//! mid-walk), so it owns its tree instead of going through ammonia's
//! fixed pipeline. Every traversal here is iterative: hostile input can nest
//! arbitrarily deep and must not overflow the stack.

use std::borrow::Cow;
use std::cell::{Ref, RefCell};

use html5ever::interface::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{Attribute, LocalName, Namespace, ParseOpts, QualName, local_name, ns};

/// Index of a node in [`Dom::nodes`].
pub(super) type Id = usize;

/// What a node is.
pub(super) enum Kind {
    Document,
    Text(String),
    Comment(String),
    Element(Elem),
}

/// Element payload.
pub(super) struct Elem {
    pub(super) name: QualName,
    pub(super) attrs: Vec<Attribute>,
    /// html5ever's `mathml_annotation_xml_integration_point` flag.
    annotation_xml_integration_point: bool,
}

/// A node with doubly linked sibling pointers (O(1) detach and insert).
pub(super) struct Node {
    pub(super) kind: Kind,
    parent: Option<Id>,
    first: Option<Id>,
    last: Option<Id>,
    prev: Option<Id>,
    next: Option<Id>,
}

/// The parsed document.
pub(super) struct Dom {
    nodes: Vec<Node>,
    /// Returned by `elem_name` for a non-element handle (never asked for by
    /// html5ever in practice, but the trait needs a reference to give back).
    fallback_name: QualName,
}

const DOCUMENT: Id = 0;

impl Dom {
    fn new() -> Self {
        let mut dom = Dom {
            nodes: Vec::new(),
            fallback_name: QualName::new(None, ns!(html), local_name!("div")),
        };
        dom.push(Kind::Document);
        dom
    }

    fn push(&mut self, kind: Kind) -> Id {
        self.nodes.push(Node {
            kind,
            parent: None,
            first: None,
            last: None,
            prev: None,
            next: None,
        });
        self.nodes.len() - 1
    }

    /// Create a detached text node.
    pub(super) fn new_text(&mut self, text: &str) -> Id {
        self.push(Kind::Text(text.to_owned()))
    }

    pub(super) fn kind(&self, id: Id) -> &Kind {
        &self.nodes[id].kind
    }

    pub(super) fn elem(&self, id: Id) -> Option<&Elem> {
        match &self.nodes[id].kind {
            Kind::Element(e) => Some(e),
            _ => None,
        }
    }

    pub(super) fn elem_mut(&mut self, id: Id) -> Option<&mut Elem> {
        match &mut self.nodes[id].kind {
            Kind::Element(e) => Some(e),
            _ => None,
        }
    }

    pub(super) fn parent(&self, id: Id) -> Option<Id> {
        self.nodes[id].parent
    }

    pub(super) fn first_child(&self, id: Id) -> Option<Id> {
        self.nodes[id].first
    }

    pub(super) fn next_sibling(&self, id: Id) -> Option<Id> {
        self.nodes[id].next
    }

    pub(super) fn prev_sibling(&self, id: Id) -> Option<Id> {
        self.nodes[id].prev
    }

    /// Direct children, in order.
    pub(super) fn children(&self, id: Id) -> Vec<Id> {
        let mut out = Vec::new();
        let mut cur = self.nodes[id].first;
        while let Some(c) = cur {
            out.push(c);
            cur = self.nodes[c].next;
        }
        out
    }

    /// Unlink `id` from its parent (no-op when already detached).
    pub(super) fn detach(&mut self, id: Id) {
        let Node {
            parent, prev, next, ..
        } = self.nodes[id];
        let Some(parent) = parent else { return };
        match prev {
            Some(p) => self.nodes[p].next = next,
            None => self.nodes[parent].first = next,
        }
        match next {
            Some(n) => self.nodes[n].prev = prev,
            None => self.nodes[parent].last = prev,
        }
        let node = &mut self.nodes[id];
        node.parent = None;
        node.prev = None;
        node.next = None;
    }

    /// Append `child` (detached first) as the last child of `parent`.
    pub(super) fn append(&mut self, parent: Id, child: Id) {
        self.detach(child);
        let last = self.nodes[parent].last;
        {
            let node = &mut self.nodes[child];
            node.parent = Some(parent);
            node.prev = last;
        }
        match last {
            Some(l) => self.nodes[l].next = Some(child),
            None => self.nodes[parent].first = Some(child),
        }
        self.nodes[parent].last = Some(child);
    }

    /// Insert `child` (detached first) right before `sibling`.
    pub(super) fn insert_before(&mut self, sibling: Id, child: Id) {
        let Some(parent) = self.nodes[sibling].parent else {
            return;
        };
        self.detach(child);
        let prev = self.nodes[sibling].prev;
        {
            let node = &mut self.nodes[child];
            node.parent = Some(parent);
            node.prev = prev;
            node.next = Some(sibling);
        }
        self.nodes[sibling].prev = Some(child);
        match prev {
            Some(p) => self.nodes[p].next = Some(child),
            None => self.nodes[parent].first = Some(child),
        }
    }

    /// Insert `child` as the first child of `parent`.
    pub(super) fn prepend(&mut self, parent: Id, child: Id) {
        match self.nodes[parent].first {
            Some(first) => self.insert_before(first, child),
            None => self.append(parent, child),
        }
    }

    /// The `<body>` element of the parsed document.
    pub(super) fn body(&self) -> Option<Id> {
        let html = self
            .children(DOCUMENT)
            .into_iter()
            .find(|&c| self.is_html_element(c, "html"))?;
        self.children(html)
            .into_iter()
            .find(|&c| self.is_html_element(c, "body"))
    }

    fn is_html_element(&self, id: Id, local: &str) -> bool {
        self.elem(id)
            .is_some_and(|e| e.name.ns == ns!(html) && &*e.name.local == local)
    }

    /// DOM `textContent` of an element (descendant text, comments excluded).
    pub(super) fn text_content(&self, id: Id) -> String {
        let mut out = String::new();
        let mut stack = self.children(id);
        stack.reverse();
        while let Some(c) = stack.pop() {
            match &self.nodes[c].kind {
                Kind::Text(t) => out.push_str(t),
                Kind::Element(_) => {
                    let mut kids = self.children(c);
                    kids.reverse();
                    stack.extend(kids);
                }
                _ => {}
            }
        }
        out
    }

    /// True when `id` has an element child (`firstElementChild`).
    pub(super) fn has_element_child(&self, id: Id) -> bool {
        self.children(id)
            .into_iter()
            .any(|c| matches!(self.nodes[c].kind, Kind::Element(_)))
    }

    /// DOM `innerHTML`: the HTML fragment serialization of `id`'s children.
    pub(super) fn inner_html(&self, id: Id) -> String {
        enum Step {
            Node(Id),
            Close(Id),
        }
        let mut out = String::new();
        let mut stack: Vec<Step> = self
            .children(id)
            .into_iter()
            .rev()
            .map(Step::Node)
            .collect();
        while let Some(step) = stack.pop() {
            let node = match step {
                Step::Close(e) => {
                    if let Some(elem) = self.elem(e) {
                        out.push_str("</");
                        out.push_str(&elem.name.local);
                        out.push('>');
                    }
                    continue;
                }
                Step::Node(n) => n,
            };
            match &self.nodes[node].kind {
                Kind::Text(text) => {
                    if self.nodes[node]
                        .parent
                        .is_some_and(|p| self.is_raw_text_parent(p))
                    {
                        out.push_str(text);
                    } else {
                        escape_into(&mut out, text, false);
                    }
                }
                Kind::Comment(data) => {
                    out.push_str("<!--");
                    out.push_str(data);
                    out.push_str("-->");
                }
                Kind::Document => {}
                Kind::Element(elem) => {
                    out.push('<');
                    out.push_str(&elem.name.local);
                    for attr in &elem.attrs {
                        out.push(' ');
                        out.push_str(&attr_name(attr));
                        out.push_str("=\"");
                        escape_into(&mut out, &attr.value, true);
                        out.push('"');
                    }
                    out.push('>');
                    if elem.name.ns == ns!(html) && is_void(&elem.name.local) {
                        continue;
                    }
                    stack.push(Step::Close(node));
                    stack.extend(self.children(node).into_iter().rev().map(Step::Node));
                }
            }
        }
        out
    }

    /// Children of these HTML elements serialize as raw text.
    fn is_raw_text_parent(&self, id: Id) -> bool {
        self.elem(id).is_some_and(|e| {
            e.name.ns == ns!(html)
                && matches!(
                    &*e.name.local,
                    "style" | "script" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext"
                )
        })
    }
}

/// The attribute's qualified name as the DOM reports it (`xlink:href`).
pub(super) fn attr_name(attr: &Attribute) -> Cow<'_, str> {
    match &attr.name.prefix {
        Some(prefix) if !prefix.is_empty() => Cow::Owned(format!("{}:{}", prefix, attr.name.local)),
        _ => Cow::Borrowed(&attr.name.local),
    }
}

fn is_void(local: &LocalName) -> bool {
    matches!(
        &**local,
        "area"
            | "base"
            | "basefont"
            | "bgsound"
            | "br"
            | "col"
            | "embed"
            | "frame"
            | "hr"
            | "img"
            | "input"
            | "keygen"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// HTML serialization escaping. `<`/`>` are escaped in attribute values too
/// (current HTML spec), which closes re-parse mXSS tricks.
fn escape_into(out: &mut String, text: &str, attribute: bool) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Parse `html` as a whole document the way `DOMParser.parseFromString(html,
/// "text/html")` does (scripting disabled, so `<noscript>` content is markup).
pub(super) fn parse_document(html: &str) -> Dom {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled: false,
            ..Default::default()
        },
        ..Default::default()
    };
    html5ever::parse_document(Sink(RefCell::new(Dom::new())), opts).one(html)
}

/// html5ever tree sink over [`Dom`]. Template contents are stored as the
/// template element's own children (DOMPurify sanitizes them the same way).
struct Sink(RefCell<Dom>);

impl Sink {
    fn append_text_or_node(dom: &mut Dom, parent: Id, child: NodeOrText<Id>) {
        match child {
            NodeOrText::AppendNode(node) => dom.append(parent, node),
            NodeOrText::AppendText(text) => {
                if let Some(last) = dom.nodes[parent].last
                    && let Kind::Text(existing) = &mut dom.nodes[last].kind
                {
                    existing.push_str(&text);
                    return;
                }
                let node = dom.new_text(&text);
                dom.append(parent, node);
            }
        }
    }
}

impl TreeSink for Sink {
    type Handle = Id;
    type Output = Dom;
    type ElemName<'a>
        = Ref<'a, QualName>
    where
        Self: 'a;

    fn finish(self) -> Dom {
        self.0.into_inner()
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> Id {
        DOCUMENT
    }

    fn elem_name<'a>(&'a self, target: &'a Id) -> Ref<'a, QualName> {
        Ref::map(self.0.borrow(), |dom| match &dom.nodes[*target].kind {
            Kind::Element(e) => &e.name,
            _ => &dom.fallback_name,
        })
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> Id {
        self.0.borrow_mut().push(Kind::Element(Elem {
            name,
            attrs,
            annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        }))
    }

    fn create_comment(&self, text: StrTendril) -> Id {
        self.0.borrow_mut().push(Kind::Comment(text.to_string()))
    }

    fn create_pi(&self, _target: StrTendril, data: StrTendril) -> Id {
        // Never produced by the HTML parser (`<?x>` is a bogus comment); kept
        // as a comment so it is removed like one.
        self.0.borrow_mut().push(Kind::Comment(data.to_string()))
    }

    fn append(&self, parent: &Id, child: NodeOrText<Id>) {
        Self::append_text_or_node(&mut self.0.borrow_mut(), *parent, child);
    }

    fn append_based_on_parent_node(&self, element: &Id, prev_element: &Id, child: NodeOrText<Id>) {
        let has_parent = self.0.borrow().nodes[*element].parent.is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
    }

    fn get_template_contents(&self, target: &Id) -> Id {
        *target
    }

    fn same_node(&self, x: &Id, y: &Id) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &Id, new_node: NodeOrText<Id>) {
        let mut dom = self.0.borrow_mut();
        match new_node {
            NodeOrText::AppendNode(node) => dom.insert_before(*sibling, node),
            NodeOrText::AppendText(text) => {
                if let Some(prev) = dom.nodes[*sibling].prev
                    && let Kind::Text(existing) = &mut dom.nodes[prev].kind
                {
                    existing.push_str(&text);
                    return;
                }
                let node = dom.new_text(&text);
                dom.insert_before(*sibling, node);
            }
        }
    }

    fn add_attrs_if_missing(&self, target: &Id, attrs: Vec<Attribute>) {
        let mut dom = self.0.borrow_mut();
        if let Some(elem) = dom.elem_mut(*target) {
            for attr in attrs {
                if !elem.attrs.iter().any(|a| a.name == attr.name) {
                    elem.attrs.push(attr);
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &Id) {
        self.0.borrow_mut().detach(*target);
    }

    fn reparent_children(&self, node: &Id, new_parent: &Id) {
        let mut dom = self.0.borrow_mut();
        for child in dom.children(*node) {
            dom.append(*new_parent, child);
        }
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Id) -> bool {
        self.0
            .borrow()
            .elem(*handle)
            .is_some_and(|e| e.annotation_xml_integration_point)
    }
}

/// Namespace of an element, as DOMPurify distinguishes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ns {
    Html,
    Svg,
    MathMl,
    Other,
}

impl Ns {
    pub(super) fn of(ns: &Namespace) -> Ns {
        if *ns == ns!(html) {
            Ns::Html
        } else if *ns == ns!(svg) {
            Ns::Svg
        } else if *ns == ns!(mathml) {
            Ns::MathMl
        } else {
            Ns::Other
        }
    }
}
