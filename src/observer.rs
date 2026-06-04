pub trait RepositoryObserver: Send + Sync {
    fn on_db_query(&self, _op: &str, _table: &str, _duration_secs: f64, _success: bool) {}
    fn on_redis_op(&self, _op: &str, _duration_secs: f64, _success: bool) {}
}
