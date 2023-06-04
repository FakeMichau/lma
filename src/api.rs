mod mal;
pub use mal::*;
pub trait Details {
    fn get_title_list(&mut self, potential_title: &str) -> Vec<String>;
}
pub trait Synchronization {

}
