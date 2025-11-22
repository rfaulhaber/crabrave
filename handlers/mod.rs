//! API modules for different endpoint categories

pub mod blog;
pub mod communities;
pub mod posts;
pub mod tagged;
pub mod user;

pub use blog::Blogs;
pub use communities::Communities;
pub use posts::Posts;
pub use tagged::Tagged;
pub use user::Users;
