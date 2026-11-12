// Generated TypeSafe System One types using typify
// Source: specs/typesafe/openapi.json
// Regenerate with `cargo run --bin generate-types -- typesafe`
#![allow(clippy::clone_on_copy, clippy::derivable_impls)]

/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
///An answer whose type matches the corresponding question.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Answer {
    ///NoulAnswer
    ///
    ///The probability of a yes answer or a true statement.
    #[serde(rename = "noul")]
    Noul { noul: f64 },
    ///ChoiceAnswer
    ///
    ///The selected choice, confidence, and probabilities for a choice question.
    #[serde(rename = "choice")]
    Choice {
        ///The name of the choice with the highest probability among the question's criteria.
        choice: ::std::string::String,
        confidence: f64,
        ///Probability of each choice in criteria, keyed by choice name, from 0 to 1. Shows how likely the alternatives are; values sum to approximately 1.
        probabilities: ::indexmap::IndexMap<::std::string::String, f64>,
    },
    ///ScoreAnswer
    ///
    ///An expected score with its rubric, confidence, and score-level probabilities.
    #[serde(rename = "score")]
    Score {
        confidence: f64,
        ///The requested criteria mapped to their score levels, so you can interpret the score.
        legend: ::indexmap::IndexMap<::std::string::String, super::TypeSafeEntry>,
        ///Probability of each score level, from 0 to 1, using the same keys as legend. Shows how likely the alternatives are; values sum to approximately 1.
        probabilities: ::indexmap::IndexMap<::std::string::String, f64>,
        score: f64,
    },
    ///BoxAnswer
    ///
    ///A located box with normalized edge coordinates and confidence.
    #[serde(rename = "box")]
    Box {
        ///The box's edges as fractions of image height and width, from 0 to 1. Each coordinate is the expected value of its edge's predicted position.
        #[serde(rename = "box")]
        box_: BoxCoordinates,
        ///Confidence for the given box, from 0 to 1, with 1 being more confident. Null when the model does not provide confidence.
        confidence: ::std::option::Option<f64>,
    },
}
impl ::std::convert::From<&Self> for Answer {
    fn from(value: &Answer) -> Self {
        value.clone()
    }
}
///An image that replaces its placeholder: its bytes in data_base64 with their format, or, in a multipart request, the name of the image part that holds them. Requests with images must send the header TypeSafe-Danger-Zone-Images: true.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
pub struct Asset {
    ///Base64-encoded image data.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub data_base64: ::std::option::Option<::std::string::String>,
    ///The image format, required with data_base64. The actual format is detected from the bytes.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub format: ::std::option::Option<Format>,
    ///In a multipart request, the name of the image part this asset uses. With part, data_base64 is ignored.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub part: ::std::option::Option<::std::string::String>,
    ///Identifies an image asset.
    #[serde(
        rename = "type",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub type_: ::std::option::Option<Type>,
}
impl ::std::convert::From<&Asset> for Asset {
    fn from(value: &Asset) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for Asset {
    fn default() -> Self {
        Self {
            data_base64: Default::default(),
            format: Default::default(),
            part: Default::default(),
            type_: Default::default(),
        }
    }
}
///Normalized box edges. Vertical coordinates are measured from the top of the image; horizontal coordinates are measured from the left.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
pub struct BoxCoordinates {
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
    pub top: f64,
}
impl ::std::convert::From<&BoxCoordinates> for BoxCoordinates {
    fn from(value: &BoxCoordinates) -> Self {
        value.clone()
    }
}
///The image format, required with data_base64. The actual format is detected from the bytes.
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum Format {
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "jpg")]
    Jpg,
    #[serde(rename = "jpeg")]
    Jpeg,
}
impl ::std::convert::From<&Self> for Format {
    fn from(value: &Format) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for Format {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Png => f.write_str("png"),
            Self::Jpg => f.write_str("jpg"),
            Self::Jpeg => f.write_str("jpeg"),
        }
    }
}
impl ::std::str::FromStr for Format {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "png" => Ok(Self::Png),
            "jpg" => Ok(Self::Jpg),
            "jpeg" => Ok(Self::Jpeg),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for Format {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for Format {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Format {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///Criteria defining what counts as a yes or no answer.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
pub struct NoulCriteria {
    ///What counts as a no answer.
    #[serde(
        rename = "false",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub false_: ::std::option::Option<super::TypeSafeEntry>,
    ///What counts as a yes answer.
    #[serde(
        rename = "true",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub true_: ::std::option::Option<super::TypeSafeEntry>,
}
impl ::std::convert::From<&NoulCriteria> for NoulCriteria {
    fn from(value: &NoulCriteria) -> Self {
        value.clone()
    }
}
impl ::std::default::Default for NoulCriteria {
    fn default() -> Self {
        Self {
            false_: Default::default(),
            true_: Default::default(),
        }
    }
}
///A question about the supplied content.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Question {
    ///NoulQuestion
    ///
    ///A yes/no question or statement, answered with the probability of yes or true.
    #[serde(rename = "noul")]
    Noul {
        ///Criteria clarifying what counts as a yes or no answer.
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        criteria: ::std::option::Option<NoulCriteria>,
        ///The yes/no question or statement to evaluate.
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        instructions: ::std::option::Option<super::TypeSafeEntry>,
    },
    ///ChoiceQuestion
    ///
    ///A question that selects one option from the choices you define.
    #[serde(rename = "choice")]
    Choice {
        ///Choice names and descriptions of when each applies. A choice without a description is interpreted by its name alone.
        criteria: ::indexmap::IndexMap<
            ::std::string::String,
            ::std::option::Option<super::TypeSafeEntry>,
        >,
        ///What the model should decide when choosing an option.
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        instructions: ::std::option::Option<super::TypeSafeEntry>,
    },
    ///ScoreQuestion
    ///
    ///A question that assigns a score using an ordered rubric.
    #[serde(rename = "score")]
    Score {
        ///Ordered descriptions of the score levels. Each description's position determines its score, starting at zero.
        criteria: ::std::vec::Vec<super::TypeSafeEntry>,
        ///What the model should rate.
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        instructions: ::std::option::Option<super::TypeSafeEntry>,
    },
    ///BoxQuestion
    ///
    ///A question that locates what its instructions describe in the request's image. Requests with box questions must send the header TypeSafe-Danger-Zone-Images: true.
    #[serde(rename = "box")]
    Box {
        ///What to locate in the image.
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        instructions: ::std::option::Option<super::TypeSafeEntry>,
    },
}
impl ::std::convert::From<&Self> for Question {
    fn from(value: &Question) -> Self {
        value.clone()
    }
}
///Content and named questions to evaluate together using a TypeSafe model.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
pub struct SystemOneRequest {
    ///Media assets keyed by an opaque placeholder (recommend a per-image UUID). Each placeholder, wherever it appears as or inside a string in state, question instructions, or criteria, is replaced with the asset. Image assets require a vision-language model.
    #[serde(default, skip_serializing_if = ":: indexmap :: IndexMap::is_empty")]
    pub assets: ::indexmap::IndexMap<::std::string::String, Asset>,
    ///Name or alias of the model to use. Available names are returned by GET /v1/models.
    pub model: ::std::string::String,
    ///Questions to ask about the content, each with a name you choose. The response uses those names to identify the answers.
    pub questions: ::indexmap::IndexMap<::std::string::String, Question>,
    ///The content all questions in this request refer to.
    pub state: super::TypeSafeEntry,
}
impl ::std::convert::From<&SystemOneRequest> for SystemOneRequest {
    fn from(value: &SystemOneRequest) -> Self {
        value.clone()
    }
}
///Answers grouped by question name, with the model used and token usage.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
pub struct SystemOneResponse {
    ///Answers keyed by the question names supplied in the request. Each answer's type matches its question's type.
    pub answers: ::indexmap::IndexMap<::std::string::String, Answer>,
    ///Name of the model that answered the questions. May differ from the alias supplied in the request.
    pub model: ::std::string::String,
    ///Input and output token counts for this evaluation.
    pub usage: Usage,
}
impl ::std::convert::From<&SystemOneResponse> for SystemOneResponse {
    fn from(value: &SystemOneResponse) -> Self {
        value.clone()
    }
}
///Identifies an image asset.
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum Type {
    #[serde(rename = "image")]
    Image,
}
impl ::std::convert::From<&Self> for Type {
    fn from(value: &Type) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for Type {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Image => f.write_str("image"),
        }
    }
}
impl ::std::str::FromStr for Type {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "image" => Ok(Self::Image),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for Type {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for Type {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Type {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///Token usage for the request.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
pub struct Usage {
    ///Number of billable input tokens used to evaluate the request.
    pub input_tokens: i64,
    ///Number of output tokens used to answer the questions. Output tokens are currently free of charge.
    pub output_tokens: i64,
}
impl ::std::convert::From<&Usage> for Usage {
    fn from(value: &Usage) -> Self {
        value.clone()
    }
}
