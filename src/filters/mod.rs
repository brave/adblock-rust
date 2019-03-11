pub mod network;

trait Filter {
    // mask: u32;
    fn get_id(&self) -> u32;
    fn get_tokens(&self) -> Vec<u32>;
}
