use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// 流程模型
#[derive(Debug)]
pub struct Flow {
    pub id: Uuid,                                                       // 流程的唯一标识符
    pub space_id: Uuid,                                                 // 流程所属的空间ID
    pub name: String,                                                   // 流程名称
    pub description: Option<String>,                                    // 流程描述
    pub created_by: Uuid,                                               // 流程创建者的用户ID
    pub created_at: OffsetDateTime,                                     // 流程创建时间
    pub updated_at: OffsetDateTime,                                     // 流程更新时间
}

// 流程运行模型
#[derive(Debug)]
pub struct FlowRun {
    pub id: Uuid,                                                       // 流程运行的唯一标识符
    pub flow_id: Uuid,                                                  // 流程运行所属的流程ID 
    pub space_id: Uuid,                                                 // 流程运行所属的空间ID
    pub status: FlowRunStatus,                                          // 流程运行的状态
    pub started_by: Uuid,                                               // 流程运行的发起者用户ID
    pub current_task_id: Option<Uuid>,                                  // 当前正在执行的任务ID
    pub started_at: OffsetDateTime,                                     // 流程运行的开始时间
    pub completed_at: Option<OffsetDateTime>,                           // 流程运行的完成时间   
}

// 流程任务模型
#[derive(Debug)]
pub struct FlowTask {
    pub id: Uuid,                                                       // 流程任务的唯一标识符
    pub flow_run_id: Uuid,                                              // 流程任务所属的流程运行ID
    pub space_id: Uuid,                                                 // 流程任务所属的空间ID
    pub assignee_id: Uuid,                                              // 流程任务的执行者用户ID
    pub title: String,                                                  // 流程任务的标题
    pub description: Option<String>,                                    // 流程任务的描述
    pub status: FlowTaskStatus,                                         // 流程任务的状态
    pub result: Option<String>,                                         // 流程任务的执行结果
    pub created_at: OffsetDateTime,                                     // 流程任务的创建时间
    pub completed_at: Option<OffsetDateTime>,                           // 流程任务的完成时间
    pub completed_by: Option<Uuid>,                                     // 流程任务的完成者用户ID
}

// 流程事件模型
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FlowEvent {
    pub id: Uuid,                                                       // 流程事件的唯一标识符
    pub flow_run_id: Uuid,                                              // 流程事件所属的流程运行ID
    pub space_id: Uuid,                                                 // 流程事件所属的空间ID
    pub event_type: String,                                             // 流程事件的类型
    pub actor_id: Option<Uuid>,                                         // 流程事件的触发者用户ID
    pub payload: String,                                                // 流程事件的附加数据
    pub created_at: OffsetDateTime,                                     // 流程事件的创建时间
}

// 流程事件类型枚举
#[derive(Debug, Deserialize)]
pub struct CreateFlowRequest {
    pub name: String,                                                   // 流程名称
    pub description: Option<String>,                                    // 流程描述
}

// 流程事件类型枚举
#[derive(Debug, Serialize)]
pub struct FlowResponse {
    pub id: Uuid,                                                       // 流程的唯一标识符
    pub space_id: Uuid,                                                 // 流程所属的空间ID
    pub name: String,                                                   // 流程名称
    pub description: Option<String>,                                    // 流程描述
    pub created_by: Uuid,                                               // 流程的创建者用户ID
}

// 流程运行请求体
#[derive(Debug, Deserialize)]
pub struct StartFlowRunRequest {
    pub assignee_id: Uuid,                                              // 流程运行的初始任务执行者用户ID
    pub task_title: String,                                             // 流程运行的初始任务标题
    pub task_description: Option<String>,                               // 流程运行的初始任务描述
}

// 流程运行响应体
#[derive(Debug, Deserialize)]
pub struct CompleteFlowTaskRequest {
    pub result: String,                                                 // 流程任务的执行结果
}

// 流程运行响应体
#[derive(Debug, Serialize)]
pub struct StartFlowRunResponse {
    pub run_id: Uuid,                                                   // 流程运行的唯一标识符
    pub task_id: Uuid,                                                  // 流程运行的初始任务唯一标识符
    pub status: String,                                                 // 流程运行的状态
}

// 流程运行响应体
#[derive(Debug, Serialize)]
pub struct CompleteFlowTaskResponse {
    pub task_id: Uuid,                                                  // 流程任务的唯一标识符
    pub run_id: Uuid,                                                   // 流程运行的唯一标识符 
    pub status: String,                                                 // 流程任务的状态
}

// 流程事件状态枚举 
#[derive(Debug)]
pub enum FlowRunStatus {
    Running,                                                            // 流程运行中
    WaitingAction,                                                      // 流程运行等待用户操作
    Completed,                                                          // 流程运行已完成
    Canceled,                                                           // 流程运行已取消
    Failed,                                                             // 流程运行失败
}

// 流程任务状态枚举
#[derive(Debug)]
pub enum FlowTaskStatus {
    Pending,                                                            // 流程任务待处理
    Completed,                                                          // 流程任务已完成
    Canceled,                                                           // 流程任务已取消
}

// 流程事件类型枚举
#[derive(Debug)]
pub enum FlowEventType {
    FlowStarted,                                                        // 流程运行已启动
    TaskCreated,                                                        // 流程任务已创建
    TaskCompleted,                                                      // 流程任务已完成
    SpaceNotified,                                                      // 空间已通知
    FlowCompleted,                                                      // 流程已完成
    FlowCanceled,                                                       // 流程已取消
    FlowFailed,                                                         // 流程失败
}

// 流程运行状态类型转换
// 转换: 流程运行状态枚举 -> 流程运行状态字符串
impl FlowRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowRunStatus::Running => "running",                        // 流程运行中
            FlowRunStatus::WaitingAction => "waiting_action",           // 流程运行等待用户操作
            FlowRunStatus::Completed => "completed",                    // 流程运行已完成
            FlowRunStatus::Canceled => "canceled",                      // 流程运行已取消
            FlowRunStatus::Failed => "failed",                          // 流程运行失败
        }
    }
}

// 流程运行状态类型转换
// 转换: 流程运行状态字符串 -> 流程运行状态枚举
impl TryFrom<&str> for FlowRunStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "running" => Ok(FlowRunStatus::Running),
            "waiting_action" => Ok(FlowRunStatus::WaitingAction),
            "completed" => Ok(FlowRunStatus::Completed),
            "canceled" => Ok(FlowRunStatus::Canceled),
            "failed" => Ok(FlowRunStatus::Failed),
            _ => Err(()),
        }
    }
}

// 流程任务状态类型转换
// 转换: 流程任务状态枚举 -> 流程任务状态字符串
impl FlowTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowTaskStatus::Pending => "pending",
            FlowTaskStatus::Completed => "completed",
            FlowTaskStatus::Canceled => "canceled",
        }
    }
}

// 流程任务状态类型转换
// 转换: 流程任务状态字符串 -> 流程任务状态枚举
impl TryFrom<&str> for FlowTaskStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(FlowTaskStatus::Pending),
            "completed" => Ok(FlowTaskStatus::Completed),
            "canceled" => Ok(FlowTaskStatus::Canceled),
            _ => Err(()),
        }
    }
}

// 流程事件类型转换
// 转换: 流程事件类型枚举 -> 流程事件类型字符串
impl FlowEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowEventType::FlowStarted => "flow_started",
            FlowEventType::TaskCreated => "task_created",
            FlowEventType::TaskCompleted => "task_completed",
            FlowEventType::SpaceNotified => "space_notified",
            FlowEventType::FlowCompleted => "flow_completed",
            FlowEventType::FlowCanceled => "flow_canceled",
            FlowEventType::FlowFailed => "flow_failed",
        }
    }
}

// 流程事件类型转换
// 转换: 流程事件类型字符串 -> 流程事件类型枚举
impl TryFrom<&str> for FlowEventType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "flow_started" => Ok(FlowEventType::FlowStarted),
            "task_created" => Ok(FlowEventType::TaskCreated),
            "task_completed" => Ok(FlowEventType::TaskCompleted),
            "space_notified" => Ok(FlowEventType::SpaceNotified),
            "flow_completed" => Ok(FlowEventType::FlowCompleted),
            "flow_canceled" => Ok(FlowEventType::FlowCanceled),
            "flow_failed" => Ok(FlowEventType::FlowFailed),
            _ => Err(()),
        }
    }
}

// 流程模型转换:
// 转换: 流程模型 -> 流程响应体
impl Flow {
    pub fn into_response(self) -> FlowResponse {
        FlowResponse {
            id: self.id,
            space_id: self.space_id,
            name: self.name,
            description: self.description,
            created_by: self.created_by,
        }
    }
}
