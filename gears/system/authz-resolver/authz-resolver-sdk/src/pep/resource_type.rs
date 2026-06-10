//! [`ResourceType`] — descriptor for a resource type and its supported
//! constraint properties.

use std::borrow::Cow;

/// Descriptor for a resource type and its supported constraint properties.
///
/// Passed per call to [`PolicyEnforcer`](crate::pep::PolicyEnforcer) methods so
/// a single enforcer can serve multiple resource types within one service.
///
/// Construct with [`ResourceType::from_static`] for compile-time literals
/// (typical case) or [`ResourceType::new`] for runtime-built names such as
/// chained GTS schema ids (e.g. `gts.cf.core.am.tenant_metadata.v1~<chain>~`).
#[derive(Debug, Clone)]
pub struct ResourceType {
    name: Cow<'static, str>,
    supported_properties: &'static [&'static str],
}

impl ResourceType {
    /// Create a descriptor from a compile-time string literal.
    ///
    /// Usable in `const` context; performs no allocation.
    #[must_use]
    pub const fn from_static(
        name: &'static str,
        supported_properties: &'static [&'static str],
    ) -> Self {
        Self {
            name: Cow::Borrowed(name),
            supported_properties,
        }
    }

    /// Create a descriptor with a name owned or borrowed at runtime.
    ///
    /// Accepts any value convertible to [`Cow<'static, str>`] — `&'static str`,
    /// [`String`], or [`Cow<'static, str>`]. Use this when the resource type
    /// name is built at runtime (e.g. a chained GTS schema id whose tail
    /// depends on a configured plugin).
    #[must_use]
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        supported_properties: &'static [&'static str],
    ) -> Self {
        Self {
            name: name.into(),
            supported_properties,
        }
    }

    /// Dotted resource type name (e.g. `"gts.cf.core.users.user.v1~"`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Properties the PEP can compile from PDP constraints.
    #[must_use]
    pub fn supported_properties(&self) -> &'static [&'static str] {
        self.supported_properties
    }
}
