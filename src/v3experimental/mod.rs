use pprof2::{
    Error as PprofError, ProfilerGuard, ProfilerGuardBuilder, Result as PprofResult,
};
use std::marker::PhantomData;

pub struct Agent<'a> {
    pprof: ProfilerGuard<'a>,
    _marker: PhantomData<*const ()>, // !Send
}

pub enum AgentError {
    PprofInit(PprofError),
}

impl<'a> Agent<'a> {
    pub fn new(pprof_config: PprofConfig) -> Result<Self, AgentError> {
        let pprof = ProfilerGuardBuilder::default()
            .frequency(pprof_config.sample_rate as i32)
            .build()
            .map_err(|e| AgentError::PprofInit(e))?;
        Ok(Agent {
            pprof,
            _marker: PhantomData,
        })
    }

    pub fn report(&mut self) -> PprofResult<()> {
        // let buffer: HashMap<UnresolvedFrames, usize> = HashMap::new();
        let _timing = self.pprof.reset(|e| println!("# {:?}", e.count))?;

        Ok(())
    }
}

pub struct PprofConfig {
    pub sample_rate: u32,
}

impl Default for PprofConfig {
    fn default() -> Self {
        PprofConfig { sample_rate: 100 }
    }
}
