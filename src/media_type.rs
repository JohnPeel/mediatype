use super::{error::*, media_type_buf::*, name::*, params::*, parse::*, value::*};
use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

/// A borrowed MediaType.
///
/// ```
/// use mediatype::{names::*, MediaType, Value, WriteParams};
///
/// let mut multipart = MediaType::new(MULTIPART, FORM_DATA);
///
/// let boundary = Value::new("dyEV84n7XNJ").unwrap();
/// multipart.set_param(BOUNDARY, boundary);
/// assert_eq!(
///     multipart.to_string(),
///     "multipart/form-data; boundary=dyEV84n7XNJ"
/// );
///
/// multipart.subty = RELATED;
/// assert_eq!(
///     multipart.to_string(),
///     "multipart/related; boundary=dyEV84n7XNJ"
/// );
///
/// const IMAGE_SVG: MediaType = MediaType::from_parts(IMAGE, SVG, Some(XML), &[]);
/// let svg = MediaType::parse("IMAGE/SVG+XML").unwrap();
/// assert_eq!(svg, IMAGE_SVG);
/// ```
#[derive(Debug, Clone)]
pub struct MediaType<'a> {
    /// Top-level type.
    pub ty: Name<'a>,

    /// Subtype.
    pub subty: Name<'a>,

    /// Optional suffix.
    pub suffix: Option<Name<'a>>,

    params: Cow<'a, [(Name<'a>, Value<'a>)]>,
}

impl<'a> MediaType<'a> {
    /// Constructs a `MediaType` from a top-level type and a subtype.
    /// ```
    /// # use mediatype::{names::*, MediaType};
    /// const IMAGE_PNG: MediaType = MediaType::new(IMAGE, PNG);
    /// assert_eq!(IMAGE_PNG, MediaType::parse("image/png").unwrap());
    /// ```
    pub const fn new(ty: Name<'a>, subty: Name<'a>) -> Self {
        Self {
            ty,
            subty,
            suffix: None,
            params: Cow::Borrowed(&[]),
        }
    }

    /// Constructs a `MediaType` with an optional suffix and parameters.
    ///
    /// ```
    /// # use mediatype::{names::*, values::*, MediaType};
    /// const IMAGE_SVG: MediaType = MediaType::from_parts(IMAGE, SVG, Some(XML), &[(CHARSET, UTF_8)]);
    /// assert_eq!(
    ///     IMAGE_SVG,
    ///     MediaType::parse("image/svg+xml; charset=UTF-8").unwrap()
    /// );
    /// ```
    pub const fn from_parts(
        ty: Name<'a>,
        subty: Name<'a>,
        suffix: Option<Name<'a>>,
        params: &'a [(Name<'a>, Value<'a>)],
    ) -> Self {
        Self {
            ty,
            subty,
            suffix,
            params: Cow::Borrowed(params),
        }
    }

    pub(crate) const fn from_parts_unchecked(
        ty: Name<'a>,
        subty: Name<'a>,
        suffix: Option<Name<'a>>,
        params: Cow<'a, [(Name<'a>, Value<'a>)]>,
    ) -> Self {
        Self {
            ty,
            subty,
            suffix,
            params,
        }
    }

    /// Constructs a `MediaType` from `str` without copying the string.
    pub fn parse<'s: 'a>(s: &'s str) -> Result<Self, MediaTypeError> {
        let (indices, _) = Indices::parse(s)?;
        let params = indices
            .params()
            .iter()
            .map(|param| {
                (
                    Name::new_unchecked(&s[param[0] as usize..param[1] as usize]),
                    Value::new_unchecked(&s[param[2] as usize..param[3] as usize]),
                )
            })
            .collect();
        Ok(Self {
            ty: Name::new_unchecked(&s[indices.ty()]),
            subty: Name::new_unchecked(&s[indices.subty()]),
            suffix: indices.suffix().map(|range| Name::new_unchecked(&s[range])),
            params: Cow::Owned(params),
        })
    }
}

impl<'a> ReadParams for MediaType<'a> {
    fn params(&self) -> Params {
        Params::from_slice(&self.params)
    }

    fn get_param(&self, key: Name) -> Option<Value> {
        self.params
            .iter()
            .rev()
            .find(|&&param| key == param.0)
            .map(|&(_, value)| value)
    }
}

impl<'a> WriteParams<'a> for MediaType<'a> {
    fn set_param<'k: 'a, 'v: 'a>(&mut self, key: Name<'k>, value: Value<'v>) {
        self.remove_params(key);
        self.params.to_mut().push((key, value));
    }

    fn remove_params(&mut self, key: Name) {
        let key_exists = self.params.iter().any(|&param| key == param.0);
        if key_exists {
            self.params.to_mut().retain(|&param| key != param.0);
        }
    }

    fn clear_params(&mut self) {
        if !self.params.is_empty() {
            self.params.to_mut().clear();
        }
    }
}

impl<'a> fmt::Display for MediaType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.ty, self.subty)?;
        if let Some(suffix) = self.suffix {
            write!(f, "+{}", suffix)?;
        }
        for (key, value) in &*self.params {
            write!(f, "; {}={}", key, value)?;
        }
        Ok(())
    }
}

impl<'a> From<&'a MediaTypeBuf> for MediaType<'a> {
    fn from(t: &'a MediaTypeBuf) -> Self {
        t.to_ref()
    }
}

impl<'a> PartialEq for MediaType<'a> {
    fn eq(&self, other: &MediaType) -> bool {
        self.ty == other.ty
            && self.subty == other.subty
            && self.suffix == other.suffix
            && self.params().eq(other.params())
    }
}

impl<'a> Eq for MediaType<'a> {}

impl<'a> PartialOrd for MediaType<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for MediaType<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ty.cmp(&other.ty) {
            Ordering::Equal => (),
            ne => return ne,
        }
        match self.subty.cmp(&other.subty) {
            Ordering::Equal => (),
            ne => return ne,
        }
        match self.suffix.cmp(&other.suffix) {
            Ordering::Equal => (),
            ne => return ne,
        }
        self.params().cmp(other.params())
    }
}

impl<'a> Hash for MediaType<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.subty.hash(state);
        self.suffix.hash(state);
        for param in self.params() {
            param.hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{names::*, values::*};

    #[test]
    fn to_string() {
        assert_eq!(MediaType::new(TEXT, PLAIN).to_string(), "text/plain");
        assert_eq!(
            MediaType::from_parts(IMAGE, SVG, Some(XML), &[]).to_string(),
            "image/svg+xml"
        );
        assert_eq!(
            MediaType::from_parts(TEXT, PLAIN, None, &[(CHARSET, UTF_8)]).to_string(),
            "text/plain; charset=UTF-8"
        );
        assert_eq!(
            MediaType::from_parts(IMAGE, SVG, Some(XML), &[(CHARSET, UTF_8)]).to_string(),
            "image/svg+xml; charset=UTF-8"
        );
    }

    #[test]
    fn get_param() {
        assert_eq!(MediaType::new(TEXT, PLAIN).get_param(CHARSET), None);
        assert_eq!(
            MediaType::from_parts(TEXT, PLAIN, None, &[(CHARSET, UTF_8)]).get_param(CHARSET),
            Some(UTF_8)
        );
        assert_eq!(
            MediaType::parse("image/svg+xml; charset=UTF-8; HELLO=WORLD; HELLO=world")
                .unwrap()
                .get_param(Name::new("hello").unwrap()),
            Some(Value::new("world").unwrap())
        );
    }

    #[test]
    fn set_param() {
        let mut media_type = MediaType::from_parts(TEXT, PLAIN, None, &[(CHARSET, UTF_8)]);
        let lower_utf8 = Value::new("utf-8").unwrap();
        media_type.set_param(CHARSET, lower_utf8);
        assert_eq!(media_type.to_string(), "text/plain; charset=utf-8");

        let alice = Name::new("ALICE").unwrap();
        let bob = Value::new("bob").unwrap();
        media_type.set_param(alice, bob);
        media_type.set_param(alice, bob);

        assert_eq!(
            media_type.to_string(),
            "text/plain; charset=utf-8; ALICE=bob"
        );
    }

    #[test]
    fn remove_params() {
        let mut media_type = MediaType::from_parts(TEXT, PLAIN, None, &[(CHARSET, UTF_8)]);
        media_type.remove_params(CHARSET);
        assert_eq!(media_type.to_string(), "text/plain");

        let mut media_type =
            MediaType::parse("image/svg+xml; hello=WORLD; charset=UTF-8; HELLO=WORLD").unwrap();
        media_type.remove_params(Name::new("hello").unwrap());
        assert_eq!(media_type.to_string(), "image/svg+xml; charset=UTF-8");
    }

    #[test]
    fn clear_params() {
        let mut media_type = MediaType::parse("image/svg+xml; charset=UTF-8; HELLO=WORLD").unwrap();
        media_type.clear_params();
        assert_eq!(media_type.to_string(), "image/svg+xml");
    }

    #[test]
    fn cmp() {
        assert_eq!(
            MediaType::parse("text/plain").unwrap(),
            MediaType::parse("TEXT/PLAIN").unwrap()
        );
        assert_eq!(
            MediaType::parse("image/svg+xml; charset=UTF-8").unwrap(),
            MediaType::parse("IMAGE/SVG+XML; CHARSET=UTF-8").unwrap()
        );
        assert_eq!(
            MediaType::parse("image/svg+xml; hello=WORLD; charset=UTF-8").unwrap(),
            MediaType::parse("IMAGE/SVG+XML; HELLO=WORLD; CHARSET=UTF-8").unwrap()
        );
    }
}
