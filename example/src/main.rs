use minijinja::{context, Environment};

fn main() {
    let mut jinja = Environment::new();
    let _user = context! {
        first_name => "John",
        last_name => "Doe",
        email => "johndoe@example.com",
        phone_number => "(123) 456-7890",
        street => "123 Main St",
        city => "Dallas",
        header_info => "This is some information about the user.",
    };
    jinja.add_global("PROJECT_NAME", "Example");
}
