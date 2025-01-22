use arctis_types::{
    DexType, EncodedTransactionWithStatusMeta, NewToken, ParserResult, SolTransfer,
    SplTokenTransfer, SupplyChange, SwapInfo, SwapType,
};
use duckdb::arrow::array::Array;
use duckdb::arrow::datatypes::DataType;
use duckdb::types::{EnumType, ListType};
use duckdb::{params, Connection, Result};
use serde::Serialize;
use serde_json::{json, Value};

use crate::utils::print_json_objects_as_table;

#[allow(dead_code)]
#[derive(Debug)]
struct TokenStats {
    pub token: String,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub buy_count: i64,
    pub sell_count: i64,
    pub first_price: f64,
    pub last_price: f64,
    pub avg_price: f64,
    pub price_increase_pct: f64,
    pub unique_signers: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProgramParserData {
    pub signature: String,
    pub ix_idx: u8,
    pub program_id: String,
    pub ix_type: String,
    pub parsed: bool,
    pub error: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProcessedTransaction {
    pub slot: u64,
    pub block_time: i64,
    pub signer: String,
    pub signature: String,
    pub has_error: bool,
    pub top_level_ix_count: u8,
    pub inner_ix_count: u8,
    pub compute_units_consumed: u64,
    pub fee: u64,
    pub version: i8,
    pub is_discarded: bool,
    pub discard_reason: Option<String>,
    pub parsed_programs: Vec<ProgramParserData>,
    pub parsed_ix: Vec<ParserResult>,
    pub data: Option<EncodedTransactionWithStatusMeta>,
}

pub struct ProcessedBlock {
    pub slot: u64,
    pub block_time: i64,
    pub parent_slot: u64,
    pub transaction_count: u32,
}

pub struct ComputeBudgetProcessed {
    pub slot: u64,
    pub block_time: i64,
    pub signature: String,
    pub c_unit_limit: u64,
    pub fee: u64,
}

fn create_connection(file_path: Option<&str>, use_primary_keys: bool) -> Result<Connection> {
    let conn = match file_path {
        Some(path) => {
            // make sure the dir path exists
            let dir_path = std::path::Path::new(path).parent().unwrap();
            if !dir_path.exists() {
                std::fs::create_dir_all(dir_path).unwrap();
            }

            Connection::open(path)?
        }
        None => Connection::open_in_memory()?,
    };

    // IMPORTANT: do not change order of tables
    conn.execute_batch(
        format!(
            "
      BEGIN;

      CREATE TYPE SwapType AS ENUM ('Buy', 'Sell', 'Token');
      CREATE TYPE DexType AS ENUM ('Jupiterv6', 'Pumpfun', 'RaydiumAmm', 'Unknown');

      CREATE table blocks (
        slot BIGINT {},
        block_time BIGINT,
        parent_slot BIGINT,
        transaction_count INTEGER,
      );
      CREATE TABLE transactions (
        slot BIGINT,
        block_time BIGINT,
        signature TEXT {},
        signer TEXT,
        error BOOLEAN,
        top_level_ix_count INTEGER,
        inner_ix_count INTEGER,
        compute_units BIGINT,
        fee BIGINT,
        version INTEGER,
        is_discarded BOOLEAN,
        discard_reason TEXT,
        data JSON
      );
      CREATE TABLE swaps (
        slot BIGINT,
        block_time INTEGER,
        signer TEXT,
        signature TEXT,
        error BOOLEAN,
        dex DexType,
        swap_type SwapType,
        amount_in FLOAT,
        token_in TEXT,
        amount_out FLOAT,
        token_out TEXT,
        token TEXT
      );
      CREATE TABLE sol_transfers (
        slot BIGINT,
        block_time BIGINT,
        signature TEXT,
        src TEXT,
        dst TEXT,
        lamports BIGINT,
        sol FLOAT
      );
      CREATE table tokens (
        signer TEXT,
        mint TEXT {},
        factory TEXT, -- the program that created the token
        create_tx TEXT, -- the tx that created the token
        create_block_time BIGINT,
        create_slot BIGINT,
        initial_supply BIGINT DEFAULT 0,
        supply BIGINT DEFAULT 0,
        decimals INTEGER,
        name TEXT,
        symbol TEXT,
        uri TEXT
      );
      CREATE table supply_changes (
        signature TEXT,
        ix_index INTEGER,
        mint TEXT,
        amount HUGEINT, -- i128
        authority TEXT DEFAULT NULL 
        {}
      );
      CREATE TABLE token_transfers (
        slot BIGINT,
        block_time BIGINT,
        signature TEXT,
        src TEXT DEFAULT NULL,
        dst TEXT DEFAULT NULL,
        from_acc TEXT,
        to_acc TEXT,
        amount FLOAT,
        token TEXT DEFAULT NULL,
        decimals INTEGER DEFAULT 0,
        authority TEXT DEFAULT NULL
      );
      CREATE TABLE fees (
        slot BIGINT,
        block_time BIGINT,
        signature TEXT {},
        compute_unit_limit INTEGER DEFAULT 0,
        priority_fee FLOAT DEFAULT 0.0
      );
      CREATE TABLE cant_discard (
        slot BIGINT,
        signature TEXT {},
        program_id TEXT,
        fn_name TEXT DEFAULT '?'
      );
      CREATE TABLE tx_programs (
        signature TEXT,
        ix_index INTEGER,
        program_id TEXT,
        ix_type TEXT,
        can_parse BOOLEAN,
        has_error BOOLEAN 
        {}
      );
      COMMIT;
      ",
            if use_primary_keys { "PRIMARY KEY" } else { "" }, // blocks
            if use_primary_keys { "PRIMARY KEY" } else { "" }, // transactions
            if use_primary_keys { "PRIMARY KEY" } else { "" }, // swaps
            if use_primary_keys {
                ", PRIMARY KEY (signature, ix_index)"
            } else {
                ""
            }, // supply_changes
            if use_primary_keys { "PRIMARY KEY" } else { "" }, // fees
            if use_primary_keys { "PRIMARY KEY" } else { "" }, // cant_discard
            if use_primary_keys {
                ", PRIMARY KEY (signature, ix_index)"
            } else {
                ""
            }, // tx_programs
        )
        .as_str(),
    )?;

    Ok(conn)
}

pub struct SolanaDatabase {
    pub conn: Connection,
    #[allow(dead_code)]
    use_primary_keys: bool,
    no_op: bool,
    path: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExportFormat {
    PARQUET,
    #[allow(non_camel_case_types)]
    PARQUET_ZSTD,
    CSV,
}

pub enum DatabaseMode {
    InMemory,
    File,
}

pub struct DatabaseConfig {
    pub path: Option<String>,
    pub mode: DatabaseMode,
    pub with_primary_keys: bool,
    pub enable_s3: bool,
}

impl SolanaDatabase {
    pub fn new() -> Result<SolanaDatabase> {
        // default create in-memory db
        let conn = create_connection(None, true)?;
        Ok(SolanaDatabase {
            conn,
            no_op: false,
            path: None,
            use_primary_keys: true,
        })
    }

    pub fn new_with_config(config: DatabaseConfig) -> Result<SolanaDatabase> {
        let conn = match config.mode {
            DatabaseMode::InMemory => create_connection(None, config.with_primary_keys)?,
            DatabaseMode::File => {
                create_connection(config.path.as_deref(), config.with_primary_keys)?
            }
        };
        let mut db = SolanaDatabase {
            conn,
            no_op: false,
            path: config.path,
            use_primary_keys: config.with_primary_keys,
        };
        if config.enable_s3 {
            db.enable_s3();
        }
        Ok(db)
    }

    pub fn new_with_primary_keys(with_primary_keys: bool) -> Result<SolanaDatabase> {
        let conn = create_connection(None, with_primary_keys)?;
        Ok(SolanaDatabase {
            conn,
            no_op: false,
            path: None,
            use_primary_keys: with_primary_keys,
        })
    }

    pub fn new_from_file(file_path: &str) -> Result<SolanaDatabase> {
        let conn = create_connection(Some(file_path), true)?;
        Ok(SolanaDatabase {
            conn,
            no_op: false,
            path: Some(file_path.to_string()),
            use_primary_keys: true,
        })
    }

    pub fn new_from_file_with_primary_keys(
        file_path: &str,
        with_primary_keys: bool,
    ) -> Result<SolanaDatabase> {
        let conn = create_connection(Some(file_path), with_primary_keys)?;
        Ok(SolanaDatabase {
            conn,
            no_op: false,
            path: Some(file_path.to_string()),
            use_primary_keys: with_primary_keys,
        })
    }

    pub fn new_from_connection(conn: Connection) -> SolanaDatabase {
        // TODO we should tell if primary keys are used if we intend to insert data
        SolanaDatabase {
            conn,
            no_op: false,
            path: None,
            use_primary_keys: true,
        }
    }

    pub fn enable_s3(&mut self) {
        let conn = &self.conn;
        conn.execute_batch("INSTALL httpfs; LOAD httpfs;").unwrap();

        conn.execute_batch(
            format!(
                "
      SET s3_access_key_id='{}';
      SET s3_secret_access_key='{}';
      SET s3_region='{}';
    ",
                "",
                "",
                "" // FIXME get from config
            )
            .as_str(),
        )
        .unwrap();
    }

    pub fn set_no_op(&mut self, no_op: bool) {
        self.no_op = no_op;
    }

    pub fn get_path(&self) -> Option<String> {
        self.path.clone()
    }

    pub fn get_temp_dir(&self) -> Option<String> {
        // FIXME get from config
        None
    }

    pub fn get_file_name(&self, with_extension: bool) -> Option<String> {
        match &self.path {
            Some(path) => {
                let path = std::path::Path::new(path);
                match path.file_name() {
                    Some(file_name) => {
                        if with_extension {
                            Some(file_name.to_str().unwrap().to_string())
                        } else {
                            let extension = path.extension().unwrap();
                            Some(
                                file_name
                                    .to_str()
                                    .unwrap()
                                    .replace(&format!(".{}", extension.to_str().unwrap()), ""),
                            )
                        }
                    }
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn get_setting<T>(&self, setting: &str) -> Result<T>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Debug,
    {
        let query = format!("SELECT current_setting('{}')", setting);
        let mut stmt = self.conn.prepare(&query)?;
        let value: String = stmt.query_row([], |row| row.get(0))?;
        let value: T = value.parse().unwrap();
        Ok(value)
    }

    pub fn print_setting(&self, setting: &str) -> Result<()> {
        let value = self.get_setting(setting);
        if value.is_err() {
            return Ok(());
        }
        let value: String = value.unwrap();
        println!("DuckDB: {}: {}", setting, value);
        Ok(())
    }

    fn get_column_names(&self, query: &str) -> Result<Vec<String>> {
        let schema_query = format!("DESCRIBE {}", query);
        let mut stmt = self.conn.prepare(&schema_query)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut column_names = Vec::new();
        for row in rows {
            column_names.push(row?);
        }
        Ok(column_names)
    }

    /// this is hack to avoid query_to_json_parsed- TODO there might be a better way
    pub fn query_to_json_file(&self, query: &str) -> Result<Vec<Value>> {
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let temp_path = self.get_temp_dir();
        let temp_path = temp_path.unwrap_or(".".to_string());
        let temp_file_path = format!("{}/temp_{}.json", temp_path, ts_now);

        let query_wrapper = format!("COPY ({}) TO '{}'", query, temp_file_path);
        let mut stmt = self.conn.prepare(&query_wrapper)?;
        stmt.execute([])?;

        // read the json file
        let json_file = std::fs::read_to_string(&temp_file_path);
        // delete the temp file
        std::fs::remove_file(&temp_file_path).unwrap();

        if json_file.is_err() {
            // TODO error handling
            return Ok(vec![]);
        }
        let json_file = json_file.unwrap();
        // parse the json
        // let json: Result<Value, _> = serde_json::from_str(&json_file);
        let json = serde_json::Deserializer::from_str(&json_file).into_iter::<Value>();

        let json: Vec<Value> = json.filter_map(Result::ok).collect();
        Ok(json)
    }

    pub fn query_to_json_parsed(&self, query: &str) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare(query)?;

        // on views schema() call panics
        let has_schema = false;
        let column_names = match has_schema {
            // view
            false => self.get_column_names(query)?,
            // table
            true => stmt.column_names().to_vec(),
        };

        let rows = stmt.query_map([], |row| {
            let mut json_row = serde_json::Map::new();
            for (i, column_name) in column_names.iter().enumerate() {
                let value: Value = match row.get_ref(i)? {
                    duckdb::types::ValueRef::Null => Value::Null,
                    duckdb::types::ValueRef::Int(v) => json!(v),
                    duckdb::types::ValueRef::TinyInt(v) => json!(v),
                    duckdb::types::ValueRef::SmallInt(v) => json!(v),
                    duckdb::types::ValueRef::BigInt(v) => json!(v),
                    duckdb::types::ValueRef::Float(v) => json!(v),
                    duckdb::types::ValueRef::Double(v) => json!(v),
                    duckdb::types::ValueRef::Text(v) => {
                        // Try to decode the text as UTF-8 text
                        match std::str::from_utf8(v) {
                            Ok(decoded) => json!(decoded),
                            Err(_) => json!(v), // Fallback to raw bytes if not UTF-8
                        }
                    }
                    duckdb::types::ValueRef::Boolean(v) => json!(v),
                    duckdb::types::ValueRef::Blob(v) => {
                        // Try to decode the BLOB as UTF-8 text
                        match std::str::from_utf8(v) {
                            Ok(decoded) => json!(decoded),
                            Err(_) => json!(v), // Fallback to raw bytes if not UTF-8
                        }
                    }
                    // handle arrays
                    duckdb::types::ValueRef::List(v, _i) => {
                        match v {
                            ListType::Regular(t) => {
                                let arr_data_type = t.value_type();
                                match arr_data_type {
                                    DataType::Utf8 => {
                                        let values = t.values();
                                        let values = values.as_any().downcast_ref::<duckdb::arrow::array::StringArray>().unwrap();
                                        let mut arr_values: Vec<Value> = Vec::new();
                                        for i in 0..values.len() {
                                            arr_values.push(json!(values.value(i)));
                                        }
                                        json!(arr_values)
                                    }
                                    DataType::Struct(fields) => {
                                        let values = t.values();
                                        let values = values.as_any().downcast_ref::<duckdb::arrow::array::StructArray>().unwrap();
                                        let mut arr_values: Vec<Value> = Vec::new();
                                        for i in 0..values.len() {
                                            let mut struct_values = serde_json::Map::new();
                                            for (j, field) in fields.iter().enumerate() {
                                                let field_name = field.name();
                                                let field_values = values.column(j);
                                                let field_value = match field_values.data_type() {
                                                    DataType::Utf8 => {
                                                        let values = field_values.as_any().downcast_ref::<duckdb::arrow::array::StringArray>().unwrap();
                                                        json!(values.value(i))
                                                    }
                                                    DataType::Int64 => {
                                                        let values = field_values.as_any().downcast_ref::<duckdb::arrow::array::Int64Array>().unwrap();
                                                        json!(values.value(i))
                                                    }
                                                    DataType::Float64 => {
                                                        let values = field_values.as_any().downcast_ref::<duckdb::arrow::array::Float64Array>().unwrap();
                                                        json!(values.value(i))
                                                    }
                                                    _ => json!(format!("unsupported struct field type: {}", field_values.data_type())),
                                                };
                                                struct_values.insert(field_name.to_string(), field_value);
                                            }
                                            arr_values.push(Value::Object(struct_values));
                                        }
                                        json!(arr_values)
                                    }
                                    _ => json!("unsupported list type"),
                                }
                            }
                            _ => json!("unsupported list type"),
                        }
                    }
                    duckdb::types::ValueRef::Enum(v, idx) => {
                        match v {
                            EnumType::UInt8(dict) => {
                                let values = dict.values();
                                let key = dict.key(idx).unwrap();
                                // https://github.com/duckdb/duckdb-rs/issues/365#issuecomment-2263195641
                                // it definitely doesn't live up to the headline of "an ergonomic wrapper". lmao
                                let val = values.as_any().downcast_ref::<duckdb::arrow::array::StringArray>().unwrap().value(key);
                                json!(val)
                            }
                            _ => json!("unsupported dictionary type"),
                        }
                    }
                    _ => {
                        println!("unsupported type: {:?}", row.get_ref(i)?);
                        json!(format!("unsupported: {:?}", row.get_ref(i)?))
                    }
                };
                json_row.insert(column_name.to_string(), value);
            }
            Ok(Value::Object(json_row))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_block_time(&self, slot: u64) -> Result<i64> {
        let query = format!("SELECT block_time FROM blocks WHERE slot = {}", slot);
        let mut stmt = self.conn.prepare(&query)?;
        let block_time: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(block_time)
    }

    pub fn count_rows(&self, table: &str) -> Result<i64> {
        let count_query = format!("SELECT COUNT(*) FROM {}", table);
        let mut stmt = self.conn.prepare(&count_query)?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    pub fn count_rows_where(&self, table: &str, where_clause: &str) -> Result<i64> {
        let count_query = format!("SELECT COUNT(*) FROM {} WHERE {}", table, where_clause);
        let mut stmt = self.conn.prepare(&count_query)?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    pub fn count_distinct(&self, table: &str, column: &str) -> Result<i64> {
        let count_query = format!("SELECT COUNT(DISTINCT {}) FROM {}", column, table);
        let mut stmt = self.conn.prepare(&count_query)?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    pub fn min(&self, table: &str, column: &str) -> Result<i64> {
        let min_query = format!("SELECT MIN({}) FROM {}", column, table);
        let mut stmt = self.conn.prepare(&min_query)?;
        let min: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(min)
    }

    pub fn max(&self, table: &str, column: &str) -> Result<i64> {
        let max_query = format!("SELECT MAX({}) FROM {}", column, table);
        let mut stmt = self.conn.prepare(&max_query)?;
        let max: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(max)
    }

    pub fn insert_block(&mut self, block: &ProcessedBlock) -> Result<usize> {
        if self.no_op {
            return Ok(0);
        }
        self.conn.execute(
            "INSERT INTO blocks (slot, block_time, parent_slot, transaction_count) VALUES (?1, ?2, ?3, ?4)",
            params![block.slot, block.block_time, block.parent_slot, block.transaction_count],
        )
    }

    pub fn insert_transactions_bulk(
        &mut self,
        transactions: &Vec<ProcessedTransaction>,
    ) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("transactions")?;
        for transaction in transactions {
            appender.append_row(params![
                transaction.slot,
                transaction.block_time,
                transaction.signature,
                transaction.signer,
                transaction.has_error,
                transaction.top_level_ix_count,
                transaction.inner_ix_count,
                transaction.compute_units_consumed,
                transaction.fee,
                transaction.version,
                transaction.is_discarded,
                transaction.discard_reason,
                transaction
                    .data
                    .as_ref()
                    .map(|data| serde_json::to_string(data).unwrap())
            ])?;
        }
        Ok(0)
    }

    pub fn insert_sol_transfer_bulk(&mut self, transfers: &Vec<&SolTransfer>) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("sol_transfers")?;
        for transfer in transfers {
            appender.append_row(params![
                transfer.slot,
                transfer.block_time,
                transfer.signature,
                transfer.from,
                transfer.to,
                transfer.lamports,
                transfer.sol
            ])?;
        }
        return Ok(transfers.len());
    }

    pub fn insert_token_transfers_bulk(
        &mut self,
        transfers: &Vec<&SplTokenTransfer>,
    ) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("token_transfers")?;
        for transfer in transfers {
            appender.append_row(params![
                transfer.slot,
                transfer.block_time,
                transfer.signature,
                transfer.from,
                transfer.to,
                transfer.from_acc,
                transfer.to_acc,
                transfer.amount,
                transfer.token,
                transfer.decimals,
                transfer.authority
            ])?;
        }
        return Ok(transfers.len());
    }

    pub fn insert_swaps_bulk(&mut self, swaps: &Vec<&SwapInfo>) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("swaps")?;
        for swap in swaps {
            let token = match swap.swap_type {
                SwapType::Buy => swap.token_out.clone(),
                SwapType::Sell => swap.token_in.clone(),
                SwapType::Token => "".to_string(),
            };
            appender.append_row(params![
                swap.slot,
                swap.block_time,
                swap.signer,
                swap.signature,
                swap.error,
                swap.dex.to_db(),
                swap.swap_type.to_db(),
                swap.amount_in,
                swap.token_in,
                swap.amount_out,
                swap.token_out,
                token
            ])?;
        }
        return Ok(swaps.len());
    }

    pub fn insert_tokens_bulk(&mut self, tokens: &Vec<&NewToken>) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("tokens")?;
        for token in tokens {
            appender.append_row(params![
                token.signer,
                token.mint,
                token.factory,
                token.signature,
                token.block_time,
                token.slot,
                token.initial_supply,
                token.supply,
                token.decimals,
                token.name,
                token.symbol,
                token.uri
            ])?;
        }
        return Ok(tokens.len());
    }

    pub fn insert_supply_changes_bulk(
        &mut self,
        supply_changes: &Vec<&SupplyChange>,
    ) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("supply_changes")?;
        for supply_change in supply_changes {
            appender.append_row(params![
                supply_change.signature,
                supply_change.ix_index,
                supply_change.mint,
                supply_change.amount,
                supply_change.authority
            ])?;
        }
        return Ok(supply_changes.len());
    }

    pub fn insert_parsed_programs_bulk(
        &mut self,
        programs: &Vec<&ProgramParserData>,
    ) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("tx_programs")?;
        for program in programs {
            appender.append_row(params![
                program.signature,
                program.ix_idx,
                program.program_id,
                program.ix_type,
                program.parsed,
                program.error
            ])?;
        }
        return Ok(programs.len());
    }

    pub fn insert_compute_budget_bulk(
        &mut self,
        budget: &Vec<ComputeBudgetProcessed>,
    ) -> Result<usize> {
        let conn = &self.conn;
        let mut appender = conn.appender("fees")?;
        for budget in budget {
            appender.append_row(params![
                budget.slot,
                budget.block_time,
                budget.signature,
                budget.c_unit_limit,
                budget.fee
            ])?;
        }
        return Ok(budget.len());
    }

    pub fn get_swaps(&self) -> Result<Vec<SwapInfo>> {
        let mut stmt = self.conn.prepare("SELECT slot, block_time, signer, signature, error, dex, swap_type, amount_in, token_in, amount_out, token_out FROM swaps")?;
        let swaps_iter = stmt.query_map([], |row| {
            let dex_type_str: String = row.get(5)?;
            let swap_type_str: String = row.get(6)?;
            Ok(SwapInfo {
                slot: row.get(0)?,
                block_time: row.get(1)?,
                signer: row.get(2)?,
                signature: row.get(3)?,
                error: row.get(4)?,
                dex: DexType::from_db(&dex_type_str).unwrap(),
                swap_type: SwapType::from_db(&swap_type_str).unwrap(),
                amount_in: row.get(7)?,
                token_in: row.get(8)?,
                amount_out: row.get(9)?,
                token_out: row.get(10)?,
            })
        })?;
        let swaps: Result<Vec<_>> = swaps_iter.collect();
        swaps
    }

    pub fn load_parquet_table(&self, table: &str, file_path: &str) -> Result<()> {
        let connection = &self.conn;
        let _ = connection.execute(
            format!(
                "COPY {} FROM '{}' (FORMAT 'parquet', COMPRESSION 'ZSTD');",
                table, file_path
            )
            .as_str(),
            [],
        )?;
        Ok(())
    }

    pub fn print_table(&self, table: &str) -> Result<()> {
        let limit = 10;
        self.print_table_with_limit(table, limit)?;
        Ok(())
    }

    pub fn print_table_with_limit(&self, table: &str, limit: i32) -> Result<()> {
        let query = format!("SELECT * FROM {} limit {}", table, limit);
        let results = self.query_to_json_file(&query)?;
        print_json_objects_as_table(&results);
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
