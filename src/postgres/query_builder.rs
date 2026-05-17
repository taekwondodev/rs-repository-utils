use crate::error::RepositoryError;

macro_rules! impl_where_clause {
    ($type:ty) => {
        impl $type {
            pub fn where_clause(mut self, condition: &str) -> Self {
                self.wheres.push(condition.to_string());
                self
            }

            pub fn where_param<T>(mut self, column: &str, _value: &T) -> Self {
                self.param_count += 1;
                let count = self.param_count;
                self.wheres.push(format!("{column} = ${count}"));
                self
            }
        }
    };
}

macro_rules! impl_returning_clause {
    ($type:ty) => {
        impl $type {
            pub fn returning(mut self, column: &str) -> Self {
                self.returning.push(column.to_string());
                self
            }

            pub fn returning_all(mut self) -> Self {
                self.returning.push("*".to_string());
                self
            }
        }
    };
}

struct QueryFragment {
    base: String,
}

impl QueryFragment {
    fn new(base: String) -> Self {
        Self { base }
    }

    fn append_if(mut self, prefix: &str, items: &[String], separator: &str) -> Self {
        if !items.is_empty() {
            if !prefix.is_empty() {
                self.base.push(' ');
                self.base.push_str(prefix);
                self.base.push(' ');
                self.base.push_str(&items.join(separator));
            } else {
                self.base.push(' ');
                self.base.push_str(&items.join(separator));
            }
        }
        self
    }

    fn append_option(mut self, prefix: &str, value: Option<i64>) -> Self {
        if let Some(v) = value {
            self.base.push_str(&format!(" {prefix} {v}"));
        }
        self
    }

    fn build(self) -> String {
        self.base
    }
}

pub struct SelectBuilder {
    columns: Vec<String>,
    from: Option<String>,
    joins: Vec<String>,
    wheres: Vec<String>,
    order_by: Vec<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    param_count: i32,
}

impl SelectBuilder {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            from: None,
            joins: Vec::new(),
            wheres: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            param_count: 0,
        }
    }

    pub fn select(mut self, column: &str) -> Self {
        self.columns.push(column.to_string());
        self
    }

    pub fn select_all(mut self) -> Self {
        self.columns.push("*".to_string());
        self
    }

    pub fn from(mut self, table: &str) -> Self {
        self.from = Some(table.to_string());
        self
    }

    pub fn inner_join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(format!("INNER JOIN {table} ON {on}"));
        self
    }

    pub fn left_join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(format!("LEFT JOIN {table} ON {on}"));
        self
    }

    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.order_by.push(format!("{column} {}", direction.as_str()));
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn build(self) -> Result<String, RepositoryError> {
        let from = self.from.ok_or_else(|| {
            RepositoryError::InvalidQuery("FROM clause is required".into())
        })?;

        let columns = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };

        let query = QueryFragment::new(format!("SELECT {columns} FROM {from}"))
            .append_if("", &self.joins, " ")
            .append_if("WHERE", &self.wheres, " AND ")
            .append_if("ORDER BY", &self.order_by, ", ")
            .append_option("LIMIT", self.limit)
            .append_option("OFFSET", self.offset)
            .build();

        Ok(query)
    }

    pub fn param_count(&self) -> i32 {
        self.param_count
    }
}

impl_where_clause!(SelectBuilder);

impl Default for SelectBuilder {
    fn default() -> Self { Self::new() }
}

pub struct InsertBuilder {
    table: Option<String>,
    columns: Vec<String>,
    param_count: i32,
    returning: Vec<String>,
}

impl InsertBuilder {
    pub fn new() -> Self {
        Self { table: None, columns: Vec::new(), param_count: 0, returning: Vec::new() }
    }

    pub fn into(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    pub fn column<T>(mut self, name: &str, _value: &T) -> Self {
        self.columns.push(name.to_string());
        self.param_count += 1;
        self
    }

    pub fn build(self) -> Result<String, RepositoryError> {
        let table = self.table.ok_or_else(|| {
            RepositoryError::InvalidQuery("table name is required".into())
        })?;
        if self.columns.is_empty() {
            return Err(RepositoryError::InvalidQuery("at least one column is required".into()));
        }

        let placeholders: Vec<String> = (1..=self.param_count).map(|i| format!("${i}")).collect();
        let query = QueryFragment::new(format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            self.columns.join(", "),
            placeholders.join(", ")
        ))
        .append_if("RETURNING", &self.returning, ", ")
        .build();

        Ok(query)
    }
}

impl_returning_clause!(InsertBuilder);

impl Default for InsertBuilder {
    fn default() -> Self { Self::new() }
}

pub struct UpdateBuilder {
    table: Option<String>,
    sets: Vec<String>,
    wheres: Vec<String>,
    param_count: i32,
    returning: Vec<String>,
}

impl UpdateBuilder {
    pub fn new() -> Self {
        Self {
            table: None,
            sets: Vec::new(),
            wheres: Vec::new(),
            param_count: 0,
            returning: Vec::new(),
        }
    }

    pub fn table(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    pub fn set<T>(mut self, column: &str, value: &Option<T>) -> Self {
        if value.is_some() {
            self.param_count += 1;
            self.sets.push(format!("{column} = ${}", self.param_count));
        }
        self
    }

    pub fn set_always<T>(mut self, column: &str, _value: &T) -> Self {
        self.param_count += 1;
        self.sets.push(format!("{column} = ${}", self.param_count));
        self
    }

    pub fn where_id(mut self, _id: i32) -> Self {
        self.param_count += 1;
        self.wheres.push(format!("id = ${}", self.param_count));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    pub fn build(self) -> Result<String, RepositoryError> {
        let table = self.table.ok_or_else(|| {
            RepositoryError::InvalidQuery("table name is required".into())
        })?;
        if self.sets.is_empty() {
            return Err(RepositoryError::InvalidQuery("at least one SET clause is required".into()));
        }
        if self.wheres.is_empty() {
            return Err(RepositoryError::InvalidQuery("WHERE clause is required for UPDATE".into()));
        }

        let query = QueryFragment::new(format!("UPDATE {table} SET {}", self.sets.join(", ")))
            .append_if("WHERE", &self.wheres, " AND ")
            .append_if("RETURNING", &self.returning, ", ")
            .build();

        Ok(query)
    }
}

impl_where_clause!(UpdateBuilder);
impl_returning_clause!(UpdateBuilder);

impl Default for UpdateBuilder {
    fn default() -> Self { Self::new() }
}

pub struct DeleteBuilder {
    table: Option<String>,
    wheres: Vec<String>,
    param_count: i32,
}

impl DeleteBuilder {
    pub fn new() -> Self {
        Self { table: None, wheres: Vec::new(), param_count: 0 }
    }

    pub fn from(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    pub fn build(self) -> Result<String, RepositoryError> {
        let table = self.table.ok_or_else(|| {
            RepositoryError::InvalidQuery("table name is required".into())
        })?;
        if self.wheres.is_empty() {
            return Err(RepositoryError::InvalidQuery("WHERE clause is required for DELETE".into()));
        }

        let query = QueryFragment::new(format!("DELETE FROM {table}"))
            .append_if("WHERE", &self.wheres, " AND ")
            .build();

        Ok(query)
    }
}

impl_where_clause!(DeleteBuilder);

impl Default for DeleteBuilder {
    fn default() -> Self { Self::new() }
}

pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_basic() {
        let query = SelectBuilder::new().select("id").select("name").from("users").build().unwrap();
        assert_eq!(query, "SELECT id, name FROM users");
    }

    #[test]
    fn select_with_where() {
        let username = "test";
        let query = SelectBuilder::new().select_all().from("users").where_param("username", &username).build().unwrap();
        assert_eq!(query, "SELECT * FROM users WHERE username = $1");
    }

    #[test]
    fn select_with_join() {
        let query = SelectBuilder::new()
            .select("u.id").select("c.passkey")
            .from("users u")
            .inner_join("credentials c", "u.id = c.user_id")
            .where_clause("u.status = 'active'")
            .build().unwrap();
        assert_eq!(query, "SELECT u.id, c.passkey FROM users u INNER JOIN credentials c ON u.id = c.user_id WHERE u.status = 'active'");
    }

    #[test]
    fn insert_with_returning() {
        let name = "product";
        let price = 100;
        let query = InsertBuilder::new().into("products").column("name", &name).column("price", &price).returning_all().build().unwrap();
        assert_eq!(query, "INSERT INTO products (name, price) VALUES ($1, $2) RETURNING *");
    }

    #[test]
    fn update_with_optional_fields() {
        let name = Some("new_name");
        let price = Some(200);
        let query = UpdateBuilder::new().table("products").set("name", &name).set("price", &price).where_id(1).returning_all().build().unwrap();
        assert_eq!(query, "UPDATE products SET name = $1, price = $2 WHERE id = $3 RETURNING *");
    }

    #[test]
    fn update_skips_none_fields() {
        let name: Option<String> = None;
        let price = Some(200);
        let query = UpdateBuilder::new().table("products").set("name", &name).set("price", &price).where_id(1).build().unwrap();
        assert_eq!(query, "UPDATE products SET price = $1 WHERE id = $2");
    }

    #[test]
    fn delete_with_where() {
        let id = 1;
        let query = DeleteBuilder::new().from("products").where_param("id", &id).build().unwrap();
        assert_eq!(query, "DELETE FROM products WHERE id = $1");
    }
}
