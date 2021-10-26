use crate::environment::mem_backend::SimpleBackend;
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::raw_overlay::RawOverlay;
use crate::environment::Environment;


pub trait EnvPolicy: Default {
    // Note the 'static requirement due to the fact that
    // Primitive::Env uses dyn Environment, which implicitly is
    // + 'static.
    type BaseEnv: Environment + Default + 'static;
    // Form of BaseEnv to be stored.
    //
    // Ideally, this would be set to something that will clone sanely
    // (like an overlay), such that structure clones from e.g.
    // AgentState::designate() work as expected even for env nodes.
    // However, certain deployments may be able to side-step this
    // matter without needing this to clone nicely.
    //
    // This also allows, for example, Overlay types which require
    // ownership of the BaseEnv.
    type StoredEnv: Environment + 'static;
    type Overlay: Environment + 'static;

    fn new_stored_env(&mut self, base: Self::BaseEnv) -> Box<Self::StoredEnv>;
}


#[derive(Default)]
pub struct SimplePolicy {}

impl EnvPolicy for SimplePolicy {
    type BaseEnv = MemEnvironment<SimpleBackend>;
    type StoredEnv = Self::Overlay;
    type Overlay = RawOverlay<Self::BaseEnv>;

    fn new_stored_env(&mut self, base: Self::BaseEnv) -> Box<Self::StoredEnv> {
        Box::new(Self::StoredEnv::new(base))
    }
}
