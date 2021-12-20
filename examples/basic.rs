extern crate pyroscope;

use pyroscope::pyroscope::PyroscopeAgentBuilder;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[tokio::main]
async fn main() {
    let guard =
        PyroscopeAgentBuilder::new("http://localhost:4040", "fibonacci")
            .frequency(100)
            .tags(
                [
                    ("TagA".to_owned(), "ValueA".to_owned()),
                    ("TagB".to_owned(), "ValueB".to_owned()),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .build()
            .unwrap();

    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }

    guard.stop().await.unwrap();

    return;
}
