fn main() {
    embuild::espidf::sysenv::output();
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
}
