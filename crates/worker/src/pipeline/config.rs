use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct MergePipelineConfig {
  pub dryrun: bool,
  pub bucket: String,
  pub date: String,
  /// merge blocks from this range e.g. 00_00-60
  /// which means starting at 00 hour and from 00 to 60 minutes
  pub merge_range: String,
  /// which input minute interval to use for merging e.g. 5
  pub input_minute_interval: u32,

  pub tmp_dir_path: String,

  pub delete_intermediate_files: bool,
}

#[derive(Debug, Clone)]
pub struct ParsePipelineConfig {
  pub (super) dryrun: bool,
  /// date is used for block cache partitioning
  pub (super) date: String,
  pub (super) slot_start: u64,
  pub (super) slot_end: u64,
  pub (super) download_config: DownloadConfig,
  pub (super) parse_config: ParseConfig,
  pub (super) upload_config: UploadConfig,
  
  // operations
  // download_blocks: bool, always
  pub (super) parse_blocks: bool,
  pub (super) upload_blocks: bool,
}

impl Default for ParsePipelineConfig {
  fn default() -> Self {
    ParsePipelineConfig {
      dryrun: false,
      date: "".to_string(),
      slot_start: 0,
      slot_end: 0,
      download_config: DownloadConfig::default(),
      parse_config: ParseConfig::default(),
      upload_config: UploadConfig::default(),
      parse_blocks: true,
      upload_blocks: true,
    }
  }
}

pub struct PipelineConfigBuilder {
  config: ParsePipelineConfig,
  has_download_config: bool,
}

#[allow(dead_code)]
impl PipelineConfigBuilder {
  pub fn new(start_slot: u64, end_slot: u64, date: String) -> Self {
    PipelineConfigBuilder {
      has_download_config: false,
      config: ParsePipelineConfig {
        date: date,
        slot_start: start_slot,
        slot_end: end_slot,
        ..Default::default()
      },
    }
  }

  pub fn with_dryrun(mut self, dryrun: bool) -> Self {
    self.config.dryrun = dryrun;
    self
  }

  pub fn with_date(mut self, date: &str) -> Self {
    self.config.date = date.to_string();
    self
  }

  pub fn with_slot_range(mut self, start_slot: u64, end_slot: u64) -> Self {
    self.config.slot_start = start_slot;
    self.config.slot_end = end_slot;
    self
  }

  pub fn with_download_config(mut self, config: DownloadConfig) -> Self {
    self.has_download_config = true;
    self.config.download_config = config;
    self
  }

  pub fn with_parse_config(mut self, config: ParseConfig) -> Self {
    self.config.parse_config = config;
    self
  }

  pub fn with_upload_config(mut self, config: UploadConfig) -> Self {
    self.config.upload_config = config;
    self
  }

  pub fn with_parse_blocks(mut self, parse: bool) -> Self {
    self.config.parse_blocks = parse;
    self
  }

  pub fn with_upload_blocks(mut self, upload: bool) -> Self {
    self.config.upload_blocks = upload;
    self
  }

  /// if the majority of blocks in range is already on S3 we can lift rpc concurrency limits
  /// by setting the data location to S3 however the semaphore should be skipped for cache hits in any case -> probably no-op
  pub fn with_data_location(mut self, data_location: DataLocation) -> Self {
    if self.has_download_config {
      panic!("ParsePipelineConfigBuilder: DataLocation overwrites existing download config");
    }
    self.config.download_config = DownloadConfig::with_data_location(data_location);
    self
  }

  pub fn with_delete_intermediate_files(mut self, delete: bool) -> Self {
    self.config.parse_config.delete_intermediate_files = delete;
    self
  }

  pub fn build(self) -> Result<ParsePipelineConfig> {
    if self.config.date.is_empty() {
      // Date is partition key for block cache
      return Err(anyhow!("ParsePipelineConfig: Date required for cache"));
    }
    if self.config.slot_start == 0 || self.config.slot_end == 0 {
      return Err(anyhow!("ParsePipelineConfig: Slot range required"));
    }
    if self.config.upload_blocks == false && self.config.parse_blocks == false {
      return Err(anyhow!("ParsePipelineConfig: At least one operation required"));
    }

    Ok(self.config)
  }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DataLocation {
  DISK,
  S3,
  RPC
}

#[derive(Debug, Clone)]
pub struct DownloadConfig {
  /// we have two levels of retries:
  /// 1. when we get a block that is recent we might want to try up to 7 retries with exponential backoff until available
  /// 2. for older blocks if the cache has no data it might just be that the rpc had an issue and we should retry
  pub (super) max_retry_global: u8,
  /// how many times to retry to get a block from the rpc in a row using exponential backoff
  pub (super) max_retry: u8,
  /// how long to sleep before two block fetches
  /// used for exponential backoff
  pub (super) sleep_duration_ms: u64,

  /// if blocks are prefetched to S3 we can fetch them more aggressively
  /// with concurrent downloads
  #[allow(dead_code)]
  pub (super) data_location: DataLocation,
}

impl DownloadConfig {
    pub fn with_data_location(data_location: DataLocation) -> Self {
      match data_location {
          DataLocation::DISK => DownloadConfig {
              sleep_duration_ms: 0,
              data_location,
              ..Default::default()
          },
          DataLocation::S3 => DownloadConfig {
              sleep_duration_ms: 0,
              data_location,
              ..Default::default()
          },
          DataLocation::RPC => DownloadConfig {
              sleep_duration_ms: 40,
              data_location,
              ..Default::default()
          },
      }
  }
}

impl Default for DownloadConfig {
  fn default() -> Self {
    DownloadConfig {
      max_retry_global: 3,
      sleep_duration_ms: 40,
      data_location: DataLocation::RPC,
      max_retry: 7,
    }
  }
}

#[derive(Debug, Clone)]
pub struct UploadConfig {
  /// s3 bucket to upload blocks and db files
  pub (super) bucket: String,
  /// blocks are stored in the bucket based on the block time
  /// for 5 (default) minute intervals we will have a folders 00_05, 05_10, 10_15, ...
  /// containing the individual blocks
  pub (super) block_partition_interval: u32,
}
impl Default for UploadConfig {
  fn default() -> Self {
    UploadConfig {
      // FIXME get from config,
      block_partition_interval: 5,
    }
  }
}

#[derive(Debug, Clone)]
pub struct ParseConfig {
  /// dir path where to store the parsed blocks
  pub (super) parsed_db_path: String,
  
  /// WARNING: should be true
  /// when this is run on the same database it will create primary key conflicts
  /// if primary keys are off it will create duplicates
  /// if run on a different database it will parse blocks into it which might be more efficient
  pub (super) overwrite_existing: bool,

  /// will parse blocks in memory without writing intermediate databases to disk
  pub (super) in_memory: bool,

  /// this will delete blocks as soon as they are parsed
  /// it will delete *.db files when they are exported to parquet
  /// it will delete parquet files when they are uploaded to s3
  pub (super) delete_intermediate_files: bool,
}

impl Default for ParseConfig {
  fn default() -> Self {

    let cache_path = "" // FIXME get from config
    // let output_dir = format!("{}/parsed/{}_{}", cache_path, start_slot, end_slot);
    let parsed_db = format!("{}/parsed", cache_path);

    ParseConfig {
      parsed_db_path: parsed_db,
      overwrite_existing: true,
      in_memory: false,
      delete_intermediate_files: true,
    }
  }
}