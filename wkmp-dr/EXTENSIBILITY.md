# wkmp-dr Extensibility Guide

## Adding New Filters

wkmp-dr uses a modular filter system that makes it easy to add new predefined database queries without modifying core routing logic.

### Architecture

**Filter Module Location:** `wkmp-dr/src/api/filters.rs`

Each filter is implemented as an async function that:
1. Accepts `State(AppState)` and `Query(FilterQuery)` parameters
2. Executes a SQL query with pagination
3. Returns `Result<Json<FilterResponse>, FilterError>`

### Step-by-Step: Adding a New Filter

#### 1. Add Filter Function to `filters.rs`

```rust
/// Filter: Songs without albums
/// Returns songs that aren't linked to any album
pub async fn songs_without_albums(
    State(state): State<AppState>,
    Query(query): Query<FilterQuery>,
) -> Result<Json<FilterResponse>, FilterError> {
    const PAGE_SIZE: i64 = 100;

    // Get total count
    let total_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM songs
         WHERE guid NOT IN (SELECT DISTINCT song_id FROM album_songs)"
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

    // Calculate pagination
    let total_pages = (total_results + PAGE_SIZE - 1) / PAGE_SIZE;
    let page = query.page.unwrap_or(1).max(1).min(total_pages.max(1));
    let offset = (page - 1) * PAGE_SIZE;

    // Fetch paginated results
    let rows = sqlx::query(
        "SELECT guid, recording_mbid, work_id, created_at
         FROM songs
         WHERE guid NOT IN (SELECT DISTINCT song_id FROM album_songs)
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?"
    )
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

    // Format response
    let columns = vec!["guid", "recording_mbid", "work_id", "created_at"];
    let row_data: Vec<Vec<Value>> = rows.iter().map(|row| {
        vec![
            row.try_get::<String, _>(0).ok().map(|v| json!(v)).unwrap_or(Value::Null),
            row.try_get::<Option<String>, _>(1).ok().flatten().map(|v| json!(v)).unwrap_or(Value::Null),
            row.try_get::<Option<String>, _>(2).ok().flatten().map(|v| json!(v)).unwrap_or(Value::Null),
            row.try_get::<String, _>(3).ok().map(|v| json!(v)).unwrap_or(Value::Null),
        ]
    }).collect();

    Ok(Json(FilterResponse {
        filter_name: "songs-without-albums".to_string(),
        description: Some("Songs not linked to any album".to_string()),
        columns,
        rows: row_data,
        total_results,
        page,
        total_pages,
    }))
}
```

#### 2. Export Filter from `api/filters.rs`

At the top of `filters.rs`, add:
```rust
pub use songs_without_albums;
```

#### 3. Add Route to Router

In `wkmp-dr/src/lib.rs`, update the router:

```rust
let protected = Router::new()
    .route("/api/table/:name", get(api::get_table_data))
    .route("/api/filters/passages-without-mbid", get(api::passages_without_mbid))
    .route("/api/filters/files-without-passages", get(api::files_without_passages))
    .route("/api/filters/songs-without-albums", get(api::songs_without_albums))  // NEW
    .route("/api/search/by-work-id", get(api::search_by_work_id))
    .route("/api/search/by-path", get(api::search_by_path))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        api::auth_middleware,
    ));
```

#### 4. Add UI Option

In `wkmp-dr/src/ui/index.html`, add the filter to the dropdown:

```html
<select id="viewType">
    <option value="table">Browse Table</option>
    <option value="filter-passages">Filter: Passages without MBID</option>
    <option value="filter-files">Filter: Files without passages</option>
    <option value="filter-songs">Filter: Songs without albums</option>  <!-- NEW -->
    <option value="search-work">Search: By Work ID</option>
    <option value="search-path">Search: By File Path</option>
</select>
```

#### 5. Add JavaScript Handler

In `wkmp-dr/src/ui/app.js`, update the `loadData()` function:

```javascript
} else if (viewType === 'filter-files') {
    url = `/api/filters/files-without-passages?page=${currentPage}`;
} else if (viewType === 'filter-songs') {  // NEW
    url = `/api/filters/songs-without-albums?page=${currentPage}`;
} else if (viewType === 'search-work') {
```

### Filter Naming Conventions

- **Route:** `/api/filters/{filter-name-kebab-case}`
- **Function:** `{filter_name_snake_case}`
- **UI dropdown value:** `filter-{shortname}`
- **Response filter_name:** `{filter-name-kebab-case}`

### Testing Your Filter

```bash
# Build
cargo build -p wkmp-dr

# Run server
cargo run -p wkmp-dr

# Test API endpoint
curl -s "http://127.0.0.1:5725/api/filters/songs-without-albums?page=1" | jq

# Test UI
# Open http://127.0.0.1:5725/ and select your filter from dropdown
```

### Common Filter Patterns

**Find orphaned records (no relationships):**
```sql
SELECT * FROM table1
WHERE guid NOT IN (SELECT DISTINCT table1_id FROM junction_table)
```

**Find records with missing optional fields:**
```sql
SELECT * FROM table WHERE optional_field IS NULL
```

**Find records matching specific criteria:**
```sql
SELECT * FROM table WHERE condition = true
```

### Performance Considerations

- Always use pagination (LIMIT/OFFSET)
- Add ORDER BY for consistent results
- Create indexes on frequently filtered columns
- Use `COUNT(*)` separately for total count (more efficient than counting result set)

### Error Handling

All filters return `Result<Json<FilterResponse>, FilterError>` where:
- `DatabaseError`: SQL execution failed
- Return meaningful error messages via `FilterError::DatabaseError(e.to_string())`

### Best Practices

1. **Keep filters simple** - One clear purpose per filter
2. **Document SQL queries** - Comment the business logic
3. **Test with real data** - Verify results make sense
4. **Consider performance** - Large result sets should use appropriate indexes
5. **Follow existing patterns** - Match the style of `passages_without_mbid` and `files_without_passages`

### Example: Multi-Join Filter

```rust
/// Filter: Passages with multiple songs
pub async fn passages_with_multiple_songs(/*...*/) -> Result</*...*/, FilterError> {
    let total_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT p.guid)
         FROM passages p
         JOIN passage_songs ps ON p.guid = ps.passage_id
         GROUP BY p.guid
         HAVING COUNT(ps.song_id) > 1"
    )
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query(
        "SELECT p.guid, p.file_id, COUNT(ps.song_id) as song_count
         FROM passages p
         JOIN passage_songs ps ON p.guid = ps.passage_id
         GROUP BY p.guid
         HAVING COUNT(ps.song_id) > 1
         ORDER BY song_count DESC
         LIMIT ? OFFSET ?"
    )
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    // Format response...
}
```

This extensible architecture allows new filters to be added with minimal changes to existing code.
