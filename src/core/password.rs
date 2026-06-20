use argon2::{
    Argon2,
    PasswordHasher,
    PasswordVerifier,
    Params,
    password_hash::{PasswordHash, SaltString},
};

use rand_core::OsRng;

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    // 随机盐
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 配置
    let params = Params::new(
        32 * 1024,    // m_cost: 内存成本 (32MB)
        2,            // t_cost: 时间成本 (迭代次数)
        2,            // p_cost: 并行度
        Some(32),     // output_len: 输出长度
    )?;

    // 创建 Argon2 实例
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    );

    // 哈希密码
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

pub fn verify_password(
    password: &str,
    password_hash: &str,
) -> Result<bool, argon2::password_hash::Error>{
    let parsed_hash = PasswordHash::new(password_hash)?;
    let argon2 = Argon2::default();

    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}