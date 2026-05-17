use crate::job::CronJob;
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

pub struct CronScheduler {
    jobs: Mutex<HashMap<String, CronJob>>,
    active_jobs: Mutex<HashMap<String, bool>>,
    tick_interval: Duration,
}

impl CronScheduler {
    pub fn new(tick_secs: u64) -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            active_jobs: Mutex::new(HashMap::new()),
            tick_interval: Duration::from_secs(tick_secs),
        }
    }

    pub async fn add_job(&self, job: CronJob) -> Result<(), String> {
        self.jobs.lock().await.insert(job.id.clone(), job);
        Ok(())
    }

    pub async fn list_jobs(&self) -> Vec<CronJob> {
        self.jobs.lock().await.values().cloned().collect()
    }

    pub async fn remove_job(&self, id: &str) {
        self.jobs.lock().await.remove(id);
    }

    pub async fn pause_job(&self, id: &str) {
        if let Some(job) = self.jobs.lock().await.get_mut(id) {
            job.enabled = false;
        }
    }

    pub async fn tick_loop<F>(&self, execute_fn: F)
    where
        F: FnMut(CronJob) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let execute_fn = std::sync::Arc::new(Mutex::new(execute_fn));
        let mut ticker = interval(self.tick_interval);

        loop {
            ticker.tick().await;
            self.check_jobs(execute_fn.clone()).await;
        }
    }

    async fn check_jobs<F>(
        &self,
        execute_fn: std::sync::Arc<Mutex<F>>
    ) where
        F: FnMut(CronJob) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send,
    {
        let now = Utc::now();
        let mut to_execute = Vec::new();

        {
            let mut jobs = self.jobs.lock().await;
            let active = self.active_jobs.lock().await;

            for job in jobs.values_mut() {
                if !job.enabled {
                    continue;
                }
                if active.get(&job.id).copied().unwrap_or(false) {
                    continue;
                }

                if let Some(next) = job.next_run {
                    if now >= next {
                        to_execute.push(job.clone());
                        job.last_run = Some(now);
                        job.next_run = job.schedule.next_after(now);
                    }
                }
            }
        }

        for job in to_execute {
            let mut exec = execute_fn.lock().await;
            (exec)(job).await;
        }
    }
}
