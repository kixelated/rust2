// This fixture deliberately uses non-idiomatic, all-caps-acronym type names
// (`URLString`, `HTTPMetadata`, `APIResult`) to exercise identifier-casing in
// the Dart generator. That is exactly what `clippy::upper_case_acronyms`
// (in aggressive mode) would flag, so allow it here on purpose.
#![allow(clippy::upper_case_acronyms)]

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonBuffer(pub Vec<u8>);

uniffi::custom_newtype!(JsonBuffer, Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenEngineTrace {
    pub id: String,
    pub value: JsonBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenEngineResponse {
    pub performance: String,
    pub result: JsonBuffer,
    pub trace: Option<HashMap<String, ZenEngineTrace>>,
}

pub fn get_zen_engine_response() -> ZenEngineResponse {
    ZenEngineResponse {
        performance: "ready".to_string(),
        result: JsonBuffer(vec![1, 2, 3]),
        trace: Some(HashMap::from([(
            "primary".to_string(),
            ZenEngineTrace { id: "primary".to_string(), value: JsonBuffer(vec![4, 5, 6]) },
        )])),
    }
}

pub fn return_zen_engine_response(response: ZenEngineResponse) -> ZenEngineResponse {
    response
}

// Regression coverage for identifier casing of names containing all-caps
// acronyms, so `class_name()` normalizes `URLString` to `UrlString`, for example.
// Here we're testing several examples in different positions.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct URLString(pub String);

uniffi::custom_newtype!(URLString, String);

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HTTPMetadata {
    pub url: URLString,
    pub status: u32,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct APIResult {
    pub primary: HTTPMetadata,
    pub fallback: Option<HTTPMetadata>,
}

pub fn roundtrip_api_result(value: APIResult) -> APIResult {
    value
}

uniffi::include_scaffolding!("api");
