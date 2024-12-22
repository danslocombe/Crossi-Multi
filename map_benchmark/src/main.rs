fn main() {
    let map = crossy_multi_core::map::Map::exact_seed(123);
    println!("Seed {}", map.get_seed());
    let mut y = 20;
    loop {
        y -= 1;
        map.get_row(1, y);
        println!("y = {}", y);
    }
}
