pub mod delay;
pub mod html;
pub mod url;

pub use delay::{apply_random_delay, apply_rate_limit};
pub use html::extract_title;
pub use url::normalize_url_scheme;
