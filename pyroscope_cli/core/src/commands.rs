use utils::app_config::AppConfig;
use utils::error::Result;

use pyroscope::pyroscope_backends::rbspy::{Rbspy, RbspyConfig};
use pyroscope::PyroscopeAgent;

/// adhoc command
pub fn adhoc() -> Result<()> {
    println!("adhoc command");
    Ok(())
}

/// exec command
pub fn exec() -> Result<()> {
    println!("exec command");
    Ok(())
}

/// connect command
pub fn connect() -> Result<()> {
    println!("connect command");
    let pid: i32 = AppConfig::get("pid")?;

    let config = RbspyConfig::new(pid, 100, true, None, true);

    let mut agent = PyroscopeAgent::builder("http://localhost:4040", "rbspy.basic")
        .backend(Rbspy::new(config))
        .build()
        .unwrap();

    agent.start().unwrap();

    std::thread::sleep(std::time::Duration::from_secs(100));

    println!("agent started");

    agent.stop().unwrap();

    Ok(())
}

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}
