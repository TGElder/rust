pub trait Persistable {
    fn save(&self, path: &str);
    fn load(&mut self, path: &str);
}
