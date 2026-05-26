pub fn run() -> anyhow::Result<()> {
    println!(
        "Registered Tools:
  • read_file    - 读取文件内容 (支持 offset/limit)
  • write_file   - 写入文件 (支持创建父目录/追加)
  • execute_command - 执行 Shell 命令 (带超时控制)
  • todo         - 管理待办清单
  • cron_job     - 创建/管理定时任务
  • web_search   - 网络搜索"
    );
    Ok(())
}
