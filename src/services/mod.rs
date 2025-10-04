pub mod bulletin_service;
pub mod user_service;

pub use bulletin_service::BulletinService;
pub use user_service::UserService;

use std::sync::{Arc, Mutex};

// Container for all services
pub struct CoreServices {
    pub users: UserService,
    pub bulletins: BulletinService,
}

impl CoreServices {
    pub fn new(
        user_storage: Arc<Mutex<dyn crate::user_repository::UserStorage + Send>>,
        bulletin_storage: Arc<Mutex<dyn crate::bulletin_repository::BulletinStorage + Send>>,
    ) -> Self {
        Self {
            users: UserService::new(user_storage),
            bulletins: BulletinService::new(bulletin_storage),
        }
    }
}
