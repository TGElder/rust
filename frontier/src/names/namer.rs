pub trait Namer {
    fn next_name(&mut self) -> String;
}
