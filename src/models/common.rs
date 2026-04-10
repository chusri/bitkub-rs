use serde::{Deserialize, Serialize};

/// V3 API response wrapper.
///
/// Most V3 endpoints return `{"error": 0, "result": ...}` where `error == 0`
/// indicates success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub error: i32,
    pub result: Option<T>,
}

/// V4 API response wrapper.
///
/// V4 endpoints return `{"code": "...", "message": "...", "data": ...}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiV4Response<T> {
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

/// V4 paginated response containing items plus page metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedItems<T> {
    pub page: i32,
    pub total_page: i32,
    pub total_item: i32,
    pub items: Vec<T>,
}

/// V3 pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: i32,
    pub last: i32,
    pub next: Option<i32>,
    pub prev: Option<i32>,
}

/// Keyset-based pagination using an opaque cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetPagination {
    pub cursor: Option<String>,
    pub has_next: bool,
}
