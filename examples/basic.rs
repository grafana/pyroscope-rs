extern crate pyroscope;

use pyroscope::pyroscope::PyroscopeAgent;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[tokio::main]
async fn main() {
    let mut agent =
        PyroscopeAgent::builder("http://localhost:4040", "fibonacci")
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

    agent.start().unwrap();
    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }
    agent.stop().await.unwrap();

    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }

    agent.start().unwrap();
    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }
    agent.stop().await.unwrap();

    return;
}
