use super::Details;

pub struct MAL {
    token: String
}

impl MAL {
    pub fn new(token: &str) -> Self {
        Self { token: "".to_owned() }
    }
    pub fn set_token(&mut self, token: &str) {
        self.token = token.to_owned();
    }
}

impl Details for MAL {
    fn get_title_list(&self, potential_title: &str) -> Vec<String> {
        Vec::new()
    }
}
