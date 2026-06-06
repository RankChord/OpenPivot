use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, PasswordHash},
};

use rand_core::OsRng;

pub fn hash_password(password: String) -> Result<PasswordHash, argon2::password_hash::Error> {
    // 随机盐
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 配置
    let params = Params::new(
        15,           // m_cost: 内存成本 (2^15 = 32MB)
        2,            // t_cost: 时间成本 (迭代次数)
        2,            // p_cost: 并行度
        None,         // keyid
        None,         // data
        32,           // output_len: 输出长度
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