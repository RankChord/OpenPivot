use agent_core::budget::IterationBudget;

#[test]
fn test_budget_counts_iterations() {
    let budget = IterationBudget::new(3);
    assert!(!budget.exhausted());
    assert_eq!(budget.remaining(), 3);

    budget.increment();
    assert!(!budget.exhausted());
    assert_eq!(budget.remaining(), 2);

    budget.increment();
    budget.increment();
    assert!(budget.exhausted());
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn test_budget_large_value() {
    let budget = IterationBudget::new(9999);
    assert!(!budget.exhausted());
    assert_eq!(budget.remaining(), 9999);
}

#[test]
fn test_build_messages() {
    use agent_core::types::build_api_messages;

    let messages = build_api_messages(
        "You are a helpful assistant",
        "Hello!",
        &[],
    );

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, agent_llm::MessageRole::System);
    assert_eq!(messages[1].role, agent_llm::MessageRole::User);
    assert_eq!(messages[1].content.as_deref(), Some("Hello!"));
}
