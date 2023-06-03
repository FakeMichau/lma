mod mal;
pub use mal::*;
pub trait Details {
    fn get_title_list(&self, potential_title: &str) -> Vec<String>;
}
pub trait Synchronization {

}
