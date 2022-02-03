use serde::{Deserialize, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use zeroize::Zeroize;

/// 内存密文
///
/// 格式化不显示实际内容的字符串
///
#[derive(Clone, Deserialize, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct Secret(String);

impl Secret {
    /// 使用 `{:?}` 或 `{}` 格式化时显示的字符串
    ///
    pub const DEBUG_STRING: &'static str = "<SECRET>";

    /// 构造一个密文
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// 暴露明文为切片引用
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// 暴露明文为字符串引用
    pub fn expose_string(&self) -> &String {
        &self.0
    }

    /// 暴露明文为字节切片
    pub fn expose_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Zeroize for Secret {
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.zeroize()
    }
}

impl Debug for Secret {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Secret::DEBUG_STRING)
    }
}

impl Display for Secret {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Secret::DEBUG_STRING)
    }
}

impl Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::secret::Secret;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::iter;
    use zeroize::Zeroize;

    #[test]
    fn expose() {
        let plain = random_string();
        let secret = Secret::new(&plain);
        assert_eq!(plain, secret.expose());
        assert_eq!(&plain, secret.expose_string());
        assert_eq!(plain.as_bytes(), secret.expose_bytes());
    }

    #[test]
    fn debug() {
        let plain = random_string();
        let secret = Secret::new(plain);
        let secret_debug = format!("{:?}", secret);
        let secret_display = format!("{}", secret);
        assert_eq!(secret_debug, Secret::DEBUG_STRING);
        assert_eq!(secret_display, Secret::DEBUG_STRING);
    }

    #[test]
    fn zeroize() {
        let mut secret = Secret::new(random_string());
        secret.zeroize();
        assert!(secret.expose_string().is_empty());
    }

    #[test]
    fn serde() -> serde_json::Result<()> {
        let plain = random_string();
        let secret = Secret::new(&plain);

        let s = serde_json::to_string(&plain)?;
        let ss = serde_json::to_string(&secret)?;
        assert_eq!(s, ss);
        assert_eq!(serde_json::from_str::<Secret>(&s)?, secret);

        Ok(())
    }

    fn random_string() -> String {
        let mut rng = rand::thread_rng();
        let s: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(32)
            .collect();
        s
    }
}
