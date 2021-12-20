extern crate pyroscope;

use pyroscope::{PyroscopeAgent, Result};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[tokio::main]
async fn main() -> Result<()>{
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
            ?;

    agent.start()?;
    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }
    agent.stop().await?;

    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }

    agent.start()?;
    for s in &[1, 10, 40, 50] {
        let result = fibonacci(44);
        println!("fibonacci({}) -> {}", *s, result);
    }
    agent.stop().await?;

    Ok(())
}
