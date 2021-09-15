use crate::environment::mem_backend::SimpleBackend;
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::serial_overlay::SerialOverlay;
use crate::environment::Environment;


pub trait EnvPolicy {
    // Note the 'static requirement due to the fact that
    // Primitive::Env uses dyn Environment, which implicitly is
    // + 'static.
    type DefaultEnv: Environment + Default + 'static;
    type Overlay: Environment + 'static;

    fn from_root_env(env: Self::DefaultEnv) -> Self;
    fn new_overlay(&self) -> Box<Self::Overlay>;
}


pub struct SimplePolicy {
    root: <Self as EnvPolicy>::Overlay,
}

impl EnvPolicy for SimplePolicy {
    type DefaultEnv = MemEnvironment<SimpleBackend>;
    type Overlay = SerialOverlay<Self::DefaultEnv>;

    fn from_root_env(env: Self::DefaultEnv) -> Self {
        Self {
            root: Self::Overlay::new(env),
        }
    }
    fn new_overlay(&self) -> Box<Self::Overlay> {
        Box::new(self.root.clone())
    }
}
