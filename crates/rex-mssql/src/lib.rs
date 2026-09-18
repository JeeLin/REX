//! SQL Server (MSSQL) protocol implementation — 基于 tiberius 的 SqlConnector。

use anyhow::{Context, Result};
use rex_common::bracket_host;
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// SQL Server 连接器
pub struct SqlServerConnector {
    client: Option<Client<Compat<TcpStream>>>,
}

impl SqlServerConnector {
    /// 建立 SQL Server 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let host = bracket_host(&req.host);
        let database = req.database.as_deref().unwrap_or("master");

        let mut config = Config::new();
        config.host(host);
        config.port(req.port);
        config.database(database);
        config.authentication(AuthMethod::sql_server(
            &req.username,
            req.password.as_deref().unwrap_or(""),
        ));

        let tcp = TcpStream::connect(config.get_addr())
            .await
            .with_context(|| {
                format!(
                    "failed to connect to SQL Server at {}:{}",
                    req.host, req.port
                )
            })?;

        tcp.set_nodelay(true)?;

        let client = Client::connect(config, tcp.compat_write())
            .await
            .with_context(|| {
                format!(
                    "failed to connect to SQL Server at {}:{}",
                    req.host, req.port
                )
            })?;

        Ok(Self {
            client: Some(client),
        })
    }

    fn client(&mut self) -> Result<&mut Client<Compat<TcpStream>>> {
        self.client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("SQL Server connection is closed"))
    }
}

#[async_trait::async_trait]
impl SqlConnector for SqlServerConnector {
    fn database_type(&self) -> rex_common::sql::DatabaseType {
        rex_common::sql::DatabaseType::SqlServer
    }

    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        let start = std::time::Instant::now();
        let trimmed = sql.trim_start().to_uppercase();
        let is_query = trimmed.starts_with("SELECT")
            || trimmed.starts_with("SHOW")
            || trimmed.starts_with("EXEC")
            || trimmed.starts_with("EXECUTE");

        let client = self.client()?;
        let result = client
            .simple_query(sql)
            .await
            .with_context(|| format!("failed to execute query: {sql}"))?;

        if is_query {
            let rows = result.into_first_result().await?;

            let columns = if let Some(first) = rows.first() {
                first
                    .columns()
                    .iter()
                    .map(|c| ColumnInfo {
                        name: c.name().to_string(),
                        data_type: format!("{:?}", c.column_type()),
                        nullable: true,
                        is_primary_key: false,
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let mut data = Vec::new();
            for row in rows {
                let mut values = Vec::new();
                for i in 0..row.len() {
                    let val = if let Ok(Some(v)) = row.try_get::<i32, _>(i) {
                        serde_json::json!(v)
                    } else if let Ok(Some(v)) = row.try_get::<i64, _>(i) {
                        serde_json::json!(v)
                    } else if let Ok(Some(v)) = row.try_get::<f64, _>(i) {
                        serde_json::json!(v)
                    } else if let Ok(Some(v)) = row.try_get::<&str, _>(i) {
                        serde_json::json!(v)
                    } else if let Ok(Some(v)) = row.try_get::<bool, _>(i) {
                        serde_json::json!(v)
                    } else if let Ok(Some(v)) = row.try_get::<chrono::NaiveDateTime, _>(i) {
                        serde_json::json!(v.to_string())
                    } else {
                        serde_json::Value::Null
                    };
                    values.push(val);
                }
                data.push(values);
            }

            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns,
                rows: data,
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        } else {
            let _ = result.into_results().await?;
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        }
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let client = self.client()?;
        let result = client
            .simple_query("SELECT name FROM sys.databases ORDER BY name")
            .await?
            .into_first_result()
            .await?;

        Ok(result
            .iter()
            .filter_map(|row| row.try_get::<&str, _>(0).ok().flatten().map(String::from))
            .collect())
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let query = format!(
            "SELECT TABLE_NAME, TABLE_TYPE FROM {}.INFORMATION_SCHEMA.TABLES ORDER BY TABLE_NAME",
            db
        );
        let client = self.client()?;
        let result = client
            .simple_query(&query)
            .await?
            .into_first_result()
            .await?;

        Ok(result
            .iter()
            .filter_map(|row| {
                let name = row.try_get::<&str, _>(0).ok().flatten()?.to_string();
                let table_type = row.try_get::<&str, _>(1).ok().flatten()?.to_string();
                Some(TableInfo { name, table_type })
            })
            .collect())
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let query = format!(
            "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMNPROPERTY(OBJECT_ID('{}.{}'), COLUMN_NAME, 'IsIdentity') as IS_IDENTITY
             FROM {}.INFORMATION_SCHEMA.COLUMNS
             WHERE TABLE_NAME = '{}'
             ORDER BY ORDINAL_POSITION",
            db, table, db, table
        );
        let client = self.client()?;
        let result = client
            .simple_query(&query)
            .await?
            .into_first_result()
            .await?;

        Ok(result
            .iter()
            .filter_map(|row| {
                let name = row.try_get::<&str, _>(0).ok().flatten()?.to_string();
                let data_type = row.try_get::<&str, _>(1).ok().flatten()?.to_string();
                let nullable = row
                    .try_get::<&str, _>(2)
                    .ok()
                    .flatten()
                    .map(|v| v == "YES")
                    .unwrap_or(true);
                let is_pk = row
                    .try_get::<i32, _>(3)
                    .ok()
                    .flatten()
                    .map(|v| v == 1)
                    .unwrap_or(false);
                Some(ColumnInfo {
                    name,
                    data_type,
                    nullable,
                    is_primary_key: is_pk,
                })
            })
            .collect())
    }

    async fn indexes(&mut self, db: &str, table: &str) -> Result<Vec<IndexInfo>> {
        let client = self.client()?;
        let query = format!(
            "SELECT i.name AS index_name, i.type_desc, 
                    STRING_AGG(c.name, ',') WITHIN GROUP (ORDER BY ic.key_ordinal) AS columns,
                    i.is_unique
             FROM {}.sys.indexes i
             INNER JOIN {}.sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id
             INNER JOIN {}.sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id
             INNER JOIN {}.sys.objects o ON i.object_id = o.object_id
             WHERE o.name = '{}' AND i.name IS NOT NULL
             GROUP BY i.name, i.type_desc, i.is_unique
             ORDER BY i.name",
            db, db, db, db, table
        );
        let result = client
            .simple_query(&query)
            .await?
            .into_first_result()
            .await?;

        let mut indexes = Vec::new();
        for row in result.iter() {
            let name = row
                .try_get::<&str, _>(0)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing index name"))?
                .to_string();
            let index_type = row
                .try_get::<&str, _>(1)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing index type"))?
                .to_string();
            let columns_str = row
                .try_get::<&str, _>(2)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing columns"))?
                .to_string();
            let unique = row.try_get::<bool, _>(3).ok().flatten().unwrap_or(false);

            let columns = columns_str.split(',').map(String::from).collect();

            indexes.push(IndexInfo {
                name,
                index_type,
                columns,
                unique,
            });
        }

        Ok(indexes)
    }

    async fn foreign_keys(&mut self, db: &str, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        let client = self.client()?;
        let query = format!(
            "SELECT fk.name AS fk_name,
                    STRING_AGG(cp.name, ',') WITHIN GROUP (ORDER BY fkc.constraint_column_id) AS fk_columns,
                    OBJECT_NAME(fk.referenced_object_id) AS ref_table,
                    STRING_AGG(rp.name, ',') WITHIN GROUP (ORDER BY fkc.constraint_column_id) AS ref_columns,
                    fk.delete_referential_action_desc,
                    fk.update_referential_action_desc
             FROM {}.sys.foreign_keys fk
             INNER JOIN {}.sys.foreign_key_columns fkc ON fk.object_id = fkc.constraint_object_id
             INNER JOIN {}.sys.columns cp ON fkc.parent_object_id = cp.object_id AND fkc.parent_column_id = cp.column_id
             INNER JOIN {}.sys.columns rp ON fkc.referenced_object_id = rp.object_id AND fkc.referenced_column_id = rp.column_id
             INNER JOIN {}.sys.objects o ON fk.parent_object_id = o.object_id
             WHERE o.name = '{}'
             GROUP BY fk.name, fk.referenced_object_id, fk.delete_referential_action_desc, fk.update_referential_action_desc
             ORDER BY fk.name",
            db, db, db, db, db, table
        );
        let result = client
            .simple_query(&query)
            .await?
            .into_first_result()
            .await?;

        let mut fks = Vec::new();
        for row in result.iter() {
            let name = row
                .try_get::<&str, _>(0)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing FK name"))?
                .to_string();
            let columns_str = row
                .try_get::<&str, _>(1)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing FK columns"))?
                .to_string();
            let ref_table = row
                .try_get::<&str, _>(2)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing reference table"))?
                .to_string();
            let ref_columns_str = row
                .try_get::<&str, _>(3)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing reference columns"))?
                .to_string();
            let on_delete = row
                .try_get::<&str, _>(4)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing ON DELETE"))?
                .to_string();
            let on_update = row
                .try_get::<&str, _>(5)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing ON UPDATE"))?
                .to_string();

            let columns = columns_str.split(',').map(String::from).collect();
            let ref_columns = ref_columns_str.split(',').map(String::from).collect();

            fks.push(ForeignKeyInfo {
                name,
                columns,
                ref_table,
                ref_columns,
                on_delete,
                on_update,
            });
        }

        Ok(fks)
    }

    async fn ddl(&mut self, db: &str, table: &str) -> Result<DdlResult> {
        let client = self.client()?;
        // Note: sp_help returns multiple result sets with complex structure
        // For simplicity, we'll just get column information from INFORMATION_SCHEMA

        // Get column information
        let columns_query = format!(
            "SELECT COLUMN_NAME, DATA_TYPE, CHARACTER_MAXIMUM_LENGTH, IS_NULLABLE, COLUMN_DEFAULT
             FROM {}.INFORMATION_SCHEMA.COLUMNS
             WHERE TABLE_NAME = '{}'
             ORDER BY ORDINAL_POSITION",
            db, table
        );
        let columns_result = client
            .simple_query(&columns_query)
            .await?
            .into_first_result()
            .await?;

        let mut ddl = format!("CREATE TABLE {}.{} (\n", db, table);
        let mut column_defs = Vec::new();
        for row in columns_result.iter() {
            let name = row
                .try_get::<&str, _>(0)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing column name"))?
                .to_string();
            let data_type = row
                .try_get::<&str, _>(1)
                .ok()
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("missing data type"))?
                .to_string();
            let max_length = row.try_get::<i32, _>(2).ok().flatten();
            let nullable = row.try_get::<&str, _>(3).ok().flatten().unwrap_or("YES") == "YES";
            let default = row.try_get::<&str, _>(4).ok().flatten();

            let mut col_def = format!("  {} {}", name, data_type);
            if let Some(max_len) = max_length {
                if max_len > 0 {
                    col_def.push_str(&format!("({})", max_len));
                }
            }
            if !nullable {
                col_def.push_str(" NOT NULL");
            }
            if let Some(default_val) = default {
                col_def.push_str(&format!(" DEFAULT {}", default_val));
            }
            column_defs.push(col_def);
        }

        ddl.push_str(&column_defs.join(",\n"));
        ddl.push_str("\n)");

        Ok(DdlResult { ddl })
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            client.close().await?;
        }
        Ok(())
    }
}
