//! 検証エラー型。
//!
//! 公開 API は panic-free とし、契約違反の入力値（長さ不一致・非 2 冪・
//! レート 0 等）は `Error::InvalidValue` で返す。`&'static str` には違反した
//! フィールドを示す短いタグを入れる（人間可読メッセージの多言語化・確保を
//! 避けるため固定タグとする）。

/// 検証エラー。Copy・alloc なしの値型。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// 契約違反の入力値。タグは違反フィールドを示す固定文字列
    /// （`"length"`・`"n"`・`"rate"` 等）。
    InvalidValue(&'static str),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidValue(field) => write!(f, "invalid value: {field}"),
        }
    }
}

impl std::error::Error for Error {}
