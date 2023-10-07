
use error_mapper::{map_to_new_error, SystemErrorCodes, TheError, TheResult};
use mysql_async::prelude::Queryable;
use tokio::io::AsyncReadExt;

pub mod db_conn;

pub const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub async fn load_schema_reset() -> TheResult<String> {

    let mut file = tokio::fs::File::options()
        .read(true)
        .open("sql/schema_reset.sql")
        .await
        .map_err(|e| map_to_new_error!(e))?;

    let mut schema_reset = String::new();
    let size = file.read_to_string(&mut schema_reset).await.map_err(|e| map_to_new_error!(e))?;

    if size == 0 {
        return Err(
            map_to_new_error!(
                TheError::new(
                    SystemErrorCodes::ReadWriteError,
                    "Schema reset file was empty or corrupted".to_string()
                )
                // TheError::default()
                // .with_type(SystemErrorCodes::ReadWriteError)
                // .with_content("Schema reset file was empty or corrupted".to_string())
            )
        );
    }

    Ok(schema_reset)
}

pub async fn reset_db() -> TheResult<()> {

    let schema_reset = match load_schema_reset().await {
        Ok(schema) => schema,
        Err(e) => return Err(e)
    };

    let conn = &mut db_conn::get_conn().await?;

    conn.query_drop(schema_reset).await.map_err(|e| map_to_new_error!(e))?;

    Ok(())

}