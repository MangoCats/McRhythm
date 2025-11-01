//! Pagination utilities for wkmp-dr
//!
//! [REQ-DR-F-020]: Paginated browsing (100 rows/page)

/// Page size constant for all pagination
pub const PAGE_SIZE: i64 = 100;

/// Pagination metadata calculated from total results
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// Current page number (1-indexed)
    pub page: i64,
    /// Total number of pages
    pub total_pages: i64,
    /// Offset for SQL LIMIT/OFFSET query
    pub offset: i64,
}

/// Calculate pagination metadata from total results and requested page
///
/// Ensures page is within valid bounds [1, total_pages]
///
/// # Arguments
/// * `total_results` - Total number of rows in result set
/// * `requested_page` - Page number requested by user (may be out of bounds)
///
/// # Returns
/// Pagination metadata with sanitized page number and calculated offset
///
/// # Examples
/// ```
/// use wkmp_dr::pagination::calculate_pagination;
///
/// // 250 total results = 3 pages (100 + 100 + 50)
/// let p = calculate_pagination(250, 2);
/// assert_eq!(p.page, 2);
/// assert_eq!(p.total_pages, 3);
/// assert_eq!(p.offset, 100);
///
/// // Requesting out-of-bounds page gets clamped
/// let p = calculate_pagination(250, 99);
/// assert_eq!(p.page, 3);  // Clamped to last page
/// assert_eq!(p.offset, 200);
/// ```
pub fn calculate_pagination(total_results: i64, requested_page: i64) -> Pagination {
    let total_pages = (total_results + PAGE_SIZE - 1) / PAGE_SIZE;
    let page = requested_page.max(1).min(total_pages.max(1));
    let offset = (page - 1) * PAGE_SIZE;

    Pagination {
        page,
        total_pages,
        offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_normal() {
        let p = calculate_pagination(250, 2);
        assert_eq!(p.page, 2);
        assert_eq!(p.total_pages, 3);
        assert_eq!(p.offset, 100);
    }

    #[test]
    fn test_pagination_first_page() {
        let p = calculate_pagination(150, 1);
        assert_eq!(p.page, 1);
        assert_eq!(p.total_pages, 2);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn test_pagination_last_page() {
        let p = calculate_pagination(250, 3);
        assert_eq!(p.page, 3);
        assert_eq!(p.total_pages, 3);
        assert_eq!(p.offset, 200);
    }

    #[test]
    fn test_pagination_out_of_bounds_high() {
        let p = calculate_pagination(150, 99);
        assert_eq!(p.page, 2);  // Clamped to last page
        assert_eq!(p.total_pages, 2);
        assert_eq!(p.offset, 100);
    }

    #[test]
    fn test_pagination_out_of_bounds_low() {
        let p = calculate_pagination(150, 0);
        assert_eq!(p.page, 1);  // Clamped to first page
        assert_eq!(p.total_pages, 2);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn test_pagination_empty() {
        let p = calculate_pagination(0, 1);
        assert_eq!(p.page, 1);
        assert_eq!(p.total_pages, 0);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn test_pagination_exact_page_boundary() {
        let p = calculate_pagination(200, 2);
        assert_eq!(p.page, 2);
        assert_eq!(p.total_pages, 2);
        assert_eq!(p.offset, 100);
    }
}
