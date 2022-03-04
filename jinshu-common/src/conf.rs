use crate::{Error, Result};
use clap::Parser;
use config::Environment;
use serde::Deserialize;
use std::path::PathBuf;

/// 配置
///
pub trait Config: Sized {
    /// 根据命令行参数获得所有配置文件并构造配置对象
    ///
    fn from_cli() -> Result<Self> {
        let opts = Opts::parse();
        Self::from_opts(opts)
    }

    /// 从 Opts 获得所有配置文件并构造配置对象
    fn from_opts(opts: Opts) -> Result<Self>;
}

impl<'de, T: Deserialize<'de>> Config for T {
    fn from_opts(opts: Opts) -> Result<Self> {
        let Opts {
            mut configs,
            config_root_path,
        } = opts;

        if configs.is_empty() {
            return Err(Error::Other(
                "Please use the command line option '-c' to set the configuration file path".into(),
            ));
        }

        let mut config = config::Config::builder();

        for conf in configs.drain(..).rev() {
            config = config.add_source(config::File::from(match &config_root_path {
                None => conf,
                Some(root) => root.join(conf),
            }));
        }

        config = config.add_source(Environment::with_prefix("JINSHU").separator("__"));

        Ok(config.build()?.try_deserialize()?)
    }
}

/// 锦书模块命令行启动器
#[derive(Debug, Parser)]
#[clap(name = "JinShu Command Line Launcher")]
pub struct Opts {
    /// 模块配置文件，支持多个文件，用空格分隔
    #[clap(short = 'c', long, multiple_values = true)]
    pub configs: Vec<PathBuf>,

    /// 配置文件路径，配置文件会从该路径寻找配置文件
    #[clap(short = 'r', long)]
    pub config_root_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::{Config, Opts};
    use serde::{Deserialize, Serialize};
    use std::fs::OpenOptions;
    use std::net::SocketAddr;
    use std::path::{Path, PathBuf};
    use temp_dir::TempDir;

    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
    struct TestConfig {
        v1: i32,
        v2: String,
        v3: SocketAddr,
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                v1: 123,
                v2: "456".to_string(),
                v3: ([127, 0, 0, 1], 8888u16).into(),
            }
        }
    }

    #[test]
    fn no_arg() {
        assert!(TestConfig::from_cli().is_err());
    }

    #[test]
    fn v3_not_found() -> std::io::Result<()> {
        #[derive(Debug, Serialize)]
        struct NoV3Config {
            v1: i32,
            v2: String,
            // v3: SocketAddr,
        }

        let error_config = NoV3Config {
            v1: 123,
            v2: "456".into(),
        };

        let d = TempDir::new()?;
        let filename = format!("{}.json", uuid::Uuid::new_v4().as_simple());

        write_to_file(&filename, d.path(), &error_config)?;

        let opts = Opts {
            configs: vec![PathBuf::from(filename)],
            config_root_path: Some(d.path().to_path_buf()),
        };

        let config = TestConfig::from_opts(opts);
        assert!(config.is_err());

        Ok(())
    }

    #[test]
    fn with_arg() -> crate::Result<()> {
        let default_config = TestConfig::default();

        let d = TempDir::new()?;
        let filename = format!("{}.json", uuid::Uuid::new_v4().as_simple());

        write_to_file(&filename, d.path(), &default_config)?;

        let opts = Opts {
            configs: vec![PathBuf::from(&filename)],
            config_root_path: Some(d.path().to_path_buf()),
        };

        let config = TestConfig::from_opts(opts)?;
        assert_eq!(config, default_config);

        let no_root = Opts {
            configs: vec![d.path().join(filename)],
            config_root_path: None,
        };

        let config = TestConfig::from_opts(no_root)?;
        assert_eq!(config, default_config);

        Ok(())
    }

    #[test]
    fn layered() -> crate::Result<()> {
        let default_config = TestConfig::default();

        let d = TempDir::new()?;
        let default_filename = format!("{}.json", uuid::Uuid::new_v4().as_simple());

        write_to_file(&default_filename, d.path(), &default_config)?;

        let modified_config = TestConfig {
            v1: 789,
            v2: "101112".to_string(),
            v3: ([192, 168, 0, 1], 9999u16).into(),
        };

        let modified_filename = format!("{}.json", uuid::Uuid::new_v4().as_simple());
        write_to_file(&modified_filename, d.path(), &modified_config)?;

        let opts = Opts {
            configs: vec![
                PathBuf::from(&default_filename),
                PathBuf::from(&modified_filename),
            ],
            config_root_path: Some(d.path().to_path_buf()),
        };

        let config = TestConfig::from_opts(opts)?;
        assert_eq!(config, default_config);

        let opts = Opts {
            configs: vec![
                PathBuf::from(&modified_filename),
                PathBuf::from(&default_filename),
            ],
            config_root_path: Some(d.path().to_path_buf()),
        };

        let config = TestConfig::from_opts(opts)?;
        assert_eq!(config, modified_config);

        Ok(())
    }

    fn write_to_file<T: ?Sized + Serialize>(
        filename: &str,
        path: &Path,
        value: &T,
    ) -> std::io::Result<()> {
        let fullpath = path.join(filename);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&fullpath)?;
        serde_json::to_writer(&mut file, value).map_err(|_e| std::io::ErrorKind::Other)?;
        Ok(())
    }
}
