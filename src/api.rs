mod mal;
pub use mal::*;
pub trait Details {
    fn search_title(&mut self, potential_title: &str) -> Vec<String>;
}
pub trait Synchronization {

}
