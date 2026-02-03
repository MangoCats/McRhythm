    use super::*;

    #[test]
    fn test_global_params_has_all_fields() {
        // TC-U-001-01: Verify all 15 parameter fields exist
        let params = GlobalParams::default();

        // DBD-PARAM-010 - dereference to avoid lock warning
        let _: f32 = *params.volume_level.read().unwrap();

        // DBD-PARAM-020
        let _: u32 = *params.working_sample_rate.read().unwrap();

        // DBD-PARAM-030
        let _: usize = *params.output_ringbuffer_size.read().unwrap();

        // DBD-PARAM-050
        let _: usize = *params.maximum_decode_streams.read().unwrap();

        // DBD-PARAM-060
        let _: u64 = *params.decode_work_period.read().unwrap();

        // DBD-PARAM-065
        let _: u64 = *params.chunk_duration_ms.read().unwrap();

        // DBD-PARAM-070
        let _: usize = *params.playout_ringbuffer_size.read().unwrap();

        // DBD-PARAM-080
        let _: usize = *params.playout_ringbuffer_headroom.read().unwrap();

        // DBD-PARAM-085
        let _: u64 = *params.decoder_resume_hysteresis_samples.read().unwrap();

        // DBD-PARAM-088
        let _: usize = *params.mixer_min_start_level.read().unwrap();

        // DBD-PARAM-090
        let _: f64 = *params.pause_decay_factor.read().unwrap();

        // DBD-PARAM-100
        let _: f64 = *params.pause_decay_floor.read().unwrap();

        // DBD-PARAM-110
        let _: u32 = *params.audio_buffer_size.read().unwrap();

        // DBD-PARAM-111
        let _: u64 = *params.mixer_check_interval_ms.read().unwrap();

        // If we reach here, all 15 fields exist and are accessible
        assert!(true, "All 15 parameter fields exist");
    }

    #[test]
    fn test_parameter_field_types() {
        // TC-U-001-01: Verify types (compile-time check via type inference)
        let params = GlobalParams::default();

        let _: f32 = *params.volume_level.read().unwrap();
        let _: u32 = *params.working_sample_rate.read().unwrap();
        let _: usize = *params.output_ringbuffer_size.read().unwrap();
        let _: usize = *params.maximum_decode_streams.read().unwrap();
        let _: u64 = *params.decode_work_period.read().unwrap();
        let _: u64 = *params.chunk_duration_ms.read().unwrap();
        let _: usize = *params.playout_ringbuffer_size.read().unwrap();
        let _: usize = *params.playout_ringbuffer_headroom.read().unwrap();
        let _: u64 = *params.decoder_resume_hysteresis_samples.read().unwrap();
        let _: usize = *params.mixer_min_start_level.read().unwrap();
        let _: f64 = *params.pause_decay_factor.read().unwrap();
        let _: f64 = *params.pause_decay_floor.read().unwrap();
        let _: u32 = *params.audio_buffer_size.read().unwrap();
        let _: u64 = *params.mixer_check_interval_ms.read().unwrap();
    }

    #[test]
    fn test_default_values() {
        // TC-U-001-02: Verify default values match SPEC016
        let params = GlobalParams::default();

        assert_eq!(*params.volume_level.read().unwrap(), 0.5);
        assert_eq!(*params.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 8192); // [DBD-PARAM-030] 8192 frames = 186ms @ 44.1kHz
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 12);
        assert_eq!(*params.decode_work_period.read().unwrap(), 5000);
        assert_eq!(*params.chunk_duration_ms.read().unwrap(), 1000);
        assert_eq!(*params.playout_ringbuffer_size.read().unwrap(), 661941);
        assert_eq!(*params.playout_ringbuffer_headroom.read().unwrap(), 4410);
        assert_eq!(*params.decoder_resume_hysteresis_samples.read().unwrap(), 44100);
        assert_eq!(*params.mixer_min_start_level.read().unwrap(), 22050);
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.95);
        assert_eq!(*params.pause_decay_floor.read().unwrap(), 0.0001778);
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 2208);
        assert_eq!(*params.mixer_check_interval_ms.read().unwrap(), 10);
    }

    #[test]
    fn test_rwlock_read_access() {
        // TC-U-002-01: Verify RwLock read access succeeds
        let params = GlobalParams::default();

        let sample_rate = *params.working_sample_rate.read().unwrap();
        assert_eq!(sample_rate, 44100);
    }

    #[test]
    fn test_rwlock_write_access() {
        // TC-U-002-02: Verify RwLock write access succeeds
        let params = GlobalParams::default();

        *params.working_sample_rate.write().unwrap() = 48000;
        assert_eq!(*params.working_sample_rate.read().unwrap(), 48000);
    }

    #[test]
    fn test_concurrent_reads() {
        // TC-U-002-03: Verify concurrent RwLock reads succeed
        use std::sync::Arc;
        use std::thread;

        let params = Arc::new(GlobalParams::default());
        let mut handles = vec![];

        // Spawn 10 threads all reading simultaneously
        for _ in 0..10 {
            let params_clone = Arc::clone(&params);
            let handle = thread::spawn(move || {
                let rate = *params_clone.working_sample_rate.read().unwrap();
                assert_eq!(rate, 44100);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_set_working_sample_rate_valid() {
        // TC-U-102-02: Validate working_sample_rate range
        let params = GlobalParams::default();

        assert!(params.set_working_sample_rate(48000).is_ok());
        assert_eq!(*params.working_sample_rate.read().unwrap(), 48000);

        assert!(params.set_working_sample_rate(44100).is_ok());
        assert_eq!(*params.working_sample_rate.read().unwrap(), 44100);
    }

    #[test]
    fn test_set_working_sample_rate_out_of_range() {
        // TC-U-102-02: Validate working_sample_rate range enforcement
        let params = GlobalParams::default();

        assert!(params.set_working_sample_rate(7999).is_err());
        assert!(params.set_working_sample_rate(192001).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.working_sample_rate.read().unwrap(), 44100);
    }

    #[test]
    fn test_set_volume_level_clamping() {
        // TC-U-102-01: Validate volume_level range (metadata validator)
        // **[PLAN019-REQ-DRY-050]** Updated test: volume_level now uses metadata validator,
        // which rejects out-of-range values instead of clamping
        let params = GlobalParams::default();

        assert!(params.set_volume_level(0.75).is_ok());
        assert_eq!(*params.volume_level.read().unwrap(), 0.75);

        // Out of range values now rejected (old behavior: clamped)
        assert!(params.set_volume_level(1.5).is_err());
        assert_eq!(*params.volume_level.read().unwrap(), 0.75); // Unchanged

        assert!(params.set_volume_level(-0.1).is_err());
        assert_eq!(*params.volume_level.read().unwrap(), 0.75); // Unchanged
    }

    // Database loading tests
    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_all_values() {
        // TC-DB-001: Load all parameters from database when all values present
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert all parameter values
        insert_setting(&pool, "volume_level", "0.75").await;
        insert_setting(&pool, "working_sample_rate", "48000").await;
        insert_setting(&pool, "output_ringbuffer_size", "16384").await;
        insert_setting(&pool, "maximum_decode_streams", "8").await;
        insert_setting(&pool, "decode_work_period", "3000").await;
        insert_setting(&pool, "chunk_duration_ms", "500").await;
        insert_setting(&pool, "playout_ringbuffer_size", "882000").await;
        insert_setting(&pool, "playout_ringbuffer_headroom", "8820").await;
        insert_setting(&pool, "decoder_resume_hysteresis_samples", "88200").await;
        insert_setting(&pool, "mixer_min_start_level", "44100").await;
        insert_setting(&pool, "pause_decay_factor", "0.90").await;
        insert_setting(&pool, "pause_decay_floor", "0.0002").await;
        insert_setting(&pool, "audio_buffer_size", "4096").await;
        insert_setting(&pool, "mixer_check_interval_ms", "20").await;

        // Initialize from database
        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify all values loaded
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.75);
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 48000);
        assert_eq!(*PARAMS.output_ringbuffer_size.read().unwrap(), 16384);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 8);
        assert_eq!(*PARAMS.decode_work_period.read().unwrap(), 3000);
        assert_eq!(*PARAMS.chunk_duration_ms.read().unwrap(), 500);
        assert_eq!(*PARAMS.playout_ringbuffer_size.read().unwrap(), 882000);
        assert_eq!(*PARAMS.playout_ringbuffer_headroom.read().unwrap(), 8820);
        assert_eq!(*PARAMS.decoder_resume_hysteresis_samples.read().unwrap(), 88200);
        assert_eq!(*PARAMS.mixer_min_start_level.read().unwrap(), 44100);
        assert_eq!(*PARAMS.pause_decay_factor.read().unwrap(), 0.90);
        assert_eq!(*PARAMS.pause_decay_floor.read().unwrap(), 0.0002);
        assert_eq!(*PARAMS.audio_buffer_size.read().unwrap(), 4096);
        assert_eq!(*PARAMS.mixer_check_interval_ms.read().unwrap(), 20);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_missing_values() {
        // TC-DB-002: Use defaults when parameters missing from database
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Don't insert any parameters
        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used (should match Default implementation)
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.5);
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*PARAMS.output_ringbuffer_size.read().unwrap(), 8192);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 12);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_out_of_range_values() {
        // TC-DB-003: Use defaults when parameters out of range
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert out-of-range values
        insert_setting(&pool, "working_sample_rate", "7000").await;  // Too low (min: 8000)
        insert_setting(&pool, "maximum_decode_streams", "50").await;  // Too high (max: 32)
        insert_setting(&pool, "audio_buffer_size", "100000").await;   // Too high (max: 8192)

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used for out-of-range values
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 12);
        assert_eq!(*PARAMS.audio_buffer_size.read().unwrap(), 2208);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_type_mismatch() {
        // TC-DB-004: Use defaults when type mismatch (invalid parse)
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert non-numeric values for numeric parameters
        insert_setting(&pool, "working_sample_rate", "not-a-number").await;
        insert_setting(&pool, "volume_level", "invalid").await;

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used when parsing fails
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.5);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_null_values() {
        // TC-DB-005: Use defaults when values are NULL
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert NULL values
        sqlx::query("INSERT INTO settings (key, value) VALUES (?, NULL)")
            .bind("working_sample_rate")
            .execute(&pool)
            .await
            .unwrap();

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used for NULL values
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_partial_values() {
        // TC-DB-006: Load some parameters, use defaults for others
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert only some parameters
        insert_setting(&pool, "volume_level", "0.8").await;
        insert_setting(&pool, "working_sample_rate", "96000").await;
        // Omit other parameters

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify loaded parameters
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.8);
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 96000);

        // Verify defaults for missing parameters
        assert_eq!(*PARAMS.output_ringbuffer_size.read().unwrap(), 8192);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 12);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_volume_level_clamping_from_database() {
        // TC-DB-007: Volume level out-of-range handling (metadata validation)
        // **[PLAN019-REQ-DRY-040]** Updated test: metadata validators reject out-of-range,
        // use default per error handling policy
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert out-of-range volume (should be rejected, use default)
        insert_setting(&pool, "volume_level", "1.5").await;

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify default (0.5) used instead of clamping to 1.0
        // Old behavior: clamped to 1.0
        // New behavior (PLAN019): rejected, use default (0.5)
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.5);
    }

    // Setter validation tests
    #[test]
    fn test_set_output_ringbuffer_size_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_output_ringbuffer_size(2048).is_ok());
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 2048);

        assert!(params.set_output_ringbuffer_size(16384).is_ok());
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 16384);

        assert!(params.set_output_ringbuffer_size(262144).is_ok());
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 262144);
    }

    #[test]
    fn test_set_output_ringbuffer_size_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_output_ringbuffer_size(2047).is_err());
        assert!(params.set_output_ringbuffer_size(262145).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 8192);
    }

    #[test]
    fn test_set_pause_decay_factor_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_pause_decay_factor(0.5).is_ok());
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.5);

        assert!(params.set_pause_decay_factor(0.90).is_ok());
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.90);

        assert!(params.set_pause_decay_factor(0.99).is_ok());
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.99);
    }

    #[test]
    fn test_set_pause_decay_factor_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_pause_decay_factor(0.49).is_err());
        assert!(params.set_pause_decay_factor(1.0).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.95);
    }

    #[test]
    fn test_set_audio_buffer_size_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_audio_buffer_size(512).is_ok());
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 512);

        assert!(params.set_audio_buffer_size(4096).is_ok());
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 4096);

        assert!(params.set_audio_buffer_size(8192).is_ok());
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 8192);
    }

    #[test]
    fn test_set_audio_buffer_size_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_audio_buffer_size(511).is_err());
        assert!(params.set_audio_buffer_size(8193).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 2208);
    }

    #[test]
    fn test_set_maximum_decode_streams_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_maximum_decode_streams(1).is_ok());
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 1);

        assert!(params.set_maximum_decode_streams(16).is_ok());
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 16);

        assert!(params.set_maximum_decode_streams(32).is_ok());
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 32);
    }

    #[test]
    fn test_set_maximum_decode_streams_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_maximum_decode_streams(0).is_err());
        assert!(params.set_maximum_decode_streams(33).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 12);
    }

    // Test helper functions
    async fn create_test_db() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create settings table");

        pool
    }

    async fn insert_setting(pool: &sqlx::SqlitePool, key: &str, value: &str) {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(pool)
            .await
            .expect("Failed to insert setting");
    }
