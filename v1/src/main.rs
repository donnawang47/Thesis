pub mod dijkstra;
pub mod database;

fn get_shortest_path(start: i32, end: i32) -> Vec<i32> {
    println!("get_shortest_path: start{}, end{}", start, end);
    dijkstra::dijkstra(start, end)
}

fn main() {
    get_shortest_path(0, 8);
}
