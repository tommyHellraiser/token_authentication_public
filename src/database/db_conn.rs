
use error_mapper::{map_to_new_error, TheResult};
use mysql_async::{Conn};
use crate::config::environment::EnvironmentConfig;

pub async fn get_conn() -> TheResult<Conn> {
    let pool = mysql_async::Pool::new(
        EnvironmentConfig::instance().get_db_url().await.as_str()
    );

    pool.get_conn().await.map_err(|e| map_to_new_error!(e))
}
