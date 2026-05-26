use agent_delegate::executor::*;
use agent_delegate::task::*;

#[test]
fn test_leaf_cannot_delegate() {
    let config = DelegateConfig::default();
    let exec = DelegateExecutor::new(config);
    let task = DelegateTask::new("do something").role(DelegateRole::Leaf);
    assert!(exec.can_delegate(&task.role).is_err());
}

#[test]
fn test_orchestrator_can_delegate() {
    let config = DelegateConfig::default();
    let exec = DelegateExecutor::new(config);
    let task = DelegateTask::new("coordinate").role(DelegateRole::Orchestrator);
    assert!(exec.can_delegate(&task.role).is_ok());
}

#[test]
fn test_max_depth() {
    let config = DelegateConfig {
        max_spawn_depth: 2,
        ..Default::default()
    };
    let exec = DelegateExecutor::new(config).with_depth(2);
    let task = DelegateTask::new("x").role(DelegateRole::Orchestrator);
    assert!(exec.can_delegate(&task.role).is_err());
}

#[test]
fn test_spawn_tracking() {
    let config = DelegateConfig::default();
    let mut exec = DelegateExecutor::new(config);
    let task = DelegateTask::new("test").role(DelegateRole::Orchestrator);
    let handle = exec.spawn(&task).unwrap();
    assert_eq!(exec.active_children_count(), 1);
    exec.complete(&handle);
    assert_eq!(exec.active_children_count(), 0);
}
