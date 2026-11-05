//! Structured XML streaming adapter (`feature = "xml"`).
//!
//! # Scope
//!
//! XML is the event-streaming member of `ruviform-io-structured`: the
//! adapter exposes borrowed semantic events from a [`BufRead`] source and
//! never materializes a DOM. One [`XmlEventReader`] pulls
//! [`XmlEventRef`] values out of a caller-owned [`XmlEventBuffer`] until
//! the document ends.
//!
//! ```rust
//! # fn example() -> Result<(), ruviform_io_structured::DocumentError> {
//! use ruviform_io_structured::xml::{
//!     XmlEventBuffer, XmlEventReader, XmlEventRef, XmlReadOptions,
//! };
//!
//! let xml = br#"<root xmlns="urn:example"><item a="1">text</item></root>"#;
//! let mut cursor = std::io::Cursor::new(xml);
//! let mut reader = XmlEventReader::new(&mut cursor, &XmlReadOptions::default());
//! let mut buffer = XmlEventBuffer::new();
//! while let Some(event) = reader.read_event(&mut buffer)? {
//!     match event {
//!         XmlEventRef::Start(element) | XmlEventRef::Empty(element) => {
//!             println!(
//!                 "<{} ns={:?} depth={}>",
//!                 element.name().qualified_name(),
//!                 element.name().namespace(),
//!                 reader.depth()
//!             );
//!         }
//!         XmlEventRef::End(name) => println!("</{}>", name.qualified_name()),
//!         XmlEventRef::Text(text) => println!("text: {text}"),
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! # example().unwrap();
//! ```
//!
//! # Event model
//!
//! Events preserve the source distinction between [`XmlEventRef::Start`],
//! [`XmlEventRef::Empty`], [`XmlEventRef::End`], [`XmlEventRef::Text`],
//! [`XmlEventRef::CData`], [`XmlEventRef::Comment`],
//! [`XmlEventRef::Declaration`], [`XmlEventRef::ProcessingInstruction`],
//! and [`XmlEventRef::Reference`]. `Text`/`CData`/`Comment` payloads borrow
//! the buffer directly; `Text` additionally carries XML 1.0 / 1.1
//! line-ending normalization from the backend. Element names, attribute
//! names/values, namespace URIs, and the declaration/PI metadata are copied
//! into owned storage (bounded by `max_event_bytes`) because the backend
//! cannot lend them with the buffer lifetime.
//!
//! Namespace prefixes are resolved eagerly per event: default namespaces
//! apply to unprefixed element names but never to unprefixed attribute
//! names, `xml`/`xmlns` behave per the XML Namespaces recommendation, and
//! `End` names are resolved before the element's namespace scope is
//! popped. Undeclared prefixes surface as [`DocumentErrorKind::Syntax`]
//! errors.
//!
//! # Secure defaults
//!
//! - Any `<!DOCTYPE ...>` declaration — internal subset *or* external
//!   identifier — is rejected as [`DocumentErrorKind::Unsupported`]. No
//!   DTD is fetched or expanded.
//! - Undefined general entities (`&foo;`) are rejected as `Unsupported`;
//!   predefined (`&amp;` `&lt;` `&gt;` `&apos;` `&quot;`) and numeric
//!   (`&#65;`/`&#x41;`) references are reported as explicit
//!   [`XmlEventRef::Reference`] events and are never expanded inline.
//! - A declaration naming a character encoding other than UTF-8 is
//!   rejected as `Unsupported`; invalid UTF-8 input surfaces as
//!   [`DocumentErrorKind::InvalidEncoding`].
//!
//! # Limits
//!
//! [`XmlReadOptions`] enforces the input byte budget plus depth,
//! attribute-count, event-byte, event-count, and namespace-binding caps —
//! each classified with a [`DocumentLimit`] so callers can tell a resource
//! violation from malformed input. `max_event_bytes` is checked when an
//! event is published: the backend may already have buffered that event
//! internally, so the option bounds exposed payloads rather than the
//! backend's internal read.
//!
//! # Backend
//!
//! Streaming uses a namespace-aware `quick-xml` reader over
//! [`BufRead`]. The dependency is private: no backend type, error, or
//! feature name appears in the public API or in the `Display` surface of
//! [`DocumentError`].
//!
//! Reading allocates on every event and performs blocking I/O — never
//! call this module from an RT audio callback.

use quick_xml::escape::{EscapeError, resolve_xml_entity};
use quick_xml::events::attributes::{AttrError, Attribute};
use quick_xml::events::{BytesDecl, BytesStart, Event};
use quick_xml::name::{NamespaceError, QName, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::{Error as BackendError, XmlVersion};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{BufRead, BufReader};

use crate::input::BoundedRead;
use crate::{
    DocumentError, DocumentErrorKind, DocumentFormat, DocumentLimit, DocumentPosition, InputLimits,
};

const FORMAT: DocumentFormat = DocumentFormat::Xml;

/// Bytes of buffered lookahead between the caller's [`BufRead`] and the
/// event parser, so one event usually fits inside a single fill.
const READ_CHUNK: usize = 8 * 1024;

/// Read-side configuration for [`XmlEventReader`].
///
/// `None` disables a cap. The byte budget is enforced on the raw input
/// before the backend sees it; depth is compared against the backend's
/// open-element count after each structural event; the attribute,
/// event-byte, and event-count caps are checked before an event is
/// published; the namespace-binding cap is enforced inside the backend's
/// resolver. Note that `max_event_bytes` bounds the published event
/// payload — the backend may have already buffered that many bytes
/// internally when the check fires.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct XmlReadOptions {
    /// Raw input budget shared with the other formats.
    pub limits: InputLimits,
    /// Maximum open-element depth counted after each `Start`/`End`/`Empty`
    /// event.
    pub max_depth: Option<u64>,
    /// Maximum attribute count per element, including `xmlns`
    /// declarations.
    pub max_attributes: Option<u64>,
    /// Maximum bytes for a single published event payload.
    pub max_event_bytes: Option<u64>,
    /// Maximum number of published events before EOF.
    pub max_events: Option<u64>,
    /// Maximum live namespace bindings across the whole document.
    /// `None` keeps the backend resolver's own ceiling (128).
    pub max_namespace_bindings: Option<usize>,
}

impl Default for XmlReadOptions {
    fn default() -> Self {
        Self {
            limits: InputLimits::default(),
            max_depth: Some(512),
            max_attributes: Some(512),
            max_event_bytes: Some(16 * 1024 * 1024),
            max_events: Some(1_000_000),
            max_namespace_bindings: None,
        }
    }
}

/// Caller-owned buffer that [`XmlEventReader::read_event`] fills.
///
/// The buffer's capacity is reused across reads so a well-formed stream
/// allocates once per oversized event rather than once per event. Events
/// borrow from the buffer until the next `read_event` call or [`clear`];
/// converting with [`XmlEventRef::to_owned`] detaches the event from the
/// buffer's lifetime.
///
/// [`clear`]: Self::clear
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct XmlEventBuffer {
    /// Raw bytes of the current event as emitted by the backend.
    raw: Vec<u8>,
}

impl XmlEventBuffer {
    /// Empty buffer, equivalent to [`Default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocated capacity of the event buffer, in bytes.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.raw.capacity()
    }

    /// Drops the current event payload but keeps the allocation so the
    /// next read reuses it.
    pub fn clear(&mut self) {
        self.raw.clear();
    }
}

/// Namespace-resolved element or attribute name on a borrowed event.
///
/// `qualified_name` is the raw text as written in the source; `prefix` and
/// `local_name` decompose it. `namespace` is the resolved namespace URI —
/// `Some` for bound prefixed names and for unprefixed element names under
/// a default namespace, `None` for unprefixed names with no default
/// namespace and for all unprefixed attribute names.
///
/// Internally the parts are copied out of the backend's parse view; they
/// are typed with the event lifetime so a future borrowed-backend path
/// does not change the API.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlNameRef<'a> {
    /// Qualified name as written in the source (`"prefix:local"`).
    qualified: Cow<'a, str>,
    /// Namespace prefix, if the name carries one.
    prefix: Option<Cow<'a, str>>,
    /// Local name without the prefix.
    local: Cow<'a, str>,
    /// Resolved namespace URI, if bound.
    namespace: Option<Cow<'a, str>>,
}

impl XmlNameRef<'_> {
    /// Qualified name exactly as written in the source.
    #[must_use]
    pub fn qualified_name(&self) -> &str {
        &self.qualified
    }

    /// Namespace prefix (`"p"` in `"p:item"`), if present.
    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// Local name without any prefix.
    #[must_use]
    pub fn local_name(&self) -> &str {
        &self.local
    }

    /// Resolved namespace URI. `None` when the name is unbound —
    /// including every unprefixed attribute name.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Detaches the name from the event buffer.
    #[must_use]
    pub fn to_owned(&self) -> XmlName {
        XmlName {
            qualified: self.qualified.clone().into_owned(),
            prefix: self.prefix.clone().map(Cow::into_owned),
            local: self.local.clone().into_owned(),
            namespace: self.namespace.clone().map(Cow::into_owned),
        }
    }
}

/// Owned counterpart of [`XmlNameRef`], produced by
/// [`XmlNameRef::to_owned`] or [`XmlEventRef::to_owned`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlName {
    /// Qualified name as written in the source.
    qualified: String,
    /// Namespace prefix, if the name carries one.
    prefix: Option<String>,
    /// Local name without the prefix.
    local: String,
    /// Resolved namespace URI, if bound.
    namespace: Option<String>,
}

impl XmlName {
    /// Qualified name exactly as written in the source.
    #[must_use]
    pub fn qualified_name(&self) -> &str {
        &self.qualified
    }

    /// Namespace prefix, if present.
    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// Local name without any prefix.
    #[must_use]
    pub fn local_name(&self) -> &str {
        &self.local
    }

    /// Resolved namespace URI, if bound.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}

/// Element payload of [`XmlEventRef::Start`] and [`XmlEventRef::Empty`].
///
/// The attribute list preserves source order and includes `xmlns`/`xmlns:*`
/// namespace declarations as ordinary attributes, since they are part of
/// the raw stream.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlElementRef<'a> {
    /// Resolved element name.
    name: XmlNameRef<'a>,
    /// Attributes in source order, bounded by `max_attributes`.
    attributes: Vec<XmlAttributeRef<'a>>,
    /// Byte offset immediately after the start/empty tag.
    position: DocumentPosition,
}

impl<'a> XmlElementRef<'a> {
    /// Resolved element name.
    #[must_use]
    pub fn name(&self) -> &XmlNameRef<'a> {
        &self.name
    }

    /// Attributes in source order, including namespace declarations.
    #[must_use]
    pub fn attributes(&self) -> &[XmlAttributeRef<'a>] {
        &self.attributes
    }

    /// Byte offset immediately after this tag.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }

    /// Detaches the element from the event buffer.
    #[must_use]
    pub fn to_owned(&self) -> XmlElement {
        XmlElement {
            name: self.name.to_owned(),
            attributes: self
                .attributes
                .iter()
                .map(XmlAttributeRef::to_owned)
                .collect(),
            position: self.position,
        }
    }
}

/// Owned counterpart of [`XmlElementRef`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlElement {
    /// Resolved element name.
    name: XmlName,
    /// Attributes in source order.
    attributes: Vec<XmlAttribute>,
    /// Byte offset immediately after the tag.
    position: DocumentPosition,
}

impl XmlElement {
    /// Resolved element name.
    #[must_use]
    pub fn name(&self) -> &XmlName {
        &self.name
    }

    /// Attributes in source order, including namespace declarations.
    #[must_use]
    pub fn attributes(&self) -> &[XmlAttribute] {
        &self.attributes
    }

    /// Byte offset immediately after this tag.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }
}

/// One element attribute on a borrowed event.
///
/// `name` carries namespace resolution — a prefixed attribute resolves to
/// its prefix's namespace while an unprefixed attribute stays unbound even
/// under a default namespace. `value` is the raw attribute text as written;
/// `normalized_value` additionally unescapes references and applies the
/// XML attribute-value normalization (whitespace collapsing plus
/// line-ending folding) of the document's declared version.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlAttributeRef<'a> {
    /// Resolved attribute name.
    name: XmlNameRef<'a>,
    /// Raw attribute value as written in the source.
    value: Cow<'a, str>,
    /// Document XML version used by [`normalized_value`].
    ///
    /// [`normalized_value`]: Self::normalized_value
    version: XmlVersion,
}

impl<'a> XmlAttributeRef<'a> {
    /// Resolved attribute name.
    #[must_use]
    pub fn name(&self) -> &XmlNameRef<'a> {
        &self.name
    }

    /// Raw attribute value as written (entities left unescaped).
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Normalized attribute value: references resolved and whitespace /
    /// line-ending folding applied per the document's XML version.
    ///
    /// # Errors
    ///
    /// [`DocumentErrorKind::Syntax`] when the value contains a malformed or
    /// unterminated reference, and [`DocumentErrorKind::Unsupported`] when
    /// it references an undeclared general entity.
    pub fn normalized_value(&self) -> Result<Cow<'_, str>, DocumentError> {
        let attribute = Attribute {
            key: QName(self.name.qualified.as_ref()),
            value: Cow::Borrowed(self.value.as_ref()),
        };
        attribute.normalized_value(self.version).map_err(|error| {
            let kind = match &error {
                BackendError::Escape(EscapeError::UnrecognizedEntity(..)) => {
                    DocumentErrorKind::Unsupported
                }
                _ => classify_backend(&error),
            };
            DocumentError::with_source(kind, FORMAT, error)
        })
    }

    /// Detaches the attribute from the event buffer.
    #[must_use]
    pub fn to_owned(&self) -> XmlAttribute {
        XmlAttribute {
            name: self.name.to_owned(),
            value: self.value.clone().into_owned(),
        }
    }
}

/// Owned counterpart of [`XmlAttributeRef`]. The owned form keeps the raw
/// value; call [`XmlAttributeRef::normalized_value`] before `to_owned` if
/// the normalized value is needed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlAttribute {
    /// Resolved attribute name.
    name: XmlName,
    /// Raw attribute value as written.
    value: String,
}

impl XmlAttribute {
    /// Resolved attribute name.
    #[must_use]
    pub fn name(&self) -> &XmlName {
        &self.name
    }

    /// Raw attribute value as written.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// `<?xml ...?>` document declaration on a borrowed event.
///
/// Emitted at most once at the head of the stream. `version` is the
/// declared version string (`"1.0"` or `"1.1"`); `encoding` echoes the
/// declared label — the adapter rejects any non-UTF-8 label before this
/// event is published; `standalone` mirrors the `standalone` flag.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlDeclarationRef<'a> {
    /// Declared XML version (`"1.0"` or `"1.1"`).
    version: Cow<'a, str>,
    /// Declared encoding label (always UTF-8-flavoured, else rejected).
    encoding: Option<Cow<'a, str>>,
    /// `standalone="yes"` / `"no"`, absent otherwise.
    standalone: Option<bool>,
    /// Byte offset immediately after the declaration.
    position: DocumentPosition,
}

impl XmlDeclarationRef<'_> {
    /// Declared XML version string.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Declared encoding label, if present.
    #[must_use]
    pub fn encoding(&self) -> Option<&str> {
        self.encoding.as_deref()
    }

    /// Standalone flag, if declared.
    #[must_use]
    pub fn standalone(&self) -> Option<bool> {
        self.standalone
    }

    /// Byte offset immediately after the declaration.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }

    /// Detaches the declaration from the event buffer.
    #[must_use]
    pub fn to_owned(&self) -> XmlDeclaration {
        XmlDeclaration {
            version: self.version.clone().into_owned(),
            encoding: self.encoding.clone().map(Cow::into_owned),
            standalone: self.standalone,
            position: self.position,
        }
    }
}

/// Owned counterpart of [`XmlDeclarationRef`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlDeclaration {
    /// Declared XML version.
    version: String,
    /// Declared encoding label.
    encoding: Option<String>,
    /// Standalone flag.
    standalone: Option<bool>,
    /// Byte offset immediately after the declaration.
    position: DocumentPosition,
}

impl XmlDeclaration {
    /// Declared XML version string.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Declared encoding label, if present.
    #[must_use]
    pub fn encoding(&self) -> Option<&str> {
        self.encoding.as_deref()
    }

    /// Standalone flag, if declared.
    #[must_use]
    pub fn standalone(&self) -> Option<bool> {
        self.standalone
    }

    /// Byte offset immediately after the declaration.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }
}

/// Processing instruction on a borrowed event (`<?target content?>`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlPiRef<'a> {
    /// PI target (the name after `<?`).
    target: Cow<'a, str>,
    /// PI content after the target.
    content: Cow<'a, str>,
    /// Byte offset immediately after the PI.
    position: DocumentPosition,
}

impl XmlPiRef<'_> {
    /// PI target.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// PI content after the target.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Byte offset immediately after the PI.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }

    /// Detaches the PI from the event buffer.
    #[must_use]
    pub fn to_owned(&self) -> XmlPi {
        XmlPi {
            target: self.target.clone().into_owned(),
            content: self.content.clone().into_owned(),
            position: self.position,
        }
    }
}

/// Owned counterpart of [`XmlPiRef`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlPi {
    /// PI target.
    target: String,
    /// PI content after the target.
    content: String,
    /// Byte offset immediately after the PI.
    position: DocumentPosition,
}

impl XmlPi {
    /// PI target.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// PI content after the target.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Byte offset immediately after the PI.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }
}

/// A character or entity reference value reported as its own event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlReferenceValue {
    /// Predefined XML entity (`amp`, `lt`, `gt`, `apos`, `quot`) — the
    /// `&'static str` is the replacement text (`"&"`, `"<"`, ...).
    Entity(&'static str),
    /// Numeric character reference (`&#65;`, `&#x41;`) decoded to its
    /// Unicode scalar value.
    Char(char),
}

/// `&name;` / `&#N;` reference event payload.
///
/// The adapter never expands references inline: each reference in the
/// source becomes one [`XmlEventRef::Reference`]. Only the five predefined
/// entities and numeric character references are representable; any other
/// general entity is rejected before the event is published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct XmlReference {
    /// The reference value.
    value: XmlReferenceValue,
    /// Byte offset immediately after the reference.
    position: DocumentPosition,
}

impl XmlReference {
    /// The reference value.
    #[must_use]
    pub fn value(&self) -> XmlReferenceValue {
        self.value
    }

    /// Byte offset immediately after the reference.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        self.position
    }
}

/// One borrowed event out of [`XmlEventReader::read_event`].
///
/// Borrowed variants (`Text`, `CData`, `Comment`) reference the
/// [`XmlEventBuffer`] handed to `read_event` and stay valid until the next
/// read on the same buffer or [`XmlEventBuffer::clear`]. Use [`to_owned`]
/// to detach an event for longer storage.
///
/// [`to_owned`]: Self::to_owned
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlEventRef<'a> {
    /// `<name ...>` — open tag. Pushes a namespace scope.
    Start(XmlElementRef<'a>),
    /// `</name>` — close tag, resolved in the element's own scope.
    End(XmlNameRef<'a>),
    /// `<name .../>` — self-contained element.
    Empty(XmlElementRef<'a>),
    /// Character data between markup, line-ending-normalized per the
    /// document's XML version.
    Text(Cow<'a, str>),
    /// Raw `<![CDATA[...]]>` content (no normalization applied).
    CData(Cow<'a, str>),
    /// `<!-- ... -->` comment text.
    Comment(Cow<'a, str>),
    /// `<?xml ...?>` document declaration (head of stream).
    Declaration(XmlDeclarationRef<'a>),
    /// `<?target ...?>` processing instruction.
    ProcessingInstruction(XmlPiRef<'a>),
    /// `&name;` / `&#N;` reference, never expanded inline.
    Reference(XmlReference),
}

impl XmlEventRef<'_> {
    /// Detaches the event from the event buffer so it can outlive it.
    #[must_use]
    pub fn to_owned(&self) -> XmlEvent {
        match self {
            Self::Start(element) => XmlEvent::Start(element.to_owned()),
            Self::End(name) => XmlEvent::End(name.to_owned()),
            Self::Empty(element) => XmlEvent::Empty(element.to_owned()),
            Self::Text(text) => XmlEvent::Text(text.clone().into_owned()),
            Self::CData(text) => XmlEvent::CData(text.clone().into_owned()),
            Self::Comment(text) => XmlEvent::Comment(text.clone().into_owned()),
            Self::Declaration(decl) => XmlEvent::Declaration(decl.to_owned()),
            Self::ProcessingInstruction(pi) => XmlEvent::ProcessingInstruction(pi.to_owned()),
            Self::Reference(reference) => XmlEvent::Reference(*reference),
        }
    }
}

/// Owned counterpart of [`XmlEventRef`], produced by
/// [`XmlEventRef::to_owned`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlEvent {
    /// `<name ...>` — open tag.
    Start(XmlElement),
    /// `</name>` — close tag.
    End(XmlName),
    /// `<name .../>` — self-contained element.
    Empty(XmlElement),
    /// Character data between markup.
    Text(String),
    /// Raw `<![CDATA[...]]>` content.
    CData(String),
    /// `<!-- ... -->` comment text.
    Comment(String),
    /// `<?xml ...?>` document declaration.
    Declaration(XmlDeclaration),
    /// `<?target ...?>` processing instruction.
    ProcessingInstruction(XmlPi),
    /// `&name;` / `&#N;` reference.
    Reference(XmlReference),
}

/// Streaming XML event reader over a borrowed [`BufRead`] source.
///
/// Wraps the input in the shared [`BoundedRead`] byte budget plus a
/// buffered layer and a namespace-resolving pull parser. `read_event`
/// returns `Ok(None)` at end of document — and keeps returning `Ok(None)`
/// on repeated calls — or `Err` once for a malformed/limited stream, after
/// which the reader stays poisoned: further calls keep reporting a
/// failure so the stream cannot be resumed mid-error.
///
/// While an event is held the reader's [`position`] and [`depth`] remain
/// callable — the event borrows only the buffer.
///
/// [`position`]: Self::position
/// [`depth`]: Self::depth
pub struct XmlEventReader<'a, R: BufRead> {
    /// Namespace-resolving parser over budget + buffered layers.
    inner: NsReader<BufReader<BoundedRead<&'a mut R>>>,
    /// Effective options (immutable for the reader's life).
    options: XmlReadOptions,
    /// Events published so far (drives `max_events`).
    events: u64,
    /// Latched once the backend reports `Eof`.
    done: bool,
    /// Latched on the first error — subsequent reads keep failing.
    failed: bool,
    /// Declared XML version, for attribute/text normalization.
    version: XmlVersion,
    /// Stream offset at which the current event started — drives the
    /// `max_event_bytes` source-span check.
    last_byte: u64,
}

impl<R: BufRead> fmt::Debug for XmlEventReader<'_, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XmlEventReader")
            .field("options", &self.options)
            .field("events", &self.events)
            .field("done", &self.done)
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

impl<'a, R: BufRead> XmlEventReader<'a, R> {
    /// Builds a reader over `reader` with the given options. The input
    /// byte budget is taken from `options.limits`.
    #[must_use]
    pub fn new(reader: &'a mut R, options: &XmlReadOptions) -> Self {
        let bounded = BoundedRead::new(reader, options.limits.max_bytes);
        let mut inner = NsReader::from_reader(BufReader::with_capacity(READ_CHUNK, bounded));
        inner.config_mut().check_comments = true;
        if let Some(max) = options.max_namespace_bindings {
            inner.resolver_mut().set_max_namespace_bindings(max);
        }
        Self {
            inner,
            options: options.clone(),
            events: 0,
            done: false,
            failed: false,
            version: XmlVersion::Implicit1_0,
            last_byte: 0,
        }
    }

    /// Number of events published so far.
    #[must_use]
    pub fn events_emitted(&self) -> u64 {
        self.events
    }

    /// Byte offset immediately after the most recently read event (or `0`
    /// before the first read). Stays callable while an event is held.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        DocumentPosition {
            byte: Some(self.inner.buffer_position()),
            line: None,
            column: None,
            record: None,
        }
    }

    /// Open-element depth, including the element being ended until the
    /// next read pops its scope. Stays callable while an event is held.
    #[must_use]
    pub fn depth(&self) -> u64 {
        u64::from(self.inner.resolver().level())
    }

    /// Pulls the next event into `buffer`.
    ///
    /// Returns `Ok(None)` at end of document — and on every call after —
    /// or `Err` for malformed input, rejected features (DTD, undefined
    /// entities, non-UTF-8 encodings), or an exceeded limit. After an
    /// `Err` the reader is poisoned and later calls keep reporting a
    /// failure.
    ///
    /// The returned event borrows `buffer` until the next `read_event` on
    /// it or [`XmlEventBuffer::clear`]; use [`XmlEventRef::to_owned`] to
    /// detach.
    ///
    /// # Errors
    ///
    /// - [`DocumentErrorKind::Io`] — underlying read failure.
    /// - [`DocumentErrorKind::InvalidEncoding`] — input that is not
    ///   well-formed UTF-8.
    /// - [`DocumentErrorKind::Syntax`] — malformed markup, mismatched end
    ///   tags, duplicate attributes, undeclared prefixes, malformed
    ///   character references.
    /// - [`DocumentErrorKind::Unsupported`] — DTD/doctype constructs,
    ///   general entity references beyond the predefined five, declared
    ///   encodings other than UTF-8.
    /// - [`DocumentErrorKind::LimitExceeded`] — input bytes, depth,
    ///   attribute-count, event-byte, event-count, or namespace-binding
    ///   limits.
    pub fn read_event<'b>(
        &mut self,
        buffer: &'b mut XmlEventBuffer,
    ) -> Result<Option<XmlEventRef<'b>>, DocumentError> {
        if self.done {
            return Ok(None);
        }
        if self.failed {
            return Err(poisoned_error());
        }
        let result = self.read_event_inner(buffer);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Single read + map step; an error poisons the reader at the caller.
    fn read_event_inner<'b>(
        &mut self,
        buffer: &'b mut XmlEventBuffer,
    ) -> Result<Option<XmlEventRef<'b>>, DocumentError> {
        let event = match self.inner.read_event_into(&mut buffer.raw) {
            Ok(event) => event,
            Err(error) => return Err(self.map_error(error)),
        };
        let position = self.position();
        let event = match event {
            Event::Start(e) => XmlEventRef::Start(self.build_element(&e, position)?),
            Event::Empty(e) => XmlEventRef::Empty(self.build_element(&e, position)?),
            Event::End(e) => XmlEventRef::End(self.resolve_name(e.name(), true, position)?),
            Event::Text(e) => XmlEventRef::Text(match self.version {
                XmlVersion::Explicit1_1 => e.xml11_content(),
                _ => e.xml10_content(),
            }),
            Event::CData(e) => XmlEventRef::CData(e.into_inner()),
            Event::Comment(e) => XmlEventRef::Comment(e.into_inner()),
            Event::PI(e) => XmlEventRef::ProcessingInstruction(XmlPiRef {
                target: Cow::Owned(e.target().to_owned()),
                content: Cow::Owned(e.content().to_owned()),
                position,
            }),
            Event::Decl(e) => XmlEventRef::Declaration(self.build_decl(&e, position)?),
            Event::DocType(_) => {
                return Err(unsupported_error(
                    "doctype declarations and DTD processing",
                    position,
                ));
            }
            Event::GeneralRef(e) => XmlEventRef::Reference(build_reference(&e, position)?),
            Event::Eof => {
                self.done = true;
                return Ok(None);
            }
        };
        let event_bytes = position.byte.unwrap_or(0).saturating_sub(self.last_byte);
        self.last_byte = position.byte.unwrap_or(0);
        if let Some(max) = self.options.max_event_bytes
            && event_bytes > max
        {
            return Err(limit_error(
                DocumentLimit {
                    name: "max_event_bytes",
                    max,
                },
                position,
            ));
        }
        self.events += 1;
        if let Some(max) = self.options.max_events
            && self.events > max
        {
            return Err(limit_error(
                DocumentLimit {
                    name: "max_events",
                    max,
                },
                position,
            ));
        }
        Ok(Some(event))
    }

    /// Builds a resolved element (name + attributes) for `Start`/`Empty`.
    fn build_element<'b>(
        &self,
        element: &BytesStart<'b>,
        position: DocumentPosition,
    ) -> Result<XmlElementRef<'b>, DocumentError> {
        let name = self.resolve_name(element.name(), true, position)?;
        let mut attributes = Vec::new();
        for attribute in element.attributes() {
            let attribute = attribute.map_err(|error| map_attr_error(error, position))?;
            attributes.push(self.build_attribute(&attribute, position)?);
        }
        if let Some(max) = self.options.max_attributes
            && attributes.len() as u64 > max
        {
            return Err(limit_error(
                DocumentLimit {
                    name: "max_attributes",
                    max,
                },
                position,
            ));
        }
        let depth = self.depth();
        if let Some(max) = self.options.max_depth
            && depth > max
        {
            return Err(limit_error(
                DocumentLimit {
                    name: "max_depth",
                    max,
                },
                position,
            ));
        }
        Ok(XmlElementRef {
            name,
            attributes,
            position,
        })
    }

    /// Resolves a qualified name through the backend namespace resolver.
    /// The name parts are copied out, so the input `QName` may borrow the
    /// backend event view while the result carries the event lifetime.
    fn resolve_name<'b>(
        &self,
        qname: QName<'_>,
        is_element: bool,
        position: DocumentPosition,
    ) -> Result<XmlNameRef<'b>, DocumentError> {
        let resolver = self.inner.resolver();
        let (result, local) = if is_element {
            resolver.resolve_element(qname)
        } else {
            resolver.resolve_attribute(qname)
        };
        let namespace = match result {
            ResolveResult::Unbound => None,
            ResolveResult::Bound(ns) => Some(Cow::Owned(ns.as_ref().to_owned())),
            ResolveResult::Unknown(prefix) => {
                return Err(map_ns_error(
                    NamespaceError::UnknownPrefix(prefix),
                    position,
                ));
            }
        };
        Ok(XmlNameRef {
            qualified: Cow::Owned(qname.into_inner().to_owned()),
            prefix: qname
                .prefix()
                .map(|prefix| Cow::Owned(prefix.into_inner().to_owned())),
            local: Cow::Owned(local.into_inner().to_owned()),
            namespace,
        })
    }

    /// Builds a resolved attribute, preserving the raw value.
    fn build_attribute<'b>(
        &self,
        attribute: &Attribute<'_>,
        position: DocumentPosition,
    ) -> Result<XmlAttributeRef<'b>, DocumentError> {
        let name = self.resolve_name(attribute.key, false, position)?;
        Ok(XmlAttributeRef {
            name,
            value: Cow::Owned(attribute.value.clone().into_owned()),
            version: self.version,
        })
    }

    /// Validates and maps the `<?xml ...?>` declaration.
    fn build_decl<'b>(
        &mut self,
        decl: &BytesDecl<'b>,
        position: DocumentPosition,
    ) -> Result<XmlDeclarationRef<'b>, DocumentError> {
        self.version = decl.xml_version().map_err(|error| self.map_error(error))?;
        let version = decl
            .version()
            .map_err(|error| self.map_error(error))?
            .into_owned();
        let encoding = decl
            .encoding()
            .transpose()
            .map_err(|error| map_attr_error(error, position))?
            .map(Cow::into_owned);
        if let Some(encoding) = &encoding
            && !encoding.eq_ignore_ascii_case("utf-8")
            && !encoding.eq_ignore_ascii_case("utf8")
        {
            return Err(unsupported_error(
                "a declared character encoding other than UTF-8",
                position,
            ));
        }
        let standalone = decl
            .standalone()
            .transpose()
            .map_err(|error| map_attr_error(error, position))?
            .map(|value| match value.as_ref() {
                "yes" => Ok(true),
                "no" => Ok(false),
                _ => Err(DocumentError::with_source(
                    DocumentErrorKind::Syntax,
                    FORMAT,
                    XmlErrorDetail::Malformed("standalone must be \"yes\" or \"no\""),
                )),
            })
            .transpose()?;
        Ok(XmlDeclarationRef {
            version: Cow::Owned(version),
            encoding: encoding.map(Cow::Owned),
            standalone,
            position,
        })
    }

    /// Normalizes a backend parser/namespace error into `DocumentError`.
    fn map_error(&self, error: BackendError) -> DocumentError {
        let position = DocumentPosition {
            byte: Some(self.inner.error_position()),
            line: None,
            column: None,
            record: None,
        };
        let bounded = self.inner.get_ref().get_ref();
        match error {
            BackendError::Io(io) if bounded.was_limit_exceeded() => DocumentError::with_parts(
                DocumentErrorKind::LimitExceeded,
                FORMAT,
                Some(position),
                Some(DocumentLimit {
                    name: "max_bytes",
                    max: self.options.limits.max_bytes,
                }),
                Some(Box::new(io)),
            ),
            BackendError::Io(io) => DocumentError::with_parts(
                DocumentErrorKind::Io,
                FORMAT,
                Some(position),
                None,
                Some(Box::new(io)),
            ),
            BackendError::Namespace(error) => map_ns_error(error, position),
            other => DocumentError::with_parts(
                classify_backend(&other),
                FORMAT,
                Some(position),
                None,
                Some(Box::new(other)),
            ),
        }
    }
}

/// Normalizes an attribute-iteration error (duplicates, malformed
/// `key=value` pairs).
fn map_attr_error(error: AttrError, position: DocumentPosition) -> DocumentError {
    DocumentError::with_parts(
        DocumentErrorKind::Syntax,
        FORMAT,
        Some(position),
        None,
        Some(Box::new(error)),
    )
}

/// Normalizes a namespace-resolution error.
fn map_ns_error(error: NamespaceError, position: DocumentPosition) -> DocumentError {
    let (kind, limit) = match &error {
        NamespaceError::TooManyBindings(max) => (
            DocumentErrorKind::LimitExceeded,
            Some(DocumentLimit {
                name: "max_namespace_bindings",
                max: *max as u64,
            }),
        ),
        NamespaceError::TooDeeplyNested(max) => (
            DocumentErrorKind::LimitExceeded,
            Some(DocumentLimit {
                name: "max_depth",
                max: *max as u64,
            }),
        ),
        _ => (DocumentErrorKind::Syntax, None),
    };
    DocumentError::with_parts(kind, FORMAT, Some(position), limit, Some(Box::new(error)))
}

/// Maps a `&name;` / `&#N;` event into an explicit reference payload.
fn build_reference(
    reference: &quick_xml::events::BytesRef<'_>,
    position: DocumentPosition,
) -> Result<XmlReference, DocumentError> {
    let value = match reference.resolve_char_ref() {
        Ok(Some(ch)) => XmlReferenceValue::Char(ch),
        Ok(None) => match resolve_xml_entity(reference.as_ref()) {
            Some(entity) => XmlReferenceValue::Entity(entity),
            None => {
                return Err(unsupported_error(
                    "general entity references other than the five predefined ones",
                    position,
                ));
            }
        },
        Err(error) => {
            return Err(DocumentError::with_parts(
                DocumentErrorKind::Syntax,
                FORMAT,
                Some(position),
                None,
                Some(Box::new(error)),
            ));
        }
    };
    Ok(XmlReference { value, position })
}

/// Kind classification for backend errors that aren't special-cased.
fn classify_backend(error: &BackendError) -> DocumentErrorKind {
    match error {
        BackendError::Io(_) => DocumentErrorKind::Io,
        BackendError::Encoding(_) => DocumentErrorKind::InvalidEncoding,
        BackendError::Syntax(_)
        | BackendError::IllFormed(_)
        | BackendError::InvalidAttr(_)
        | BackendError::Escape(_)
        | BackendError::Namespace(_) => DocumentErrorKind::Syntax,
    }
}

/// Limit-classified error with position attached.
fn limit_error(limit: DocumentLimit, position: DocumentPosition) -> DocumentError {
    DocumentError::with_parts(
        DocumentErrorKind::LimitExceeded,
        FORMAT,
        Some(position),
        Some(limit),
        Some(Box::new(XmlErrorDetail::Limit)),
    )
}

/// Error for a rejected-by-policy construct (DTD, entities, encodings).
fn unsupported_error(feature: &'static str, position: DocumentPosition) -> DocumentError {
    DocumentError::with_parts(
        DocumentErrorKind::Unsupported,
        FORMAT,
        Some(position),
        None,
        Some(Box::new(XmlErrorDetail::Unsupported(feature))),
    )
}

/// Error reported by a poisoned reader after a previous failure.
fn poisoned_error() -> DocumentError {
    DocumentError::with_source(
        DocumentErrorKind::Syntax,
        FORMAT,
        XmlErrorDetail::Malformed("previous error left the stream unrecoverable"),
    )
}

/// Private source attached to [`DocumentError::source`] for adapter-raised
/// rejections, so backend details survive for diagnostics without leaking
/// `quick-xml` types through the public error surface. The `Display` text
/// is intentionally generic and never folded into `DocumentError`'s own.
#[derive(Debug)]
enum XmlErrorDetail {
    /// Feature rejected by the adapter's secure defaults.
    Unsupported(&'static str),
    /// Malformed input detail that doesn't map to a backend error type.
    Malformed(&'static str),
    /// Limit detail carried only in `source`; the named limit is on
    /// `DocumentError::limit`.
    Limit,
}

impl fmt::Display for XmlErrorDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(feature) => {
                write!(f, "the XML document uses an unsupported feature: {feature}")
            }
            Self::Malformed(detail) => write!(f, "malformed XML input: {detail}"),
            Self::Limit => f.write_str("the XML document exceeded a configured limit"),
        }
    }
}

impl Error for XmlErrorDetail {}

/// Reads all XML events from `path` into an owned vector.
///
/// This is the `xml + file` composition helper: the file is opened through
/// `ruviform-io-file` (its open failures surface as
/// [`DocumentFileReadError::Open`]) and the event stream is decoded through
/// [`XmlEventReader`] under `options`. I/O failures mid-read surface as
/// [`DocumentErrorKind::Io`] inside [`DocumentFileReadError::Decode`].
///
/// The whole event list is materialized in memory — for large documents
/// prefer [`XmlEventReader`] directly.
///
/// # Errors
///
/// - [`DocumentFileReadError::Open`] — the path could not be opened.
/// - [`DocumentFileReadError::Decode`] — malformed XML, a rejected feature,
///   an exceeded limit, or a mid-read I/O failure.
///
/// [`DocumentFileReadError::Open`]: crate::DocumentFileReadError::Open
/// [`DocumentFileReadError::Decode`]: crate::DocumentFileReadError::Decode
#[cfg(feature = "file")]
pub fn read_file(
    path: impl AsRef<std::path::Path>,
    options: &XmlReadOptions,
) -> Result<Vec<XmlEvent>, crate::DocumentFileReadError> {
    crate::file::open_document(path, |reader| {
        let mut events = Vec::new();
        let mut xml = XmlEventReader::new(reader, options);
        let mut buffer = XmlEventBuffer::new();
        while let Some(event) = xml.read_event(&mut buffer)? {
            events.push(event.to_owned());
        }
        Ok(events)
    })
}
