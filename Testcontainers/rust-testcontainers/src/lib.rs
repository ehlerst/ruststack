use testcontainers::core::WaitFor;
use testcontainers::GenericImage;

pub const RUSTSTACK_IMAGE: &str = "ehlers320/ruststack";
pub const RUSTSTACK_TAG: &str = "latest";
pub const RUSTSTACK_PORT: u16 = 4566;

pub fn ruststack_image() -> GenericImage {
    GenericImage::new(RUSTSTACK_IMAGE, RUSTSTACK_TAG)
        .with_exposed_port(RUSTSTACK_PORT.into())
        .with_wait_for(WaitFor::message_on_stdout("RustStack gateway listening"))
}
