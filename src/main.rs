use lma;

fn main() {
    let anime_list = lma::create();
    let data = anime_list.get_list();
    if let Ok(result) = data {
        println!("{:#?}", result);
    }
}
