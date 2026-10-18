//! 幂等记录与业务修改在同一事务中写入，防止网络重试重复派单或通知。
use super::*;

pub(super) fn cached(
    tx: &Transaction<'_>,
    actor: &str,
    scope: &str,
    key: &str,
    hash: &str,
) -> Result<Option<Value>> {
    let cached: Option<(String, String)> = tx
        .query_row(
            "SELECT request_hash,result_json FROM dedup WHERE actor=?1 AND scope=?2 AND key=?3",
            params![actor, scope, key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    cached
        .map(|(old_hash, result)| {
            ensure(
                old_hash == hash,
                "idempotency_key reused with different arguments",
            )?;
            Ok(serde_json::from_str(&result)?)
        })
        .transpose()
}
pub(super) fn cache(
    tx: &Transaction<'_>,
    actor: &str,
    scope: &str,
    key: &str,
    hash: &str,
    result: &Value,
) -> Result<()> {
    tx.execute(
        "INSERT INTO dedup VALUES(?1,?2,?3,?4,?5)",
        params![actor, scope, key, hash, result.to_string()],
    )?;
    Ok(())
}
