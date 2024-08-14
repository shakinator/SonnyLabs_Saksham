use axum::http::header::{
    HeaderMap,
    HeaderName,
    HeaderValue,
    InvalidHeaderName,
    InvalidHeaderValue,
};

#[derive(Debug)]
pub enum HeaderParseError {
    Name(InvalidHeaderName),
    Value(InvalidHeaderValue)
}
pub fn into_headers(headers: &[(&[u8], &str)]) -> Result<HeaderMap, HeaderParseError> {
    let mut hm = HeaderMap::new();
    for h in headers.iter() {
        let name = HeaderName::from_lowercase(h.0)
            .map_err(|e| HeaderParseError::Name(e))?;
        let value = HeaderValue::from_str(h.1)
            .map_err(|e| HeaderParseError::Value(e))?;

        hm.insert(name, value);
    }

    Ok(hm)
}
