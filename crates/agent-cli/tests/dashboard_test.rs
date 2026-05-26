use agent_cli::cmd::serve::{cron_dashboard_html, tasks_dashboard_html};

#[test]
fn cron_dashboard_points_at_cron_jobs_api() {
    let html = cron_dashboard_html();

    assert!(html.contains("Cron Jobs"));
    assert!(html.contains("/api/cron/jobs"));
}

#[test]
fn tasks_dashboard_lists_kanban_statuses() {
    let html = tasks_dashboard_html();

    assert!(html.contains("Kanban Tasks"));
    assert!(html.contains("queued"));
    assert!(html.contains("running"));
    assert!(html.contains("failed"));
}
