use amlang::agent::env_policy::{EnvPolicy, SimplePolicy};
use amlang::agent::{Agent, AmlangInterpreter, EnvManager, VmInterpreter};
use amlang::env::LocalNode;
use amlang::primitive::Node;
use amlang::InitOptions;


pub fn setup() -> Result<(Agent, EnvManager<impl EnvPolicy>), String> {
    amlang::init(InitOptions::RootRun).unwrap();

    // Integration tests will call this method multiple times; ignore the error.
    if let Err(_err) = env_logger::try_init() {}

    // Bootstrap/deserialize.
    let manager = match amlang::agent::env_manager::EnvManager::<SimplePolicy>::bootstrap(".") {
        Ok(val) => val,
        Err(err) => return Err(format!("{}", err)),
    };

    // Prep agent.
    let pre_agent = manager.agent();
    let history_env = pre_agent.find_env("history.env").unwrap();
    let impl_env = pre_agent.find_env("impl.env").unwrap();
    let mut agent = pre_agent.fork(VmInterpreter::new(history_env, impl_env));
    agent
        .set_eval(move |frame| {
            let mut interpreter = AmlangInterpreter::new(impl_env);
            if let Some(frame) = frame {
                interpreter.eval_state.push(frame);
            }
            Ok(Box::new(interpreter))
        })
        .unwrap();
    // TODO(func) Rm once we figure out right d-chain abstraction.
    let lang_env = agent.context().lang_env();
    agent
        .designation_chain_mut()
        .push_front(Node::new(lang_env, LocalNode::default()));
    let working_env = agent.find_env("working.env").unwrap();
    let pos = agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(pos);

    Ok((agent, manager))
}
