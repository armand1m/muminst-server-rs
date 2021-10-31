#[derive(Queryable)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub file_name: String,
    pub file_hash: String,
}
