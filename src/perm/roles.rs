use mysql::{prelude::Queryable, PooledConn};

pub static mut ROLE_TABLES: RoleTable = RoleTable::empty();
#[derive(Debug)]
pub struct RoleTable {
    table: Vec<(String, String)>,
}

pub fn role_to_name(role: &str) -> String {
    unsafe {
        ROLE_TABLES.get_name(role).map_or(String::new(), |s|s.to_owned())
    }
}
pub fn name_to_role(name: &str) -> String {
    unsafe {
        ROLE_TABLES.get_id(name).map_or(String::new(), |s|s.to_owned())
    }
}

impl RoleTable {
    pub const fn empty() -> RoleTable {
        RoleTable { table: Vec::new() }
    }
    pub fn init(&mut self, conn: &mut PooledConn) {
        let map: Vec<(String, String)> = conn
            .query_map("SELECT id, name FROM roles", |(id, name)| (id, name))
            .expect("初始化角色表时查询失败");
        for (id, name) in map {
            self.table.push((id, name))
        }
    }
    pub fn update(&mut self, conn: &mut PooledConn) -> mysql::Result<()> {
        self.table = conn.query_map("SELECT id, name FROM roles", |(id, name)| (name, id))?;
        Ok(())
    }
    pub fn get_name(&self, id: &str) -> Option<&str> {
        for (id_k, name) in &self.table {
            if id_k == id {
                return Some(name);
            }
        }
        None
    }
    pub fn get_name_uncheck(&self, id: &str) -> String {
        self.get_name(id).unwrap().to_owned()
    }
    pub fn get_id(&self, name: &str) -> Option<&str> {
        for (id, name_k) in &self.table {
            if name == name_k {
                return Some(id);
            }
        }
        None
    }
}
