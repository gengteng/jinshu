pub mod config;
mod protocol;
pub mod registry;
mod status;

pub use status::*;

pub mod domain {
    pub mod message {
        tonic::include_proto!("domain.message");
    }
}

pub mod comet {
    tonic::include_proto!("comet");
}

pub mod receiver {
    tonic::include_proto!("receiver");
}

pub mod authorizer {
    tonic::include_proto!("authorizer");
}

#[cfg(test)]
pub mod test {
    tonic::include_proto!("test");
}
