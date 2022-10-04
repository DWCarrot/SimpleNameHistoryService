use std::str::FromStr;
use std::time::Duration;

use sqlx::Database;
use sqlx::Pool;
use sqlx::SqlitePool;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteSynchronous;
use uuid::Uuid;

use self::config::DatabaseConfig;
use self::data::NameHistory;
use self::data::NameHistoryElement;
use self::data::Update;
use self::data::into_argument_uuid;

pub mod config;
pub mod data;


#[derive(Clone)]
pub struct NameHistoryDatabase {
    pool: SqlitePool,
}


impl NameHistoryDatabase {

    pub async fn init(config: &DatabaseConfig) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(config.url.as_str())?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(config.timeout.clone());
        let pool = SqlitePoolOptions::new()
            .max_connections(config.pool_max_connections)
            .acquire_timeout(config.pool_timeout.clone())
            .connect_with(options)
            .await?;
        let r11 = sqlx::query(data::CREATE_TABLE_NAMES).execute(&pool).await?;
        let r12 = sqlx::query(data::CREATE_INDEX_NAMES).execute(&pool).await?;
        let r21 = sqlx::query(data::CREATE_TABLE_UPDATES).execute(&pool).await?;
        let r22 = sqlx::query(data::CREATE_INDEX_UPDATES).execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn close(self) {
        self.pool.close().await;
    }

    pub async fn get_name_history(&self, uuid: &Uuid) -> Result<NameHistory, sqlx::Error> {
        let q = sqlx::query_as::<_, NameHistoryElement>(data::QUERY_NAME_HISTORY)
            .bind(into_argument_uuid(uuid))
            .fetch_all(&self.pool)
            .await?;
        Ok(q)
    }

    pub async fn add_name_history(&self, uuid: &Uuid, record: &NameHistoryElement, source: u32) -> Result<u64, sqlx::Error> {
        let r = if let Some(changed_to_at) = &record.changed_to_at {
            sqlx::query(data::INSERT_NAME)
                .bind(into_argument_uuid(uuid))
                .bind(record.name.as_str())
                .bind(NameHistoryElement::into_argument_systemtime(changed_to_at))
                .bind(source)
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query(data::INSERT_FIRST_NAME)
                .bind(into_argument_uuid(uuid))
                .bind(record.name.as_str())
                .bind(source)
                .execute(&self.pool)
                .await?
        };
        Ok(r.rows_affected())
    }

    pub async fn get_update(&self, uuid: &Uuid) -> Result<Option<Update>, sqlx::Error> {
        sqlx::query_as::<_, Update>(data::QUERY_UPDATE)
            .bind(into_argument_uuid(uuid))
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn refresh_update(&self, uuid: &Uuid, record: &Update) -> Result<u64, sqlx::Error> {
        let r = sqlx::query(data::REFRESH_UPDATE)
            .bind(Update::into_argument_systemtime(&record.update))
            .bind(record.changed)
            .bind(into_argument_uuid(uuid))
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected())
    }

    pub async fn insert_update(&self, uuid: &Uuid, record: &Update) -> Result<u64, sqlx::Error> {
        let r = sqlx::query(data::NEW_UPDATE)
            .bind(into_argument_uuid(uuid))
            .bind(Update::into_argument_systemtime(&record.update))
            .bind(record.changed)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected())
    }
}


mod test {

    use std::time::SystemTime;

    use super::*;

    #[test]
    fn run() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cfg = DatabaseConfig::default();
        rt.block_on(run_db(cfg)).unwrap();
    }

    async fn run_db(cfg: DatabaseConfig) -> Result<(), sqlx::Error> {
        let db = NameHistoryDatabase::init(&cfg).await?;
        println!("success step 0");
        let uuid1 = Uuid::parse_str("4566e69fc90748ee8d71d7ba5aa00d20").unwrap();
        let r1 = NameHistoryElement::new_initial("name1".to_string());
        let q1 = db.add_name_history(&uuid1, &r1, 1).await?;
        println!("success step 1: {}", q1);
        let r2 = NameHistoryElement::new("name2".to_string(), SystemTime::now());
        let q2 = db.add_name_history(&uuid1, &r2, 1).await?;
        println!("success step 2: {}", q2);
        let nh = db.get_name_history(&uuid1).await?;
        let s = serde_json::to_string(&nh).unwrap();
        println!("{}", s);
        let q3 = db.get_update(&uuid1).await?;
        println!("success step 3: {:?}", &q3);
        if let Some(u) = q3 {
            let rec = Update::new(SystemTime::now(), true);
            let q4 = db.refresh_update(&uuid1, &rec).await?;
            println!("success step 4: {}", q4);
        } else {
            let rec = Update::new(SystemTime::now(), false);
            let q4 = db.insert_update(&uuid1, &rec).await?;
            println!("success step 4: {}", q4);
        }
        Ok(())
    }
}