use sqlx::PgPool;
use time::OffsetDateTime;


use crate::models::friend::{FriendItem, FriendRequest, FriendRequestStatus};


#[derive(sqlx::FromRow)]
struct FriendRequestRow {
    id: i64,
    requester_id: i64,
    addressee_id: i64,
    status: String,
    message: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<FriendRequestRow> for FriendRequest {
    type Error = ();

    fn try_from(row: FriendRequestRow) -> Result<Self, Self::Error> {
        let status = FriendRequestStatus::try_from(row.status.as_str())?;

        Ok(FriendRequest {
            id: row.id,
            requester_id: row.requester_id,
            addressee_id: row.addressee_id,
            status,
            message: row.message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn create_friend_request(
    pool: &PgPool,
    requester_id: i64,
    addressee_id: i64,
    message: Option<&str>,
) -> Result<FriendRequest, sqlx::Error>{
    let row = sqlx::query_as::<_, FriendRequestRow>(
        r#"
        INSERT INTO friend_requests (requester_id, addressee_id, message)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            requester_id,
            addressee_id,
            status,
            message,
            created_at,
            updated_at
        "#,
    )
    .bind(requester_id)
    .bind(addressee_id)
    .bind(message)
    .fetch_one(pool)
    .await?;

    let request = FriendRequest::try_from(row)
        .map_err(|_| sqlx::Error::RowNotFound)?;

    Ok(request)
}

pub async fn find_pending_request_between_users(
    pool: &PgPool,
    user_a_id: i64,
    user_b_id: i64,
) -> Result<Option<FriendRequest>, sqlx::Error>{
    let row = sqlx::query_as::<_, FriendRequestRow>(
        r#"
        SELECT
            id,
            requester_id,
            addressee_id,
            status,
            message,
            created_at,
            updated_at
        FROM friend_requests
        WHERE status = 'pending'
        AND (
            (requester_id = $1 AND addressee_id = $2)
            OR
            (requester_id = $2 AND addressee_id = $1)
        )
        LIMIT 1
        "#,
    )
    .bind(user_a_id)
    .bind(user_b_id)
    .fetch_optional(pool)
    .await?;

    match row {
    Some(row) => {
        let request = FriendRequest::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)?;

        Ok(Some(request))
    }
    None => Ok(None),
}
}

pub async fn create_friendship(
    pool: &PgPool,
    user_a_id: i64,
    user_b_id: i64,
) -> Result<(), sqlx::Error> {
    let user_low_id = user_a_id.min(user_b_id);
    let user_high_id = user_a_id.max(user_b_id);

    sqlx::query(
        r#"
        INSERT INTO friendships (user_low_id, user_high_id)
        VALUES ($1, $2)
        ON CONFLICT (user_low_id, user_high_id) DO NOTHING
        "#,
    )
    .bind(user_low_id)
    .bind(user_high_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn accept_friend_request(
    pool: &PgPool,
    request_id: i64,
    current_user_id: i64,
) -> Result<Option<FriendRequest>, sqlx::Error> {
    let row = sqlx::query_as::<_, FriendRequestRow>(
        r#"
        UPDATE friend_requests
        SET status = 'accepted',
            updated_at = NOW()
        WHERE id = $1
          AND addressee_id = $2
          AND status = 'pending'
        RETURNING
            id,
            requester_id,
            addressee_id,
            status,
            message,
            created_at,
            updated_at
        "#,
    )
    .bind(request_id)
    .bind(current_user_id)
    .fetch_optional(pool)
    .await?;

    let request = match row {
        Some(row) => FriendRequest::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)?,
        None => return Ok(None),
    };

    create_friendship(
        pool,
        request.requester_id,
        request.addressee_id,
    )
    .await?;

    Ok(Some(request))
}

pub async fn reject_friend_request(
    pool: &PgPool,
    request_id: i64,
    current_user_id: i64,
) -> Result<Option<FriendRequest>, sqlx::Error> {
    let row = sqlx::query_as::<_, FriendRequestRow>(
        r#"
        UPDATE friend_requests
        SET status = 'rejected',
            updated_at = NOW()
        WHERE id = $1
          AND addressee_id = $2
          AND status = 'pending'
        RETURNING
            id,
            requester_id,
            addressee_id,
            status,
            message,
            created_at,
            updated_at
        "#,
    )
    .bind(request_id)
    .bind(current_user_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let request = FriendRequest::try_from(row)
                .map_err(|_| sqlx::Error::RowNotFound)?;

            Ok(Some(request))
        }
        None => Ok(None),
    }
}

pub async fn friendship_exists(
    pool: &PgPool,
    user_a_id: i64,
    user_b_id: i64,
) -> Result<bool, sqlx::Error> {
    let user_low_id = user_a_id.min(user_b_id);
    let user_high_id = user_a_id.max(user_b_id);

    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM friendships
            WHERE user_low_id = $1
              AND user_high_id = $2
        )
        "#,
    )
    .bind(user_low_id)
    .bind(user_high_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn list_received_pending_requests(
    pool: &PgPool,
    current_user_id: i64,
) -> Result<Vec<FriendRequest>, sqlx::Error> {
    let rows = sqlx::query_as::<_, FriendRequestRow>(
        r#"
        SELECT
            id,
            requester_id,
            addressee_id,
            status,
            message,
            created_at,
            updated_at
        FROM friend_requests
        WHERE addressee_id = $1
          AND status = 'pending'
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(current_user_id)
    .fetch_all(pool)
    .await?;

    let mut requests = Vec::with_capacity(rows.len());

    for row in rows {
        let request = FriendRequest::try_from(row)
            .map_err(|_| sqlx::Error::RowNotFound)?;

        requests.push(request);
    }

    Ok(requests)
}

pub async fn list_friends(
    pool: &PgPool,
    current_user_id: i64,
) -> Result<Vec<FriendItem>, sqlx::Error> {
    let friends = sqlx::query_as::<_, FriendItem>(
        r#"
        SELECT
            u.id,
            u.username,
            u.nickname
        FROM friendships f
        JOIN users u
          ON u.id = CASE
              WHEN f.user_low_id = $1 THEN f.user_high_id
              ELSE f.user_low_id
          END
        WHERE f.user_low_id = $1
           OR f.user_high_id = $1
        ORDER BY u.username ASC
        "#,
    )
    .bind(current_user_id)
    .fetch_all(pool)
    .await?;

    Ok(friends)
}