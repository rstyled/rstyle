mod email_worker;
mod image_worker;
mod runner;

use email_worker::EmailWorker;
use image_worker::ImageWorker;
use runner::{Handler, Registry};

#[macro_export]
macro_rules! register_jobs {
    ($registry:expr, $($handler:expr),+ $(,)?) => {{
        $( $registry.add(Box::new($handler)); )+
    }};
}

fn main() {
    let mut registry = Registry::default();
    register_jobs!(registry, EmailWorker, ImageWorker);

    for (kind, payload) in [("email", "welcome:user-42"), ("image", "avatar.png")] {
        match registry.run(kind, payload, 2) {
            Ok(message) => println!("{message}"),
            Err(message) => eprintln!("{message}"),
        }
    }
}
