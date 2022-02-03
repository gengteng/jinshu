use std::fmt::Display;
use tonic::Status;

/// 用可打印内容构造 gRPC 状态 `Internal = 13`
pub fn internal<E: Display>(e: E) -> Status {
    Status::internal(e.to_string())
}

/// 用可打印内容构造 gRPC 状态 `InvalidArgument = 3`
pub fn invalid_argument<E: Display>(e: E) -> Status {
    Status::invalid_argument(e.to_string())
}

#[cfg(test)]
mod test {
    use tonic::Code;

    #[test]
    fn internal() {
        assert_eq!(super::internal("").code(), Code::Internal);
        assert_eq!(super::invalid_argument("").code(), Code::InvalidArgument);
    }
}
