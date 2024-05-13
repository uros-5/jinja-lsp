from jinja2 import Environment


def main():
    jinja_env = Environment()

    template = jinja_env.get_template("account.jinja")
    template.render(
        first_name="John",
        last_name="Doe",
        email="johndoe@example.com",
        phone_number="(123) 456-7890",
        street="123 Main St",
        city="Dallas",
        header_info="This is some information about the user.",
    )

    jinja_env.globals["PROJECT_NAME"] = "example"
